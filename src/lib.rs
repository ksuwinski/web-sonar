use core::f32;
use ndarray::Array2;
use realfft::RealFftPlanner;
use rustfft::{
    num_complex::{Complex, Complex32, ComplexFloat},
    FftPlanner,
};
use wasm_bindgen::prelude::*;

#[allow(unused_imports)]
use log::debug;

mod clutterfilter;
mod doppler_processing;
mod matchedfilter;
use clutterfilter::{
    ClutterFilter, ClutterMap, LeakyIntegratorFilter, TwoPulseCanceller, DUMMY_FILTER,
};
use doppler_processing::{RangeDopplerProcessor, SLOW_TIME_AXIS};
use matchedfilter::MatchedFilter;

#[wasm_bindgen]
#[derive(PartialEq, Clone, Copy)]
pub enum ClutterFilterOption {
    None,
    RemoveZero,
    TwoPulse,
    Slow,
}

struct InputBuffer {
    buffer: Vec<f32>,
    output_length: usize,
}
impl InputBuffer {
    fn new(output_length: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(output_length),
            output_length,
        }
    }
    fn handle_input(&mut self, samples: &[f32]) -> Option<&mut [f32]> {
        if self.buffer.len() >= self.output_length {
            self.buffer.copy_within(self.output_length.., 0);
            self.buffer.truncate(self.buffer.len() - self.output_length)
        }

        assert!(self.buffer.len() <= self.output_length);
        self.buffer.extend_from_slice(samples);

        if self.buffer.len() < self.output_length {
            return None;
        } else {
            return Some(&mut self.buffer[..self.output_length]);
        }
    }
}

/*
    input_buffer ----> matched_filter -----+----> clutter_filter ----> range_doppler ----> output
                                           |                              ^
                                           |                              | (argmax)
                                           +-------> clutter_map ---------+
*/
#[wasm_bindgen]
pub struct Sonar {
    input_buffer: InputBuffer,
    range_doppler_output: Array2<Complex32>,
    range_doppler: RangeDopplerProcessor,
    matched_filter: MatchedFilter,
    clutter_filter: Box<dyn ClutterFilter>,
    clutter_map: ClutterMap,

    /// if true, fast time axis is circularly shifted to position the direct path at 0
    track_offset: bool,
    remove_zero: bool,
}
#[wasm_bindgen]
impl Sonar {
    pub fn new(
        impulse: &[f32],
        normalized_f_carrier: f32,
        decimation: usize,
        slow_time_window: &[f32],
        clutter_alpha: f32,
        filter_option: ClutterFilterOption,
        track_offset: bool,
    ) -> Self {
        console_log::init_with_level(log::Level::Debug).unwrap();
        console_error_panic_hook::set_once();

        let mut real_planner = RealFftPlanner::new();
        let mut complex_planner = FftPlanner::new();

        let n_slow = slow_time_window.len();
        let n_fast = impulse.len().div_ceil(decimation);

        Sonar {
            input_buffer: InputBuffer::new(impulse.len()),
            range_doppler_output: Array2::zeros((n_slow, n_fast)),
            range_doppler: RangeDopplerProcessor::new(
                slow_time_window,
                n_fast,
                &mut complex_planner,
            ),
            matched_filter: MatchedFilter::new(
                impulse,
                normalized_f_carrier,
                decimation,
                &mut complex_planner,
                &mut real_planner,
            ),
            clutter_filter: Self::create_filter(n_fast, filter_option, clutter_alpha),
            clutter_map: ClutterMap::new(n_fast, clutter_alpha),
            remove_zero: filter_option == ClutterFilterOption::RemoveZero,
            track_offset,
        }
    }
    fn create_filter(
        n_fast: usize,
        filter_option: ClutterFilterOption,
        alpha: f32,
    ) -> Box<dyn ClutterFilter> {
        match filter_option {
            ClutterFilterOption::None => Box::new(DUMMY_FILTER),
            ClutterFilterOption::RemoveZero => Box::new(DUMMY_FILTER),
            ClutterFilterOption::TwoPulse => Box::new(TwoPulseCanceller::new(n_fast)),
            ClutterFilterOption::Slow => Box::new(LeakyIntegratorFilter::new(n_fast, alpha)),
        }
    }
    pub fn handle_input(&mut self, samples: &[f32]) -> bool {
        if let Some(input) = self.input_buffer.handle_input(samples) {
            let xcorr_output = self.range_doppler.input_buffer();
            self.matched_filter.handle_impulse(input, xcorr_output);
            self.clutter_map.process(xcorr_output);
            self.clutter_filter.process_inplace(xcorr_output);
            self.range_doppler.next_impulse();

            if self.track_offset {
                self.range_doppler
                    .set_fast_time_shift(self.clutter_map.argmax());
            }
            true
        } else {
            false
        }
    }
    // pub fn clutter(&self) -> Vec<f32> {
    //     self.range_doppler.clutter.iter().map(|x| x.abs()).collect()
    // }
    pub fn get_data_cube(&mut self) -> Vec<f32> {
        self.range_doppler
            .range_doppler(&mut self.range_doppler_output);

        if self.remove_zero {
            for x in self
                .range_doppler_output
                .index_axis_mut(SLOW_TIME_AXIS, self.range_doppler.n_slow() / 2)
            {
                *x = Complex::ZERO;
            }
        }
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
    fn test_input_buffer() {
        let mut buffer = InputBuffer::new(512);
        let chunk1: Vec<f32> = (0..128).map(|x| (x * 10 + 1) as f32).collect();
        let chunk2: Vec<f32> = (0..128).map(|x| (x * 10 + 2) as f32).collect();
        let chunk3: Vec<f32> = (0..(256 + 10)).map(|x| (x * 10 + 3) as f32).collect();
        let chunk4: Vec<f32> = (0..128).map(|x| (x * 10 + 4) as f32).collect();
        let chunk5: Vec<f32> = (0..128 * 3).map(|x| (x * 10 + 5) as f32).collect();

        let r1 = buffer.handle_input(&chunk1);
        assert_eq!(r1, None);
        let r2 = buffer.handle_input(&chunk2);
        assert_eq!(r2, None);

        let r3 = buffer.handle_input(&chunk3);
        let full_buffer = r3.unwrap();
        assert_eq!(full_buffer.len(), 512);
        assert_eq!(full_buffer[0..128], chunk1);
        assert_eq!(full_buffer[128..256], chunk2);
        assert_eq!(full_buffer[256..512], chunk3[0..256]);
        assert_eq!(buffer.buffer.len(), 512 + 10);

        let r4 = buffer.handle_input(&chunk4);
        assert_eq!(r4, None);
        let r5 = buffer.handle_input(&chunk5);
        let full_buffer = r5.unwrap();
        assert_eq!(full_buffer.len(), 512);
        assert_eq!(full_buffer[0..10], chunk3[256..256 + 10]);
        assert_eq!(full_buffer[10..128 + 10], chunk4);
        assert_eq!(full_buffer[128 + 10..512], chunk5[0..128 * 3 - 10]);
    }
}
