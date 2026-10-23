import init, { DriveViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { APP_VERSION } from "../src/version.generated.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";
import { setupTouchControls } from "../src/ui/touch.ts";
import { setupCarSetupDialog } from "../src/ui/car-setup.ts";
import { setupAssistsDialog } from "../src/ui/assists.ts";

declare global {
  interface Window {
    __driveViewerReady?: boolean;
    __driveViewerError?: string;
    __driveViewer?: DriveViewer;
  }
}

const PACK_VERSION = encodeURIComponent(APP_VERSION);
setupHud();
checkForUpdate();

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
const cameraButton = document.querySelector<HTMLButtonElement>("#camera-mode")!;
const bodyToggle = document.querySelector<HTMLButtonElement>("#body-toggle")!;

const keys = new Set<string>();
const STOP_SPEED_MPS = 0.3;
let steer = 0;
let gear = 1;
let lastSpeedMps = 0;
let previousForward = false;
let previousReverse = false;
let cameraMode: "chase" | "bonnet" | "cockpit" = "chase";
let bodyVisible = true;
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

function setBodyVisible(visible: boolean) {
  bodyVisible = visible;
  window.__driveViewer?.set_body_visible(visible);
  bodyToggle.textContent = visible ? "Body: Visible" : "Body: Hidden";
}

function setCameraMode(mode: "chase" | "bonnet" | "cockpit") {
  cameraMode = mode;
  window.__driveViewer?.set_camera_mode(mode);
  cameraButton.textContent =
    mode === "cockpit"
      ? "Camera: Cockpit"
      : mode === "bonnet"
        ? "Camera: Bonnet"
        : "Camera: Chase";
  if (mode === "chase") {
    setBodyVisible(true);
  }
}

function toggleCameraMode() {
  setCameraMode(
    cameraMode === "chase"
      ? "bonnet"
      : cameraMode === "bonnet"
        ? "cockpit"
        : "chase",
  );
}

function reset() {
  const viewer = window.__driveViewer;
  if (!viewer) return;
  viewer.reset();
  steer = 0;
  gear = 1;
  lastSpeedMps = 0;
  previousForward = false;
  previousReverse = false;
  keys.clear();
  lastFrame = performance.now() / 1000;
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
    if (key === "c") {
      toggleCameraMode();
    }
    if (key === "0") {
      if (Math.abs(lastSpeedMps) < STOP_SPEED_MPS) {
        gear = -1;
      }
    } else if (key >= "1" && key <= "6") {
      gear = Number(key);
    }
  });
  window.addEventListener("keyup", (event) => {
    keys.delete(event.key.toLowerCase());
  });
  window.addEventListener("blur", () => keys.clear());
  resetButton.addEventListener("click", reset);
  cameraButton.addEventListener("click", toggleCameraMode);
  bodyToggle.addEventListener("click", () => setBodyVisible(!bodyVisible));
  setCameraMode("chase");
  setupCarSetupDialog(viewer, info.car.id, info.asset.id);
  setupAssistsDialog(viewer);
  setupTouchControls(keys);

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
    const forwardPressed = forwardKey && !previousForward;
    const reversePressed = reverseKey && !previousReverse;
    const stopped = Math.abs(lastSpeedMps) < STOP_SPEED_MPS;
    previousForward = forwardKey;
    previousReverse = reverseKey;
    let throttle = 0;
    let brake = 0;

    if (forwardKey && reverseKey) {
      brake = 1;
    } else if (forwardKey) {
      if (gear < 0) {
        brake = 1;
        if (stopped && forwardPressed) {
          gear = 1;
        }
      } else {
        gear = 1;
        throttle = 1;
      }
    } else if (reverseKey) {
      if (gear > 0) {
        brake = 1;
        if (stopped && reversePressed) {
          gear = -1;
        }
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
