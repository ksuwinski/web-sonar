export function initClutterPlot(n_fast) {
  const clutterPlotCanvas = document.getElementById("myChart");
  const clutterPlot = new Chart(clutterPlotCanvas, {
    type: "line",
    data: {
      labels: [...Array(n_fast).keys()],
      datasets: [
        {
          label: "clutter amplitude",
          borderWidth: 1,
        },
      ],
    },
    options: {
      animation: { easing: "linear" },
      scales: {
        y: {
          // min: 0,
          // max: 100,
          // beginAtZero: true,
        },
      },
    },
  });
  return clutterPlot;
}
