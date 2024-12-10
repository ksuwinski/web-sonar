import "./polyfill.js";
import init, { ClutterFilterOption, Sonar } from "../pkg/sonar.js";

function parseFilterOption(option) {
  switch (option) {
    case "none":
      return ClutterFilterOption.None;
    case "two-pulse":
      return ClutterFilterOption.TwoPulse;
    case "slow":
      return ClutterFilterOption.Slow;
    case "remove-zero":
      return ClutterFilterOption.RemoveZero;
    default:
      console.error("unknown clutter filter", option);
  }
}
class SonarProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.peak = 0;
    this.peak_decay = 0.001;
    this.n = 0;
    this.sonar = undefined;
    this.wasm_ready = false;
    this.port.onmessage = (ev) => {
      init(ev.data.wasm_blob).then(() => {
        this.sonar = Sonar.new(
          ev.data.chirp,
          ev.data.normalizedCarrier,
          ev.data.decimation,
          ev.data.slow_time_window,
          ev.data.clutter_alpha,
          parseFilterOption(ev.data.clutterFilterOption),
          ev.data.track_offset,
        );
        console.log("initialized");
        this.wasm_ready = true;
      });
    };
  }

  process(inputs, outputs) {
    if (!this.wasm_ready) {
      return true;
    }
    const input = inputs[0];
    if (input.length == 0) {
      return true;
    }

    // for (let channel = 0; channel < input.length; ++channel) {
    // }
    this.sonar.handle_input(input[0]);
    this.peak = Math.max(
      this.peak * (1 - this.peak_decay),
      Math.max(...input[0]),
    );

    if (this.n % (((512 / 128) * 20) / 4) == 0) {
      this.port.postMessage({
        peak: this.peak,
        fast_slow: this.sonar.get_data_cube(),
        // clutter: this.sonar.clutter(),
      });
    }
    this.n += 1;
    return true;
  }
}

registerProcessor("sonar-processor", SonarProcessor);
