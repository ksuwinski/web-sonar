use itertools::zip_eq;
use realfft::{RealFftPlanner, RealToComplex};
use rustfft::{num_complex::Complex32, num_traits::Zero, Fft, FftPlanner};
use std::sync::Arc;

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
    pub fn new(
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
            .map(|n| {
                Complex32::cis(-2.0 * std::f32::consts::PI * (n as f32) * normalized_f_carrier)
            })
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

    pub fn handle_impulse(&mut self, input_buffer: &mut [f32], output_buffer: &mut [Complex32]) {
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

    pub fn input_length(&self) -> usize {
        self.input_length
    }

    // fn output_length(&self) -> usize {
    //     self.impulse_fft.len() / self.decimation
    // }
}
