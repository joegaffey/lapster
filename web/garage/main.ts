import init, { GarageViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { APP_VERSION } from "../src/version.generated.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";

declare global {
  interface Window {
    __garageViewerReady?: boolean;
    __garageViewerError?: string;
    __garageViewer?: GarageViewer;
  }
}

const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
const status = document.querySelector<HTMLElement>("#status")!;
const asset = document.querySelector<HTMLElement>("#asset")!;
const carStats = document.querySelector<HTMLElement>("#car-stats")!;
const wheels = document.querySelector<HTMLElement>("#wheels")!;
const stats = document.querySelector<HTMLElement>("#stats")!;
const surfaces = document.querySelector<HTMLElement>("#surfaces")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const frameLabel = document.querySelector<HTMLElement>("#frame-ms")!;
const steerInput = document.querySelector<HTMLInputElement>("#steer")!;
const spinInput = document.querySelector<HTMLInputElement>("#spin")!;
const steerValue = document.querySelector<HTMLElement>("#steer-value")!;
const spinValue = document.querySelector<HTMLElement>("#spin-value")!;
const animateInput = document.querySelector<HTMLInputElement>("#animate")!;

setupHud();
checkForUpdate();

const params = new URLSearchParams(window.location.search);
const TRACK_URL =
  params.get("track") ??
  `../../fixtures/tracks/skidpad05/pack.sima?v=${encodeURIComponent(APP_VERSION)}`;
const CAR_URL =
  params.get("car") ??
  `../../fixtures/cars/mazda-rx7/mazda-rx7.sima?v=${encodeURIComponent(APP_VERSION)}`;

const SURFACE_COLORS: Record<number, string> = {
  0: "#33a6ff",
  1: "#ff4d33",
  2: "#ffe633",
  3: "#cc4dff",
  4: "#33ffb3",
};

async function loadPack(url: string) {
  try {
    return await loadPackFromUrl(url, new IndexedDbPackCache());
  } catch {
    return await loadPackFromUrl(url);
  }
}

function resizeCanvas() {
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
  const height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    return true;
  }
  return false;
}

function addRows(target: HTMLElement, rows: Array<[string, string]>) {
  target.replaceChildren();
  for (const [label, value] of rows) {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = value;
    target.append(dt, dd);
  }
}

function renderDebug(debug: any) {
  asset.textContent = `${debug.asset.id} + ${debug.car.id}`;

  addRows(carStats, [
    ["mass", `${debug.car.mass.toFixed(0)} kg`],
    ["wheelbase", `${debug.car.wheelbase.toFixed(2)} m`],
    ["track width", debug.car.trackWidth.map((value: number) => value.toFixed(2)).join(" / ")],
    ["meshes", String(debug.car.meshCount)],
    ["steering wheel", debug.car.steeringWheel ? "yes" : "no"],
  ]);

  wheels.replaceChildren();
  for (const wheel of debug.car.wheels) {
    const item = document.createElement("li");
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${wheel.name} ${wheel.front ? "front" : "rear"}${wheel.driven ? " drive" : ""}`;
    const detail = document.createElement("span");
    detail.className = "muted";
    detail.textContent = `r ${wheel.radius.toFixed(2)} @ ${wheel.position
      .map((value: number) => value.toFixed(2))
      .join(", ")}`;
    item.append(name, detail);
    wheels.append(item);
  }

  addRows(stats, [
    ["vertices", String(debug.stats.vertices)],
    ["triangles", String(debug.stats.triangles)],
    ["track tris", String(debug.stats.trackTriangles)],
    ["car tris", String(debug.stats.carTriangles)],
    ["draw calls", String(debug.stats.drawCalls)],
    ["collision tris", String(debug.stats.collisionTriangles)],
  ]);

  surfaces.replaceChildren();
  for (const surface of debug.surfaces) {
    const item = document.createElement("li");
    const swatch = document.createElement("span");
    swatch.className = "name";
    swatch.style.color = SURFACE_COLORS[surface.id] ?? "#e5e7eb";
    swatch.textContent = `${surface.id}: ${surface.name}`;
    const detail = document.createElement("span");
    detail.className = "muted";
    detail.textContent = `μ ${surface.friction.toFixed(2)}`;
    item.append(swatch, detail);
    surfaces.append(item);
  }
}

async function boot() {
  status.textContent = "starting";
  if (await setupPwa()) {
    return;
  }
  status.textContent = "loading wasm";
  await init();

  status.textContent = "loading packs";
  const [trackPack, carPack] = await Promise.all([
    loadPack(TRACK_URL),
    loadPack(CAR_URL),
  ]);

  resizeCanvas();
  status.textContent = "creating renderer";
  const viewer = await GarageViewer.create(canvas, trackPack.bytes, carPack.bytes);
  window.__garageViewer = viewer;

  const debug = JSON.parse(viewer.debug_info());
  renderDebug(debug);

  let steer = 0;
  let spin = 0;
  let animating = false;

  const applyAngles = () => {
    steerValue.textContent = steer.toFixed(2);
    spinValue.textContent = spin.toFixed(2);
    viewer.set_wheel_angles(steer, spin);
  };

  steerInput.addEventListener("input", () => {
    steer = Number(steerInput.value);
    applyAngles();
  });
  spinInput.addEventListener("input", () => {
    spin = Number(spinInput.value);
    applyAngles();
  });
  animateInput.addEventListener("change", () => {
    animating = animateInput.checked;
  });

  const observer = new ResizeObserver(() => {
    if (resizeCanvas()) {
      viewer.resize(canvas.width, canvas.height);
    }
  });
  observer.observe(canvas);

  let last = performance.now();
  let frames = 0;
  let accumulated = 0;
  let frameTime = 0;

  const frame = (now: number) => {
    const delta = now - last;
    last = now;
    frames += 1;
    accumulated += delta;
    frameTime += delta;

    if (animating) {
      spin = (spin + (delta / 1000) * 4.0) % (Math.PI * 2);
      spinInput.value = String(spin);
      applyAngles();
    }

    if (accumulated >= 500) {
      fpsLabel.textContent = ((frames * 1000) / accumulated).toFixed(1);
      frameLabel.textContent = `${(frameTime / frames).toFixed(2)} ms`;
      frames = 0;
      accumulated = 0;
      frameTime = 0;
    }

    try {
      viewer.render();
      requestAnimationFrame(frame);
    } catch (error) {
      fail(error);
    }
  };

  status.textContent = "ready";
  status.className = "status ready";
  window.__garageViewerReady = true;
  requestAnimationFrame(frame);
}

function fail(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  window.__garageViewerError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
