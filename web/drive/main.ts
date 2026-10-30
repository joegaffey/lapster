import init, { DriveViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { APP_VERSION, BUILD_CHANNEL } from "../src/version.generated.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";
import { setupTouchControls } from "../src/ui/touch.ts";
import { setupCarSetupDialog } from "../src/ui/car-setup.ts";
import { setupAssistsDialog } from "../src/ui/assists.ts";
import { setupDevMenu } from "../src/ui/dev-menu.ts";
import { recordFrame } from "../src/ui/fps.ts";
import { hideLoading, showLoadingFailure } from "../src/ui/loading.ts";
import { saveSelectedBay, selectedBay } from "../src/ui/car-selection.ts";
import { loadSelectedTrack } from "../src/ui/track-selection.ts";
import { PhysicsClient } from "../src/sim/physics.ts";
import { SnapshotSmoother } from "../src/sim/interpolate.ts";
import { initializeSab, validateSab } from "../src/sim/sab.ts";
import { SAB_TOTAL_SIZE } from "../src/types/asset-layout.ts";
import { EngineSimAudio } from "../src/audio/engine-sim-audio.ts";
import { WheelFfb } from "../src/input/wheel-ffb.ts";

declare global {
  interface Window {
    __driveViewerReady?: boolean;
    __driveViewerError?: string;
    __driveViewer?: DriveViewer;
    __driveAudio?: EngineSimAudio;
    __wheelFfb?: WheelFfb;
  }
}

const PACK_VERSION = encodeURIComponent(APP_VERSION);
const AUDIO_STORAGE_KEY =
  BUILD_CHANNEL === "dev" ? "lapster.audio.dev.v1" : "lapster.audio.v1";
setupHud();
checkForUpdate();

const params = new URLSearchParams(window.location.search);
const devMode = params.has("dev") && params.get("dev") !== "0";
const DEV_TUNING_KEY = "lapster.tuning.dev.v1";
let storedDevTuning: string | null = null;
let storedTuning: any = null;
if (devMode) {
  try {
    storedDevTuning = localStorage.getItem(DEV_TUNING_KEY);
    storedTuning = storedDevTuning ? JSON.parse(storedDevTuning) : null;
    if (
      storedTuning?.camera &&
      (storedTuning.camera.cockpitGlassAlpha === 0.12 ||
        storedTuning.camera.cockpitGlassAlpha === 0.32)
    ) {
      storedTuning.camera.cockpitGlassAlpha = 0.06;
      storedDevTuning = JSON.stringify(storedTuning);
    }
  } catch {
    storedDevTuning = null;
    storedTuning = null;
  }
}
const defaultTrack = loadSelectedTrack();
const TRACK_URL = params.get("track") ?? `${defaultTrack.url}?v=${PACK_VERSION}`;
const CAR_URL =
  params.get("car") ?? `../../fixtures/cars/mazda-rx7/mazda-rx7.sima?v=${PACK_VERSION}`;
const carBay = selectedBay();
const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
let renderScale = 1;
const status = document.querySelector<HTMLElement>("#status")!;
const asset = document.querySelector<HTMLElement>("#asset")!;
const speedLabel = document.querySelector<HTMLElement>("#speed")!;
const rpmLabel = document.querySelector<HTMLElement>("#rpm")!;
const gearLabel = document.querySelector<HTMLElement>("#gear")!;
const contactsLabel = document.querySelector<HTMLElement>("#contacts")!;
const rackTorqueLabel = document.querySelector<HTMLElement>("#rack-torque")!;
const surfaceLabel = document.querySelector<HTMLElement>("#surface")!;
const tickLabel = document.querySelector<HTMLElement>("#tick")!;
const stepP50Label = document.querySelector<HTMLElement>("#step-p50")!;
const stepP99Label = document.querySelector<HTMLElement>("#step-p99")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const wheelsLabel = document.querySelector<HTMLElement>("#wheels")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset")!;
const cameraButton = document.querySelector<HTMLButtonElement>("#camera-mode")!;
const bodyToggle = document.querySelector<HTMLButtonElement>("#body-toggle")!;
const audioButton = document.querySelector<HTMLButtonElement>("#audio-toggle")!;

const CAMERA_STORAGE_KEY = BUILD_CHANNEL === "dev" ? "lapster.camera.dev.v1" : "lapster.camera.v1";
const keys = new Set<string>();
const wheelButtons = new Map<string, boolean>();
const STOP_SPEED_MPS = 0.3;
let steer = 0;
let gear = 1;
let autoGears = true;
let autoClutch = true;
let shiftTimer = 0;
let wheelGearUntil = 0;
let queuedShift = 0;
let queuedNeutral = false;
let queuedReverse = false;
let heldNumberedGear = 0;
let lastSpeedMps = 0;
let previousForward = false;
let previousReverse = false;
type CameraMode = "chase" | "bonnet" | "cockpit" | "isometric";
let cameraMode: CameraMode = "chase";
let bodyVisible = true;
let lastFrame = performance.now() / 1000;
let frames = 0;
let accumulated = 0;
let physics: PhysicsClient | null = null;
let smoother: SnapshotSmoother | null = null;
let physicsWorker = false;
let audio: EngineSimAudio | null = null;
let wheel: WheelFfb | null = null;
let ffbSab: SharedArrayBuffer | null = null;
const storedAudioPreference = loadAudioPreference();
let audioEnabled = storedAudioPreference.enabled;
let audioMuted = storedAudioPreference.muted;
let audioGestureCleanup: (() => void) | null = null;
let lastRpm = 900;
let redlineRpm = 8_000;
let gearRatios: number[] = [3.48, 2.02, 1.39, 1.0, 0.72, 0.6];
let maxGear = gearRatios.length;
let autoShiftCooldown = 0;
let previousAutoThrottle = 0;
let workerAssists = {
  tc: true,
  tcStrength: 0.5,
  abs: true,
  esc: true,
  escStrength: 0.5,
  steeringSensitivity: 1,
};

async function loadPack(url: string) {
  try {
    return await loadPackFromUrl(url, new IndexedDbPackCache());
  } catch {
    return await loadPackFromUrl(url);
  }
}

function resizeCanvas() {
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

function updateHud(telemetry: any) {
  speedLabel.textContent = telemetry.speedKph.toFixed(1);
  rpmLabel.textContent = String(Math.round(telemetry.rpm));
  gearLabel.textContent =
    telemetry.gear < 0
      ? "R"
      : telemetry.gear === 0
        ? "N"
        : autoGears
          ? `A${telemetry.gear}`
          : `${telemetry.gear}`;
  contactsLabel.textContent = `${telemetry.contacts}/4`;
  rackTorqueLabel.textContent = `${telemetry.steeringRackTorque.toFixed(1)} Nm`;
  const surfaces = Array.from(
    new Set(
      telemetry.wheels
        .filter((wheel: any) => wheel.contact && wheel.surface)
        .map((wheel: any) => wheel.surface as string),
    ),
  );
  const road = audio?.roadState();
  surfaceLabel.textContent = surfaces.length
    ? `${surfaces.join(" / ")} · ${road ? road.gain.toFixed(3) : "no audio"}`
    : `air · ${road ? road.gain.toFixed(3) : "no audio"}`;
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

function isCameraMode(value: string | null): value is CameraMode {
  return value === "chase" || value === "bonnet" || value === "cockpit" || value === "isometric";
}

function loadCameraMode(): CameraMode {
  try {
    const stored = localStorage.getItem(CAMERA_STORAGE_KEY);
    return isCameraMode(stored) ? stored : "chase";
  } catch {
    return "chase";
  }
}

function saveCameraMode(mode: CameraMode): void {
  try {
    localStorage.setItem(CAMERA_STORAGE_KEY, mode);
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

function setCameraMode(mode: CameraMode) {
  cameraMode = mode;
  window.__driveViewer?.set_camera_mode(mode);
  saveCameraMode(mode);
  cameraButton.textContent =
    mode === "isometric"
      ? "Camera: Isometric"
      : mode === "cockpit"
        ? "Camera: Cockpit"
        : mode === "bonnet"
          ? "Camera: Bonnet"
          : "Camera: Chase";
  if (mode === "chase" || mode === "isometric") {
    setBodyVisible(true);
  }
}

function toggleCameraMode() {
  setCameraMode(
    cameraMode === "chase"
      ? "bonnet"
      : cameraMode === "bonnet"
        ? "cockpit"
        : cameraMode === "cockpit"
          ? "isometric"
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
  lastRpm = telemetryRpm();
  autoShiftCooldown = 0;
  previousAutoThrottle = 0;
  previousForward = false;
  previousReverse = false;
  wheelGearUntil = 0;
  shiftTimer = 0;
  queuedShift = 0;
  queuedNeutral = false;
  queuedReverse = false;
  heldNumberedGear = 0;
  wheelButtons.clear();
  keys.clear();
  lastFrame = performance.now() / 1000;
}

function telemetryRpm(): number {
  const viewer = window.__driveViewer;
  if (!viewer) {
    return lastRpm;
  }
  try {
    return JSON.parse(viewer.telemetry()).rpm;
  } catch {
    return lastRpm;
  }
}

function wheelChannel(name: string): number {
  const value = wheel?.channels?.[name];
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(1, Math.max(0, value))
    : 0;
}

function wheelButtonPressed(name: string): boolean {
  const pressed = wheelChannel(name) > 0.5;
  const previous = wheelButtons.get(name) === true;
  wheelButtons.set(name, pressed);
  return pressed && !previous;
}

function updateAutoGear(delta: number, throttle: number, brake: number) {
  autoShiftCooldown = Math.max(0, autoShiftCooldown - delta);
  const kickdown = throttle >= 0.85 && previousAutoThrottle < 0.5;
  previousAutoThrottle = throttle;
  if (gear < 1 || autoShiftCooldown > 0) {
    return;
  }
  const upRpm = redlineRpm * (0.62 + 0.3 * throttle);
  const downRpm = redlineRpm * (0.28 + 0.16 * throttle);
  if (gear < maxGear && throttle > 0.05 && brake < 0.1 && lastRpm >= upRpm) {
    gear += 1;
    autoShiftCooldown = 0.35;
    return;
  }
  const lowerRpm =
    gear > 1 ? lastRpm * (gearRatios[gear - 2] / gearRatios[gear - 1]) : lastRpm;
  if (
    gear > 1 &&
    (throttle < 0.5 || brake > 0.1) &&
    lastRpm <= downRpm &&
    lowerRpm < redlineRpm * 0.97
  ) {
    gear -= 1;
    autoShiftCooldown = 0.35;
    return;
  }
  if (
    kickdown &&
    gear > 1 &&
    lastRpm < redlineRpm * 0.55 &&
    lowerRpm < redlineRpm * 0.97
  ) {
    gear -= 1;
    autoShiftCooldown = 0.35;
  }
}

function syncCarSetup(setup: any): void {
  redlineRpm = setup.engine?.redlineRpm ?? redlineRpm;
  if (Array.isArray(setup.drivetrain?.gearRatios) && setup.drivetrain.gearRatios.length > 1) {
    gearRatios = setup.drivetrain.gearRatios;
    maxGear = gearRatios.length;
  }
}

function createDriveFacade(target: DriveViewer, client: PhysicsClient): DriveViewer {
  let assistPending = false;
  const queueAssists = () => {
    if (assistPending) {
      return;
    }
    assistPending = true;
    queueMicrotask(() => {
      assistPending = false;
      client.setAssists(workerAssists);
    });
  };
  const syncLatest = () => {
    if (!smoother) {
      return;
    }
    smoother.update(performance.now());
    const state = smoother.latest();
    if (state) {
      target.render_remote(state);
    }
  };
  return new Proxy(target, {
    get(object, property, receiver) {
      switch (property) {
        case "simulate":
          return async (
            seconds: number,
            throttle: number,
            brake: number,
            steering: number,
            requestedGear: number,
          ) => {
            await client.pause();
            try {
              const tick = await client.simulate(
                seconds,
                throttle,
                brake,
                steering,
                requestedGear,
              );
              syncLatest();
              return tick;
            } finally {
              setTimeout(() => void client.resume(), 0);
            }
          };
        case "reset":
          return () => {
            client.writeInput({
              steering: 0,
              throttle: 0,
              brake: 0,
              clutch: 0,
              gear: 1,
              handbrake: 0,
              buttons: 0,
            });
            return client.reset().then(() => syncLatest());
          };
        case "telemetry":
          return () => {
            const base = JSON.parse(object.telemetry());
            base.stepP50Ms = client.diagnostics.stepP50Ms;
            base.stepP99Ms = client.diagnostics.stepP99Ms;
            base.physicsHz = client.diagnostics.physicsHz;
            base.physicsMode = "sab";
            base.workerDroppedTicks = client.diagnostics.droppedTicks;
            base.workerCatchupEvents = client.diagnostics.catchupEvents;
            return JSON.stringify(base);
          };
        case "apply_car_setup":
          return (json: string) => {
            const result = object.apply_car_setup(json);
            client.applySetup(object.car_setup());
            return result;
          };
        case "reset_car_setup":
          return () => {
            const result = object.reset_car_setup();
            client.resetSetup();
            return result;
          };
        case "set_traction_control":
          return (value: boolean) => {
            object.set_traction_control(value);
            workerAssists.tc = value;
            queueAssists();
          };
        case "set_traction_strength":
          return (value: number) => {
            object.set_traction_strength(value);
            workerAssists.tcStrength = value;
            queueAssists();
          };
        case "set_abs_control":
          return (value: boolean) => {
            object.set_abs_control(value);
            workerAssists.abs = value;
            queueAssists();
          };
        case "set_stability_control":
          return (value: boolean) => {
            object.set_stability_control(value);
            workerAssists.esc = value;
            queueAssists();
          };
        case "set_stability_strength":
          return (value: number) => {
            object.set_stability_strength(value);
            workerAssists.escStrength = value;
            queueAssists();
          };
        case "set_steering_sensitivity":
          return (value: number) => {
            object.set_steering_sensitivity(value);
            workerAssists.steeringSensitivity = value;
            queueAssists();
          };
        default: {
          const value = Reflect.get(object, property, object);
          return typeof value === "function" ? value.bind(object) : value;
        }
      }
    },
  }) as DriveViewer;
}

function loadAudioPreference(): { enabled: boolean; muted: boolean } {
  try {
    const raw = localStorage.getItem(AUDIO_STORAGE_KEY);
    if (!raw) {
      return { enabled: false, muted: false };
    }
    const value = JSON.parse(raw) as { enabled?: unknown; muted?: unknown };
    return { enabled: value?.enabled === true, muted: value?.muted === true };
  } catch {
    return { enabled: false, muted: false };
  }
}

function saveAudioPreference(): void {
  try {
    localStorage.setItem(
      AUDIO_STORAGE_KEY,
      JSON.stringify({ enabled: audioEnabled, muted: audioMuted }),
    );
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

function clearAudioGesture(): void {
  audioGestureCleanup?.();
  audioGestureCleanup = null;
}

function armAudioGesture(): void {
  if (!audio || !audioEnabled || audio.status === "running" || audioGestureCleanup) {
    return;
  }
  const onGesture = (event: Event) => {
    const target = event.target;
    if (target instanceof Element && target.closest("#audio-toggle, #dev-audio-toggle, #dev-audio-mute")) {
      return;
    }
    clearAudioGesture();
    void startAudio();
  };
  window.addEventListener("pointerdown", onGesture, true);
  window.addEventListener("keydown", onGesture, true);
  audioGestureCleanup = () => {
    window.removeEventListener("pointerdown", onGesture, true);
    window.removeEventListener("keydown", onGesture, true);
  };
}

function updateAudioButton() {
  if (!audio) {
    audioButton.disabled = true;
    audioButton.textContent = "Audio: N/A";
    audioButton.setAttribute("aria-pressed", "false");
    return;
  }
  audioButton.disabled = false;
  if (audio.status === "starting") {
    audioButton.textContent = "Audio: Starting";
  } else if (audio.status === "error") {
    audioButton.textContent = "Audio: Error";
  } else if (audio.status === "running") {
    audioButton.textContent = audio.muted ? "Audio: Muted" : "Audio: On";
  } else if (audioEnabled) {
    audioButton.textContent = audioMuted ? "Audio: Muted" : "Audio: On";
  } else {
    audioButton.textContent = "Audio: Off";
  }
  audioButton.setAttribute(
    "aria-pressed",
    String(
      (audio.status === "running" && !audio.muted) ||
        (audio.status !== "running" && audioEnabled && !audioMuted),
    ),
  );
}

async function startAudio() {
  if (!audio) {
    return;
  }
  try {
    if (audio.status !== "running") {
      audio.setMuted(audioMuted);
    }
    await audio.start();
    if (audio.status === "running") {
      audioEnabled = true;
      audioMuted = audio.muted;
      saveAudioPreference();
      clearAudioGesture();
    }
  } catch (error) {
    console.warn("audio unavailable", error);
  }
  updateAudioButton();
}

function isEditableTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      target.tagName === "INPUT" ||
      target.tagName === "SELECT" ||
      target.tagName === "TEXTAREA")
  );
}

function getAudioState(): { enabled: boolean; muted: boolean; status: string } {
  return {
    enabled: audioEnabled,
    muted: audioMuted,
    status: audio?.status ?? "unavailable",
  };
}

function toggleAudio(): void {
  if (!audio) {
    return;
  }
  if (audio.status === "running") {
    if (audio.contextState !== "running") {
      clearAudioGesture();
      void audio.resume().then(updateAudioButton);
      return;
    }
    audio.toggleMuted();
    audioMuted = audio.muted;
    audioEnabled = true;
    saveAudioPreference();
    updateAudioButton();
  } else if (audio.status !== "starting") {
    void startAudio();
  }
}

function toggleAudioMute(): void {
  if (!audio) {
    return;
  }
  if (audio.status === "running") {
    audio.toggleMuted();
    audioMuted = audio.muted;
    audioEnabled = true;
  } else {
    audioMuted = !audioMuted;
    audio.setMuted(audioMuted);
  }
  saveAudioPreference();
  updateAudioButton();
}

async function loadDialTexture(
  file: string,
  apply: (width: number, height: number, rgba: Uint8Array) => void,
): Promise<void> {
  try {
    const url = new URL(`../assets/${file}`, import.meta.url);
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`${file} returned ${response.status}`);
    }
    const bitmap = await createImageBitmap(await response.blob());
    const canvas = document.createElement("canvas");
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext("2d");
    if (!context) {
      throw new Error(`could not create ${file} texture context`);
    }
    context.drawImage(bitmap, 0, 0);
    bitmap.close();
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    apply(canvas.width, canvas.height, new Uint8Array(pixels.buffer));
  } catch (error) {
    console.warn(`${file} texture unavailable`, error);
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
  status.textContent = "creating physics";
  const viewer = await DriveViewer.create(
    canvas,
    trackPack.bytes,
    carPack.bytes,
    carBay,
    storedDevTuning ?? undefined,
  );
  saveSelectedBay(carBay);

  const info = JSON.parse(viewer.info());
  const initial = JSON.parse(viewer.telemetry());
  const setup = JSON.parse(viewer.car_setup());
  syncCarSetup(setup);
  lastRpm = initial.rpm;
  asset.textContent = `${info.asset.id} + ${info.car.id} · ${initial.carColor}`;

  let driver: DriveViewer = viewer;
  if (params.get("physics") !== "main" && window.crossOriginIsolated) {
    try {
      status.textContent = "starting physics worker";
      const sab = new SharedArrayBuffer(SAB_TOTAL_SIZE);
      initializeSab(sab);
      validateSab(sab);
      ffbSab = sab;
      const client = await PhysicsClient.create(sab, trackPack.bytes, carPack.bytes);
      physics = client;
      smoother = new SnapshotSmoother(client.reader);
      physicsWorker = true;
      workerAssists = {
        tc: initial.tractionControl,
        tcStrength: initial.tractionStrength,
        abs: initial.absControl,
        esc: initial.stabilityControl,
        escStrength: initial.stabilityStrength,
        steeringSensitivity: initial.steeringSensitivity,
      };
      client.applySetup(viewer.car_setup());
      client.setAssists(workerAssists);
      if (storedTuning?.physics) {
        await client.applyPhysicsTuning(JSON.stringify(storedTuning.physics));
      }
      client.onFatalError = (message) => {
        console.warn("physics worker failed", message);
      };
      document.addEventListener("visibilitychange", () => {
        if (!physics) {
          return;
        }
        if (document.hidden) {
          physics.pause();
        } else {
          physics.resume();
        }
      });
      window.addEventListener("pagehide", () => physics?.dispose());
      driver = createDriveFacade(viewer, client);
    } catch (error) {
      console.warn("SAB physics unavailable; using main thread", error);
      physics?.dispose();
      physics = null;
      ffbSab = null;
      smoother = null;
      physicsWorker = false;
      driver = viewer;
    }
  }
  window.__driveViewer = driver;

  if (physicsWorker) {
    audio = new EngineSimAudio();
    if (storedTuning?.audio) {
      audio.applyTuning(storedTuning.audio);
    }
    audio.setMuted(audioMuted);
    window.__driveAudio = audio;
    updateAudioButton();
    armAudioGesture();
    document.addEventListener("visibilitychange", () => {
      if (!audio) {
        return;
      }
      if (document.hidden) {
        audio.suspend();
      } else {
        void audio.resume();
      }
    });
    window.addEventListener("pagehide", () => audio?.dispose());
    audioButton.addEventListener("click", toggleAudio);
  } else {
    updateAudioButton();
  }

  window.addEventListener("keydown", (event) => {
    if (isEditableTarget(event.target)) {
      return;
    }
    const key = event.key.toLowerCase();
    if ([
      "w",
      "a",
      "s",
      "d",
      "r",
      "e",
      "q",
      "n",
      "z",
      " ",
      "arrowleft",
      "arrowright",
      "arrowup",
      "arrowdown",
    ].includes(key)) {
      event.preventDefault();
    }
    keys.add(key);
    if (!event.repeat) {
      if (key === "e") queuedShift = 1;
      if (key === "q") queuedShift = -1;
      if (key === "n") queuedNeutral = true;
      if (key === "z") queuedReverse = true;
    }
    if (key === "r") {
      reset();
    }
    if (key === "c") {
      toggleCameraMode();
    }
  });
  window.addEventListener("keyup", (event) => {
    keys.delete(event.key.toLowerCase());
    if (isEditableTarget(event.target)) {
      return;
    }
  });
  window.addEventListener("blur", () => keys.clear());
  resetButton.addEventListener("click", reset);
  cameraButton.addEventListener("click", toggleCameraMode);
  bodyToggle.addEventListener("click", () => setBodyVisible(!bodyVisible));
  setCameraMode(loadCameraMode());
  setupCarSetupDialog(driver, info.car.id, info.asset.id);
  if (typeof (navigator as any).hid?.requestDevice === "function") {
    wheel = new WheelFfb(ffbSab ?? undefined);
    window.__wheelFfb = wheel;
  }
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      wheel?.stop();
    }
  });
  window.addEventListener("pagehide", () => wheel?.stop());
  const settings = setupAssistsDialog(driver, {
    wheel: wheel ?? undefined,
    onChange: (next) => {
      autoGears = next.autoGears;
      autoClutch = next.autoClutch;
    },
    onRenderScale: (scale) => {
      renderScale = scale;
      if (resizeCanvas()) {
        viewer.resize(canvas.width, canvas.height);
      }
    },
  });
  await Promise.all([
    loadDialTexture("speedometer.png", (width, height, rgba) =>
      viewer.set_speedometer_texture(width, height, rgba),
    ),
    loadDialTexture("rpms.png", (width, height, rgba) =>
      viewer.set_rpm_texture(width, height, rgba),
    ),
    loadDialTexture("7seg.png", (width, height, rgba) =>
      viewer.set_gear_texture(width, height, rgba),
    ),
  ]);
  if (devMode && settings) {
    setupDevMenu(driver, {
      carId: info.car.id,
      settings,
      getCameraMode: () => cameraMode,
      setCameraMode,
      getBodyVisible: () => bodyVisible,
      setBodyVisible,
      getAudioState,
      toggleAudio,
      toggleAudioMute,
      onCarSetup: syncCarSetup,
      restartPhysics: async (tuning) => {
        if (physics) {
          await physics.applyPhysicsTuning(JSON.stringify(tuning));
        } else {
          viewer.apply_physics_tuning(JSON.stringify(tuning));
        }
      },
      restartAudio: async (tuning) => {
        if (!audio) {
          return;
        }
        audio.applyTuning(tuning);
        await audio.restart();
        updateAudioButton();
      },
      reloadGpu: () => window.location.reload(),
      applyAudio: (tuning) => {
        audio?.applyTuning(tuning);
        updateAudioButton();
      },
    });
  }
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
    shiftTimer = Math.max(0, shiftTimer - delta);

    if (wheel?.hasSteering) {
      steer = wheel.steering;
    } else {
      const steerLeft = keys.has("a") || keys.has("arrowleft");
      const steerRight = keys.has("d") || keys.has("arrowright");
      const targetSteer = (steerLeft ? 1 : 0) - (steerRight ? 1 : 0);
      steer += (targetSteer - steer) * Math.min(1, delta * 8);
    }

    const forwardKey = keys.has("w") || keys.has("arrowup");
    const reverseKey = keys.has("s") || keys.has("arrowdown");
    const forwardPressed = forwardKey && !previousForward;
    const reversePressed = reverseKey && !previousReverse;
    const stopped = Math.abs(lastSpeedMps) < STOP_SPEED_MPS;
    const gearBefore = gear;
    if (autoGears && stopped && gear === 0) {
      if (forwardPressed && !reverseKey) {
        gear = 1;
      } else if (reversePressed && !forwardKey) {
        gear = -1;
      }
    }
    previousForward = forwardKey;
    previousReverse = reverseKey;
    let throttle = 0;
    let brake = 0;

    if (forwardKey && reverseKey) {
      brake = 1;
    } else if (forwardKey) {
      if (gear < 0) {
        brake = 1;
        if (autoGears && stopped && forwardPressed) {
          gear = 1;
        }
      } else {
        throttle = 1;
      }
    } else if (reverseKey) {
      if (gear > 0) {
        brake = 1;
        if (autoGears && stopped && reversePressed) {
          gear = -1;
        }
      } else if (gear < 0) {
        throttle = 1;
      }
    }

    throttle = Math.max(throttle, wheelChannel("throttle"));
    brake = Math.max(brake, wheelChannel("brake"));
    const handbrake = Math.max(
      keys.has(" ") ? 1 : 0,
      wheelChannel("handbrake"),
    );

    const keyShiftUp = queuedShift > 0;
    const keyShiftDown = queuedShift < 0;
    const keyNeutral = queuedNeutral;
    const keyReverse = queuedReverse;
    queuedShift = 0;
    queuedNeutral = false;
    queuedReverse = false;

    const shiftUp = wheelButtonPressed("shiftUp");
    const gearUp = wheelButtonPressed("gearUp");
    const shiftDown = wheelButtonPressed("shiftDown");
    const gearDown = wheelButtonPressed("gearDown");
    const neutral = wheelButtonPressed("neutral");
    const reverse = wheelButtonPressed("reverse");
    let manualGear = false;

    if (neutral || keyNeutral) {
      gear = 0;
      manualGear = true;
    }
    if ((reverse || keyReverse) && stopped) {
      gear = -1;
      manualGear = true;
    }
    let numberedGear = 0;
    for (let index = 1; index <= 8; index += 1) {
      if (index <= maxGear && wheelChannel(`gear${index}`) > 0.5) {
        numberedGear = index;
        break;
      }
    }
    if (numberedGear > 0) {
      if (gear !== numberedGear) {
        gear = numberedGear;
      }
      heldNumberedGear = numberedGear;
      manualGear = true;
    } else if (heldNumberedGear > 0) {
      heldNumberedGear = 0;
      gear = 0;
      manualGear = true;
    }
    if (shiftUp || gearUp || keyShiftUp) {
      if (gear > 0 && gear < maxGear) {
        gear += 1;
      } else if (gear <= 0) {
        gear = 1;
      }
      manualGear = true;
    }
    if (shiftDown || gearDown || keyShiftDown) {
      if (gear > 1) {
        gear -= 1;
      } else if (gear > -1) {
        gear = 0;
      }
      manualGear = true;
    }
    if (manualGear) {
      wheelGearUntil = now + 1.5;
    }
    const manualGearHold = now < wheelGearUntil;

    if (autoGears && stopped && throttle === 0 && gear !== 0 && !manualGearHold) {
      gear = 0;
      autoShiftCooldown = 0;
    }
    if (autoGears && !manualGearHold && shiftTimer <= 0 && gear === gearBefore) {
      updateAutoGear(delta, throttle, brake);
    }
    if (gear !== gearBefore) {
      shiftTimer = 0.2;
      autoShiftCooldown = Math.max(autoShiftCooldown, 0.35);
    }

    const manualClutch = keys.has("shift") || wheelChannel("clutch");
    let clutch = manualClutch;
    if (autoClutch) {
      if (shiftTimer > 0) {
        clutch = 1;
      } else if (stopped && throttle < 0.02) {
        clutch = 1;
      } else {
        clutch = 0;
      }
    }
    clutch = Math.min(1, Math.max(0, clutch));

    try {
      if (physicsWorker && physics && smoother) {
        physics.writeInput({
          steering: steer,
          throttle,
          brake,
          clutch,
          gear,
          handbrake,
          buttons: 0,
        });
        const renderNow = performance.now();
        smoother.update(renderNow);
        const remoteState = smoother.state(renderNow);
        if (remoteState) {
          viewer.render_remote(remoteState);
          const telemetry = JSON.parse(driver.telemetry());
          lastSpeedMps = telemetry.speedMps;
          lastRpm = telemetry.rpm;
          audio?.update(
            delta,
            throttle,
            brake,
            gear,
            telemetry.rpm,
            telemetry.speedMps,
            telemetry.wheels,
          );
          wheel?.updateTelemetry(telemetry);
          updateHud(telemetry);
        }
      } else {
        driver.update_with_clutch(now, throttle, brake, steer, gear, clutch, handbrake);
        const telemetry = JSON.parse(driver.telemetry());
        lastSpeedMps = telemetry.speedMps;
        lastRpm = telemetry.rpm;
        audio?.update(
          delta,
          throttle,
          brake,
          gear,
          telemetry.rpm,
          telemetry.speedMps,
          telemetry.wheels,
        );
        wheel?.updateTelemetry(telemetry);
        updateHud(telemetry);
      }

      frames += 1;
      accumulated += delta;
      recordFrame(delta * 1000);
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
  requestAnimationFrame(hideLoading);
}

function fail(error: unknown) {
  wheel?.stop();
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__driveViewerError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
