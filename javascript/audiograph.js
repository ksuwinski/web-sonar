import { generateChirp, hannWindow, rectWindow } from "./chirp.js";

export class SonarAudioGraph {
  initialized;
  chirpSource;
  sonarProcessor;
  micSource;
  onWorkletMessage;
  sonarParameters;
  sampleRate;

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
      // sampleRate: 44100,
    });

    this.sampleRate = this.audioContext.sampleRate;

    console.log("audiocontext fs", this.sampleRate);
    const normalizedCarrier = fc / this.sampleRate;

    const chirp = generateChirp(this.sampleRate, impulseLength, fc, bandwidth);
    this.chirpSource = initAudioOutput(this.audioContext, chirp);

    const n_slow = this.sonarParameters.n_slow;

    let slow_time_window = undefined;
    if (this.sonarParameters.apply_window) {
      slow_time_window = hannWindow(n_slow);
    } else {
      slow_time_window = rectWindow(n_slow);
    }

    const tau = 0.1; //idk this is probably stupid
    const clutter_alpha = impulseLength / (this.sampleRate * tau);

    this.sonarProcessor = await initSonarWorklet(this.audioContext, {
      chirp,
      normalizedCarrier,
      clutter_alpha,
      slow_time_window,
      decimation: this.sonarParameters.decimation,
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
      await this.#initialize();
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
      // sampleRate: 44100,
    },
  });
  const audioTrack = stream.getAudioTracks()[0];
  console.log("audio input settings:", audioTrack.getSettings());
  console.log("audio track constraints", audioTrack.getConstraints());
  console.log("audio track capabilities", audioTrack.getCapabilities());
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
    "pkg/sonar_bg.wasm"
    // "/pkg/sonar_bg.wasm?dontcache=" + Math.round(Math.random() * 1000000),
  );
  const wasm_blob = await response.arrayBuffer();

  await audioContext.audioWorklet.addModule("javascript/sonar-processor.js");
  const sonarProcessor = new AudioWorkletNode(audioContext, "sonar-processor", {
    numberOfInputs: 1,
    numberOfOutputs: 0,
  });

  sonarProcessor.port.postMessage({ wasm_blob, ...params });

  return sonarProcessor;
}
