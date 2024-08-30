const imagedata = new ImageData(64, 20);
const imgbuffer = imagedata.data;

export function draw_2d_array(ctx, array, width, height) {
  if (array.length != width * height) {
    console.error(
      "length of the array (%d) doesn't match required dimensions %dx%d = %x",
      array.length,
      width,
      height,
      width * height,
    );
    return;
  }

  const max = Math.max(...array);

  for (let row = 0; row < width; row++) {
    for (let col = 0; col < height; col++) {
      const val = array[row * height + col] / max;

      imgbuffer[row * height * 4 + col * 4 + 0] = Math.round(val * 256);
      imgbuffer[row * height * 4 + col * 4 + 3] = 256;
    }
  }
  ctx.putImageData(imagedata, 0, 0);
}
