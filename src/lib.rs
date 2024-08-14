use core::f32;
use std::sync::Arc;

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

#[wasm_bindgen]
pub struct Sonar {
    impulse: Vec<f32>,
    impulse_fft: Vec<Complex32>,
    input_buffer: Vec<f32>,
    fast_time_rfft_plan: Arc<dyn RealToComplex<f32>>,
    fast_time_fft_plan: Arc<dyn Fft<f32>>,
    fast_time_ifft_plan: Arc<dyn Fft<f32>>,
    data_cube: Array2<Complex32>,
    fft_scratch: Vec<Complex32>,
    impulse_counter: usize,
    negative_carrier: Vec<Complex32>,
    decimation: usize,
}

#[wasm_bindgen]
impl Sonar {
    pub fn new(impulse: &[f32], normalized_f_carrier: f32) -> Self {
        console_error_panic_hook::set_once();

        let decimation = 8;

        assert!(
            impulse.len() % CHUNK_SIZE == 0,
            "length of impulse must be a multiple of 128"
        );
        let mut real_planner = RealFftPlanner::new();
        let fast_time_rfft_plan = real_planner.plan_fft_forward(impulse.len());

        let mut complex_planner = FftPlanner::new();
        let fast_time_fft_plan = complex_planner.plan_fft_forward(impulse.len());
        let fast_time_ifft_plan = complex_planner.plan_fft_inverse(impulse.len());

        let mut impulse_fft = fast_time_rfft_plan.make_output_vec();
        fast_time_rfft_plan
            .process(&mut impulse.to_vec(), &mut impulse_fft)
            .unwrap();

        let scratch_size = fast_time_fft_plan
            .get_inplace_scratch_len()
            .max(fast_time_rfft_plan.get_scratch_len())
            .max(impulse.len());

        let negative_carrier = (0..impulse.len())
            .map(|n| Complex32::cis(-2.0 * f32::consts::PI * (n as f32) * normalized_f_carrier))
            .collect();

        let n_fast = impulse.len() / decimation;

        Sonar {
            impulse: impulse.to_vec(),
            impulse_fft,
            input_buffer: Vec::with_capacity(impulse.len()),
            fft_scratch: vec![Complex32::zero(); scratch_size],
            fast_time_rfft_plan,
            fast_time_fft_plan,
            fast_time_ifft_plan,
            data_cube: Array2::zeros((10, n_fast)),
            impulse_counter: 0,
            negative_carrier,
            decimation,
        }
    }
    pub fn handle_input(&mut self, samples: &[f32]) {
        assert!(
            samples.len() == CHUNK_SIZE,
            "expected a chunk of 128 samples"
        );

        if self.input_buffer.len() < self.impulse.len() {
            self.input_buffer.extend_from_slice(samples)
        } else {
            self.handle_impulse();
            self.input_buffer.clear();
        }
    }
    pub fn clutter(&self) -> Vec<f32> {
        self.data_cube
            .slice(s![0, ..])
            // .map(|x| 20.0 * x.abs().log10())
            .map(|x| x.abs())
            .to_vec()
    }

    fn handle_impulse(&mut self) {
        let mut input_fft = vec![Complex32::zero(); self.impulse.len()];
        self.fast_time_rfft_plan
            .process_with_scratch(
                &mut self.input_buffer,
                &mut input_fft[..257],
                &mut self.fft_scratch,
            )
            .unwrap();

        for (input_bin, impulse_bin) in std::iter::zip(&mut input_fft, &self.impulse_fft) {
            *input_bin *= impulse_bin.conj();
        }
        let mut xcorr_fft = input_fft;

        self.fast_time_ifft_plan
            .process_with_scratch(&mut xcorr_fft, &mut self.fft_scratch);
        let mut xcorr = xcorr_fft;
        for ((x_in, x_cis), out) in xcorr
            .iter()
            .step_by(self.decimation)
            .zip(self.negative_carrier.iter().step_by(self.decimation))
            .zip(self.data_cube.slice_mut(s![0, ..]))
        {
            *out = x_in * x_cis;
        }
        // self.fast_time_fft_plan
        //     .process_with_scratch(&mut xcorr, &mut self.fft_scratch);

        // self.save_fast_time_data(&xcorr);
    }

    // fn save_fast_time_data(&mut self, xcorr: &[Complex32]) {
    //     self.data_cube
    //         .slice_mut(s![0, ..])
    //         .into_slice()
    //         .expect("data cube strides are wrong")
    //         .clone_from_slice(&xcorr);
    // }
}
