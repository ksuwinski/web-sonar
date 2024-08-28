export function generateChirp(fs, impulseLength, fc, bandwidth) {
  const chirpBuffer = new Float32Array(impulseLength);
  const T = impulseLength / fs;
  const f0 = fc - bandwidth / 2;
  const f1 = fc + bandwidth / 2;

  for (let i = 0; i < impulseLength; i++) {
    const phase =
      ((2 * Math.PI * (-f0 * f1 * T)) / bandwidth) *
      Math.log(1 - (bandwidth / (f1 * T)) * (i / fs));
    const win = 0.5 * (1 - Math.cos((2 * Math.PI * i) / impulseLength)); //Hann window
    chirpBuffer[i] = Math.sin(phase) * win;
  }
  return chirpBuffer;
}
