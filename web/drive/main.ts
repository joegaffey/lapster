import init, { DriveViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { setupTouchControls } from "../src/ui/touch.ts";

declare global {
  interface Window {
    __driveViewerReady?: boolean;
    __driveViewerError?: string;
    __driveViewer?: DriveViewer;
  }
}

const PACK_VERSION = "cp6";
setupHud();

const params = new URLSearchParams(window.location.search);
const TRACK_URL =
  params.get("track") ?? `../../fixtures/tracks/skidpad05/pack.sima?v=${PACK_VERSION}`;
const CAR_URL =
  params.get("car") ?? `../../fixtures/cars/mazda-rx7/mazda-rx7.sima?v=${PACK_VERSION}`;
const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
const status = document.querySelector<HTMLElement>("#status")!;
const asset = document.querySelector<HTMLElement>("#asset")!;
const speedLabel = document.querySelector<HTMLElement>("#speed")!;
const rpmLabel = document.querySelector<HTMLElement>("#rpm")!;
const gearLabel = document.querySelector<HTMLElement>("#gear")!;
const contactsLabel = document.querySelector<HTMLElement>("#contacts")!;
const rackTorqueLabel = document.querySelector<HTMLElement>("#rack-torque")!;
const tickLabel = document.querySelector<HTMLElement>("#tick")!;
const stepP50Label = document.querySelector<HTMLElement>("#step-p50")!;
const stepP99Label = document.querySelector<HTMLElement>("#step-p99")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const wheelsLabel = document.querySelector<HTMLElement>("#wheels")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset")!;

const keys = new Set<string>();
let steer = 0;
let gear = 1;
let lastSpeedMps = 0;
let lastFrame = performance.now() / 1000;
let frames = 0;
let accumulated = 0;

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

function updateHud(telemetry: any) {
  speedLabel.textContent = telemetry.speedKph.toFixed(1);
  rpmLabel.textContent = String(Math.round(telemetry.rpm));
  gearLabel.textContent = String(telemetry.gear);
  contactsLabel.textContent = `${telemetry.contacts}/4`;
  rackTorqueLabel.textContent = `${telemetry.steeringRackTorque.toFixed(1)} Nm`;
  tickLabel.textContent = String(telemetry.tick);
  stepP50Label.textContent = `${telemetry.stepP50Ms.toFixed(3)} ms`;
  stepP99Label.textContent = `${telemetry.stepP99Ms.toFixed(3)} ms`;

  wheelsLabel.replaceChildren();
  for (const wheel of telemetry.wheels) {
    const item = document.createElement("li");
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${wheel.name}${wheel.contact ? "" : " air"}`;
    const detail = document.createElement("span");
    detail.className = "muted";
    detail.textContent = `${Math.round(wheel.load)} N  slip ${wheel.slipRatio.toFixed(
      2,
    )}  μ ${wheel.surfaceFriction.toFixed(2)}`;
    item.append(name, detail);
    wheelsLabel.append(item);
  }
}

function reset() {
  const viewer = window.__driveViewer;
  if (!viewer) return;
  viewer.reset();
  steer = 0;
  gear = 1;
  lastSpeedMps = 0;
  keys.clear();
  lastFrame = performance.now() / 1000;
}

async function boot() {
  status.textContent = "loading wasm";
  await init();

  status.textContent = "loading packs";
  const [trackPack, carPack] = await Promise.all([
    loadPack(TRACK_URL),
    loadPack(CAR_URL),
  ]);

  resizeCanvas();
  status.textContent = "creating physics";
  const viewer = await DriveViewer.create(canvas, trackPack.bytes, carPack.bytes);
  window.__driveViewer = viewer;

  const info = JSON.parse(viewer.info());
  asset.textContent = `${info.asset.id} + ${info.car.id}`;

  window.addEventListener("keydown", (event) => {
    const key = event.key.toLowerCase();
    if ([
      "w",
      "a",
      "s",
      "d",
      "r",
      "0",
      "1",
      "2",
      "3",
      "4",
      "5",
      "6",
      "arrowleft",
      "arrowright",
      "arrowup",
      "arrowdown",
    ].includes(key)) {
      event.preventDefault();
    }
    keys.add(key);
    if (key === "r") {
      reset();
    }
    if (key === "0") {
      gear = -1;
    } else if (key >= "1" && key <= "6") {
      gear = Number(key);
    }
  });
  window.addEventListener("keyup", (event) => {
    keys.delete(event.key.toLowerCase());
  });
  window.addEventListener("blur", () => keys.clear());
  resetButton.addEventListener("click", reset);
  setupTouchControls(keys, reset);

  const observer = new ResizeObserver(() => {
    if (resizeCanvas()) {
      viewer.resize(canvas.width, canvas.height);
    }
  });
  observer.observe(canvas);

  status.textContent = "ready";
  status.className = "status ready";
  window.__driveViewerReady = true;

  const frame = (nowMs: number) => {
    const now = nowMs / 1000;
    const delta = Math.min(Math.max(now - lastFrame, 0), 0.05);
    lastFrame = now;

    const steerLeft = keys.has("a") || keys.has("arrowleft");
    const steerRight = keys.has("d") || keys.has("arrowright");
    const targetSteer = (steerLeft ? 1 : 0) - (steerRight ? 1 : 0);
    steer += (targetSteer - steer) * Math.min(1, delta * 8);

    const forwardKey = keys.has("w") || keys.has("arrowup");
    const reverseKey = keys.has("s") || keys.has("arrowdown");
    let throttle = 0;
    let brake = 0;

    if (forwardKey && reverseKey) {
      brake = 1;
    } else if (forwardKey) {
      if (gear < 0 && lastSpeedMps > 0.5) {
        brake = 1;
      } else {
        gear = 1;
        throttle = 1;
      }
    } else if (reverseKey) {
      if (gear > 0 && lastSpeedMps > 0.5) {
        brake = 1;
      } else {
        gear = -1;
        throttle = 1;
      }
    }

    try {
      viewer.update(now, throttle, brake, steer, gear);
      const telemetry = JSON.parse(viewer.telemetry());
      lastSpeedMps = telemetry.speedMps;
      updateHud(telemetry);

      frames += 1;
      accumulated += delta;
      if (accumulated >= 0.5) {
        fpsLabel.textContent = (frames / accumulated).toFixed(1);
        frames = 0;
        accumulated = 0;
      }
      requestAnimationFrame(frame);
    } catch (error) {
      fail(error);
    }
  };

  requestAnimationFrame(frame);
}

function fail(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  window.__driveViewerError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
