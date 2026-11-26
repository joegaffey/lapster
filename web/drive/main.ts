import init, { DriveViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import {
  EDITOR_TRACK_ID,
  EDITOR_TRACK_CACHE_KEY,
  TRACK_ED_TRACK_ID,
  TRACK_ED_TRACK_CACHE_KEY,
} from "../src/assets/editor-track.ts";
import { loadPackFromBytes, loadPackFromUrl, withPackVersion } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { setupMenu } from "../src/ui/menu.ts";
import { isRaceStateView, type RaceStateView } from "../src/contracts/race-state.ts";
import { RaceDirector, raceLayoutLength, type RaceView } from "../src/race/race-director.ts";
import { buildRaceResultRows } from "../src/race/race-results.ts";
import { resolveSpectatorSlot } from "../src/race/spectator.ts";
import { APP_VERSION, BUILD_CHANNEL } from "../src/version.generated.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";
import { setupTouchControls } from "../src/ui/touch.ts";
import { setupCarSetupDialog } from "../src/ui/car-setup.ts";
import { setupAssistsDialog } from "../src/ui/assists.ts";
import { cockpitRpmOffset } from "../src/ui/cockpit-offsets.ts";
import { setupDevMenu } from "../src/ui/dev-menu.ts";
import { recordFrame } from "../src/ui/fps.ts";
import { hideLoading, showLoadingFailure } from "../src/ui/loading.ts";
import {
  CAR_OPTIONS,
  carsInCategory,
  categoryFromQuery,
  findCar,
  findCarInCategory,
  loadCarPaint,
  loadSelectedCar,
  loadSelectedCategory,
  saveSelectedBay,
  selectedBay,
} from "../src/ui/car-selection.ts";
import { selectAiRoster } from "../src/race/ai-roster.ts";
import { findTrack, loadSelectedTrack } from "../src/ui/track-selection.ts";
import { PhysicsClient } from "../src/sim/physics.ts";
import {
  shouldPlayContactSound,
  type ContactEvent,
} from "../src/sim/contacts.ts";
import { PlayerStateBuffer } from "../src/sim/player-state.ts";
import { freezeAiWheels } from "../src/sim/ai-smoothing.ts";
import {
  aiRenderSource,
  resolveAiRenderFrame,
} from "../src/sim/ai-render.ts";
import {
  createAiDebugHistory,
  pushBounded,
  sampleRawAiFrame,
  sampleRenderedAiPose,
  type AiDebugHistory,
} from "../src/sim/ai-debug.ts";
import { initializeSab, validateSab } from "../src/sim/sab.ts";
import { SAB_TOTAL_SIZE } from "../src/types/asset-layout.ts";
import { EngineSimAudio } from "../src/audio/engine-sim-audio.ts";
import {
  NEAREST_SLOT,
  RemoteEngineAudio,
  type RemoteAudioOptions,
  type RemoteEngineState,
} from "../src/audio/remote-engine-audio.ts";
import {
  aiAudioFromVoices,
  clampAiVoices,
  loadAiVoices,
  parseAiAudioOverride,
  saveAiVoices,
  type AiAudioMode,
} from "../src/audio/ai-audio-setting.ts";
import { REMOTE_MAX_DISTANCE } from "../src/audio/remote-engine-mix.ts";
import { WheelFfb } from "../src/input/wheel-ffb.ts";
import { rateLimitSteering } from "../src/input/steering-rate.ts";
import { RelayClient } from "../src/net/relay.ts";
import { RemoteStateBuffer } from "../src/net/remote-state.ts";
import {
  loadPlayerName,
  loadRoomPassword,
  normalizeRelayUrl,
  normalizeRoomCode,
  saveRelayUrl,
} from "../src/net/room.ts";
import type { PlayerInfo, StateMessage } from "../src/net/protocol.ts";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";
import { versionedWasmUrl } from "../src/wasm-init.ts";

declare global {
  interface Window {
    __driveViewerReady?: boolean;
    __driveViewerError?: string;
    __driveViewer?: DriveViewer;
    __driveAudio?: EngineSimAudio;
    __remoteAudio?: RemoteEngineAudio;
    __wheelFfb?: WheelFfb;
    __relay?: RelayClient;
    __driveContactEvents?: number;
    __aiDebug?: AiDebugHistory;
  }
}

const AUDIO_STORAGE_KEY =
  BUILD_CHANNEL === "dev" ? "lapster.audio.dev.v1" : "lapster.audio.v1";
setupHud();
setupMenu();
checkForUpdate();

const params = new URLSearchParams(window.location.search);
const devMode = params.has("dev") && params.get("dev") !== "0";
const aiDebug = params.get("aiDebug") === "1";
// Temporary `?aiNoWheels=1` diagnostic: freeze AI suspension travel and wheel
// steer/spin before rendering to isolate the AI-only visual stutter.
const aiNoWheels = params.get("aiNoWheels") === "1";
// Temporary AI stutter diagnostics: `?aiNoSmooth=1` skips the pose filter and
// `?aiRaw=1` skips the interpolation buffer and pose filter entirely.
const aiNoSmooth = params.get("aiNoSmooth") === "1";
const aiRaw = params.get("aiRaw") === "1";
// `?collisionDebug=1` draws per-car planar collision-box wireframes. Off by default.
const collisionDebug = params.get("collisionDebug") === "1";
const aiRenderMode = aiRenderSource(aiNoSmooth, aiRaw);
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
const trackParam = params.get("track");
const defaultTrack = findTrack(trackParam) ?? loadSelectedTrack();
const TRACK_URL = withPackVersion(
  trackParam && !findTrack(trackParam) ? trackParam : defaultTrack.url,
  APP_VERSION,
);
const carParam = params.get("car") || null;

function packUrlSegments(value: string): string[] {
  let path = value;
  try {
    path = new URL(value, "http://lapster.invalid/").pathname;
  } catch {
    path = value.split(/[?#]/, 1)[0];
  }
  return path.split("/").filter(Boolean);
}

function isPathSuffix(shorter: string[], longer: string[]): boolean {
  if (shorter.length === 0 || shorter.length > longer.length) {
    return false;
  }
  const offset = longer.length - shorter.length;
  return shorter.every((segment, index) => segment === longer[offset + index]);
}

function findCarByPackUrl(value: string) {
  const target = packUrlSegments(value);
  if (target.length === 0) {
    return null;
  }
  for (const car of CAR_OPTIONS) {
    const catalog = packUrlSegments(car.url);
    if (isPathSuffix(catalog, target) || isPathSuffix(target, catalog)) {
      return car;
    }
  }
  return null;
}

const catalogCarParam =
  findCar(carParam) ?? (carParam ? findCarByPackUrl(carParam) : null);
const queryCategory = categoryFromQuery(window.location.search);
const selectedCategory =
  queryCategory ?? catalogCarParam?.category ?? loadSelectedCategory();
const categoryCars = carsInCategory(selectedCategory);
const selectedCar = loadSelectedCar(selectedCategory);
const playerCar =
  catalogCarParam && catalogCarParam.category === selectedCategory
    ? catalogCarParam
    : findCarInCategory(carParam, selectedCategory) ?? selectedCar;
const customPackUrl =
  carParam !== null && !findCar(carParam) ? carParam : null;
const catalogCar =
  catalogCarParam && catalogCarParam.category === selectedCategory
    ? catalogCarParam
    : customPackUrl === null
      ? selectedCar
      : null;
const CAR_URL = withPackVersion(
  customPackUrl !== null &&
    (!catalogCarParam || catalogCarParam.category === selectedCategory)
    ? customPackUrl
    : playerCar.url,
  APP_VERSION,
);
const carBay = selectedBay(window.location.search, categoryCars.length);
const carPaint = resolveCarPaint();

function resolveCarPaint(): number | null {
  const raw = params.get("paint");
  if (raw !== null && raw.trim() !== "") {
    const value = Number(raw);
    if (Number.isFinite(value) && value >= 0) {
      return Math.trunc(value);
    }
  }
  return catalogCar ? loadCarPaint(catalogCar.id) : null;
}
const mpMode = params.get("mp") === "1";
const soloMode = params.get("race") === "0";
const REMOTE_SLOTS = 3;
const AI_FIELD_MAX = 9;
// AI/opponent audio mode: 0 is off, 1 runs one EngineSim voice for the nearest
// AI, and 2..5 keeps that many per-slot EngineSim voices. `nearest`, `full`,
// `lite`, and `off` remain available as a `?aiAudio` diagnostic override.
const aiAudioOverride = parseAiAudioOverride(params);
// Effective setting for this session. A query override wins at load but is
// never persisted; moving the selector writes the stored value instead.
let aiVoiceSetting = aiAudioOverride?.voices ?? loadAiVoices();
let AI_AUDIO_MODE: AiAudioMode =
  aiAudioOverride?.mode ?? aiAudioFromVoices(aiVoiceSetting).mode;
let AI_AUDIO_VOICES =
  aiAudioOverride?.voices ?? aiAudioFromVoices(aiVoiceSetting).voices;
const DEFAULT_AI_COUNT = 3;
const DEFAULT_AI_PACE = 1;
const AI_PACE_INDICES: Record<string, number> = {
  chill: 0,
  club: 1,
  fast: 2,
};
const MULTIPLAYER_SEND_HZ = 30;

function resolveAiCount(raceAvailable: boolean): number {
  if (mpMode || soloMode || !raceAvailable) {
    return 0;
  }
  const raw = params.get("ai");
  if (raw === null || raw.trim() === "") {
    return DEFAULT_AI_COUNT;
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return DEFAULT_AI_COUNT;
  }
  return Math.min(AI_FIELD_MAX, Math.max(0, Math.trunc(value)));
}

function resolveAiPace(): number {
  const raw = params.get("aiPace");
  if (raw === null) {
    return DEFAULT_AI_PACE;
  }
  const value = AI_PACE_INDICES[raw];
  return typeof value === "number" ? value : DEFAULT_AI_PACE;
}

const DEFAULT_RACE_LAPS = 3;
const MIN_RACE_LAPS = 1;
const MAX_RACE_LAPS = 5;

function resolveRaceLaps(): number {
  const raw = params.get("laps");
  if (raw === null || raw.trim() === "") {
    return DEFAULT_RACE_LAPS;
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return DEFAULT_RACE_LAPS;
  }
  const laps = Math.trunc(value);
  if (laps < MIN_RACE_LAPS || laps > MAX_RACE_LAPS) {
    return DEFAULT_RACE_LAPS;
  }
  return laps;
}

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
const aiVoicesSelect =
  document.querySelector<HTMLSelectElement>("#ai-audio-voices");
const netBadge = document.querySelector<HTMLElement>("#net-badge")!;
const racePanel = document.querySelector<HTMLElement>("#race-panel")!;
const racePositionStat = document.querySelector<HTMLElement>("#race-position-stat")!;
const racePositionLabel = document.querySelector<HTMLElement>("#race-position")!;
const raceLapLabel = document.querySelector<HTMLElement>("#race-lap")!;
const raceSectorLabel = document.querySelector<HTMLElement>("#race-sector")!;
const raceCurrentLabel = document.querySelector<HTMLElement>("#race-current")!;
const raceLastLabel = document.querySelector<HTMLElement>("#race-last")!;
const raceBestLabel = document.querySelector<HTMLElement>("#race-best")!;
const raceWrongWayLabel = document.querySelector<HTMLElement>("#race-wrong-way")!;
const raceAiStat = document.querySelector<HTMLElement>("#race-ai-stat")!;
const raceAiValue = document.querySelector<HTMLElement>("#race-ai")!;
const raceCountdown = document.querySelector<HTMLElement>("#race-countdown")!;
const raceResults = document.querySelector<HTMLElement>("#race-results")!;
const raceResultsRows = document.querySelector<HTMLElement>("#race-results-rows")!;
const raceRestartButton = document.querySelector<HTMLButtonElement>("#race-results-restart")!;

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
let playerState: PlayerStateBuffer | null = null;
let physicsWorker = false;
let driveContactEvents = 0;
let lastPlayerPosition: [number, number, number] | null = null;
window.__driveContactEvents = driveContactEvents;
let relay: RelayClient | null = null;
let relaySeq = 1;
let relayUploadTimer: ReturnType<typeof setInterval> | null = null;
let lastRelayTick = -1;
const remotePlayers = new Map<number, RemotePlayer>();
const aiBuffers = new Map<number, RemoteStateBuffer>();
// AI-only render smoothing: one filtered pose per slot plus the last render
// time used to derive the filter delta. Never applied to the player or remote
// multiplayer cars.
const aiSmoothedFrames = new Map<number, Float32Array>();
// Temporary `?aiRaw=1` diagnostic: the most recent worker frame per slot,
// bounded to one frame per slot. Empty unless the flag is active.
const aiRawFrames = new Map<number, Float32Array>();
let aiLastRenderMs = 0;
const AI_INTERPOLATION_MS = 32;
let aiIgnoreFrames = false;
// Temporary `?aiDebug=1` capture of raw worker frames and rendered AI poses.
// Null (and effectively inert) unless the flag is set.
const aiDebugHistory: AiDebugHistory | null = aiDebug
  ? createAiDebugHistory()
  : null;
if (aiDebugHistory) {
  window.__aiDebug = aiDebugHistory;
}
let playerSteeringRate = 6;
let audio: EngineSimAudio | null = null;
let remoteAudio: RemoteEngineAudio | null = null;
let remoteSharesPlayerContext = false;
// Whether this session has AI or multiplayer opponents to voice.
let sessionHasOpponents = false;
// Player car audio profile, reused when rebuilding remote voices.
let playerAudioProfile: { engine?: string; vehicle?: string } | undefined;
// AI slot (1..9) -> catalog engine/vehicle profile for positional voices.
const aiAudioProfiles = new Map<number, { engine: string; vehicle: string }>();
// Roster snapshot used to assign per-slot profiles when remote audio is rebuilt.
let sessionAiRoster: Array<{ engine: string; vehicle: string }> = [];
let wheel: WheelFfb | null = null;
let ffbSab: SharedArrayBuffer | null = null;
const storedAudioPreference = loadAudioPreference();
let audioEnabled = storedAudioPreference.enabled;
let audioMuted = storedAudioPreference.muted;
let audioGestureCleanup: (() => void) | null = null;
let lastRpm = 900;
let redlineRpm = 8_000;
let raceSectorCount = 0;
let raceDirector: RaceDirector | null = null;
let localRace = false;
let playerGridSlot = 0;
let raceLaps = DEFAULT_RACE_LAPS;
let raceGeneration = 0;
let inputLocked = false;
// `?spectate=1..9` follows an AI car with a locked, neutral player input.
let spectatorMode = false;
let countdownTimers: ReturnType<typeof setTimeout>[] = [];
let resultsShown = false;
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

interface RemotePlayer {
  id: number;
  name: string;
  rig: number;
  buffer: RemoteStateBuffer;
}

async function loadPack(url: string) {
  try {
    return await loadPackFromUrl(url, new IndexedDbPackCache());
  } catch {
    return await loadPackFromUrl(url);
  }
}

async function loadLocalTrackPack(
  key: string,
  label: string,
  help: string,
) {
  const bytes = await new IndexedDbPackCache().get(key);
  if (!bytes) {
    throw new Error(`No ${label} track found. ${help}`);
  }
  try {
    return await loadPackFromBytes(bytes);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} track is invalid: ${message}`);
  }
}

async function loadEditorTrackPack() {
  return loadLocalTrackPack(
    EDITOR_TRACK_CACHE_KEY,
    "editor",
    "Open the track editor and choose Test drive first.",
  );
}

async function loadTrackEdPack() {
  return loadLocalTrackPack(
    TRACK_ED_TRACK_CACHE_KEY,
    "track-ed",
    "Open track-ed and choose Test drive first.",
  );
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

function formatRaceTime(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) {
    return "—";
  }
  if (!Number.isFinite(seconds) || seconds < 0) {
    return "—";
  }
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds - minutes * 60;
  return `${minutes}:${remainder.toFixed(2).padStart(5, "0")}`;
}

function updateRaceHud(telemetry: any) {
  if (soloMode) {
    racePanel.hidden = true;
    return;
  }
  const race = telemetry?.raceState;
  if (!isRaceStateView(race) || !race.available) {
    racePanel.hidden = true;
    return;
  }
  racePanel.hidden = false;
  if (raceDirector) {
    raceDirector.updatePlayer(race);
    const view = raceDirector.view();
    const displayLap = Math.min(Math.max(1, race.lap), view.totalLaps);
    raceLapLabel.textContent = `${displayLap}/${view.totalLaps}`;
    racePositionStat.hidden = false;
    racePositionLabel.textContent = `P${view.playerPosition}/${view.cars.length}`;
    if (view.playerFinished && !resultsShown) {
      showRaceResults(view);
    }
  } else {
    raceLapLabel.textContent = String(race.lap);
    racePositionStat.hidden = true;
  }
  raceSectorLabel.textContent =
    raceSectorCount > 0
      ? `S${race.sector}/${raceSectorCount}`
      : `S${race.sector}`;
  raceCurrentLabel.textContent = formatRaceTime(race.current_lap_time);
  raceLastLabel.textContent = formatRaceTime(race.last_lap_time);
  raceBestLabel.textContent = formatRaceTime(race.best_lap_time);
  raceWrongWayLabel.hidden = !race.wrong_way;
  const aiCount =
    typeof telemetry?.aiCount === "number" && Number.isFinite(telemetry.aiCount)
      ? Math.max(0, Math.trunc(telemetry.aiCount))
      : 0;
  raceAiStat.hidden = aiCount <= 0;
  raceAiValue.textContent = String(aiCount);
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
  updateRaceHud(telemetry);
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

function setNetBadge(message: string, error = false): void {
  netBadge.hidden = !mpMode;
  netBadge.textContent = message;
  netBadge.classList.toggle("error", error);
}

function removeRemotePlayer(playerId: number): void {
  const remote = remotePlayers.get(playerId);
  if (!remote) {
    return;
  }
  remotePlayers.delete(playerId);
  remoteAudio?.remove(remote.rig);
  window.__driveViewer?.set_remote_visible(remote.rig, false);
}

function syncRemoteRoster(players: PlayerInfo[]): void {
  if (!relay) {
    return;
  }
  const ids = new Set(players.map((player) => player.id));
  for (const playerId of [...remotePlayers.keys()]) {
    if (playerId === relay.playerId || !ids.has(playerId)) {
      removeRemotePlayer(playerId);
    }
  }
  for (const player of players) {
    if (player.id === relay.playerId || remotePlayers.has(player.id)) {
      continue;
    }
    if (player.slot < 0 || player.slot > REMOTE_SLOTS) {
      continue;
    }
    remotePlayers.set(player.id, {
      id: player.id,
      name: player.name,
      rig: player.slot,
      buffer: new RemoteStateBuffer(),
    });
  }
  setNetBadge(
    `Room ${relay.room} · ${players.length}/${relay.maxPlayers} · linked`,
  );
}

function handleRelayState(message: StateMessage): void {
  if (!relay || message.player_id === relay.playerId) {
    return;
  }
  remotePlayers
    .get(message.player_id)
    ?.buffer.push(message.seq, message.tick, message.state, performance.now());
}

function frameToRemoteState(frame: Float32Array): RemoteEngineState {
  return {
    rpm: frame[SNAPSHOT_STATE.engineRpm],
    speedMps: Math.hypot(
      frame[SNAPSHOT_STATE.velocity],
      frame[SNAPSHOT_STATE.velocity + 2],
    ),
    gear: Math.round(frame[SNAPSHOT_STATE.gear]),
    throttle: frame[SNAPSHOT_STATE.throttle],
    brake: frame[SNAPSHOT_STATE.brake],
    wheels: Array.from({ length: 4 }, (_, wheel) => ({
      slipRatio: frame[SNAPSHOT_STATE.slipRatio + wheel],
      slipAngle: frame[SNAPSHOT_STATE.slipAngle + wheel],
      load: frame[SNAPSHOT_STATE.tireLoad + wheel],
      contact: frame[SNAPSHOT_STATE.contactFlags + wheel] > 0.5,
    })),
  };
}

function renderRemotePlayers(
  now: number,
  localState: Float32Array,
  deltaSeconds: number,
): void {
  const viewer = window.__driveViewer;
  if (!viewer) {
    return;
  }
  const localX = localState[SNAPSHOT_STATE.position];
  const localZ = localState[SNAPSHOT_STATE.position + 2];
  for (const remote of remotePlayers.values()) {
    const frame = remote.buffer.frame(now);
    if (frame) {
      try {
        viewer.set_remote_state(remote.rig, frame);
      } catch (error) {
        console.warn("remote car update failed", error);
      }
      const dx = frame[SNAPSHOT_STATE.position] - localX;
      const dz = frame[SNAPSHOT_STATE.position + 2] - localZ;
      remoteAudio?.update(
        remote.rig,
        frameToRemoteState(frame),
        Math.hypot(dx, dz),
        deltaSeconds,
      );
    } else if (remote.buffer.age(now) > 2000) {
      remoteAudio?.silence(remote.rig);
      viewer.set_remote_visible(remote.rig, false);
    }
  }
}

function resetAiBuffers(): void {
  aiBuffers.clear();
  aiSmoothedFrames.clear();
  aiRawFrames.clear();
  aiLastRenderMs = 0;
}

function renderAiFrames(
  now: number,
  localState: Float32Array | null,
): void {
  const viewer = window.__driveViewer;
  if (!viewer) {
    return;
  }
  const dtSeconds =
    aiLastRenderMs > 0
      ? Math.min(0.25, Math.max(0, (now - aiLastRenderMs) / 1000))
      : 0;
  aiLastRenderMs = now;
  const localX =
    localState !== null ? localState[SNAPSHOT_STATE.position] : null;
  const localZ =
    localState !== null ? localState[SNAPSHOT_STATE.position + 2] : null;
  const audioCandidates: Array<{
    slot: number;
    state: RemoteEngineState;
    distance: number;
  }> = [];
  for (const [slot, buffer] of aiBuffers) {
    const bufferFrame = aiRenderMode === "raw" ? null : buffer.frame(now);
    const rawFrame =
      aiRenderMode === "raw" ? (aiRawFrames.get(slot) ?? null) : null;
    const render = resolveAiRenderFrame(
      aiRenderMode,
      bufferFrame,
      rawFrame,
      aiSmoothedFrames.get(slot) ?? null,
      dtSeconds,
    );
    if (!render) {
      if (buffer.age(now) > 2000) {
        remoteAudio?.silence(slot);
      }
      continue;
    }
    const { source, drawn } = render;
    if (aiRenderMode === "smooth") {
      aiSmoothedFrames.set(slot, drawn);
    }
    if (aiNoWheels) {
      // Freeze after the pose filter so position and rotation stay smoothed.
      // The debug sample below reads `drawn`, so it records the drawn frame.
      freezeAiWheels(drawn);
    }
    if (aiDebugHistory) {
      const tick =
        source.length > SNAPSHOT_STATE.tick
          ? Math.round(source[SNAPSHOT_STATE.tick])
          : Number.NaN;
      pushBounded(
        aiDebugHistory.rendered,
        sampleRenderedAiPose(slot, source, drawn, now, tick, {
          // The raw diagnostic does not call buffer.frame(), so report the
          // newest frame as fully applied and reuse the buffer's clock metrics.
          alpha: aiRenderMode === "raw" ? 1 : buffer.interpolationAlpha,
          depth: buffer.depth,
          jitter: buffer.jitter,
          offsetMs: buffer.offsetMs,
          age: buffer.age(now),
        }),
        aiDebugHistory.limit,
      );
    }
    try {
      viewer.set_remote_state(slot, drawn);
    } catch (error) {
      console.warn("AI car update failed", error);
    }
    if (localX !== null && localZ !== null) {
      const dx = source[SNAPSHOT_STATE.position] - localX;
      const dz = source[SNAPSHOT_STATE.position + 2] - localZ;
      audioCandidates.push({
        slot,
        state: frameToRemoteState(source),
        distance: Math.hypot(dx, dz),
      });
    }
  }
  if (!remoteAudio) {
    return;
  }
  if (AI_AUDIO_MODE === "nearest") {
    // Multiplayer rigs keep their own per-player slots; only single-player AI
    // uses the dedicated nearest slot.
    if (mpMode) {
      return;
    }
    let nearest: (typeof audioCandidates)[number] | null = null;
    for (const candidate of audioCandidates) {
      if (candidate.distance >= REMOTE_MAX_DISTANCE) {
        continue;
      }
      if (!nearest || candidate.distance < nearest.distance) {
        nearest = candidate;
      }
    }
    if (!nearest) {
      remoteAudio.silence(NEAREST_SLOT);
      return;
    }
    const profile = aiAudioProfiles.get(nearest.slot);
    if (profile) {
      // No-op when the nearest car keeps the current engine profile.
      remoteAudio.setProfile(NEAREST_SLOT, profile.engine, profile.vehicle);
    }
    remoteAudio.update(
      NEAREST_SLOT,
      nearest.state,
      nearest.distance,
      dtSeconds,
    );
    return;
  }
  if (audioCandidates.length === 0) {
    return;
  }
  audioCandidates.sort((a, b) => a.distance - b.distance);
  for (let index = 0; index < audioCandidates.length; index += 1) {
    const candidate = audioCandidates[index];
    if (index < AI_AUDIO_VOICES) {
      remoteAudio.update(
        candidate.slot,
        candidate.state,
        candidate.distance,
        dtSeconds,
      );
    } else {
      remoteAudio.silence(candidate.slot);
    }
  }
}

function startRelayUpload(): void {
  stopRelayUpload();
  relayUploadTimer = setInterval(() => {
    if (!relay || !physics) {
      return;
    }
    const snapshot = physics.reader.readLatest();
    if (!snapshot || snapshot.tick === lastRelayTick) {
      return;
    }
    lastRelayTick = snapshot.tick;
    relay.sendState(relaySeq, snapshot.tick, snapshot.state);
    relaySeq += 1;
  }, 1000 / MULTIPLAYER_SEND_HZ);
}

function stopRelayUpload(): void {
  if (relayUploadTimer !== null) {
    clearInterval(relayUploadTimer);
    relayUploadTimer = null;
  }
}

async function setupMultiplayer(viewer: DriveViewer, info: any): Promise<void> {
  if (!mpMode) {
    netBadge.hidden = true;
    return;
  }
  netBadge.hidden = false;
  if (!physicsWorker || !physics) {
    setNetBadge("Multiplayer needs the COI physics worker", true);
    return;
  }
  const room = normalizeRoomCode(params.get("room"));
  if (!room) {
    setNetBadge("Invalid room code", true);
    return;
  }
  const relayOverride = normalizeRelayUrl(params.get("ws"));
  if (relayOverride) {
    saveRelayUrl(relayOverride);
  }
  const client = new RelayClient({
    room,
    password: loadRoomPassword(),
    name: loadPlayerName() || "Driver",
    car: info.car.id,
    track: info.asset.id,
  });
  relay = client;
  window.__relay = client;
  client.onRoster = syncRemoteRoster;
  client.onState = handleRelayState;
  client.onPlayerLeft = removeRemotePlayer;
  client.onError = (message) => setNetBadge(`Multiplayer: ${message}`, true);
  client.onClose = () => setNetBadge("Multiplayer disconnected", true);
  setNetBadge(`Room ${room} · connecting…`);

  try {
    const welcome = await client.connect();
    viewer.set_local_slot(welcome.slot);
    viewer.place_on_grid(welcome.slot);
    await physics?.placeOnGrid(welcome.slot);
    // Grid placement keeps the tick; reset the player timeline around it.
    playerState?.clear();
    for (const state of welcome.states) {
      handleRelayState({
        type: "state",
        player_id: state.player_id,
        seq: state.seq,
        tick: state.tick,
        state: state.state,
      });
    }
    lastRelayTick = -1;
    startRelayUpload();
    setNetBadge(
      `Room ${welcome.room} · ${client.players.length}/${welcome.max_players} · linked`,
    );
  } catch (error) {
    setNetBadge(
      `Multiplayer: ${error instanceof Error ? error.message : String(error)}`,
      true,
    );
    stopRelayUpload();
    client.close();
    relay = null;
  }
}

function clearRaceResults(): void {
  resultsShown = false;
  raceResults.hidden = true;
  raceResultsRows.replaceChildren();
}

function showRaceResults(view: RaceView): void {
  const rows = buildRaceResultRows(view);
  raceResultsRows.replaceChildren();
  for (const row of rows) {
    const item = document.createElement("li");
    item.className = "race-result-row";
    if (row.name === "You") {
      item.classList.add("is-player");
    }
    item.setAttribute("data-testid", "race-result-row");

    const position = document.createElement("span");
    position.className = "race-result-position";
    position.textContent = `P${row.position}`;

    const name = document.createElement("span");
    name.className = "race-result-name";
    name.textContent = row.name;

    const laps = document.createElement("span");
    laps.className = "race-result-laps";
    laps.textContent = `${row.laps} ${row.laps === 1 ? "lap" : "laps"}`;

    const time = document.createElement("span");
    time.className = "race-result-time";
    time.textContent = row.timeText;

    item.append(position, name, laps, time);
    raceResultsRows.append(item);
  }
  raceResults.hidden = false;
  resultsShown = true;
}

function clearCountdownTimers(): void {
  for (const timer of countdownTimers) {
    clearTimeout(timer);
  }
  countdownTimers = [];
}

function runRaceCountdown(generation: number): void {
  clearCountdownTimers();
  raceCountdown.hidden = false;
  raceCountdown.textContent = "3";
  const step = (delay: number, text: string) => {
    countdownTimers.push(
      setTimeout(() => {
        if (generation !== raceGeneration) {
          return;
        }
        raceCountdown.textContent = text;
      }, delay),
    );
  };
  step(1_000, "2");
  step(2_000, "1");
  countdownTimers.push(
    setTimeout(() => {
      if (generation !== raceGeneration) {
        return;
      }
      raceCountdown.textContent = "GO";
      void startRace(generation);
    }, 3_000),
  );
  countdownTimers.push(
    setTimeout(() => {
      if (generation !== raceGeneration) {
        return;
      }
      raceCountdown.hidden = true;
      raceCountdown.textContent = "";
    }, 3_800),
  );
}

async function startRace(generation: number): Promise<void> {
  if (!spectatorMode) {
    inputLocked = false;
  }
  if (!physics) {
    if (generation === raceGeneration) {
      raceDirector?.start();
    }
    return;
  }
  try {
    await physics.startRace(raceLaps);
    if (generation !== raceGeneration) {
      return;
    }
    await physics.resume();
    if (generation !== raceGeneration) {
      return;
    }
    raceDirector?.start();
  } catch (error) {
    console.warn("race start failed", error);
  }
}

async function startLocalRaceCountdown(): Promise<void> {
  const generation = raceGeneration + 1;
  raceGeneration = generation;
  clearCountdownTimers();
  raceCountdown.hidden = true;
  raceCountdown.textContent = "";
  inputLocked = true;
  clearRaceResults();
  const viewer = window.__driveViewer;
  if (!viewer) {
    inputLocked = false;
    return;
  }
  if (physics) {
    try {
      // Pause before re-gridding so the placement frames are the only fresh AI
      // frames published before GO.
      await physics.pause();
      if (generation !== raceGeneration) {
        return;
      }
    } catch (error) {
      console.warn("race grid placement failed", error);
      inputLocked = false;
      return;
    }
  }
  raceDirector?.reset();
  // Drop any frames from a previous run after the worker is paused, then let
  // the placement frames through so the AI are visible during the countdown.
  resetAiBuffers();
  playerState?.clear();
  aiIgnoreFrames = false;
  viewer.place_on_race_grid(playerGridSlot);
  if (physics) {
    try {
      await physics.placeOnRaceGrid(playerGridSlot);
      if (generation !== raceGeneration) {
        return;
      }
      // Placement keeps the tick, so drop history explicitly and let the next
      // frame seed the new grid pose instead of interpolating from the old one.
      playerState?.clear();
    } catch (error) {
      console.warn("race grid placement failed", error);
      aiIgnoreFrames = false;
      inputLocked = false;
      return;
    }
  }
  if (generation !== raceGeneration) {
    return;
  }
  runRaceCountdown(generation);
}

function reset() {
  const viewer = window.__driveViewer;
  if (!viewer) return;
  viewer.reset();
  resetAiBuffers();
  playerState?.clear();
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
  if (localRace) {
    void startLocalRaceCountdown();
  } else if (relay && physics) {
    void physics.placeOnGrid(relay.slot).finally(() => playerState?.clear());
  }
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

function currentPlayerPosition(): [number, number, number] | null {
  const state = playerState?.latest();
  if (!state || state.length < SNAPSHOT_STATE.position + 3) {
    return lastPlayerPosition;
  }
  const x = state[SNAPSHOT_STATE.position];
  const y = state[SNAPSHOT_STATE.position + 1];
  const z = state[SNAPSHOT_STATE.position + 2];
  if (Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(z)) {
    lastPlayerPosition = [x, y, z];
  }
  return lastPlayerPosition;
}

function handleContact(contact: ContactEvent, viewer: DriveViewer): void {
  driveContactEvents += 1;
  window.__driveContactEvents = driveContactEvents;
  const [x, y, z] = contact.point;
  try {
    const effect = viewer as unknown as {
      contact_effect?: (x: number, y: number, z: number, speed: number) => void;
    };
    effect.contact_effect?.(x, y, z, contact.speed);
  } catch (error) {
    // Visual contact feedback is best-effort and must not break the frame loop.
  }
  if (shouldPlayContactSound(contact, currentPlayerPosition())) {
    audio?.playImpact(contact.speed);
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
    if (!playerState) {
      return;
    }
    // A reset or one-shot simulate can teleport across the timeline. Drop the
    // old history so the delayed frame cannot draw a stale pose, then seed the
    // buffer with the newest raw sample.
    playerState.clear();
    const snapshot = client.reader.readLatest();
    if (!snapshot) {
      return;
    }
    const now = performance.now();
    playerState.push(snapshot, now);
    const state = playerState.latest();
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
            base.aiCount = client.diagnostics.aiCount;
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
  const player = audio;
  if (!player || !audioEnabled || audioGestureCleanup) {
    return;
  }
  // Also arm when the engine is running but its context was suspended by a
  // mobile background; that gesture resumes instead of rebuilding audio.
  if (player.status === "running" && player.contextState === "running") {
    return;
  }
  const onGesture = (event: Event) => {
    const target = event.target;
    if (target instanceof Element && target.closest("#audio-toggle, #dev-audio-toggle, #dev-audio-mute")) {
      return;
    }
    clearAudioGesture();
    if (player.status === "running" && player.contextState !== "running") {
      // The engine is built but its context was suspended (e.g. a mobile
      // background). Resume it and the remote voices instead of starting over.
      void player
        .resume()
        .then(() => remoteAudio?.resume())
        .catch((error) => console.warn("audio resume unavailable", error))
        .finally(updateAudioButton);
      return;
    }
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
    audioButton.textContent = "Audio: Tap to start";
  } else {
    audioButton.textContent = "Audio: Off";
  }
  audioButton.setAttribute(
    "aria-pressed",
    String(
      audio.status === "running" &&
        audio.contextState === "running" &&
        !audio.muted,
    ),
  );
}

function updateAiAudioControl(): void {
  if (!aiVoicesSelect) {
    return;
  }
  aiVoicesSelect.value = String(clampAiVoices(aiVoiceSetting));
}

function clearRemoteAudio(): void {
  remoteAudio?.dispose();
  remoteAudio = null;
  remoteSharesPlayerContext = false;
  if (window.__remoteAudio) {
    delete window.__remoteAudio;
  }
}

/**
 * Dispose and rebuild the remote opponent voices for the current setting.
 * Player audio owns its context, so this never touches the player engine.
 */
function createRemoteAudio(): void {
  clearRemoteAudio();
  if (!audio || !sessionHasOpponents) {
    return;
  }
  if (AI_AUDIO_MODE === "off" || AI_AUDIO_VOICES <= 0) {
    return;
  }
  // Opponent voices always own their AudioContext. Sharing the player's
  // context let an AI voice failure take the player engine down on mobile
  // browsers, which is why adding an `ai` param could silence the whole session.
  remoteSharesPlayerContext = false;
  const options: RemoteAudioOptions = {
    ...(playerAudioProfile ?? {}),
    mode: AI_AUDIO_MODE === "lite" ? "lite" : "full",
  };
  const instance = new RemoteEngineAudio(options);
  remoteAudio = instance;
  window.__remoteAudio = instance;
  instance.setMuted(audioMuted);
  if (!mpMode) {
    sessionAiRoster.forEach((car, index) => {
      const slot = index + 1;
      aiAudioProfiles.set(slot, { engine: car.engine, vehicle: car.vehicle });
      if (AI_AUDIO_MODE !== "nearest") {
        instance.setProfile(slot, car.engine, car.vehicle);
      }
    });
  }
  if (audio.status === "running") {
    // Apply a live change immediately without restarting the player engine.
    void instance.start().catch(() => {
      // Local engine audio still works without remote voices.
    });
  }
}

function applyAiVoices(value: number | string): void {
  aiVoiceSetting = saveAiVoices(value);
  const resolution = aiAudioFromVoices(aiVoiceSetting);
  AI_AUDIO_MODE = resolution.mode;
  AI_AUDIO_VOICES = resolution.voices;
  updateAiAudioControl();
  createRemoteAudio();
}

if (aiVoicesSelect) {
  aiVoicesSelect.addEventListener("change", () => {
    applyAiVoices(aiVoicesSelect.value);
  });
}
updateAiAudioControl();

async function startAudio() {
  if (!audio) {
    return;
  }
  try {
    audio.setMuted(audioMuted);
    remoteAudio?.setMuted(audioMuted);
    if (remoteSharesPlayerContext && remoteAudio) {
      // Nearest/lite voices reuse the player's context, so start the player
      // first and let it create the context before the remote engine attaches.
      await audio.start();
      try {
        await remoteAudio.start();
      } catch {
        // Local engine audio still works without remote voices.
      }
    } else {
      // Create both AudioContexts in the same user gesture. Remote failures
      // are isolated so the player engine still plays.
      const playerStart = audio.start();
      const remoteStart = remoteAudio
        ? remoteAudio.start().catch(() => {
            // Local engine audio still works without remote voices.
          })
        : Promise.resolve();
      await playerStart;
      await remoteStart;
    }
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
    remoteAudio?.setMuted(audioMuted);
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
    remoteAudio?.setMuted(audioMuted);
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
  await init(versionedWasmUrl("../pkg/sim_render_bg.wasm"));

  status.textContent = "loading packs";
  const trackPackPromise =
    trackParam === EDITOR_TRACK_ID
      ? loadEditorTrackPack()
      : trackParam === TRACK_ED_TRACK_ID
        ? loadTrackEdPack()
        : loadPack(TRACK_URL);
  const [trackPack, carPack] = await Promise.all([
    trackPackPromise,
    loadPack(CAR_URL),
  ]);

  resizeCanvas();
  status.textContent = "creating physics";
  const hasRace = trackPack.manifest.entrypoints.race != null;
  const aiCount = resolveAiCount(hasRace);
  const aiPace = resolveAiPace();
  const aiRoster =
    hasRace && !mpMode && !soloMode
      ? selectAiRoster(selectedCategory, carPack.manifest.id, aiCount)
      : [];
  const aiPackBytes = new Map<string, Uint8Array>();
  const aiPackUrls = new Map<string, string>();
  for (const car of aiRoster) {
    if (car.id === carPack.manifest.id) {
      aiPackBytes.set(car.id, carPack.bytes);
      continue;
    }
    aiPackUrls.set(car.id, withPackVersion(car.url, APP_VERSION));
  }
  const uniqueAiUrls = [...new Set(aiPackUrls.values())];
  await Promise.all(
    uniqueAiUrls.map(async (url) => {
      const pack = await loadPack(url);
      for (const [carId, carUrl] of aiPackUrls) {
        if (carUrl === url) {
          aiPackBytes.set(carId, pack.bytes);
        }
      }
    }),
  );
  const aiSlotPacks = Array.from(
    { length: aiCount },
    (_, index) => {
      const car = aiRoster[index];
      return car ? aiPackBytes.get(car.id) ?? carPack.bytes : carPack.bytes;
    },
  );
  const playerPaintable = catalogCar?.paintable === true;
  const aiPaintable = aiRoster.map((car) => car.paintable === true);
  let slotCarPacks: Uint8Array[];
  let slotPaintable: boolean[];
  if (mpMode) {
    slotCarPacks = Array.from({ length: REMOTE_SLOTS + 1 }, () => carPack.bytes);
    slotPaintable = Array.from(
      { length: REMOTE_SLOTS + 1 },
      () => playerPaintable,
    );
  } else if (aiCount > 0) {
    slotCarPacks = [carPack.bytes, ...aiSlotPacks];
    slotPaintable = [playerPaintable, ...aiPaintable];
  } else {
    slotCarPacks = [carPack.bytes];
    slotPaintable = [playerPaintable];
  }
  const spectatorSlot = resolveSpectatorSlot(
    params.get("spectate"),
    aiCount,
    mpMode,
    soloMode,
  );
  spectatorMode = spectatorSlot !== null;
  const viewer = await DriveViewer.create(
    canvas,
    trackPack.bytes,
    slotCarPacks,
    carBay,
    storedDevTuning ?? undefined,
    mpMode ? REMOTE_SLOTS : aiCount,
    carPaint,
    slotPaintable,
  );
  const treeDetail = new URLSearchParams(window.location.search).get("trees");
  if (treeDetail === "high" || treeDetail === "low" || treeDetail === "cards") {
    viewer.set_tree_detail(treeDetail);
  }
  if (collisionDebug) {
    viewer.set_collision_debug(true);
  }
  saveSelectedBay(carBay, categoryCars.length);

  const info = JSON.parse(viewer.info());
  const initial = JSON.parse(viewer.telemetry());
  const raceView = JSON.parse(viewer.race());
  raceSectorCount =
    raceView.available && Array.isArray(raceView.sectors)
      ? raceView.sectors.length
      : 0;
  const gridCount =
    raceView.available && Array.isArray(raceView.grid)
      ? raceView.grid.length
      : 0;
  // Clamp the effective field to the race grid. The viewer and worker are
  // created with the requested count, so a legacy 8-slot pack hides the extra
  // AI slots instead of letting standings and the player grid overrun it.
  const effectiveAiCount =
    gridCount > 0 ? Math.min(aiCount, gridCount - 1) : 0;
  playerGridSlot = effectiveAiCount;
  const aiSlots = Array.from(
    { length: effectiveAiCount },
    (_, index) => index + 1,
  );
  const configuredRaceLaps = hasRace && raceView.available
    ? resolveRaceLaps()
    : DEFAULT_RACE_LAPS;
  const configuredRaceDirector =
    hasRace && !mpMode && !soloMode && raceView.available
      ? new RaceDirector({
          totalLaps: configuredRaceLaps,
          layoutLength: raceLayoutLength(raceView.layout),
          aiSlots,
          startOrder: [...aiSlots, 0],
        })
      : null;
  const setup = JSON.parse(viewer.car_setup());
  syncCarSetup(setup);
  lastRpm = initial.rpm;
  asset.textContent = `${info.asset.id} + ${info.car.id} · ${initial.carColor}`;

  // Player audio is independent of the physics worker. Create it before the
  // worker starts so a worker failure (or a slow AI field init) cannot leave
  // the session silent. Opponent voices are only added once the worker is up.
  const carOption = findCar(carPack.manifest.id);
  playerAudioProfile = carOption
    ? { engine: carOption.engine, vehicle: carOption.vehicle }
    : undefined;
  audio = new EngineSimAudio(playerAudioProfile);
  sessionHasOpponents = mpMode || (!soloMode && aiCount > 0);
  sessionAiRoster = aiRoster;
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
      remoteAudio?.suspend();
    } else {
      void audio.resume().catch((error) => {
        console.warn("audio resume unavailable", error);
      });
      void remoteAudio?.resume();
      // If the browser still holds the context suspended, the next gesture
      // must resume it.
      armAudioGesture();
    }
  });
  window.addEventListener("pagehide", () => {
    audio?.dispose();
    remoteAudio?.dispose();
  });
  audioButton.addEventListener("click", toggleAudio);

  let driver: DriveViewer = viewer;
  if (params.get("physics") !== "main" && window.crossOriginIsolated) {
    try {
      status.textContent = "starting physics worker";
      const sab = new SharedArrayBuffer(SAB_TOTAL_SIZE);
      initializeSab(sab);
      validateSab(sab);
      ffbSab = sab;
      const client = await PhysicsClient.create(
        sab,
        trackPack.bytes,
        carPack.bytes,
        aiSlotPacks,
        aiPace,
      );
      physics = client;
      client.onAiFrame = (slot, frame, race) => {
        if (aiIgnoreFrames) {
          return;
        }
        const tick =
          frame.length > SNAPSHOT_STATE.tick
            ? Math.round(frame[SNAPSHOT_STATE.tick])
            : Number.NaN;
        if (raceDirector) {
          raceDirector.updateAi(slot, race);
        }
        if (!Number.isFinite(tick) || tick < 0) {
          return;
        }
        const nowMs = performance.now();
        if (aiDebugHistory) {
          pushBounded(
            aiDebugHistory.raw,
            sampleRawAiFrame(slot, frame, nowMs, tick),
            aiDebugHistory.limit,
          );
        }
        let buffer = aiBuffers.get(slot);
        if (!buffer) {
          buffer = new RemoteStateBuffer(AI_INTERPOLATION_MS);
          aiBuffers.set(slot, buffer);
          // A fresh slot must not inherit a stale smoothed pose.
          aiSmoothedFrames.delete(slot);
        }
        if (aiRenderMode === "raw") {
          // Keep only the newest raw frame per slot; the raw diagnostic draws
          // it directly without touching the interpolation buffer.
          aiRawFrames.set(slot, Float32Array.from(frame));
        }
        buffer.push(tick, tick, frame, nowMs);
      };
      client.onContact = (contact) => handleContact(contact, viewer);
      playerState = new PlayerStateBuffer();
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
        if (!physics || mpMode) {
          return;
        }
        if (document.hidden) {
          physics.pause();
        } else {
          physics.resume();
        }
      });
      window.addEventListener("pagehide", () => {
        stopRelayUpload();
        relay?.close();
        physics?.dispose();
      });
      driver = createDriveFacade(viewer, client);
      if (configuredRaceDirector) {
        raceDirector = configuredRaceDirector;
        raceLaps = configuredRaceLaps;
        localRace = true;
      }
    } catch (error) {
      console.warn("SAB physics unavailable; using main thread", error);
      physics?.dispose();
      physics = null;
      ffbSab = null;
      playerState = null;
      physicsWorker = false;
      driver = viewer;
    }
  }
  if (!localRace && configuredRaceDirector) {
    raceDirector = new RaceDirector({
      totalLaps: configuredRaceLaps,
      layoutLength: configuredRaceDirector.layoutLength,
      aiSlots: [],
    });
    raceLaps = configuredRaceLaps;
    localRace = true;
  }
  window.__driveViewer = driver;

  if (spectatorSlot !== null) {
    // Follow an AI car and hide the local body; input stays locked neutral so
    // the player cannot steer the race from the spectator camera.
    driver.set_spectator_slot(spectatorSlot);
    setBodyVisible(false);
  }

  const rpmOffset = cockpitRpmOffset(info.car.id);
  if (rpmOffset) {
    driver.set_cockpit_dial_offset("rpm", ...rpmOffset);
  }

  if (mpMode) {
    status.textContent = "joining room";
    await setupMultiplayer(viewer, info);
  }

  if (physicsWorker) {
    // Opponent voices need AI frames from the worker; player audio is already
    // active even if the worker failed.
    createRemoteAudio();
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
  raceRestartButton.addEventListener("click", reset);
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
      playerSteeringRate = next.steeringRate;
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
        // The player context was recreated, so remote voices must reattach.
        createRemoteAudio();
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

  if (soloMode) {
    racePanel.hidden = true;
    raceResults.hidden = true;
    raceCountdown.hidden = true;
  }

  if (localRace) {
    await startLocalRaceCountdown();
  }

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
      steer = rateLimitSteering(steer, targetSteer, playerSteeringRate, delta);
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
    let handbrake = Math.max(
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

    if (inputLocked) {
      steer = 0;
      throttle = 0;
      brake = 0;
      clutch = 1;
      gear = 1;
      handbrake = 0;
    }

    try {
      if (physicsWorker && physics && playerState) {
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
        const snapshot = physics.reader.readLatest();
        if (snapshot) {
          playerState.push(snapshot, renderNow);
        }
        const remoteState = playerState.frame(renderNow);
        if (remoteState) {
          renderRemotePlayers(renderNow, remoteState, delta);
          renderAiFrames(renderNow, remoteState);
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
