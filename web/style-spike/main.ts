import init, { TrackViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl, withPackVersion } from "../src/assets/loader.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";
import {
  setupTrackCamera,
  type CameraInfo,
  type OrbitCameraController,
} from "../src/ui/track-camera.ts";
import { hideLoading, showLoadingFailure } from "../src/ui/loading.ts";
import { APP_VERSION } from "../src/version.generated.ts";

declare global {
  interface Window {
    __styleSpikeReady?: boolean;
    __styleSpikeError?: string;
    __styleSpike?: TrackViewer;
  }
}

type QualityChoice = "auto" | "high" | "medium" | "low";
type AppliedTier = "high" | "medium" | "low";

const QUALITY_KEY = "lapster.styleSpike.quality.v1";
const TIME_KEY = "lapster.styleSpike.time.v1";
const GRASS_KEY = "lapster.styleSpike.grass.v1";
const TREE_MASS_KEY = "lapster.styleSpike.treeMass.v1";
const BUSH_MASS_KEY = "lapster.styleSpike.bushMass.v1";
const MOBILE_QUERY = "(pointer: coarse), (max-width: 760px)";

const TRACKS: Record<string, string> = {
  drift: "../../fixtures/tracks/drift-sketchfab/pack.sima",
  skidpad: "../../fixtures/tracks/skidpad05/pack.sima",
};
const CAR_URL = "../../fixtures/cars/minimal/pack.sima";

type CameraMode = "orbit" | "chase" | "bonnet" | "cockpit" | "isometric" | "fly";
type Vec3 = [number, number, number];

interface StyleCarInfo {
  position: Vec3;
  forward: Vec3;
  right: Vec3;
  up: Vec3;
  speedKph: number;
  rpm: number;
  gear: number;
  bounds: { min: Vec3; max: Vec3 };
}

interface CameraShot {
  eye: Vec3;
  target: Vec3;
  fovY: number;
}

interface StyleCameraTuning {
  bonnet: CameraShot;
  cockpit: CameraShot;
  isometric: {
    offset: Vec3;
    focusHeight: number;
    fovY: number;
    fitAspectFloor: number;
    fitPower: number;
    fitMin: number;
    fitMax: number;
  };
  chase: {
    aspectNear: number;
    aspectFar: number;
    backNear: number;
    backFar: number;
    heightNear: number;
    heightFar: number;
    lookNear: number;
    lookFar: number;
    targetHeight: number;
    fovY: number;
    minEyeY: number;
  };
}

const CONTROL_KEYS = new Set([
  "KeyW",
  "KeyA",
  "KeyS",
  "KeyD",
  "KeyQ",
  "KeyE",
  "ShiftLeft",
  "ShiftRight",
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Space",
  "KeyR",
]);

const add = (a: Vec3, b: Vec3): Vec3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (v: Vec3, amount: number): Vec3 => [v[0] * amount, v[1] * amount, v[2] * amount];
const length = (v: Vec3) => Math.hypot(v[0], v[1], v[2]);
const normalize = (v: Vec3): Vec3 => {
  const len = length(v);
  return len > 1e-8 ? [v[0] / len, v[1] / len, v[2] / len] : [0, 0, -1];
};
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const toVec3 = (value: number[]): Vec3 => [value[0] ?? 0, value[1] ?? 0, value[2] ?? 0];
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const rotateAxis = (v: Vec3, axis: Vec3, angle: number): Vec3 => {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  const k = normalize(axis);
  const crossKV = cross(k, v);
  const d = dot(k, v) * (1 - c);
  return [
    v[0] * c + crossKV[0] * s + k[0] * d,
    v[1] * c + crossKV[1] * s + k[1] * d,
    v[2] * c + crossKV[2] * s + k[2] * d,
  ];
};

const TIERS: Record<
  AppliedTier,
  {
    renderScale: number;
    msaa: number;
    shadows: "high" | "hybrid" | "baked" | "off";
    vegetation: AppliedTier;
    grassDensity: number;
    grassTexture: boolean;
  }
> = {
  high: {
    renderScale: 1,
    msaa: 4,
    shadows: "hybrid",
    vegetation: "high",
    grassDensity: 100,
    grassTexture: true,
  },
  medium: {
    renderScale: 0.85,
    msaa: 2,
    shadows: "baked",
    vegetation: "medium",
    grassDensity: 60,
    grassTexture: true,
  },
  low: {
    renderScale: 0.67,
    msaa: 1,
    shadows: "off",
    vegetation: "low",
    grassDensity: 0,
    grassTexture: true,
  },
};

const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
const status = document.querySelector<HTMLElement>("#status")!;
const backend = document.querySelector<HTMLElement>("#backend")!;
const appliedTierLabel = document.querySelector<HTMLElement>("#applied-tier")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const frameLabel = document.querySelector<HTMLElement>("#frame-ms")!;
const treesLabel = document.querySelector<HTMLElement>("#trees")!;
const bushesLabel = document.querySelector<HTMLElement>("#bushes")!;
const grassLabel = document.querySelector<HTMLElement>("#grass")!;
const treeCardsLabel = document.querySelector<HTMLElement>("#tree-cards")!;
const bushCardsLabel = document.querySelector<HTMLElement>("#bush-cards")!;
const drawsLabel = document.querySelector<HTMLElement>("#draws")!;
const speedLabel = document.querySelector<HTMLElement>("#speed")!;
const gearLabel = document.querySelector<HTMLElement>("#gear")!;
const qualitySelect = document.querySelector<HTMLSelectElement>("#quality-select")!;
const trackSelect = document.querySelector<HTMLSelectElement>("#track-select")!;
const timeInput = document.querySelector<HTMLInputElement>("#time-of-day")!;
const cameraSelect = document.querySelector<HTMLSelectElement>("#camera-mode")!;
const cameraHint = document.querySelector<HTMLElement>("#camera-hint")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset-camera")!;
const grassSlider = document.querySelector<HTMLInputElement>("#grass-density")!;
const grassValue = document.querySelector<HTMLElement>("#grass-value")!;
const treeMassSlider = document.querySelector<HTMLInputElement>("#tree-mass-slider")!;
const treeMassValue = document.querySelector<HTMLElement>("#tree-mass-value")!;
const bushMassSlider = document.querySelector<HTMLInputElement>("#bush-mass-slider")!;
const bushMassValue = document.querySelector<HTMLElement>("#bush-mass-value")!;
const hud = document.querySelector<HTMLElement>("#hud")!;
const toggleHud = document.querySelector<HTMLButtonElement>("#toggle-hud")!;

let viewer: TrackViewer | undefined;
let renderScale = 1;
let selectedQuality: QualityChoice = "auto";
let appliedTier: AppliedTier = "high";
let autoTier: AppliedTier = "medium";
let frameTimes: number[] = [];
let stableChecks = 0;
let lastQualityCheck = 0;
let appliedGrass = 100;
let appliedTreeMass = 0;
let appliedBushMass = 0;
let grassOverride = false;
let cameraController: OrbitCameraController | undefined;
let initialOrbitHome: CameraInfo | null = null;
let orbitHome: CameraInfo | null = null;
let flyHome: CameraInfo | null = null;
let styleCar: StyleCarInfo | null = null;
let cameraTuning: StyleCameraTuning | null = null;
let cameraMode: CameraMode = "orbit";
let flyEye: Vec3 = [0, 0, 0];
let flyForward: Vec3 = [0, 0, -1];
let driveSteer = 0;
const pressedKeys = new Set<string>();

checkForUpdate();

function loadQuality(): QualityChoice {
  const param = new URLSearchParams(window.location.search).get("quality");
  if (param === "auto" || param === "high" || param === "medium" || param === "low") {
    return param;
  }
  try {
    const stored = localStorage.getItem(QUALITY_KEY);
    if (stored === "auto" || stored === "high" || stored === "medium" || stored === "low") {
      return stored;
    }
  } catch {
  }
  return "auto";
}

function saveQuality(value: QualityChoice): void {
  try {
    localStorage.setItem(QUALITY_KEY, value);
  } catch {
  }
}

function loadTime(): number {
  try {
    const stored = Number(localStorage.getItem(TIME_KEY));
    if (Number.isFinite(stored) && stored >= 6 && stored <= 20) {
      return stored;
    }
  } catch {
  }
  return 15;
}

function readNumber(key: string, param: string): number | null {
  const query = Number(new URLSearchParams(window.location.search).get(param));
  if (Number.isFinite(query) && query >= 0) {
    return Math.floor(query);
  }
  try {
    const stored = Number(localStorage.getItem(key));
    if (Number.isFinite(stored) && stored >= 0) {
      return Math.floor(stored);
    }
  } catch {
  }
  return null;
}

function saveNumber(key: string, value: number): void {
  try {
    localStorage.setItem(key, String(value));
  } catch {
  }
}

function currentCameraInfo(): CameraInfo {
  return JSON.parse(viewer!.camera_info()) as CameraInfo;
}

function applyCameraInfo(info: CameraInfo): void {
  viewer?.set_camera(
    info.eye[0],
    info.eye[1],
    info.eye[2],
    info.target[0],
    info.target[1],
    info.target[2],
  );
  viewer?.set_camera_fov(info.fovY);
}

function carCameraInfo(mode: CameraMode): CameraInfo | null {
  if (!styleCar || !cameraTuning) {
    return null;
  }
  const position = toVec3(styleCar.position);
  const forward = normalize(toVec3(styleCar.forward));
  const right = normalize(toVec3(styleCar.right));
  const up = normalize(toVec3(styleCar.up));
  const localToWorld = (local: Vec3): Vec3 =>
    add(
      position,
      add(
        add(scale(right, local[0]), scale(up, local[1])),
        scale(forward, -local[2]),
      ),
    );

  if (mode === "bonnet") {
    const shot = cameraTuning.bonnet;
    return {
      eye: localToWorld(shot.eye),
      target: localToWorld(shot.target),
      fovY: shot.fovY,
    };
  }
  if (mode === "cockpit") {
    const shot = cameraTuning.cockpit;
    return {
      eye: localToWorld(shot.eye),
      target: localToWorld(shot.target),
      fovY: shot.fovY,
    };
  }
  if (mode === "isometric") {
    const tuning = cameraTuning.isometric;
    const aspect = canvas.clientWidth / Math.max(canvas.clientHeight, 1);
    const fit = Math.pow(
      1 / Math.max(aspect, tuning.fitAspectFloor),
      tuning.fitPower,
    );
    const offset = scale(
      tuning.offset,
      Math.min(tuning.fitMax, Math.max(tuning.fitMin, fit)),
    );
    const focus = add(position, scale(up, tuning.focusHeight));
    return { eye: add(focus, offset), target: focus, fovY: tuning.fovY };
  }
  if (mode === "chase") {
    const tuning = cameraTuning.chase;
    const aspect = canvas.clientWidth / Math.max(canvas.clientHeight, 1);
    const range = Math.max(tuning.aspectFar - tuning.aspectNear, 0.0001);
    const landscape = Math.min(
      1,
      Math.max(0, (aspect - tuning.aspectNear) / range),
    );
    const back = tuning.backNear + (tuning.backFar - tuning.backNear) * landscape;
    const height =
      tuning.heightNear + (tuning.heightFar - tuning.heightNear) * landscape;
    const look = tuning.lookNear + (tuning.lookFar - tuning.lookNear) * landscape;
    const eye = add(position, add(scale(up, height), scale(forward, -back)));
    eye[1] = Math.max(eye[1], tuning.minEyeY);
    const target = add(
      position,
      add(scale(up, tuning.targetHeight), scale(forward, -look)),
    );
    return { eye, target, fovY: tuning.fovY };
  }
  return null;
}

function setCameraMode(mode: CameraMode): void {
  if (mode !== "orbit" && !styleCar) {
    mode = "orbit";
  }
  if (cameraMode === "orbit" && mode !== "orbit") {
    orbitHome = currentCameraInfo();
  }
  cameraMode = mode;
  pressedKeys.clear();
  cameraSelect.value = mode;
  if (mode === "orbit") {
    cameraController?.setEnabled(true);
    if (orbitHome) {
      cameraController?.setHome(orbitHome, false);
    }
    cameraHint.textContent = "Drag to orbit · right-drag/shift-drag to pan · wheel to zoom · arrows orbit · WASD pan · Q/E zoom";
    return;
  }
  cameraController?.setEnabled(false);
  if (mode === "fly") {
    const info = currentCameraInfo();
    flyHome = info;
    flyEye = toVec3(info.eye);
    flyForward = normalize(sub(toVec3(info.target), flyEye));
    cameraHint.textContent = "WASD move · Q/E down/up · arrows/drag look · Shift boost";
    return;
  }
  const info = carCameraInfo(mode);
  if (info) {
    applyCameraInfo(info);
  }
  cameraHint.textContent =
    mode === "chase"
      ? "Chase · WASD/arrows drive · Space handbrake · R reset car"
      : mode === "isometric"
        ? "Isometric · WASD/arrows drive · R reset car"
        : `${mode[0].toUpperCase()}${mode.slice(1)} camera · WASD/arrows drive · R reset car`;
}

function resetCameraView(): void {
  if (cameraMode === "orbit") {
    if (initialOrbitHome) {
      orbitHome = initialOrbitHome;
      cameraController?.setHome(initialOrbitHome, false);
    }
    return;
  }
  if (cameraMode === "fly") {
    if (flyHome) {
      flyEye = toVec3(flyHome.eye);
      flyForward = normalize(sub(toVec3(flyHome.target), flyEye));
      applyCameraInfo({
        eye: flyEye,
        target: add(flyEye, scale(flyForward, 10)),
        fovY: flyHome.fovY,
      });
    }
    return;
  }
  const info = carCameraInfo(cameraMode);
  if (info) {
    applyCameraInfo(info);
  }
}

function isTypingTarget(target: EventTarget | null): boolean {
  if (target instanceof HTMLTextAreaElement) {
    return true;
  }
  if (target instanceof HTMLInputElement) {
    return ["text", "search", "email", "password", "number", "url", "tel"].includes(
      target.type,
    );
  }
  return target instanceof HTMLElement && target.isContentEditable;
}

function applyFlyMouseLook(dx: number, dy: number): void {
  if (cameraMode !== "fly") {
    return;
  }
  if (dx !== 0) {
    flyForward = normalize(rotateAxis(flyForward, [0, 1, 0], -dx * 0.0035));
  }
  if (dy !== 0) {
    let rightAxis = cross(flyForward, [0, 1, 0]);
    if (length(rightAxis) < 1e-5) {
      rightAxis = [1, 0, 0];
    }
    flyForward = normalize(rotateAxis(flyForward, rightAxis, -dy * 0.0035));
  }
}

function updateFly(deltaSeconds: number): void {
  if (cameraMode !== "fly") {
    return;
  }
  const flat = normalize([flyForward[0], 0, flyForward[2]]);
  const right = normalize(cross(flat, [0, 1, 0]));
  const forwardInput =
    (pressedKeys.has("KeyW") ? 1 : 0) - (pressedKeys.has("KeyS") ? 1 : 0);
  const strafeInput =
    (pressedKeys.has("KeyD") ? 1 : 0) - (pressedKeys.has("KeyA") ? 1 : 0);
  const verticalInput =
    (pressedKeys.has("KeyE") ? 1 : 0) - (pressedKeys.has("KeyQ") ? 1 : 0);
  let move = add(scale(flat, forwardInput), scale(right, strafeInput));
  move = [move[0], verticalInput, move[2]];
  if (length(move) > 1e-6) {
    move = normalize(move);
    const speed =
      pressedKeys.has("ShiftLeft") || pressedKeys.has("ShiftRight") ? 35 : 10;
    flyEye = add(flyEye, scale(move, speed * deltaSeconds));
  }

  const yawInput =
    (pressedKeys.has("ArrowRight") ? 1 : 0) - (pressedKeys.has("ArrowLeft") ? 1 : 0);
  const pitchInput =
    (pressedKeys.has("ArrowUp") ? 1 : 0) - (pressedKeys.has("ArrowDown") ? 1 : 0);
  if (yawInput !== 0) {
    flyForward = normalize(
      rotateAxis(flyForward, [0, 1, 0], -yawInput * 1.6 * deltaSeconds),
    );
  }
  if (pitchInput !== 0) {
    let rightAxis = cross(flyForward, [0, 1, 0]);
    if (length(rightAxis) < 1e-5) {
      rightAxis = [1, 0, 0];
    }
    flyForward = normalize(
      rotateAxis(flyForward, rightAxis, pitchInput * 1.2 * deltaSeconds),
    );
  }
  const target = add(flyEye, scale(flyForward, 10));
  viewer?.set_camera(flyEye[0], flyEye[1], flyEye[2], target[0], target[1], target[2]);
}

function updateOrbitKeyboard(deltaSeconds: number): void {
  if (!cameraController) {
    return;
  }
  const boost = pressedKeys.has("ShiftLeft") || pressedKeys.has("ShiftRight") ? 2.5 : 1;
  const orbitX =
    (pressedKeys.has("ArrowRight") ? 1 : 0) -
    (pressedKeys.has("ArrowLeft") ? 1 : 0);
  const orbitY =
    (pressedKeys.has("ArrowUp") ? 1 : 0) -
    (pressedKeys.has("ArrowDown") ? 1 : 0);
  const panX =
    (pressedKeys.has("KeyD") ? 1 : 0) - (pressedKeys.has("KeyA") ? 1 : 0);
  const panY =
    (pressedKeys.has("KeyW") ? 1 : 0) - (pressedKeys.has("KeyS") ? 1 : 0);
  const zoom =
    (pressedKeys.has("KeyE") ? 1 : 0) - (pressedKeys.has("KeyQ") ? 1 : 0);
  cameraController.nudge(
    deltaSeconds * boost,
    orbitX,
    orbitY,
    panX,
    panY,
    zoom,
  );
}

function updateCarHud(info: StyleCarInfo | null): void {
  if (!info) {
    speedLabel.textContent = "—";
    gearLabel.textContent = "—";
    return;
  }
  speedLabel.textContent = `${info.speedKph.toFixed(1)} km/h`;
  const gear = info.gear ?? 0;
  gearLabel.textContent = gear < 0 ? "R" : gear === 0 ? "N" : String(gear);
}

function updateDrive(deltaSeconds: number): void {
  if (!viewer || !styleCar) {
    return;
  }
  const isCarView =
    cameraMode === "chase" ||
    cameraMode === "bonnet" ||
    cameraMode === "cockpit" ||
    cameraMode === "isometric";
  let throttle = 0;
  let brake = 0;
  let handbrake = 0;
  let steerTarget = 0;
  if (isCarView) {
    throttle =
      pressedKeys.has("KeyW") || pressedKeys.has("ArrowUp") ? 1 : 0;
    brake =
      pressedKeys.has("KeyS") || pressedKeys.has("ArrowDown") ? 1 : 0;
    handbrake = pressedKeys.has("Space") ? 1 : 0;
    const left = pressedKeys.has("KeyA") || pressedKeys.has("ArrowLeft");
    const right = pressedKeys.has("KeyD") || pressedKeys.has("ArrowRight");
    steerTarget = (left ? 1 : 0) - (right ? 1 : 0);
  } else {
    brake = 1;
  }
  driveSteer += (steerTarget - driveSteer) * Math.min(1, deltaSeconds * 8);
  viewer.update_car(
    performance.now() / 1000,
    throttle,
    brake,
    driveSteer,
    handbrake,
    cameraMode === "cockpit",
  );
  const info = JSON.parse(viewer.style_car_info()) as StyleCarInfo;
  styleCar = info;
  updateCarHud(info);
  if (isCarView) {
    const cameraInfo = carCameraInfo(cameraMode);
    if (cameraInfo) {
      applyCameraInfo(cameraInfo);
    }
  }
}

function initialAutoTier(): AppliedTier {
  const mobile = window.matchMedia?.(MOBILE_QUERY)?.matches ?? false;
  if (mobile) {
    return "low";
  }
  return "gpu" in navigator ? "high" : "medium";
}

function currentTrackKey(): string {
  const track = new URLSearchParams(window.location.search).get("track") ?? "";
  return track.includes("drift") ? "drift" : "skidpad";
}

function resizeCanvas(): boolean {
  const ratio = Math.min(window.devicePixelRatio || 1, 2) * renderScale;
  const width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
  const height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    return true;
  }
  return false;
}

function applyTier(tier: AppliedTier): void {
  appliedTier = tier;
  const settings = TIERS[tier];
  renderScale = settings.renderScale;
  if (viewer) {
    if (resizeCanvas()) {
      viewer.resize(canvas.width, canvas.height);
    }
    viewer.set_msaa_samples(settings.msaa);
    viewer.set_environment_shadow_mode(settings.shadows);
    viewer.set_vegetation_tier(settings.vegetation);
    viewer.set_grass_texture_enabled(settings.grassTexture);
    if (!grassOverride) {
      applyGrassDensity(settings.grassDensity);
    }
  }
  appliedTierLabel.textContent = `${selectedQuality === "auto" ? "Auto → " : ""}${tier}`;
  if (viewer) {
    updateStats();
  }
}

function chooseQuality(choice: QualityChoice): void {
  selectedQuality = choice;
  saveQuality(choice);
  qualitySelect.value = choice;
  stableChecks = 0;
  if (choice === "auto") {
    autoTier = initialAutoTier();
    frameTimes = [];
    lastQualityCheck = performance.now();
    applyTier(autoTier);
  } else {
    applyTier(choice);
  }
}

function perhapsAdjustAutoQuality(now: number): void {
  if (selectedQuality !== "auto" || !viewer || now - lastQualityCheck < 1000) {
    return;
  }
  lastQualityCheck = now;
  const sorted = [...frameTimes].sort((a, b) => a - b);
  if (sorted.length < 30) {
    return;
  }
  const p95 = sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))];
  if (p95 > 24 && autoTier !== "low") {
    autoTier = autoTier === "high" ? "medium" : "low";
    applyTier(autoTier);
    stableChecks = 0;
    return;
  }
  if (p95 < 15.5) {
    stableChecks += 1;
    if (stableChecks >= 4 && autoTier !== "high") {
      autoTier = autoTier === "low" ? "medium" : "high";
      applyTier(autoTier);
      stableChecks = 0;
    }
  } else {
    stableChecks = 0;
  }
}

function updateStats(): void {
  if (!viewer) {
    return;
  }
  const info = JSON.parse(viewer.style_info()) as {
    backend: string;
    vegetation: { trees: number; bushes: number } | null;
    mass: {
      trees: { maxLayers: number; instances: number; activeInstances: number };
      bushes: { maxLayers: number; instances: number; activeInstances: number };
    };
  };
  const debug = JSON.parse(viewer.debug_info()) as {
    stats: { grassBlades?: number; drawCalls?: number };
  };
  backend.textContent = info.backend;
  treesLabel.textContent = info.vegetation ? String(info.vegetation.trees) : "0";
  bushesLabel.textContent = info.vegetation ? String(info.vegetation.bushes) : "0";
  grassLabel.textContent =
    appliedGrass > 0 ? String(debug.stats.grassBlades ?? 0) : "off";
  treeCardsLabel.textContent = info.mass.trees.activeInstances.toLocaleString();
  bushCardsLabel.textContent = info.mass.bushes.activeInstances.toLocaleString();
  grassValue.textContent = `${appliedGrass}%`;
  treeMassValue.textContent = `${appliedTreeMass} / ${info.mass.trees.maxLayers} layers · ${info.mass.trees.activeInstances.toLocaleString()} cards`;
  bushMassValue.textContent = `${appliedBushMass} / ${info.mass.bushes.maxLayers} layers · ${info.mass.bushes.activeInstances.toLocaleString()} cards`;
  drawsLabel.textContent = String(debug.stats.drawCalls ?? 0);
}

function applyGrassDensity(value: number): void {
  appliedGrass = Math.max(0, Math.min(100, Math.round(value)));
  grassSlider.value = String(appliedGrass);
  if (viewer) {
    viewer.set_grass_shader_enabled(appliedGrass > 0);
    viewer.set_grass_density(appliedGrass / 100);
  }
  updateStats();
}

function applyTreeMass(): void {
  if (!viewer) {
    return;
  }
  appliedTreeMass = viewer.set_tree_mass(Number(treeMassSlider.value));
  updateStats();
}

function applyBushMass(): void {
  if (!viewer) {
    return;
  }
  appliedBushMass = viewer.set_bush_mass(Number(bushMassSlider.value));
  updateStats();
}

async function boot(): Promise<void> {
  status.textContent = "starting";
  if (await setupPwa()) {
    return;
  }
  status.textContent = "loading wasm";
  await init();

  const trackKey = currentTrackKey();
  trackSelect.value = trackKey;
  const trackUrl = withPackVersion(
    new URLSearchParams(window.location.search).get("track") ?? TRACKS[trackKey],
    APP_VERSION,
  );
  const loadWithCache = async (url: string) => {
    try {
      return await loadPackFromUrl(url, new IndexedDbPackCache());
    } catch {
      return await loadPackFromUrl(url);
    }
  };
  status.textContent = "loading track";
  const pack = await loadWithCache(trackUrl);
  status.textContent = "loading car";
  const carUrl = withPackVersion(
    new URLSearchParams(window.location.search).get("car") ?? CAR_URL,
    APP_VERSION,
  );
  let carPack: Awaited<ReturnType<typeof loadWithCache>> | null = null;
  try {
    carPack = await loadWithCache(carUrl);
  } catch (error) {
    console.warn("Style spike car pack failed to load", error);
  }

  resizeCanvas();
  status.textContent = "building style scene";
  viewer = carPack
    ? await TrackViewer.create_style_with_car(canvas, pack.bytes, carPack.bytes)
    : await TrackViewer.create_style(canvas, pack.bytes);
  window.__styleSpike = viewer;
  const carInfo = JSON.parse(viewer.style_car_info()) as StyleCarInfo | null;
  styleCar = carInfo;
  cameraTuning = JSON.parse(viewer.style_camera_tuning()) as StyleCameraTuning;
  updateCarHud(carInfo);
  const carModes = ["chase", "bonnet", "cockpit", "isometric"] as const;
  for (const mode of carModes) {
    const option = cameraSelect.querySelector<HTMLOptionElement>(`option[value="${mode}"]`);
    if (option) {
      option.disabled = !styleCar;
    }
  }

  selectedQuality = loadQuality();
  qualitySelect.value = selectedQuality;
  autoTier = initialAutoTier();
  applyTier(selectedQuality === "auto" ? autoTier : selectedQuality);
  viewer.set_time_of_day(loadTime());
  timeInput.value = String(loadTime());

  cameraController = setupTrackCamera(canvas, viewer, {
    maxHeight: 60,
    attachResetButton: false,
  });
  initialOrbitHome = currentCameraInfo();
  orbitHome = initialOrbitHome;
  const style = JSON.parse(viewer.style_info()) as {
    mass: {
      trees: { maxLayers: number };
      bushes: { maxLayers: number };
    };
  };
  treeMassSlider.max = String(style.mass.trees.maxLayers);
  bushMassSlider.max = String(style.mass.bushes.maxLayers);
  const storedGrass = readNumber(GRASS_KEY, "grass");
  if (storedGrass !== null) {
    grassOverride = true;
    applyGrassDensity(storedGrass);
  } else {
    applyGrassDensity(TIERS[appliedTier].grassDensity);
  }
  treeMassSlider.value = String(
    Math.min(readNumber(TREE_MASS_KEY, "treeMass") ?? 0, style.mass.trees.maxLayers),
  );
  bushMassSlider.value = String(
    Math.min(readNumber(BUSH_MASS_KEY, "bushMass") ?? 0, style.mass.bushes.maxLayers),
  );
  applyTreeMass();
  applyBushMass();
  setCameraMode("orbit");
  frameTimes = [];
  lastQualityCheck = performance.now();
  updateStats();

  const observer = new ResizeObserver(() => {
    if (resizeCanvas()) {
      viewer!.resize(canvas.width, canvas.height);
    }
  });
  observer.observe(canvas);

  let last = performance.now();
  let frames = 0;
  let accumulated = 0;
  let frameTime = 0;
  const frame = (now: number) => {
    const delta = Math.max(0, now - last);
    last = now;
    frameTimes.push(delta);
    if (frameTimes.length > 120) {
      frameTimes.shift();
    }
    frames += 1;
    accumulated += delta;
    frameTime += delta;
    if (accumulated >= 500) {
      fpsLabel.textContent = ((frames * 1000) / accumulated).toFixed(1);
      frameLabel.textContent = `${(frameTime / frames).toFixed(2)} ms`;
      frames = 0;
      accumulated = 0;
      frameTime = 0;
    }
    perhapsAdjustAutoQuality(now);
    const deltaSeconds = Math.min(delta / 1000, 0.05);
    if (cameraMode === "orbit") {
      updateOrbitKeyboard(deltaSeconds);
    } else if (cameraMode === "fly") {
      updateFly(deltaSeconds);
    }
    updateDrive(deltaSeconds);
    try {
      viewer!.render();
      requestAnimationFrame(frame);
    } catch (error) {
      fail(error);
    }
  };

  qualitySelect.addEventListener("change", () => {
    chooseQuality(qualitySelect.value as QualityChoice);
  });
  trackSelect.addEventListener("change", () => {
    const url = new URL(window.location.href);
    url.searchParams.set("track", TRACKS[trackSelect.value] ?? TRACKS.drift);
    window.location.assign(url);
  });
  timeInput.addEventListener("change", () => {
    const value = Number(timeInput.value);
    viewer?.set_time_of_day(value);
    try {
      localStorage.setItem(TIME_KEY, String(value));
    } catch {
    }
  });
  grassSlider.addEventListener("input", () => {
    grassOverride = true;
    applyGrassDensity(Number(grassSlider.value));
    saveNumber(GRASS_KEY, appliedGrass);
  });
  treeMassSlider.addEventListener("input", () => {
    applyTreeMass();
    saveNumber(TREE_MASS_KEY, appliedTreeMass);
  });
  bushMassSlider.addEventListener("input", () => {
    applyBushMass();
    saveNumber(BUSH_MASS_KEY, appliedBushMass);
  });
  cameraSelect.addEventListener("change", () => {
    setCameraMode(cameraSelect.value as CameraMode);
  });
  resetButton.addEventListener("click", resetCameraView);
  window.addEventListener("keydown", (event) => {
    if (isTypingTarget(event.target) || !CONTROL_KEYS.has(event.code)) {
      return;
    }
    pressedKeys.add(event.code);
    event.preventDefault();
    if (event.code === "KeyR" && styleCar) {
      viewer?.reset_car();
      const info = viewer ? (JSON.parse(viewer.style_car_info()) as StyleCarInfo) : null;
      styleCar = info;
      updateCarHud(info);
      const cameraInfo = cameraMode === "orbit" ? null : carCameraInfo(cameraMode);
      if (cameraInfo) {
        applyCameraInfo(cameraInfo);
      }
    }
  });
  window.addEventListener("keyup", (event) => {
    pressedKeys.delete(event.code);
  });
  window.addEventListener("blur", () => pressedKeys.clear());
  let flyPointer: number | null = null;
  let flyPointerX = 0;
  let flyPointerY = 0;
  canvas.addEventListener("pointerdown", (event) => {
    if (cameraMode !== "fly") {
      return;
    }
    flyPointer = event.pointerId;
    flyPointerX = event.clientX;
    flyPointerY = event.clientY;
    try {
      canvas.setPointerCapture(event.pointerId);
    } catch {
    }
    canvas.classList.add("dragging");
    event.preventDefault();
  });
  canvas.addEventListener("pointermove", (event) => {
    if (cameraMode !== "fly" || flyPointer !== event.pointerId) {
      return;
    }
    const dx = event.clientX - flyPointerX;
    const dy = event.clientY - flyPointerY;
    flyPointerX = event.clientX;
    flyPointerY = event.clientY;
    applyFlyMouseLook(dx, dy);
  });
  const endFlyPointer = (event: PointerEvent) => {
    if (flyPointer === event.pointerId) {
      flyPointer = null;
      canvas.classList.remove("dragging");
    }
  };
  canvas.addEventListener("pointerup", endFlyPointer);
  canvas.addEventListener("pointercancel", endFlyPointer);
  canvas.addEventListener("lostpointercapture", endFlyPointer);
  toggleHud.addEventListener("click", () => {
    const collapsed = hud.classList.toggle("hud-collapsed");
    toggleHud.textContent = collapsed ? "Show HUD" : "Hide HUD";
  });

  status.textContent = "ready";
  status.className = "status ready";
  window.__styleSpikeReady = true;
  requestAnimationFrame(frame);
  requestAnimationFrame(hideLoading);
}

function fail(error: unknown): void {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__styleSpikeError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
