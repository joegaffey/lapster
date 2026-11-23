// @ts-nocheck
import init, { GarageViewer } from "../pkg/sim_render.js";

const DEFAULT_CAR = "67gp-cavallo-123";
const CARS = [
  { id: "67gp-cavallo-123", name: "Cavallo 123" },
  { id: "67gp-guthrie-eagle", name: "Guthrie Eagle" },
  { id: "67gp-kuiper-t81", name: "Kuiper T81" },
  { id: "67gp-mango-ms7", name: "Mango MS7" },
  { id: "67gp-murasama-37a", name: "Murasama 37A" },
  { id: "67gp-upping-ut19", name: "Upping UT19" },
  { id: "mazda-rx7", name: "Mazda RX-7" },
];
const WHEEL_KEYS = ["FL", "FR", "RL", "RR"];
const WHEEL_FLAGS = {
  FL: [true, true],
  FR: [true, false],
  RL: [false, true],
  RR: [false, false],
};
const DISTANCE_MIN = 0.3;
const DISTANCE_MAX = 24;
const PITCH_MIN = -1.15;
const PITCH_MAX = 1.35;
const FLOOR_CLEARANCE = 0.06;

const stage = document.querySelector("#stage");
const carSelect = document.querySelector("#car-select");
const reloadButton = document.querySelector("#reload");
const statusElement = document.querySelector("#status");
const errorElement = document.querySelector("#error");
const readCar = document.querySelector("#read-car");
const readWheel = document.querySelector("#read-wheel");
const readTarget = document.querySelector("#read-target");
const readView = document.querySelector("#read-view");
const steerInput = document.querySelector("#steer");
const spinInput = document.querySelector("#spin");
const steerValue = document.querySelector("#steer-value");
const spinValue = document.querySelector("#spin-value");
const autoSpinInput = document.querySelector("#auto-spin");
const wireframeInput = document.querySelector("#wireframe");
const atlasFigure = document.querySelector("#atlas");
const atlasImage = document.querySelector("#atlas-image");
const atlasCaption = document.querySelector("#atlas-caption");
const wheelButtons = Array.from(document.querySelectorAll("[data-wheel]"));
const labPanel = document.querySelector("#lab");
const labToggle = document.querySelector("#lab-toggle");
const labCollapse = document.querySelector("#lab-collapse");
const mobileQuery = window.matchMedia("(max-width: 720px)");

let canvas = document.querySelector("#view");
let activeViewer = null;
let generation = 0;
let frameHandle = 0;
let atlasToken = 0;
let wasmInit = null;
let steer = Number(steerInput.value);
let spin = Number(spinInput.value);
let selectedWheel = "whole";
let labUserToggled = false;

const cameraState = {
  yaw: 0.58,
  pitch: 0.26,
  distance: 6,
  target: [0, 0.45, 0],
  eye: [0, 0.45, 6],
};

const lab = {
  viewer: null,
  info: null,
  focusWheel,
  loadCar,
  camera: cameraState,
  get wireframe() {
    return wireframeInput.checked;
  },
  setWireframe,
};
window.__wheelLab = lab;
window.__wheelLabReady = false;

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

function setStatus(text, state = "") {
  statusElement.textContent = text;
  statusElement.className = state ? `status ${state}` : "status";
}

function clearError() {
  window.__wheelLabError = undefined;
  errorElement.hidden = true;
  errorElement.textContent = "";
}

function setLabCollapsed(collapsed, focus = false) {
  labPanel.classList.toggle("collapsed", collapsed);
  labToggle.hidden = !collapsed;
  labToggle.setAttribute("aria-expanded", String(!collapsed));
  labToggle.textContent = collapsed ? "Controls" : "Hide UI";
  if (focus) {
    (collapsed ? labToggle : labCollapse).focus();
  }
}

function fail(error) {
  const message = error instanceof Error ? error.message : String(error);
  window.__wheelLabError = message;
  window.__wheelLabReady = false;
  setStatus(message, "error");
  errorElement.hidden = false;
  errorElement.textContent = message;
  console.error("wheel lab:", error);
}

function selectedCar() {
  return CARS.find((car) => car.id === carSelect.value) ?? null;
}

function findWheel(info, which) {
  const wheels = info?.car?.wheels ?? [];
  if (typeof which === "number") {
    return wheels[which] ?? null;
  }
  const normalized = String(which ?? "").toUpperCase();
  const flags = WHEEL_FLAGS[normalized];
  if (flags) {
    const [front, left] = flags;
    const byFlags = wheels.find(
      (wheel) => wheel.front === front && wheel.left === left,
    );
    if (byFlags) {
      return byFlags;
    }
  }
  return (
    wheels.find((wheel) => {
      const name = String(wheel.name ?? "").toUpperCase();
      return name === normalized || name.endsWith(`_${normalized}`);
    }) ?? null
  );
}

function wheelWorldPosition(info, wheel) {
  const wheels = info?.car?.wheels ?? [];
  let contact = Infinity;
  for (const entry of wheels) {
    const position = entry?.position;
    const radius = Number(entry?.radius);
    if (
      Array.isArray(position) &&
      Number.isFinite(position[1]) &&
      Number.isFinite(radius)
    ) {
      contact = Math.min(contact, position[1] - radius);
    }
  }
  if (!Number.isFinite(contact)) {
    contact = 0;
  }
  const position = wheel?.position ?? [0, 0, 0];
  // One concrete bay: the car is rotated 180 deg about Y and lifted so the
  // lowest wheel contact sits on the floor (translation.y = -contact).
  return [
    -Number(position[0] ?? 0),
    Number(position[1] ?? 0) - contact,
    -Number(position[2] ?? 0),
  ];
}

function wheelModelLabel(info, key) {
  const wheels = info?.car?.wheels ?? [];
  if (!wheels.length) {
    return "—";
  }
  if (key === "whole") {
    return wheels.map((wheel) => wheel.name).join(" / ");
  }
  const wheel = findWheel(info, key);
  if (!wheel) {
    return "—";
  }
  const axle = wheel.front ? "front" : "rear";
  const side = wheel.left ? "left" : "right";
  return `${wheel.name} (${axle} ${side})`;
}

function updateReadout() {
  const car = selectedCar();
  readCar.textContent = car?.name ?? "—";
  readWheel.textContent = wheelModelLabel(lab.info, selectedWheel);
  readTarget.textContent = `[${cameraState.target
    .map((value) => Number(value).toFixed(2))
    .join(", ")}]`;
  readView.textContent = `${cameraState.distance.toFixed(2)} m · yaw ${cameraState.yaw.toFixed(
    2,
  )} · pitch ${cameraState.pitch.toFixed(2)}`;
}

function updateAtlasCaption() {
  const car = selectedCar();
  const wheels = lab.info?.car?.wheels ?? [];
  let model = "—";
  if (selectedWheel === "whole") {
    model = wheels.length
      ? wheels.map((wheel) => wheel.name).join(" / ")
      : "all four";
  } else {
    const wheel = findWheel(lab.info, selectedWheel);
    model = wheel ? wheel.name : selectedWheel;
  }
  atlasCaption.textContent = `${car?.name ?? "Car"} · wheel ${model}`;
}

function syncWheelButtons() {
  for (const button of wheelButtons) {
    const key = button.dataset.wheel ?? "";
    button.setAttribute("aria-pressed", String(key === selectedWheel));
  }
}

function applyAngles() {
  steer = Number(steerInput.value);
  spin = Number(spinInput.value);
  steerValue.textContent = steer.toFixed(2);
  spinValue.textContent = spin.toFixed(2);
  if (activeViewer) {
    activeViewer.set_wheel_angles(steer, spin);
  }
}

// Wireframe edge lines are generated at the wheel rest pose, so any steering
// or spin would leave the mesh out of alignment with them. While enabled, pin
// the angles to zero and lock the controls that would move the wheel.
function setWireframe(visible) {
  const enabled = Boolean(visible);
  wireframeInput.checked = enabled;
  if (enabled) {
    steer = 0;
    spin = 0;
    steerInput.value = "0";
    spinInput.value = "0";
    steerValue.textContent = steer.toFixed(2);
    spinValue.textContent = spin.toFixed(2);
  }
  steerInput.disabled = enabled;
  spinInput.disabled = enabled;
  autoSpinInput.disabled = enabled;
  if (activeViewer) {
    if (enabled) {
      activeViewer.set_wheel_angles(0, 0);
    }
    activeViewer.set_debug_lines_visible(enabled);
  }
  return enabled;
}

function applyCamera() {
  const target = cameraState.target;
  const cosPitch = Math.cos(cameraState.pitch);
  const eye = [
    target[0] + Math.sin(cameraState.yaw) * cosPitch * cameraState.distance,
    target[1] + Math.sin(cameraState.pitch) * cameraState.distance,
    target[2] + Math.cos(cameraState.yaw) * cosPitch * cameraState.distance,
  ];
  if (eye[1] < FLOOR_CLEARANCE) {
    eye[1] = FLOOR_CLEARANCE;
  }
  cameraState.eye = eye;
  if (activeViewer) {
    activeViewer.set_camera(
      eye[0],
      eye[1],
      eye[2],
      target[0],
      target[1],
      target[2],
    );
  }
  updateReadout();
}

function focusWholeCar() {
  const info = lab.info;
  const bounds = info?.car?.bounds;
  let target = [0, 0.45, 0];
  let span = 4.2;
  if (Array.isArray(bounds?.min) && Array.isArray(bounds?.max)) {
    const min = bounds.min.map(Number);
    const max = bounds.max.map(Number);
    target = [
      (min[0] + max[0]) * 0.5,
      (min[1] + max[1]) * 0.5,
      (min[2] + max[2]) * 0.5,
    ];
    span = Math.max(
      max[0] - min[0],
      max[1] - min[1],
      max[2] - min[2],
      1,
    );
  }
  cameraState.target = target;
  cameraState.distance = clamp(Math.max(span * 1.45, 2.5), 2.5, DISTANCE_MAX);
  cameraState.yaw = 0.58;
  cameraState.pitch = 0.26;
  selectedWheel = "whole";
  syncWheelButtons();
  updateAtlasCaption();
  applyCamera();
}

function focusWheel(which) {
  const info = lab.info;
  if (!activeViewer || !info) {
    return false;
  }
  const raw =
    typeof which === "number" ? which : String(which ?? "").toUpperCase();
  if (raw === "WHOLE" || raw === "ALL" || raw === "CAR") {
    focusWholeCar();
    return true;
  }

  const wheel = findWheel(info, raw);
  if (!wheel) {
    fail(new Error(`Wheel ${String(which)} not found in bay_info`));
    return false;
  }

  const index = WHEEL_KEYS.findIndex((key) => findWheel(info, key) === wheel);
  selectedWheel = index >= 0 ? WHEEL_KEYS[index] : String(wheel.name);

  const target = wheelWorldPosition(info, wheel);
  const radius = Number(wheel.radius);
  cameraState.target = target;
  cameraState.distance = clamp(
    Math.max((Number.isFinite(radius) ? radius : 0.3) * 4.6, 0.85),
    DISTANCE_MIN,
    4.5,
  );
  cameraState.yaw =
    Math.abs(target[0]) + Math.abs(target[2]) > 1e-4
      ? Math.atan2(target[0], target[2])
      : cameraState.yaw;
  cameraState.pitch = 0.18;

  syncWheelButtons();
  updateAtlasCaption();
  applyCamera();
  return true;
}

function buildCarOptions() {
  carSelect.replaceChildren();
  for (const car of CARS) {
    const option = document.createElement("option");
    option.value = car.id;
    option.textContent = car.name;
    carSelect.append(option);
  }
  carSelect.value = DEFAULT_CAR;
}

function loadAtlas(carId, carName) {
  const token = ++atlasToken;
  atlasFigure.classList.remove("visible");
  atlasImage.alt = `${carName} source wheel atlas`;
  atlasImage.onload = () => {
    if (token === atlasToken) {
      atlasFigure.classList.add("visible");
    }
  };
  atlasImage.onerror = () => {
    if (token === atlasToken) {
      atlasFigure.classList.remove("visible");
    }
  };
  atlasImage.src = new URL(
    `../../fixtures/sources/cars/${carId}/wheel/wheel3d.png`,
    window.location.href,
  ).href;
  updateAtlasCaption();
}

function ensureWasm() {
  if (!wasmInit) {
    wasmInit = init().catch((error) => {
      wasmInit = null;
      throw error;
    });
  }
  return wasmInit;
}

function syncCanvasSize() {
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(
    1,
    Math.floor((canvas.clientWidth || stage.clientWidth || window.innerWidth) * ratio),
  );
  const height = Math.max(
    1,
    Math.floor(
      (canvas.clientHeight || stage.clientHeight || window.innerHeight) * ratio,
    ),
  );
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    return true;
  }
  return false;
}

function replaceCanvas() {
  const next = document.createElement("canvas");
  next.id = "view";
  next.className = "view";
  next.setAttribute("aria-label", "Interactive wheel model view");
  canvas.replaceWith(next);
  canvas = next;
}

function stopViewer() {
  if (frameHandle) {
    cancelAnimationFrame(frameHandle);
    frameHandle = 0;
  }
  const viewer = activeViewer;
  activeViewer = null;
  lab.viewer = null;
  lab.info = null;
  if (viewer) {
    try {
      viewer.free?.();
    } catch (error) {
      console.warn("wheel lab: viewer cleanup failed", error);
    }
  }
}

function startLoop(token) {
  let last = performance.now();
  const frame = (now) => {
    if (token !== generation) {
      return;
    }
    const dt = Math.min(0.1, Math.max(0, (now - last) / 1000));
    last = now;
    const viewer = activeViewer;
    if (!viewer) {
      return;
    }

    if (autoSpinInput.checked && !wireframeInput.checked) {
      spin = (spin + dt * 3.5) % (Math.PI * 2);
      spinInput.value = String(spin);
      spinValue.textContent = spin.toFixed(2);
      viewer.set_wheel_angles(steer, spin);
    }

    try {
      viewer.render();
    } catch (error) {
      frameHandle = 0;
      fail(error);
      return;
    }
    frameHandle = requestAnimationFrame(frame);
  };
  frameHandle = requestAnimationFrame(frame);
}

async function loadCar(id = carSelect.value) {
  const car =
    CARS.find((entry) => entry.id === id) ??
    CARS.find((entry) => entry.id === DEFAULT_CAR) ??
    CARS[0];
  const token = ++generation;
  const hadViewer = Boolean(activeViewer);

  clearError();
  window.__wheelLabReady = false;
  carSelect.value = car.id;
  selectedWheel = "whole";
  syncWheelButtons();
  atlasFigure.classList.remove("visible");
  setStatus(`Loading ${car.name}…`, "loading");
  updateReadout();

  try {
    const url = new URL(
      `../../fixtures/cars/${car.id}/${car.id}.sima`,
      window.location.href,
    );
    const response = await fetch(url, { cache: "no-store" });
    if (!response.ok) {
      throw new Error(`${response.status} ${response.statusText}: ${url}`);
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (token !== generation) {
      return;
    }

    setStatus("Starting wasm…", "loading");
    await ensureWasm();
    if (token !== generation) {
      return;
    }

    stopViewer();
    replaceCanvas();
    syncCanvasSize();
    setStatus("Creating viewer…", "loading");

    const viewer = await GarageViewer.create_concrete(
      canvas,
      [car.id],
      [bytes],
      [false],
    );
    if (token !== generation) {
      try {
        viewer.free?.();
      } catch {
        // Stale viewer cleanup is best-effort.
      }
      return;
    }

    const info = JSON.parse(viewer.bay_info());
    if (!info?.car) {
      try {
        viewer.free?.();
      } catch {
        // Ignore cleanup failure; the status below is the useful part.
      }
      throw new Error("renderer returned no car data for this pack");
    }

    activeViewer = viewer;
    lab.viewer = viewer;
    lab.info = info;
    selectedWheel = "whole";

    applyAngles();
    setWireframe(wireframeInput.checked);
    focusWholeCar();
    loadAtlas(car.id, car.name);
    syncWheelButtons();
    startLoop(token);

    setStatus(`Ready · ${car.name}`, "ready");
    window.__wheelLabReady = true;
  } catch (error) {
    if (token !== generation) {
      return;
    }
    fail(error);
    if (hadViewer && activeViewer) {
      window.__wheelLabReady = true;
    }
  }
}

// Orbit controls live on the stage so they survive canvas swaps on reload.
let drag = null;
stage.addEventListener("pointerdown", (event) => {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
  try {
    stage.setPointerCapture(event.pointerId);
  } catch {
    // Synthetic pointers cannot always be captured.
  }
  canvas.classList.add("dragging");
});

stage.addEventListener("pointermove", (event) => {
  if (!drag || drag.id !== event.pointerId) {
    return;
  }
  const dx = event.clientX - drag.x;
  const dy = event.clientY - drag.y;
  drag.x = event.clientX;
  drag.y = event.clientY;
  if (!activeViewer) {
    return;
  }
  cameraState.yaw -= dx * 0.006;
  cameraState.pitch = clamp(
    cameraState.pitch + dy * 0.006,
    PITCH_MIN,
    PITCH_MAX,
  );
  applyCamera();
});

function endDrag(event) {
  if (!drag || drag.id !== event.pointerId) {
    return;
  }
  drag = null;
  canvas.classList.remove("dragging");
}

stage.addEventListener("pointerup", endDrag);
stage.addEventListener("pointercancel", endDrag);
stage.addEventListener("lostpointercapture", endDrag);
stage.addEventListener("contextmenu", (event) => event.preventDefault());

stage.addEventListener(
  "wheel",
  (event) => {
    if (!activeViewer) {
      return;
    }
    event.preventDefault();
    cameraState.distance = clamp(
      cameraState.distance * Math.exp(event.deltaY * 0.001),
      DISTANCE_MIN,
      DISTANCE_MAX,
    );
    applyCamera();
  },
  { passive: false },
);

window.addEventListener("resize", () => {
  if (syncCanvasSize() && activeViewer) {
    activeViewer.resize(canvas.width, canvas.height);
  }
});

steerInput.addEventListener("input", applyAngles);
spinInput.addEventListener("input", applyAngles);
wireframeInput.addEventListener("change", () => {
  setWireframe(wireframeInput.checked);
});

for (const button of wheelButtons) {
  button.addEventListener("click", () => {
    focusWheel(button.dataset.wheel);
  });
}

carSelect.addEventListener("change", () => {
  void loadCar(carSelect.value);
});

reloadButton.addEventListener("click", () => {
  void loadCar(carSelect.value);
});

labToggle.addEventListener("click", () => {
  labUserToggled = true;
  setLabCollapsed(false, true);
});

labCollapse.addEventListener("click", () => {
  labUserToggled = true;
  setLabCollapsed(true, true);
});

labPanel.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    labUserToggled = true;
    setLabCollapsed(true, true);
  }
});

const handleMobileChange = (event) => {
  if (!labUserToggled) {
    setLabCollapsed(event.matches);
  }
};
if (mobileQuery.addEventListener) {
  mobileQuery.addEventListener("change", handleMobileChange);
} else {
  mobileQuery.addListener(handleMobileChange);
}
setLabCollapsed(mobileQuery.matches);

buildCarOptions();
const requestedCar = new URLSearchParams(window.location.search).get("car");
const initialCar = CARS.some((car) => car.id === requestedCar)
  ? requestedCar
  : DEFAULT_CAR;
carSelect.value = initialCar;
setWireframe(wireframeInput.checked);
void loadCar(initialCar);
