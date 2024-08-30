use core::f32;
use std::sync::Arc;

use itertools::zip_eq;
use log::debug;
use ndarray::{s, Array2};
use realfft::{RealFftPlanner, RealToComplex};
use rustfft::{
    num_complex::{Complex32, ComplexFloat},
    num_traits::Zero,
    Fft, FftPlanner,
};
use wasm_bindgen::prelude::*;

//TODO: don't assume this
const CHUNK_SIZE: usize = 128;

struct RangeDopplerProcessor {
    impulse_counter: usize,
    clutter: Vec<Complex32>,
    clutter_alpha: f32,
    data_cube: Array2<Complex32>,
}
impl RangeDopplerProcessor {
    fn new(n_slow: usize, n_fast: usize) -> Self {
        Self {
            impulse_counter: 0,
            data_cube: Array2::zeros((n_slow, n_fast)),
            clutter: vec![Complex32::ZERO; n_fast],
            clutter_alpha: 0.9,
        }
    }

    // // does this get inlined? or is the iterator passed around?
    // fn add_impulse_from_iter<A: Iterator<Item = Complex32>>(&mut self, impulse_iter: A) {
    //     for ((x_row, x_impulse), x_clutter) in self
    //         .data_cube
    //         .slice_mut(s![self.impulse_counter, ..])
    //         .iter_mut()
    //         .zip(impulse_iter)
    //         .zip(self.clutter.iter_mut())
    //     {
    //         *x_row = x_impulse;
    //         *x_clutter = (1.0 - self.clutter_alpha) * (*x_clutter) + self.clutter_alpha * x_impulse;
    //     }

    //     if self.impulse_counter < self.data_cube.shape()[0] - 1 {
    //         self.impulse_counter += 1;
    //     } else {
    //         self.impulse_counter = 0;
    //     }
    // }

    fn input_buffer(&mut self) -> &mut [Complex32] {
        self.data_cube
            .slice_mut(s![self.impulse_counter, ..])
            .into_slice()
            .unwrap()
    }

    fn process_input(&mut self) {
        let recent_impulse = self.data_cube.slice(s![self.impulse_counter, ..]);
        for (x_impulse, x_clutter) in zip_eq(recent_impulse, &mut self.clutter) {
            *x_clutter = (1.0 - self.clutter_alpha) * (*x_clutter) + self.clutter_alpha * x_impulse;
        }

        self.impulse_counter += 1;
        if self.impulse_counter == self.data_cube.shape()[0] {
            self.impulse_counter = 0;
        }
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

        return Self {
            input_length: impulse.len(),
            impulse_fft,
            fft_scratch: vec![Complex32::zero(); scratch_size],
            scratch2: vec![Complex32::zero(); impulse.len()],
            fast_time_rfft_plan,
            fast_time_ifft_plan,
            negative_carrier,
            decimation,
        };
    }

    fn handle_impulse(&mut self, input_buffer: &mut [f32], output_buffer: &mut [Complex32]) {
        self.scratch2.fill(Complex32::ZERO);
        self.fast_time_rfft_plan
            .process_with_scratch(
                input_buffer,
                &mut self.scratch2[..257],
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
    output_buf_2d: Array2<f32>,
    range_doppler: RangeDopplerProcessor,
    matched_filter: MatchedFilter,
}
#[wasm_bindgen]
impl Sonar {
    pub fn new(impulse: &[f32], normalized_f_carrier: f32) -> Self {
        console_log::init_with_level(log::Level::Debug).unwrap();
        console_error_panic_hook::set_once();
        let decimation = 8;

        let mut real_planner = RealFftPlanner::new();
        let mut complex_planner = FftPlanner::new();

        assert!(
            impulse.len() % CHUNK_SIZE == 0,
            "length of impulse must be a multiple of 128"
        );

        let n_fast = impulse.len() / decimation;
        let n_slow = 20;

        Sonar {
            input_buffer: Vec::with_capacity(impulse.len()),
            output_buf_2d: Array2::zeros((n_fast, n_slow)),
            range_doppler: RangeDopplerProcessor::new(n_slow, n_fast),
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
        self.range_doppler.process_input();
        self.input_buffer.clear();
        return true;
    }
    pub fn clutter(&self) -> Vec<f32> {
        self.range_doppler.clutter.iter().map(|x| x.abs()).collect()
    }
    pub fn get_data_cube(&self) -> Vec<f32> {
        self.range_doppler
            .data_cube
            .as_slice()
            .unwrap()
            .into_iter()
            .map(|x| x.abs())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_buffer() {
        let mut sonar = Sonar::new(&[0.0; 512], 0.1);
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
