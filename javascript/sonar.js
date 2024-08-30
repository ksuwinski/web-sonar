import { SonarAudioGraph } from "./audiograph.js";
import { initClutterPlot } from "./clutterplot.js";
import { draw_2d_array } from "./rangedoppler.js";

const rd_ctx = document.getElementById("rangedoppler-canvas").getContext("2d");
const buttonStart = document.getElementById("button-start");

let showClutter = false;

let impulseLength = 512;
let fc = 17000;
let bandwidth = 4000;

let n_slow = 20;
let n_fast = impulseLength / 8;

let clutterPlot = undefined;
if (showClutter) {
  clutterPlot = initClutterPlot(n_fast);
} else {
  document.getElementById("plot-container").style.display = "none";
}

let started = false;
let audio_graph = new SonarAudioGraph();
audio_graph.onWorkletMessage = onWorkletMessage;
buttonStart.addEventListener("click", async () => {
  if (started) {
    audio_graph.stop();
    buttonStart.innerHTML = "start";
    started = false;
    return;
  }
  started = true;
  audio_graph.start();

  buttonStart.innerHTML = "stop";
});

function onWorkletMessage(ev) {
  // console.log(ev.data);
  if (showClutter) {
    clutterPlot.data.datasets[0].data = ev.data.clutter;
    clutterPlot.update();
  }

  const fast_slow = ev.data.fast_slow;
  draw_2d_array(rd_ctx, fast_slow, n_slow, n_fast);
}
