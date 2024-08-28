use std::f32::consts::PI;

pub fn generate_chirp(fc: f32, bandwidth: f32, fs: f32, n: usize) -> Vec<f32> {
    let f0 = bandwidth - fc / 2.0;
    let f1 = bandwidth + fc / 2.0;
    let duration = n as f32 / fs;
    (0..n)
        .map(|i| {
            let phase = ((2.0 * PI * (-f0 * f1 * duration)) / bandwidth)
                * f32::ln(1.0 - (bandwidth / (f1 * duration)) * ((i as f32) / fs));
            let win = 0.5 * (1.0 - f32::cos((2.0 * PI * (i as f32)) / (n as f32))); //Hann window
            f32::sin(phase) * win
        })
        .collect()
}

pub fn exampleChirp() -> Vec<f32> {
    generate_chirp(18000.0, 4000.0, 48000.0, 512)
}
