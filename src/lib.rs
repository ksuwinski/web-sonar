use core::f32;
use std::{iter::zip, sync::Arc};

use itertools::zip_eq;
use ndarray::{s, Array2, ArrayBase, Axis, Ix1, Slice};
use realfft::{RealFftPlanner, RealToComplex};
use rustfft::{
    num_complex::{Complex32, ComplexFloat},
    num_traits::Zero,
    Fft, FftPlanner,
};
use wasm_bindgen::prelude::*;

#[allow(unused_imports)]
use log::debug;

trait CopyInto<A> {
    fn copy_into(&self, destination: A);
}
impl<A, D> CopyInto<&mut [A]> for ArrayBase<D, Ix1>
where
    A: Copy,
    D: ndarray::Data<Elem = A>,
{
    fn copy_into(&self, destination: &mut [A]) {
        // clone_into_iter(self.into_iter().copied(), destination);
        for (dst, x) in zip_eq(destination, self) {
            *dst = *x;
        }
    }
}
// impl<A, D> CopyInto<ArrayBase<D, Ix1>> for Vec<A>
// where
//     A: Copy,
//     D: ndarray::DataMut<Elem = A>,
// {
//     fn copy_into(&self, destination: &mut ArrayBase<D, Ix1>) {
//         for (dst, x) in zip_eq(destination.iter_mut(), self) {
//             *dst = *x;
//         }
//     }
// }

// fn clone_into_iter<'a, A, I1, I2>(src: I1, dst: I2)
// where
//     A: 'a,
//     I1: IntoIterator<Item = A>,
//     I2: IntoIterator<Item = &'a mut A>,
// {
//     for x in zip_eq(dst.into_iter(), src.into_iter()) {
//         let (dst, src): (&mut A, A) = x;
//         *dst = src;
//     }
// }

#[inline]
fn fftshift_into<'a, A: 'a + Copy, I: IntoIterator<Item = &'a mut A>>(input: &[A], output: I) {
    let mut output = output.into_iter();
    let n = input.len();
    let split_idx = if n % 2 == 0 { n / 2 } else { n / 2 + 1 };
    for (x, out) in zip(&input[split_idx..], &mut output) {
        *out = *x
    }
    for (x, out) in zip_eq(&input[..split_idx], &mut output) {
        *out = *x
    }
}

//TODO: don't assume this
const CHUNK_SIZE: usize = 128;

const SLOW_TIME_AXIS: Axis = Axis(0);
const FAST_TIME_AXIS: Axis = Axis(1);
struct RangeDopplerProcessor {
    impulse_counter: usize,
    clutter: Vec<Complex32>,
    clutter_alpha: f32,
    data_cube: Array2<Complex32>,
    fft_buffer: Vec<Complex32>,
    fft_scratch: Vec<Complex32>,
    slow_time_fft_plan: Arc<dyn Fft<f32>>,
}
impl RangeDopplerProcessor {
    fn new(n_slow: usize, n_fast: usize, fft_planner: &mut FftPlanner<f32>) -> Self {
        let slow_time_fft_plan = fft_planner.plan_fft_forward(n_slow);
        Self {
            impulse_counter: 0,
            data_cube: Array2::zeros((n_slow, n_fast)),
            clutter: vec![Complex32::ZERO; n_fast],
            fft_buffer: vec![Complex32::ZERO; n_slow],
            fft_scratch: vec![Complex32::ZERO; slow_time_fft_plan.get_inplace_scratch_len()],
            clutter_alpha: 0.01,
            slow_time_fft_plan,
        }
    }

    fn input_buffer(&mut self) -> &mut [Complex32] {
        self.data_cube
            .index_axis_mut(SLOW_TIME_AXIS, self.impulse_counter)
            .into_slice()
            .unwrap()
    }

    fn next_impulse(&mut self) {
        let mut recent_impulse = self
            .data_cube
            .index_axis_mut(SLOW_TIME_AXIS, self.impulse_counter);

        for (x_impulse, x_clutter) in zip_eq(&mut recent_impulse, &mut self.clutter) {
            *x_clutter =
                (1.0 - self.clutter_alpha) * (*x_clutter) + self.clutter_alpha * *x_impulse;
            *x_impulse -= *x_clutter;
        }

        self.impulse_counter += 1;
        if self.impulse_counter == self.data_cube.shape()[0] {
            self.impulse_counter = 0;
        }
    }

    fn get_fast_time_offset(&self) -> usize {
        let mut max_value = -f32::INFINITY;
        let mut max_n = 0;
        for (n, x) in self.clutter.iter().enumerate() {
            let x_abs = x.abs();
            if x_abs > max_value {
                max_value = x_abs;
                max_n = n;
            }
        }
        max_n
    }

    fn range_doppler_slice(
        &mut self,
        output: &mut Array2<Complex32>,
        in_slice: Slice,
        out_slice: Slice,
    ) {
        for (slow_time_slice, mut output_slice) in zip_eq(
            self.data_cube
                .slice_axis(FAST_TIME_AXIS, in_slice)
                .axis_iter(FAST_TIME_AXIS),
            output
                .slice_axis_mut(FAST_TIME_AXIS, out_slice)
                .axis_iter_mut(FAST_TIME_AXIS),
        ) {
            let n_slow = self.data_cube.len_of(SLOW_TIME_AXIS);
            slow_time_slice
                .slice(s![self.impulse_counter..])
                .copy_into(&mut self.fft_buffer[..n_slow - self.impulse_counter]);
            slow_time_slice
                .slice(s![..self.impulse_counter])
                .copy_into(&mut self.fft_buffer[n_slow - self.impulse_counter..]);
            self.slow_time_fft_plan
                .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);
            fftshift_into(&self.fft_buffer, output_slice.iter_mut());
        }
    }
    fn range_doppler(&mut self, output: &mut Array2<Complex32>) {
        let offset = self.get_fast_time_offset();
        self.range_doppler_slice(
            output,
            Slice::from(offset..),
            Slice::from(..self.n_fast() - offset),
        );
        self.range_doppler_slice(
            output,
            Slice::from(..offset),
            Slice::from(self.n_fast() - offset..),
        );
    }
    fn n_fast(&self) -> usize {
        return self.data_cube.shape()[FAST_TIME_AXIS.0];
    }
}
pub struct MatchedFilter {
    input_length: usize,
    decimation: usize,
    impulse_fft: Vec<Complex32>,
    fast_time_rfft_plan: Arc<dyn RealToComplex<f32>>,
    fast_time_ifft_plan: Arc<dyn Fft<f32>>,
    fft_scratch: Vec<Complex32>,
    scratch2: Vec<Complex32>,
    negative_carrier: Vec<Complex32>,
}

impl MatchedFilter {
    fn new(
        impulse: &[f32],
        normalized_f_carrier: f32,
        decimation: usize,
        complex_planner: &mut FftPlanner<f32>,
        real_planner: &mut RealFftPlanner<f32>,
    ) -> Self {
        let fast_time_rfft_plan = real_planner.plan_fft_forward(impulse.len());

        // let fast_time_fft_plan = complex_planner.plan_fft_forward(impulse.len());
        let fast_time_ifft_plan = complex_planner.plan_fft_inverse(impulse.len());

        let mut impulse_fft = fast_time_rfft_plan.make_output_vec();
        fast_time_rfft_plan
            .process(&mut impulse.to_vec(), &mut impulse_fft)
            .unwrap();

        let scratch_size = fast_time_ifft_plan
            .get_inplace_scratch_len()
            .max(fast_time_rfft_plan.get_scratch_len())
            .max(impulse.len());

        let negative_carrier = (0..impulse.len())
            .map(|n| Complex32::cis(-2.0 * f32::consts::PI * (n as f32) * normalized_f_carrier))
            .collect();

        Self {
            input_length: impulse.len(),
            impulse_fft,
            fft_scratch: vec![Complex32::zero(); scratch_size],
            scratch2: vec![Complex32::zero(); impulse.len()],
            fast_time_rfft_plan,
            fast_time_ifft_plan,
            negative_carrier,
            decimation,
        }
    }

    fn handle_impulse(&mut self, input_buffer: &mut [f32], output_buffer: &mut [Complex32]) {
        self.scratch2.fill(Complex32::ZERO);
        self.fast_time_rfft_plan
            .process_with_scratch(
                input_buffer,
                &mut self.scratch2[..self.fast_time_rfft_plan.complex_len()],
                &mut self.fft_scratch,
            )
            .unwrap();
        //scratch2 now contains fft with negative frequencies removed

        for (input_bin, impulse_bin) in std::iter::zip(&mut self.scratch2, &self.impulse_fft) {
            *input_bin *= impulse_bin.conj();
        }
        //scratch2 now contains fft of xcorr

        self.fast_time_ifft_plan
            .process_with_scratch(&mut self.scratch2, &mut self.fft_scratch);
        //scratch2 now contains xcorr

        let decimated_xcorr_iter = zip_eq(
            self.scratch2.iter().step_by(self.decimation),
            self.negative_carrier.iter().step_by(self.decimation),
        )
        .map(|(x_xcorr, x_cis)| x_xcorr * x_cis);

        for (xc, out) in zip_eq(decimated_xcorr_iter, output_buffer) {
            *out = xc;
        }

        // self.range_doppler
        //     .add_impulse_from_iter(decimated_xcorr_iter);
    }

    fn input_length(&self) -> usize {
        self.input_length
    }
    // fn output_length(&self) -> usize {
    //     self.impulse_fft.len() / self.decimation
    // }
}

#[wasm_bindgen]
pub struct Sonar {
    input_buffer: Vec<f32>,
    range_doppler_output: Array2<Complex32>,
    range_doppler: RangeDopplerProcessor,
    matched_filter: MatchedFilter,
}
#[wasm_bindgen]
impl Sonar {
    pub fn new(
        impulse: &[f32],
        normalized_f_carrier: f32,
        decimation: usize,
        n_slow: usize,
    ) -> Self {
        console_log::init_with_level(log::Level::Debug).unwrap();
        console_error_panic_hook::set_once();

        let mut real_planner = RealFftPlanner::new();
        let mut complex_planner = FftPlanner::new();

        assert!(
            impulse.len() % CHUNK_SIZE == 0,
            "length of impulse must be a multiple of 128"
        );

        let n_fast = impulse.len().div_ceil(decimation);

        Sonar {
            input_buffer: Vec::with_capacity(impulse.len()),
            range_doppler_output: Array2::zeros((n_slow, n_fast)),
            range_doppler: RangeDopplerProcessor::new(n_slow, n_fast, &mut complex_planner),
            matched_filter: MatchedFilter::new(
                impulse,
                normalized_f_carrier,
                decimation,
                &mut complex_planner,
                &mut real_planner,
            ),
        }
    }
    pub fn handle_input(&mut self, samples: &[f32]) -> bool {
        assert!(
            samples.len() % CHUNK_SIZE == 0,
            "expected a multiple of 128 samples"
        );

        self.input_buffer.extend_from_slice(samples);

        // currently we require everything to be nicely aligned
        assert!(self.input_buffer.len() <= self.matched_filter.input_length());

        if self.input_buffer.len() < self.matched_filter.input_length() {
            return false;
        }
        self.matched_filter.handle_impulse(
            self.input_buffer.as_mut_slice(),
            self.range_doppler.input_buffer(),
        );
        self.range_doppler.next_impulse();
        self.input_buffer.clear();
        true
    }
    pub fn clutter(&self) -> Vec<f32> {
        self.range_doppler.clutter.iter().map(|x| x.abs()).collect()
    }
    pub fn get_data_cube(&mut self) -> Vec<f32> {
        self.range_doppler
            .range_doppler(&mut self.range_doppler_output);
        self.range_doppler_output.iter().map(|x| x.abs()).collect()
        // self.range_doppler
        //     .data_cube
        //     .as_slice()
        //     .unwrap()
        //     .into_iter()
        //     .map(|x| x.abs())
        //     .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fftshift_even() {
        let a = [1, 2, 3, 4, 5, 6];
        let mut b = [0; 6];
        fftshift_into(&a, &mut b);
        assert_eq!(b, [4, 5, 6, 1, 2, 3]);
    }
    #[test]
    fn test_fftshift_odd() {
        let a = [1, 2, 3, 4, 5, 6, 7];
        let mut b = [0; 7];
        fftshift_into(&a, &mut b);
        assert_eq!(b, [5, 6, 7, 1, 2, 3, 4]);
    }

    #[test]
    fn test_input_buffer() {
        let mut sonar = Sonar::new(&[0.0; 512], 0.1, 2, 1);
        let chunk1: Vec<f32> = (0..128).map(|x| x as f32).collect();
        let chunk2: Vec<f32> = (0..128).map(|x| (x * x) as f32).collect();

        // adding new chunks should extend the buffer
        sonar.handle_input(&chunk1);
        assert_eq!(sonar.input_buffer.len(), 128);
        sonar.handle_input(&chunk2);
        assert_eq!(sonar.input_buffer.len(), 256);

        // the chunks should be present in the buffer
        assert_eq!(&chunk1, &sonar.input_buffer[0..128]);
        assert_eq!(&chunk2, &sonar.input_buffer[128..256]);

        // fill the rest of the buffer
        sonar.handle_input(&chunk1);
        sonar.handle_input(&chunk2);

        // now the full buffer should be cleared
        assert_eq!(sonar.input_buffer.len(), 0);

        // now we're filling again from the start
        let chunk3: Vec<f32> = (0..128).map(|x| (x * x * x) as f32).collect();
        sonar.handle_input(&chunk3);
        assert_eq!(&chunk3, &sonar.input_buffer[0..128]);
    }
}
