import { SonarAudioGraph } from "./audiograph.js";
import { initClutterPlot } from "./clutterplot.js";
import { draw_2d_array } from "./rangedoppler.js";

class SonarApp {
  constructor() {
    this.rd_ctx = document
      .getElementById("rangedoppler-canvas")
      .getContext("2d");
    this.buttonStart = document.getElementById("button-start");
    this.inputLevelInd = document.getElementById("input-level-indicator");

    this.sonarParameters = {
      impulseLength: 512,
      fc: 17000,
      bandwidth: 4000,
      decimation: 8,
      n_slow: 20,
    };

    // let showClutter = false;
    // let clutterPlot = undefined;
    // if (showClutter) {
    //   clutterPlot = initClutterPlot(n_fast);
    // } else {
    //   document.getElementById("plot-container").style.display = "none";
    // }
    this.buttonStart.addEventListener("click", () => this.toggleState());

    this.started = false;
    this.audio_graph = new SonarAudioGraph(this.sonarParameters);
    this.audio_graph.onWorkletMessage = (ev) => this.onWorkletMessage(ev);
  }

  async toggleState() {
    if (this.started) {
      this.stop();
    } else {
      this.start();
    }
  }
  async start() {
    console.log("start");
    this.started = true;
    await this.audio_graph.start();
    this.buttonStart.innerHTML = "stop";
  }
  async stop() {
    console.log("stop");
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

    const fast_slow = ev.data.fast_slow;
    const n_fast =
      this.sonarParameters.impulseLength / this.sonarParameters.decimation;
    const n_slow = this.sonarParameters.n_slow;
    draw_2d_array(this.rd_ctx, fast_slow, n_slow, n_fast);

    this.inputLevelInd.innerHTML = `peak input level: ${Math.round(ev.data.peak * 100)}%`;
  }
}

const app = new SonarApp();
