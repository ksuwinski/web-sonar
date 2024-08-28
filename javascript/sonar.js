import { SonarAudioGraph } from "./audiograph.js";
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
