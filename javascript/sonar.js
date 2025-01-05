import { SonarAudioGraph } from "./audiograph.js";
import { RangeDopplerDisplay } from "./rangedoppler.js";
// import { initClutterPlot } from "./clutterplot.js";

const speed_of_sound = 343;

class SonarApp {
  constructor() {
    this.settingsForm = document.getElementById("settings-form");
    this.rangeDopplerCanvas = document.getElementById("rangedoppler-canvas");
    this.buttonStart = document.getElementById("button-start");
    this.inputLevelInd = document.getElementById("input-level-meter");
    this.inputLevelLabel = document.getElementById("input-level-label");
    this.fcLabel = document.getElementById("center-freq-range-label");
    this.fcRange = document.getElementById("center-freq-range");
    this.bandwidthLabel = document.getElementById("bandwidth-range-label");
    this.bandwidthRange = document.getElementById("bandwidth-range");
    this.nSlowLabel = document.getElementById("n-slow-range-label");
    this.nSlowRange = document.getElementById("n-slow-range");
    this.chirpLengthLabel = document.getElementById("chirp-length-range-label");
    this.chirpDurationLabel = document.getElementById(
      "chirp-duration-range-label",
    );
    this.chirpLengthRange = document.getElementById("chirp-length-range");
    this.decimationLabel = document.getElementById("decimation-label");
    this.rangeResolutionLabel = document.getElementById(
      "range-resolution-label",
    );
    this.velocityResolutionLabel = document.getElementById(
      "velocity-resolution-label",
    );
    this.cpiLabel = document.getElementById("CPI-label");
    this.prfLabel = document.getElementById("prf-label");
    this.wavelengthLabel = document.getElementById("wavelength-label");
    this.rangeAxisTicks = document.getElementById("range-axis-ticks");
    this.velocityAxisTicks = document.getElementById("velocity-axis-ticks");
    this.rangeAmbiguityLabel = document.getElementById("range-ambiguity-label");
    this.velocityAmbiguityLabel = document.getElementById(
      "velocity-ambiguity-label",
    );
    this.maxMigrationlessVelocityLabel = document.getElementById(
      "max-migrationless-velocity-label",
    );
    this.maxMigrationlessVelocityCellUnitsLabel = document.getElementById(
      "max-migrationless-velocity-cell-units-label",
    );
    this.rangeVelocityCouplingLabel = document.getElementById(
      "range-velocity-coupling-label",
    );
    this.cellUnitRvCouplingLabel = document.getElementById(
      "cell-unit-rv-coupling-label",
    );
    this.integrationGainLabel = document.getElementById(
      "integration-gain-label",
    );
    this.sampleRateLabel = document.getElementById("sample-rate-label");

    this.offsetCheckbox = document.getElementById("offset-checkbox");
    this.windowCheckbox = document.getElementById("window-checkbox");

    this.rangedopplerdisplay = new RangeDopplerDisplay(this.rangeDopplerCanvas);

    this.fcRange.onchange = () => this.updateParams();
    this.bandwidthRange.onchange = () => this.updateParams();
    this.offsetCheckbox.onchange = () => this.updateParams();
    this.windowCheckbox.onchange = () => this.updateParams();
    this.nSlowRange.onchange = () => this.updateParams();
    this.chirpLengthRange.onchange = () => this.updateParams();
    const radios = document.querySelectorAll(".clutter-filter-settings input");
    for (const radio of radios) {
      radio.onchange = () => this.updateParams();
    }

    this.fs = 44100;
    this.updateParams();

    // let showClutter = false;
    // let clutterPlot = undefined;
    // if (showClutter) {
    //   clutterPlot = initClutterPlot(n_fast);
    // } else {
    //   document.getElementById("plot-container").style.display = "none";
    // }
    this.buttonStart.addEventListener("click", (ev) => {
      ev.preventDefault();
      this.toggleState();
    });

    window.addEventListener("resize", () => this.onScreenResize());
    this.onScreenResize();

    this.started = false;
    this.audio_graph = new SonarAudioGraph(this.sonarParameters);
    this.audio_graph.onWorkletMessage = (ev) => this.onWorkletMessage(ev);
    this.ignoreMessages = false;
  }

  updateParams() {
    this.ignoreMessages = true;

    //const bandwidthList = [1000, 2000, 4000, 8000, 12000, 16000];
    //const bandwidth = bandwidthList[this.bandwidthRange.value];
    const bandwidth =
      Math.round(40 * Math.pow(2, this.bandwidthRange.value / 4)) * 100;

    const fc_step = 500;
    this.fcRange.min = Math.ceil((bandwidth * 1.3) / 2 / fc_step) * fc_step;
    this.fcRange.max =
      Math.floor((this.fs / 2 - (bandwidth * 1.3) / 2) / fc_step) * fc_step;
    const fc = Number(this.fcRange.value);

    const impulseLength = Math.round(
      512 * Math.pow(2, this.chirpLengthRange.value / 4),
    );

    const max_CPI = 0.4;
    let max_n_slow = Math.round((max_CPI * this.fs) / impulseLength);
    if (max_n_slow % 2 == 0) {
      max_n_slow -= 1;
    }
    this.nSlowRange.max = max_n_slow;
    const n_slow = Number(this.nSlowRange.value);
    this.nSlowLabel.textContent = n_slow;

    const clutterFilterOption = this.settingsForm.clutterfilter.value;
    const track_offset = this.offsetCheckbox.checked;
    const apply_window = this.windowCheckbox.checked && n_slow != 1;

    const decimation = Math.floor(this.fs / (bandwidth * 1.3));
    const n_fast = Math.ceil(impulseLength / decimation);

    const impulseDuration = impulseLength / this.fs;
    const PRF = 1 / impulseDuration;

    const wavelength = speed_of_sound / fc;
    const CPI = (impulseLength * n_slow) / this.fs;
    const dopplerResolution = 1 / CPI;

    const rangeAmbiguity = impulseDuration * speed_of_sound;
    const dopplerAmbiguity = 0.5 / impulseDuration; // 0.5 because we must distinguish negative from positive
    const velocityAmbiguity = dopplerAmbiguity * wavelength;

    const integrationGain = 10 * Math.log10(bandwidth * CPI);

    this.rangeResolution = (speed_of_sound / this.fs) * decimation;
    this.velocityResolution = dopplerResolution * wavelength;

    this.bandwidthLabel.innerHTML = bandwidth;
    this.fcLabel.innerHTML = fc;
    this.decimationLabel.innerHTML = decimation;
    this.wavelengthLabel.innerHTML = (wavelength * 100).toFixed(2);
    this.rangeResolutionLabel.innerHTML = (this.rangeResolution * 100).toFixed(
      2,
    );
    this.velocityResolutionLabel.innerHTML = (
      this.velocityResolution * 100
    ).toFixed(2);
    this.cpiLabel.innerHTML = (CPI * 1000).toFixed(0);
    this.prfLabel.innerHTML = PRF.toFixed(0);
    this.rangeAmbiguityLabel.textContent = rangeAmbiguity.toFixed(2);
    this.velocityAmbiguityLabel.textContent = velocityAmbiguity.toFixed(2);
    this.chirpLengthLabel.textContent = impulseLength;
    this.chirpDurationLabel.textContent = ((impulseLength / this.fs) * 1000).toFixed(
      2,
    );

    this.integrationGainLabel.textContent = integrationGain.toFixed(2);
    this.maxMigrationlessVelocityLabel.textContent = (
      (this.rangeResolution / CPI) *
      100
    ).toFixed(1);
    this.maxMigrationlessVelocityCellUnitsLabel.textContent = (
      this.rangeResolution /
      CPI /
      this.velocityResolution
    ).toFixed(1);
    this.sampleRateLabel.textContent = this.fs;
    // this.rangeVelocityCouplingLabel.textContent = ();

    // chirpRate*delta_t = delta_f
    // chirpRate*delta_r / c = delta_f
    // delta_r = delta_f*c/chirp_rate
    //         = delta_v/wavelength * c/chirp_rate
    //         = delta_v/(c/fc) * c/chirp_rate
    //         = delta_v * fc/chirp_rate
    //         = delta_v * fc/B * T
    const rangeVelocityCoupling = (fc / bandwidth) * impulseDuration;
    this.rangeVelocityCouplingLabel.textContent = (
      rangeVelocityCoupling * 100
    ).toFixed(2);
    const cellUnitRvCoupling =
      (rangeVelocityCoupling / this.rangeResolution) * this.velocityResolution;
    this.cellUnitRvCouplingLabel.textContent = cellUnitRvCoupling.toFixed(3);

    this.rangedopplerdisplay.updateDimensions(n_fast, n_slow);
    this.sonarParameters = {
      impulseLength,
      fc,
      bandwidth,
      decimation,
      n_slow,
      clutterFilterOption,
      track_offset,
      apply_window,
    };
    console.log(this.sonarParameters);
    if (this.audio_graph && this.audio_graph.audioContext) {
      this.audio_graph.audioContext.close();
    }
    this.audio_graph = new SonarAudioGraph(this.sonarParameters);
    this.audio_graph.onWorkletMessage = (ev) => this.onWorkletMessage(ev);
    if (this.started) {
      this.audio_graph.start();
    }
    this.onScreenResize();
    this.ignoreMessages = false;
  }

  onScreenResize() {
    // there is a race condition here
    this.rangeAxisTicks.textContent = "";
    const cellWidthPx =
      this.rangeDopplerCanvas.clientWidth / this.rangeDopplerCanvas.width;
    // const rangeTickSpacing = cellWidthPx / this.rangeResolution;
    const rangeTickSpacing = 100; //px
    const rangeTickStep =
      (rangeTickSpacing / cellWidthPx) * this.rangeResolution;
    const n_ticks_range = Math.ceil(
      this.rangeDopplerCanvas.clientWidth / rangeTickSpacing,
    );
    for (let i = 0; i < n_ticks_range; i += 1) {
      const tick = document.createElement("div");
      tick.style.left = `${i * rangeTickSpacing}px`;
      tick.textContent = (i * rangeTickStep).toFixed(2) + " m";
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
      await this.stop();
    } else {
      await this.start();
    }
  }
  async start() {
    this.started = true;
    await this.audio_graph.start();
    this.buttonStart.innerHTML = "stop";

    // on the first run, we must save the true sample rate chosen by the device
    this.fs = this.audio_graph.sampleRate;
    this.updateParams();
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

    this.inputLevelInd.value = ev.data.peak * 100;
    this.inputLevelLabel.innerHTML = Math.round(ev.data.peak * 100);
  }
}

const app = new SonarApp();
