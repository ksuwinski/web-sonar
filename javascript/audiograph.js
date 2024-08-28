import { generateChirp } from "./chirp.js";

export class SonarAudioGraph {
  initialized;
  chirpSource;
  sonarProcessor;
  micSource;
  onWorkletMessage;
  sonarParameters;

  constructor() {
    this.initialized = false;
    this.onWorkletMessage = (ev) => {
      console.error("audio graph callback is not registered", ev);
    };
    this.sonarParameters = {
      impulseLength: 512,
      fs: 17000,
      bandwidth: 4000,
    };
  }
  async #initialize() {
    this.audioContext = new AudioContext({ latencyHint: "playback" });

    const fs = this.audioContext.sampleRate;
    const normalizedCarrier = this.sonarParameters.fc / fs;
    console.log("fs = %f", fs);

    const chirp = generateChirp(
      fs,
      this.sonarParameters.impulseLength,
      this.sonarParameters.fc,
      this.sonarParameters.bandwidth,
    );
    this.chirpSource = initAudioOutput(this.audioContext, chirp);

    this.micSource = await initAudioInput(this.audioContext);
    this.sonarProcessor = await initSonarWorklet(this.audioContext, {
      chirp,
      normalizedCarrier,
    });
    this.sonarProcessor.port.onmessage = this.onWorkletMessage;

    this.micSource.connect(this.sonarProcessor);
    this.initialized = true;
  }
  async start() {
    if (this.initialized) {
      await this.audioContext.resume();
    } else {
      this.#initialize();
    }
  }
  async stop() {
    if (this.initialized) {
      await this.audioContext.suspend();
    } else {
      console.error("stop() called before audio graph was initialized");
    }
  }
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

  sonarProcessor.port.postMessage({ wasm_blob, ...params });

  return sonarProcessor;
}
