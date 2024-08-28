import { generateChirp } from "./chirp.js";
import { initClutterPlot } from "./clutterplot.js";

let impulseLength = 512;
let fc = 17000;
let bandwidth = 4000;

let n_slow = 20;
let n_fast = impulseLength / 8;

const rd_ctx = document.getElementById("rangedoppler-canvas").getContext("2d");
const clutterPlot = initClutterPlot(n_fast);
const buttonStart = document.getElementById("button-start");

let started = false;
let audioContext = undefined;
buttonStart.addEventListener("click", async () => {
  if (started) {
    audioContext.suspend();
    started = false;
    buttonStart.innerHTML = "start";
    return;
  }
  started = true;
  audioContext = new AudioContext({ latencyHint: "playback" });

  const response = await fetch(
    "/pkg/sonar_bg.wasm?idk=" + Math.round(Math.random() * 1000000),
  );
  const wasm_blob = await response.arrayBuffer();

  const fs = audioContext.sampleRate;
  fc = fs / Math.round(fs / fc);
  console.log("fs = %f", fs);

  const stream = await navigator.mediaDevices.getUserMedia({
    audio: {
      autoGainControl: false,
      echoCancellation: false,
      noiseSuppression: false,
      voiceIsolation: false,
      channelCount: 2,
      sampleRate: 44100,
    },
  });
  const micSource = audioContext.createMediaStreamSource(stream);

  const audioTrack = stream.getAudioTracks()[0];
  console.log(audioTrack.getSettings());

  await audioContext.audioWorklet.addModule("sonar-processor.js");
  const sonarProcessor = new AudioWorkletNode(audioContext, "sonar-processor", {
    numberOfInputs: 1,
    numberOfOutputs: 0,
  });
  sonarProcessor.port.onmessage = (ev) => {
    console.log(ev.data);
    clutterPlot.data.datasets[0].data = ev.data.clutter;
    clutterPlot.update();

    const CELL_SIZE = 5;
    rd_ctx.beginPath();

    for (let row = 0; row < n_slow; row++) {
      for (let col = 0; col < n_fast; col++) {
        // const idx = getIndex(row, col);

        rd_ctx.fillStyle = Math.random() < 0.5 ? "#ff0000" : "#00ff00";

        rd_ctx.fillRect(
          col * (CELL_SIZE + 1) + 1,
          row * (CELL_SIZE + 1) + 1,
          CELL_SIZE,
          CELL_SIZE,
        );
      }
    }

    rd_ctx.stroke();
  };
  micSource.connect(sonarProcessor);

  const myArrayBuffer = audioContext.createBuffer(2, impulseLength, fs);
  const chirp = generateChirp(fs, impulseLength, fc, bandwidth);
  myArrayBuffer.copyToChannel(chirp, 0);
  sonarProcessor.port.postMessage({
    wasm_blob,
    chirp,
    normalizedCarrier: fc / fs,
  });

  const chirpSource = audioContext.createBufferSource();
  chirpSource.buffer = myArrayBuffer;
  chirpSource.loop = true;

  chirpSource.connect(audioContext.destination);
  chirpSource.start();

  buttonStart.innerHTML = "stop";
});

function initSonarWorklet() {}
