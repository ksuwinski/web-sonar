export function hannWindow(n) {
  const window = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    window[i] = 0.5 * (1 - Math.cos((2 * Math.PI * i) / n));
  }
  return window;
}

export function rectWindow(n) {
  const window = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    window[i] = 1;
  }
  return window;
}

export function generateChirp(fs, impulseLength, fc, bandwidth) {
  const chirpBuffer = new Float32Array(impulseLength);
  const T = impulseLength / fs;
  const f0 = fc - bandwidth / 2;
  const f1 = fc + bandwidth / 2;
  const window = hannWindow(impulseLength);

  for (let i = 0; i < impulseLength; i++) {
    const phase =
      ((2 * Math.PI * (-f0 * f1 * T)) / bandwidth) *
      Math.log(1 - (bandwidth / (f1 * T)) * (i / fs));
    chirpBuffer[i] = Math.sin(phase) * window[i];
  }
  return chirpBuffer;
}
