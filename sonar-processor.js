import "./polyfill.js";
import init, { Sonar } from "./pkg/sonar.js";
class SonarProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.n = 0;
    this.sonar = undefined;
    this.wasm_ready = false;
    this.port.onmessage = (ev) => {
      init(ev.data.wasm_blob).then(() => {
        this.sonar = Sonar.new(ev.data.chirp, ev.data.normalizedCarrier);
        console.log("initialized");
        this.wasm_ready = true;
      });
    };
    // this.buffer = new ArrayBuffer(4*128*16);
  }

  process(inputs, outputs) {
    // const buffer_view = new Float32Array(this.buffer, 128*this.n % 16*128, 128);
    if (!this.wasm_ready) {
      return true;
    }
    const input = inputs[0];
    if (input[0] == undefined) {
      console.log("no channels on input");
      return true;
    }

    // for (let channel = 0; channel < input.length; ++channel) {
    this.sonar.handle_input(input[0]);
    // }
    // buffer_view.set(input[0]);

    if (this.n % 80 == 0) {
      this.port.postMessage({
        clutter: this.sonar.clutter(),
        fast_slow: this.sonar.get_data_cube(),
      });
    }
    this.n += 1;
    return true;
  }
}

registerProcessor("sonar-processor", SonarProcessor);
