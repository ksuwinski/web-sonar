import { SonarAudioGraph } from "./audiograph.js";
import { RangeDopplerDisplay } from "./rangedoppler.js";
// import { initClutterPlot } from "./clutterplot.js";

const speed_of_sound = 343;

class SonarApp {
  constructor() {
    this.rangeDopplerCanvas = document.getElementById("rangedoppler-canvas");
    this.buttonStart = document.getElementById("button-start");
    this.inputLevelInd = document.getElementById("input-level-indicator");
    this.fcLabel = document.getElementById("center-freq-range-label");
    this.fcRange = document.getElementById("center-freq-range");
    this.bandwidthLabel = document.getElementById("bandwidth-range-label");
    this.bandwidthRange = document.getElementById("bandwidth-range");
    this.decimationLabel = document.getElementById("decimation-label");
    this.rangeResolutionLabel = document.getElementById(
      "range-resolution-label",
    );
    this.velocityResolutionLabel = document.getElementById(
      "velocity-resolution-label",
    );
    this.cpiLabel = document.getElementById("CPI-label");
    this.wavelengthLabel = document.getElementById("wavelength-label");
    this.rangeAxisTicks = document.getElementById("range-axis-ticks");
    this.velocityAxisTicks = document.getElementById("velocity-axis-ticks");

    this.rangedopplerdisplay = new RangeDopplerDisplay(this.rangeDopplerCanvas);

    this.fcRange.onchange = () => this.updateParams();
    this.bandwidthRange.onchange = () => this.updateParams();
    this.updateParams();
    console.log(this.sonarParameters);

    // let showClutter = false;
    // let clutterPlot = undefined;
    // if (showClutter) {
    //   clutterPlot = initClutterPlot(n_fast);
    // } else {
    //   document.getElementById("plot-container").style.display = "none";
    // }
    this.buttonStart.addEventListener("click", () => this.toggleState());

    window.addEventListener("resize", () => this.onScreenResize());
    this.onScreenResize();

    this.started = false;
    this.audio_graph = new SonarAudioGraph(this.sonarParameters);
    this.audio_graph.onWorkletMessage = (ev) => this.onWorkletMessage(ev);
    this.ignoreMessages = false;
  }

  updateParams() {
    this.ignoreMessages = true;
    const bandwidthList = [1000, 2000, 4000, 8000, 12000, 16000];
    const bandwidth = bandwidthList[this.bandwidthRange.value];
    const fs = 44100;

    const decimation = Math.floor(fs / (bandwidth * 1.3));
    this.bandwidthLabel.innerHTML = bandwidth;

    const fc_step = 500;
    this.fcRange.min = Math.ceil(bandwidth / 2 / fc_step) * fc_step;
    this.fcRange.max = Math.floor((fs / 2 - bandwidth / 2) / fc_step) * fc_step;
    const fc = Number(this.fcRange.value);
    this.fcLabel.innerHTML = fc;

    this.decimationLabel.innerHTML = decimation;

    const impulseLength = 512;
    const n_slow = 20;
    const n_fast = Math.ceil(impulseLength / decimation);
    this.rangedopplerdisplay.updateDimensions(n_fast, n_slow);

    const rangeResolution = (speed_of_sound / fs) * decimation;
    this.rangeResolution = rangeResolution;

    const wavelength = speed_of_sound / fc;

    const CPI = (impulseLength * n_slow) / fs;
    const dopplerResolution = 1 / CPI;
    const velocityResolution = dopplerResolution * wavelength;
    this.velocityResolution = velocityResolution;
    this.wavelengthLabel.innerHTML = `wavelength: ${(wavelength * 100).toFixed(2)} cm`;
    this.rangeResolutionLabel.innerHTML = (rangeResolution * 100).toFixed(2);
    this.velocityResolutionLabel.innerHTML = `velocity cell spacing: ${(velocityResolution * 100).toFixed(2)} cm/s`;
    this.cpiLabel.innerHTML = `coherent processing interval: ${(CPI * 1000).toFixed(0)} ms`;

    this.sonarParameters = {
      impulseLength,
      fc,
      bandwidth,
      decimation,
      n_slow,
    };
    if (this.audio_graph && this.audio_graph.audioContext) {
      this.audio_graph.audioContext.close();
    }
    this.audio_graph = new SonarAudioGraph(this.sonarParameters);
    this.audio_graph.onWorkletMessage = (ev) => this.onWorkletMessage(ev);
    if (this.started) {
      this.audio_graph.start();
    }
    this.ignoreMessages = false;
  }

  onScreenResize() {
    // there is a race condition here
    this.rangeAxisTicks.textContent = "";
    const cellWidthPx =
      this.rangeDopplerCanvas.clientWidth / this.rangeDopplerCanvas.width;
    const rangeTickSpacing = cellWidthPx / this.rangeResolution;
    const n_ticks_range = Math.ceil(
      (2 * this.rangeDopplerCanvas.clientWidth) / rangeTickSpacing,
    );
    for (let i = 0; i < n_ticks_range; i += 1) {
      const tick = document.createElement("div");
      tick.style.left = `${(i * rangeTickSpacing) / 2}px`;
      tick.textContent = i / 2 + " m";
      this.rangeAxisTicks.appendChild(tick);
    }

    this.velocityAxisTicks.textContent = "";
    // const cellHeightPx =
    //   this.rangeDopplerCanvas.clientHeight / this.rangeDopplerCanvas.height;
    // const velTickSpacing = cellHeightPx / this.velocityResolution;
    // const n_ticks_velocity = Math.ceil(
    //   (4 * this.rangeDopplerCanvas.clientHeight) / velTickSpacing,
    // );
    // for (let i = 0; i < n_ticks_velocity; i += 1) {
    //   const tick = document.createElement("div");
    //   tick.style.top = `${(i / 4) * velTickSpacing}px`;
    //   tick.textContent = (i - (n_ticks_velocity - 1) / 2) / 4;
    //   this.velocityAxisTicks.appendChild(tick);
    // }
  }

  async toggleState() {
    if (this.started) {
      this.stop();
    } else {
      this.start();
    }
  }
  async start() {
    this.started = true;
    await this.audio_graph.start();
    this.buttonStart.innerHTML = "stop";
  }
  async stop() {
    this.started = false;
    await this.audio_graph.stop();
    this.buttonStart.innerHTML = "start";
  }

  async onWorkletMessage(ev) {
    // console.log(ev.data);
    // if (showClutter) {
    //   clutterPlot.data.datasets[0].data = ev.data.clutter;
    //   clutterPlot.update();
    // }

    if (this.ignoreMessages) {
      return;
    }
    const fast_slow = ev.data.fast_slow;
    const n_fast =
      this.sonarParameters.impulseLength / this.sonarParameters.decimation;
    const n_slow = this.sonarParameters.n_slow;
    this.rangedopplerdisplay.draw(fast_slow);

    this.inputLevelInd.innerHTML = Math.round(ev.data.peak * 100);
  }
}

const app = new SonarApp();
