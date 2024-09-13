export class RangeDopplerDisplay {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    // vmax_decay = 0.05;
    // vmax = 1;
  }
  updateDimensions(width, height) {
    this.width = width;
    this.height = height;
    this.canvas.width = width;
    this.canvas.height = height;
    this.imagedata = new ImageData(width, height);
    this.imgbuffer = this.imagedata.data;

    for (let x = 0; x < this.width; x++) {
      for (let y = 0; y < this.height; y++) {
        this.imgbuffer[x * this.height * 4 + y * 4 + 3] = 256;
      }
    }
  }
  draw(array) {
    if (!this.imgbuffer) {
      console.error(
        "updateDimensions must be called at least once before draw",
      );
    }
    if (array.length != this.width * this.height) {
      console.error(
        "length of the array (%d) doesn't match required dimensions %dx%d = %d",
        array.length,
        this.width,
        this.height,
        this.width * this.height,
      );
      return;
    }

    const max = Math.max(...array);
    // const sum = array.reduce((partialSum, a) => partialSum + a, 0);
    // const mean = sum / array.length;
    // vmax = Math.max(max, vmax * (1 - vmax_decay));

    for (let x = 0; x < this.width; x++) {
      for (let y = 0; y < this.height; y++) {
        const val = array[x * this.height + y] / max;
        this.imgbuffer[x * this.height * 4 + y * 4 + 0] = Math.round(val * 256);
        // this.imgbuffer[x * this.height * 4 + y * 4 + 3] = 256;
      }
    }
    this.ctx.putImageData(this.imagedata, 0, 0);
    return max;
  }
}
