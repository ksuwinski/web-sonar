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

//TODO: don't assume this
const CHUNK_SIZE: usize = 128;

#[wasm_bindgen]
#[derive(PartialEq, Clone, Copy)]
pub enum ClutterFilterOption {
    None,
    RemoveZero,
    TwoPulse,
    Slow,
}

/*
    input ----> matched_filter -----+----> clutter_filter ----> range_doppler ----> output
                                    |                              ^
                                    |                              | (argmax)
                                    +-------> clutter_map ---------+
*/
#[wasm_bindgen]
pub struct Sonar {
    input_buffer: Vec<f32>,
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
        n_slow: usize,
        clutter_alpha: f32,
        filter_option: ClutterFilterOption,
        track_offset: bool,
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
            clutter_filter: Self::create_filter(n_fast, filter_option),
            clutter_map: ClutterMap::new(n_fast, clutter_alpha),
            remove_zero: filter_option == ClutterFilterOption::RemoveZero,
            track_offset,
        }
    }
    fn create_filter(n_fast: usize, filter_option: ClutterFilterOption) -> Box<dyn ClutterFilter> {
        match filter_option {
            ClutterFilterOption::None => Box::new(DUMMY_FILTER),
            ClutterFilterOption::RemoveZero => Box::new(DUMMY_FILTER),
            ClutterFilterOption::TwoPulse => Box::new(TwoPulseCanceller::new(n_fast)),
            ClutterFilterOption::Slow => Box::new(LeakyIntegratorFilter::new(n_fast, 0.05)),
        }
    }
    // pub fn update_params(&mut self, clutter_alpha: f32) {}
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

        let xcorr_output = self.range_doppler.input_buffer();
        self.matched_filter
            .handle_impulse(self.input_buffer.as_mut_slice(), xcorr_output);
        self.clutter_map.process(xcorr_output);
        self.clutter_filter.process_inplace(xcorr_output);
        self.range_doppler.next_impulse();

        if self.track_offset {
            self.range_doppler
                .set_fast_time_shift(self.clutter_map.argmax());
        }
        self.input_buffer.clear();
        true
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
        let mut sonar = Sonar::new(&[0.0; 512], 0.1, 2, 1, 0.0, ClutterFilterOption::None);
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
