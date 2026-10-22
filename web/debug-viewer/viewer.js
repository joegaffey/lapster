const canvas = document.querySelector("#view");
const context = canvas.getContext("2d");

const scale = 110;
const cos30 = Math.cos(Math.PI / 6);
const sin30 = 0.5;
const originX = canvas.width / 2;
const originY = canvas.height / 2 + 60;

function project([x, y, z]) {
  const screenX = (x + z) * cos30 * scale;
  const screenY = ((x - z) * sin30 - y) * scale;
  return [originX + screenX, originY + screenY];
}

function line(from, to, color, width = 1) {
  const [x0, y0] = project(from);
  const [x1, y1] = project(to);
  context.strokeStyle = color;
  context.lineWidth = width;
  context.beginPath();
  context.moveTo(x0, y0);
  context.lineTo(x1, y1);
  context.stroke();
}

function label(position, text, color, offset = [10, -10]) {
  const [x, y] = project(position);
  context.fillStyle = color;
  context.font = "16px ui-monospace, monospace";
  context.fillText(text, x + offset[0], y + offset[1]);
}

function drawGrid() {
  for (let index = -3; index <= 3; index += 1) {
    const shade = index === 0 ? "#343b46" : "#20252d";
    line([index, 0, -3], [index, 0, 3], shade);
    line([-3, 0, index], [3, 0, index], shade);
  }
}

function drawCube() {
  const size = 1;
  const vertices = [
    [0, 0, 0],
    [size, 0, 0],
    [size, 0, size],
    [0, 0, size],
    [0, size, 0],
    [size, size, 0],
    [size, size, size],
    [0, size, size],
  ];
  const edges = [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4],
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
  ];
  const faces = [
    [0, 1, 2, 3],
    [4, 5, 6, 7],
  ];

  context.fillStyle = "rgba(120, 140, 170, 0.12)";
  for (const face of faces) {
    context.beginPath();
    face.forEach((index, pointIndex) => {
      const [x, y] = project(vertices[index]);
      if (pointIndex === 0) {
        context.moveTo(x, y);
      } else {
        context.lineTo(x, y);
      }
    });
    context.closePath();
    context.fill();
  }

  for (const [from, to] of edges) {
    line(vertices[from], vertices[to], "#8ea0b8", 1.5);
  }
}

function drawAxes() {
  line([0, 0, 0], [2, 0, 0], "#ef4444", 3);
  line([0, 0, 0], [0, 2, 0], "#22c55e", 3);
  line([0, 0, 0], [0, 0, -2], "#3b82f6", 3);
  label([2, 0, 0], "+X", "#ef4444");
  label([0, 2, 0], "+Y", "#22c55e");
  label([0, 0, -2], "-Z", "#3b82f6");
}

function drawOrigin() {
  const [x, y] = project([0, 0, 0]);
  context.fillStyle = "#e6e8eb";
  context.beginPath();
  context.arc(x, y, 4, 0, Math.PI * 2);
  context.fill();
}

context.clearRect(0, 0, canvas.width, canvas.height);
drawGrid();
drawCube();
drawAxes();
drawOrigin();
