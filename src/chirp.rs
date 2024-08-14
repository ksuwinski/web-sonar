use std::f32::consts::PI;


pub(crate) fn generate_chirp(f0: f32, f1: f32, fs: f32, n_samples: usize) -> Vec<f32>{
    let duration = (n_samples as f32)/fs;
    (0..n_samples-1).map(|n|{
      f32::sin(
        2.0*PI * (-f0*f1*duration)/(f1 - f0)
        * f32::ln(1.0 - (f1 - f0)/(f1*duration) * ((n as f32)/fs))
      )
    }).collect()
}