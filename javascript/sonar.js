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
    // started = false;
    return;
  }
  started = true;
  audioContext = new AudioContext({ latencyHint: "playback" });

  const fs = audioContext.sampleRate;
  const normalizedCarrier = fc / fs;
  console.log("fs = %f", fs);

  const chirp = generateChirp(fs, impulseLength, fc, bandwidth);
  const chirpSource = initAudioOutput(audioContext, chirp);

  const sonarProcessor = await initSonarWorklet(audioContext, {
    chirp,
    normalizedCarrier,
  });
  const micSource = await initAudioInput(audioContext);
  micSource.connect(sonarProcessor);

  buttonStart.innerHTML = "stop";
  buttonStart.disabled = true;
});

function onWorkletMessage(ev) {
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
}

async function initAudioInput(audioContext) {
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
  const audioTrack = stream.getAudioTracks()[0];
  console.log("audio input settings:", audioTrack.getSettings());

  const micSource = audioContext.createMediaStreamSource(stream);
  return micSource;
}

function initAudioOutput(audioContext, chirp) {
  const myArrayBuffer = audioContext.createBuffer(
    2,
    chirp.length,
    audioContext.sampleRate,
  );
  myArrayBuffer.copyToChannel(chirp, 0);
  const chirpSource = audioContext.createBufferSource();
  chirpSource.buffer = myArrayBuffer;
  chirpSource.loop = true;
  chirpSource.connect(audioContext.destination);
  chirpSource.start();
  return chirpSource;
}

async function initSonarWorklet(audioContext, params) {
  const response = await fetch(
    "/pkg/sonar_bg.wasm?idk=" + Math.round(Math.random() * 1000000),
  );
  const wasm_blob = await response.arrayBuffer();

  await audioContext.audioWorklet.addModule("sonar-processor.js");
  const sonarProcessor = new AudioWorkletNode(audioContext, "sonar-processor", {
    numberOfInputs: 1,
    numberOfOutputs: 0,
  });
  sonarProcessor.port.onmessage = onWorkletMessage;

  sonarProcessor.port.postMessage({ wasm_blob, ...params });

  return sonarProcessor;
}
