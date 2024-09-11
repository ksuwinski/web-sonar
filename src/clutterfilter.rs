use itertools::zip_eq;
use rustfft::num_complex::{Complex32, ComplexFloat};

pub struct ClutterMap {
    cluttermap: Vec<Complex32>,
    alpha: f32,
}
impl ClutterMap {
    pub fn new(n_fast: usize, alpha: f32) -> Self {
        Self {
            cluttermap: vec![Complex32::ZERO; n_fast],
            alpha,
        }
    }

    pub fn process(&mut self, impulse: &[Complex32]) {
        for (x_impulse, x_map) in zip_eq(impulse, &mut self.cluttermap) {
            *x_map = (1.0 - self.alpha) * (*x_map) + self.alpha * (*x_impulse);
        }
    }

    pub fn argmax(&self) -> usize {
        let mut max_value = -f32::INFINITY;
        let mut max_n = 0;
        for (n, x) in self.cluttermap.iter().enumerate() {
            let x_abs = x.abs();
            if x_abs > max_value {
                max_value = x_abs;
                max_n = n;
            }
        }
        max_n
    }
}

pub trait ClutterFilter {
    fn process_inplace(&mut self, impulse: &mut [Complex32]);
}

#[derive(Clone, Copy)]
pub struct DummyFilter {}
pub const DUMMY_FILTER: DummyFilter = DummyFilter {};
impl ClutterFilter for DummyFilter {
    fn process_inplace(&mut self, _impulse: &mut [Complex32]) {}
}

pub struct TwoPulseCanceller {
    prev: Vec<Complex32>,
}
impl TwoPulseCanceller {
    pub fn new(n_fast: usize) -> Self {
        Self {
            prev: vec![Complex32::ZERO; n_fast],
        }
    }
}
impl ClutterFilter for TwoPulseCanceller {
    fn process_inplace(&mut self, impulse: &mut [Complex32]) {
        for (x_impulse, x_prev) in zip_eq(impulse, &mut self.prev) {
            let tmp = *x_impulse;
            *x_impulse -= *x_prev;
            *x_prev = tmp;
        }
    }
}
pub struct LeakyIntegratorFilter {
    cluttermap: Vec<Complex32>,
    alpha: f32,
}
impl LeakyIntegratorFilter {
    pub fn new(n_fast: usize, alpha: f32) -> Self {
        Self {
            cluttermap: vec![Complex32::ZERO; n_fast],
            alpha,
        }
    }
}
impl ClutterFilter for LeakyIntegratorFilter {
    fn process_inplace(&mut self, impulse: &mut [Complex32]) {
        for (x_impulse, x_map) in zip_eq(impulse, &mut self.cluttermap) {
            *x_map = (1.0 - self.alpha) * (*x_map) + self.alpha * (*x_impulse);
            *x_impulse -= *x_map;
        }
    }
}

// pub struct ThreePulseCanceller {
//     tpc1: TwoPulseCanceller,
//     tpc2: TwoPulseCanceller,
// }
// impl ThreePulseCanceller {
//     pub fn new(n_fast: usize) -> Self {
//         Self {
//             tpc1: TwoPulseCanceller::new(n_fast),
//             tpc2: TwoPulseCanceller::new(n_fast),
//         }
//     }
// }
// impl ClutterFilter for ThreePulseCanceller {
//     fn process_inplace(&mut self, impulse: &mut [Complex32]) {
//         self.tpc1.process_inplace(impulse);
//         self.tpc2.process_inplace(impulse);
//     }
// }

// pub struct IirCanceller {
//     x1: Vec<Complex32>,
//     x2: Vec<Complex32>,
//     x3: Vec<Complex32>,
//     y1: Vec<Complex32>,
//     y2: Vec<Complex32>,
//     y3: Vec<Complex32>,
// }
// impl IirCanceller {
//     pub fn new(n_fast: usize) -> Self {
//         Self {
//             x1: vec![Complex32::ZERO; n_fast],
//             x2: vec![Complex32::ZERO; n_fast],
//             x3: vec![Complex32::ZERO; n_fast],
//             y1: vec![Complex32::ZERO; n_fast],
//             y2: vec![Complex32::ZERO; n_fast],
//             y3: vec![Complex32::ZERO; n_fast],
//         }
//     }
// }
