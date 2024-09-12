import { generateChirp } from "./chirp.js";

export class SonarAudioGraph {
  initialized;
  chirpSource;
  sonarProcessor;
  micSource;
  onWorkletMessage;
  sonarParameters;

  constructor(sonarParameters) {
    this.initialized = false;
    this.onWorkletMessage = (ev) => {
      console.error("audio graph callback is not registered", ev);
    };
    this.sonarParameters = sonarParameters;
  }
  async #initialize() {
    const impulseLength = this.sonarParameters.impulseLength;
    const fc = this.sonarParameters.fc;
    const bandwidth = this.sonarParameters.bandwidth;

    this.audioContext = new AudioContext({
      latencyHint: "playback",
      sampleRate: 44100,
    });

    const fs = this.audioContext.sampleRate;
    if (fs != 44100) {
      console.error("wrong sample rate");
    }
    const normalizedCarrier = fc / fs;

    const chirp = generateChirp(fs, impulseLength, fc, bandwidth);
    this.chirpSource = initAudioOutput(this.audioContext, chirp);

    this.sonarProcessor = await initSonarWorklet(this.audioContext, {
      chirp,
      normalizedCarrier,
      clutter_alpha: 0.1,
      decimation: this.sonarParameters.decimation,
      n_slow: this.sonarParameters.n_slow,
      clutterFilterOption: this.sonarParameters.clutterFilterOption,
      track_offset: this.sonarParameters.track_offset,
    });
    this.sonarProcessor.port.onmessage = this.onWorkletMessage;
    this.micSource = await initAudioInput(this.audioContext);
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
