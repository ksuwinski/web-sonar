use std::{iter::zip, sync::Arc};

use itertools::zip_eq;
use ndarray::{s, Array2, ArrayBase, Axis, Ix1, Slice};
use rustfft::{num_complex::Complex32, Fft, FftPlanner};

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

pub const SLOW_TIME_AXIS: Axis = Axis(0);
pub const FAST_TIME_AXIS: Axis = Axis(1);
pub struct RangeDopplerProcessor {
    impulse_counter: usize,
    fast_time_shift: usize,
    data_cube: Array2<Complex32>,
    fft_buffer: Vec<Complex32>,
    fft_scratch: Vec<Complex32>,
    slow_time_fft_plan: Arc<dyn Fft<f32>>,
}
impl RangeDopplerProcessor {
    pub fn new(n_slow: usize, n_fast: usize, fft_planner: &mut FftPlanner<f32>) -> Self {
        let slow_time_fft_plan = fft_planner.plan_fft_forward(n_slow);
        Self {
            impulse_counter: 0,
            fast_time_shift: 0,
            data_cube: Array2::zeros((n_slow, n_fast)),
            fft_buffer: vec![Complex32::ZERO; n_slow],
            fft_scratch: vec![Complex32::ZERO; slow_time_fft_plan.get_inplace_scratch_len()],
            slow_time_fft_plan,
        }
    }

    pub fn input_buffer(&mut self) -> &mut [Complex32] {
        self.data_cube
            .index_axis_mut(SLOW_TIME_AXIS, self.impulse_counter)
            .into_slice()
            .unwrap()
    }

    pub fn next_impulse(&mut self) {
        self.impulse_counter += 1;
        if self.impulse_counter == self.data_cube.shape()[0] {
            self.impulse_counter = 0;
        }
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
            let n_slow = self.n_slow();
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
    pub fn n_slow(&self) -> usize {
        self.data_cube.len_of(SLOW_TIME_AXIS)
    }
    pub fn range_doppler(&mut self, output: &mut Array2<Complex32>) {
        self.range_doppler_slice(
            output,
            Slice::from(self.fast_time_shift..),
            Slice::from(..self.n_fast() - self.fast_time_shift),
        );
        self.range_doppler_slice(
            output,
            Slice::from(..self.fast_time_shift),
            Slice::from(self.n_fast() - self.fast_time_shift..),
        );
    }
    pub fn n_fast(&self) -> usize {
        return self.data_cube.shape()[FAST_TIME_AXIS.0];
    }

    pub fn set_fast_time_shift(&mut self, shift: usize) {
        self.fast_time_shift = shift;
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
}
