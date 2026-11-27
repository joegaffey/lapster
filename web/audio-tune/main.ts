// Engine audio tuning tool.
//
// Standalone dev page: the real Drive renderer runs in an iframe for the car
// and animated wheels; this page owns a separate sample voice so audio files
// and RPM->playback-rate mappings can be auditioned per car.

import {
  CAR_OPTIONS,
  type CarOption,
} from "../src/ui/car-selection.ts";
import {
  ENGINE_PROFILES,
  VEHICLE_PRESETS,
} from "../src/audio/engine-sim/profiles.js";

interface SampleOption {
  id: string;
  label: string;
  url: string;
}

interface Mapping {
  fileId: string;
  rpmLow: number;
  rpmHigh: number;
  rateLow: number;
  rateHigh: number;
  curve: number;
  gain: number;
}

interface LiveState {
  rpm: number;
  speedKph: number;
  gear: number;
  ready: boolean;
  error: string;
}

const SAMPLES: SampleOption[] = [
  { id: "rotary2", label: "Mazda rotary2 (source)", url: "../assets/audio/speed-dreams/rotary2.wav" },
  { id: "v8-ears68", label: "Deckard v8-ears68 (source)", url: "../assets/audio/speed-dreams/v8-ears68.wav" },
  { id: "v12-smooth", label: "V12 smooth", url: "../assets/audio/speed-dreams/v12_smooth.wav" },
  { id: "f123bis", label: "Cavallo f123bis (source)", url: "../assets/audio/speed-dreams/cars/f123bis.wav" },
  { id: "gt3o", label: "GT3O (Guthrie/Kuiper/Upping)", url: "../assets/audio/speed-dreams/cars/GT3O.wav" },
  { id: "formulaford", label: "Mango formulaford (source)", url: "../assets/audio/speed-dreams/cars/formulaford.wav" },
  { id: "037a", label: "Murasama 037a (source)", url: "../assets/audio/speed-dreams/cars/037a.wav" },
  { id: "fallback-rotary2", label: "Fallback rotary2", url: "../assets/audio/fallback/rotary2.wav" },
  { id: "fallback-v8cross", label: "Fallback v8cross", url: "../assets/audio/fallback/v8cross.wav" },
  { id: "fallback-v12", label: "Fallback v12", url: "../assets/audio/fallback/v12.wav" },
];

const SAMPLE_BY_ID = new Map(SAMPLES.map((sample) => [sample.id, sample]));

const DEFAULT_SAMPLE_BY_CAR: Record<string, string> = {
  "mazda-rx7": "rotary2",
  "sc-deckard-conejo": "v8-ears68",
  "67gp-cavallo-123": "f123bis",
  "67gp-guthrie-eagle": "gt3o",
  "67gp-kuiper-t81": "gt3o",
  "67gp-murasama-37a": "037a",
  "67gp-mango-ms7": "formulaford",
  "67gp-upping-ut19": "gt3o",
};

const DEFAULT_RATES: Record<string, [number, number]> = {
  rotary2: [0.7, 1.4],
  v8cross: [0.75, 1.45],
  v12: [0.7, 1.4],
};

const STORE_KEY = "lapster.audioTune.v1";

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

const carSelect = $<HTMLSelectElement>("car");
const sampleSelect = $<HTMLSelectElement>("sample");
const fileInput = $<HTMLInputElement>("file");
const modeSelect = $<HTMLSelectElement>("mode");
const manualRow = $<HTMLDivElement>("manual-row");
const gearRow = $<HTMLDivElement>("gear-row");
const manualRpm = $<HTMLInputElement>("manual-rpm");
const gearSelect = $<HTMLSelectElement>("gear");
const speedInput = $<HTMLInputElement>("speed");
const graph = $<HTMLCanvasElement>("graph");
const status = $<HTMLDivElement>("status");
const liveBox = $<HTMLDivElement>("live");
const metaBox = $<HTMLDivElement>("meta");

let store: Record<string, Mapping> = {};
try {
  store = JSON.parse(localStorage.getItem(STORE_KEY) ?? "{}") as Record<string, Mapping>;
} catch {
  store = {};
}

let car: CarOption = CAR_OPTIONS[0];
let mapping: Mapping = defaultMapping(car);
let loadedSetup: any = null;
let live: LiveState = { rpm: 0, speedKph: 0, gear: 1, ready: false, error: "" };
let customUrls = new Map<string, string>();
let currentRpm = 0;
let lastRate = 0;
let tunerMuted = false;

function saveStore(): void {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(store));
  } catch {
    // Storage unavailable.
  }
}

function engineMeta(target: CarOption): any {
  return (ENGINE_PROFILES as Record<string, any>)[target.engine] ?? {};
}

function vehicleMeta(target: CarOption): any {
  return (VEHICLE_PRESETS as Record<string, any>)[target.vehicle] ?? {};
}

function setupDrivetrain(): any {
  return loadedSetup?.drivetrain ?? {};
}

function gearRatios(): number[] {
  const loaded = setupDrivetrain().gearRatios;
  if (Array.isArray(loaded) && loaded.length > 0) return loaded;
  const preset = vehicleMeta(car).gearRatios;
  return Array.isArray(preset) ? preset : [1];
}

function finalDrive(): number {
  const loaded = setupDrivetrain().finalDrive;
  return Number.isFinite(loaded) ? loaded : Number(vehicleMeta(car).finalDrive) || 3.7;
}

function wheelRadius(): number {
  return Number(vehicleMeta(car).wheelRadius) || 0.33;
}

function defaultMapping(target: CarOption): Mapping {
  const engine = engineMeta(target);
  const rates = DEFAULT_RATES[target.engine] ?? [0.75, 1.45];
  return {
    fileId: DEFAULT_SAMPLE_BY_CAR[target.id] ?? "v8-ears68",
    rpmLow: Number(engine.idleRpm) || 900,
    rpmHigh: Number(engine.redlineRpm) || 8000,
    rateLow: rates[0],
    rateHigh: rates[1],
    curve: 1,
    gain: 0.55,
  };
}

function mappingFor(target: CarOption): Mapping {
  return store[target.id] ?? defaultMapping(target);
}

function sampleUrl(fileId: string): string {
  if (fileId.startsWith("custom:")) {
    return customUrls.get(fileId) ?? (SAMPLE_BY_ID.get(DEFAULT_SAMPLE_BY_CAR[car.id] ?? "")?.url ?? SAMPLES[1].url);
  }
  return SAMPLE_BY_ID.get(fileId)?.url ?? SAMPLE_BY_ID.get("v8-ears68")?.url ?? SAMPLES[1].url;
}

function mapRate(m: Mapping, rpm: number): number {
  const span = Math.max(1, m.rpmHigh - m.rpmLow);
  const t = Math.max(0, Math.min(1, (rpm - m.rpmLow) / span));
  return m.rateLow + (m.rateHigh - m.rateLow) * Math.pow(t, m.curve);
}

class TunerAudio {
  private context: AudioContext | null = null;
  private source: AudioBufferSourceNode | null = null;
  private gain: GainNode | null = null;
  private master: GainNode | null = null;
  private url = "";
  private mapping: Mapping | null = null;
  private token = 0;
  private lastRpm = 0;
  private muted = false;
  running = false;

  async start(): Promise<void> {
    if (!this.context) {
      const context = new AudioContext({ latencyHint: "interactive" });
      this.context = context;
      await context.resume();
      const master = context.createGain();
      master.gain.value = 0.5;
      const compressor = context.createDynamicsCompressor();
      compressor.threshold.value = -14;
      compressor.ratio.value = 6;
      compressor.attack.value = 0.003;
      compressor.release.value = 0.25;
      master.connect(compressor);
      compressor.connect(context.destination);
      const gain = context.createGain();
      gain.gain.value = 0;
      gain.connect(master);
      this.master = master;
      this.gain = gain;
    }
    await this.context.resume();
    this.running = true;
    if (this.url) await this.load(this.url);
  }

  async load(url: string): Promise<void> {
    this.url = url;
    const context = this.context;
    if (!context) return;
    const token = ++this.token;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`${response.status} ${url}`);
    const buffer = await context.decodeAudioData(await response.arrayBuffer());
    if (token !== this.token) return;
    this.source?.stop();
    this.source?.disconnect();
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.loop = true;
    source.connect(this.gain as GainNode);
    source.start();
    this.source = source;
    if (this.mapping) this.apply(this.mapping, this.lastRpm);
  }

  setMapping(m: Mapping): void {
    this.mapping = m;
    if (this.context) this.apply(m, this.lastRpm);
  }

  update(rpm: number): void {
    this.lastRpm = rpm;
    if (this.mapping) this.apply(this.mapping, rpm);
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    if (!this.master || !this.context) return;
    this.master.gain.setTargetAtTime(muted ? 0 : 0.5, this.context.currentTime, 0.03);
  }

  private apply(m: Mapping, rpm: number): void {
    if (!this.context || !this.source || !this.gain) return;
    const now = this.context.currentTime;
    const rate = mapRate(m, rpm);
    this.source.playbackRate.setTargetAtTime(rate, now, 0.04);
    this.gain.gain.setTargetAtTime(m.gain, now, 0.05);
    lastRate = rate;
  }
}

const tuner = new TunerAudio();

// The game's own audio stays off; this page is the only sound source.
try {
  localStorage.setItem("lapster.audio.v1", JSON.stringify({ enabled: false, muted: true }));
  localStorage.setItem("lapster.audio.dev.v1", JSON.stringify({ enabled: false, muted: true }));
} catch {
  // Storage unavailable.
}

const iframe = document.createElement("iframe");
iframe.id = "car-frame";
iframe.title = "Car";
iframe.allow = "autoplay";
$("stage").append(iframe);

function loadCar(target: CarOption): void {
  live = { rpm: 0, speedKph: 0, gear: 1, ready: false, error: "" };
  iframe.src = `../drive/?track=race-loop&race=0&car=${encodeURIComponent(target.id)}&v=audio-tune-${Date.now()}`;
}

function iframeWindow(): any | null {
  try {
    return iframe.contentWindow;
  } catch {
    return null;
  }
}

function hideHud(): void {
  try {
    const doc = iframe.contentDocument;
    if (!doc) return;
    const style = doc.createElement("style");
    style.textContent =
      "#hud,#viewer-controls,#menu-open,#app-menu,#net-badge,#race-panel,#touch-controls,#viewer-loading{display:none!important}";
    doc.head.append(style);
  } catch {
    // Cross-origin or not ready.
  }
}

function readSetup(): void {
  const win = iframeWindow();
  try {
    loadedSetup = win?.__driveViewer ? JSON.parse(win.__driveViewer.car_setup()) : null;
  } catch {
    loadedSetup = null;
  }
}

function waitForCar(): void {
  const started = Date.now();
  const tick = (): void => {
    const win = iframeWindow();
    if (win?.__driveViewerReady || win?.__driveViewerError) {
      hideHud();
      readSetup();
      live.ready = win.__driveViewerReady === true;
      live.error = win.__driveViewerError ?? "";
      renderMeta();
      rebuildGearSelect();
      setStatus(live.error ? `Car failed: ${live.error}` : "Car ready. Start audio and drive.");
      return;
    }
    if (Date.now() - started > 60000) {
      setStatus("Car view timed out.");
      return;
    }
    setTimeout(tick, 150);
  };
  tick();
}

function setStatus(text: string): void {
  status.textContent = text;
}

function renderMeta(): void {
  const engine = engineMeta(car);
  const vehicle = vehicleMeta(car);
  const ratios = gearRatios();
  const fdrive = finalDrive();
  const radius = wheelRadius();
  const loaded = setupDrivetrain();
  const rows: Array<[string, string]> = [
    ["Car", `${car.name} (${car.id})`],
    ["Engine", `${car.engine} · ${engine.label ?? "unknown"}`],
    ["Firing geometry", `${engine.cylinders ?? "?"} firing events`],
    ["Engine gears", `${engine.gears ?? "?"} forward`],
    ["Idle / redline", `${engine.idleRpm ?? "?"} / ${engine.redlineRpm ?? "?"} rpm`],
    ["Peak torque", `${engine.peakTorque ?? "?"} Nm @ ${engine.peakTorqueRpm ?? "?"} rpm`],
    ["Gearbox preset", `${vehicle.label ?? car.vehicle}`],
    ["Gear ratios", ratios.map((r: number) => r.toFixed(2)).join(", ")],
    ["Final drive", `${Number(fdrive).toFixed(2)}:1`],
    ["Wheel radius", `${Number(radius).toFixed(3)} m`],
    ["Loaded setup", loaded.gearRatios ? "yes (car pack)" : "catalog preset"],
  ];
  metaBox.innerHTML = `<table>${rows
    .map(([k, v]) => `<tr><td>${k}</td><td><code>${v}</code></td></tr>`)
    .join("")}</table>`;
}

function rebuildGearSelect(): void {
  const ratios = gearRatios();
  const current = gearSelect.value;
  gearSelect.innerHTML = ratios
    .map((r: number, i: number) => `<option value="${i}">${i + 1} — ${r.toFixed(2)}</option>`)
    .join("");
  gearSelect.value = ratios[Number(current)] ? current : "0";
  updateGearRpm();
}

function computedGearRpm(): number {
  const ratios = gearRatios();
  const index = Math.max(0, Math.min(ratios.length - 1, Number(gearSelect.value) || 0));
  const ratio = ratios[index] || 1;
  const speedMps = (Number(speedInput.value) || 0) / 3.6;
  return (speedMps / (2 * Math.PI * wheelRadius())) * ratio * finalDrive() * 60;
}

function updateGearRpm(): void {
  const rpm = computedGearRpm();
  const redline = Number(engineMeta(car).redlineRpm) || 9000;
  const over = rpm > redline;
  $("gear-rpm").textContent = `${Math.round(rpm)} rpm${over ? " — over redline" : ""}`;
  $("gear-rpm").classList.toggle("warn", over);
  $("speed-value").textContent = `${Math.round(Number(speedInput.value))} km/h`;
}

function activeRpm(): number {
  if (modeSelect.value === "manual") return Number(manualRpm.value) || 0;
  if (modeSelect.value === "gear") return computedGearRpm();
  return live.rpm;
}

function updateModeRows(): void {
  manualRow.classList.toggle("hidden", modeSelect.value !== "manual");
  gearRow.classList.toggle("hidden", modeSelect.value !== "gear");
}

function applyMappingToUi(): void {
  const set = (id: string, value: number, digits = 0): void => {
    const input = $<HTMLInputElement>(id);
    input.value = String(value);
    $(`${id}-value`).textContent = digits ? value.toFixed(digits) : String(Math.round(value));
  };
  set("rpm-low", mapping.rpmLow);
  set("rpm-high", mapping.rpmHigh);
  set("rate-low", mapping.rateLow, 2);
  set("rate-high", mapping.rateHigh, 2);
  set("curve", mapping.curve, 2);
  set("gain", mapping.gain, 2);
  manualRpm.min = String(Math.min(500, mapping.rpmLow));
  manualRpm.max = String(Math.max(10000, mapping.rpmHigh));
  if (Number(manualRpm.value) < Number(manualRpm.min) || !manualRpm.value) {
    manualRpm.value = String(mapping.rpmLow);
  }
  const hasOption = [...sampleSelect.options].some(
    (option) => option.value === mapping.fileId,
  );
  sampleSelect.value = hasOption ? mapping.fileId : "";
  tuner.setMapping(mapping);
}

function readMappingFromUi(): void {
  mapping = {
    ...mapping,
    rpmLow: Number($<HTMLInputElement>("rpm-low").value),
    rpmHigh: Number($<HTMLInputElement>("rpm-high").value),
    rateLow: Number($<HTMLInputElement>("rate-low").value),
    rateHigh: Number($<HTMLInputElement>("rate-high").value),
    curve: Number($<HTMLInputElement>("curve").value),
    gain: Number($<HTMLInputElement>("gain").value),
  };
  store[car.id] = mapping;
  saveStore();
  tuner.setMapping(mapping);
  applyMappingToUi();
}

async function loadSample(fileId: string): Promise<void> {
  mapping.fileId = fileId;
  store[car.id] = mapping;
  saveStore();
  const url = sampleUrl(fileId);
  try {
    await tuner.load(url);
    setStatus(`Sample loaded: ${SAMPLE_BY_ID.get(fileId)?.label ?? fileId}`);
  } catch (error) {
    setStatus(`Sample failed: ${error}`);
  }
}

function selectCar(target: CarOption): void {
  car = target;
  mapping = { ...mappingFor(car) };
  if (mapping.fileId.startsWith("custom:") && !customUrls.has(mapping.fileId)) {
    mapping.fileId = DEFAULT_SAMPLE_BY_CAR[car.id] ?? "v8-ears68";
  }
  applyMappingToUi();
  renderMeta();
  rebuildGearSelect();
  loadCar(car);
  waitForCar();
  void loadSample(mapping.fileId);
}

function sendKey(key: string, down: boolean): void {
  const win = iframeWindow();
  if (!win) return;
  win.dispatchEvent(
    new KeyboardEvent(down ? "keydown" : "keyup", {
      key,
      code: key.length === 1 ? `Key${key.toUpperCase()}` : key,
      bubbles: true,
      cancelable: true,
    }),
  );
}

function holdButton(id: string, key: string): void {
  const button = $(id);
  let held = false;
  const down = (event: PointerEvent): void => {
    event.preventDefault();
    if (held) return;
    held = true;
    button.classList.add("active");
    sendKey(key, true);
    try {
      (event.target as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Ignore.
    }
  };
  const up = (): void => {
    if (!held) return;
    held = false;
    button.classList.remove("active");
    sendKey(key, false);
  };
  button.addEventListener("pointerdown", down);
  button.addEventListener("pointerup", up);
  button.addEventListener("pointercancel", up);
  button.addEventListener("pointerleave", up);
  window.addEventListener("pointerup", up);
}

function drawGraph(rpm: number): void {
  const context = graph.getContext("2d");
  if (!context) return;
  const dpr = window.devicePixelRatio || 1;
  const width = graph.clientWidth;
  const height = graph.clientHeight;
  if (graph.width !== Math.round(width * dpr) || graph.height !== Math.round(height * dpr)) {
    graph.width = Math.round(width * dpr);
    graph.height = Math.round(height * dpr);
  }
  context.setTransform(dpr, 0, 0, dpr, 0, 0);
  context.clearRect(0, 0, width, height);
  const pad = 8;
  const xMax = Math.max(mapping.rpmHigh, mapping.rpmLow, 1000) * 1.05;
  const yMin = Math.min(mapping.rateLow, mapping.rateHigh) * 0.85;
  const yMax = Math.max(mapping.rateLow, mapping.rateHigh) * 1.15;
  const x = (value: number): number => pad + (Math.max(0, Math.min(xMax, value)) / xMax) * (width - pad * 2);
  const y = (value: number): number =>
    height - pad - ((value - yMin) / Math.max(0.001, yMax - yMin)) * (height - pad * 2);

  context.strokeStyle = "#232b34";
  context.lineWidth = 1;
  for (let i = 0; i <= 4; i += 1) {
    const gy = pad + ((height - pad * 2) * i) / 4;
    context.beginPath();
    context.moveTo(pad, gy);
    context.lineTo(width - pad, gy);
    context.stroke();
  }
  context.strokeStyle = "#8fd3ff";
  context.lineWidth = 2;
  context.beginPath();
  for (let i = 0; i <= 120; i += 1) {
    const r = (xMax * i) / 120;
    const px = x(r);
    const py = y(mapRate(mapping, r));
    if (i === 0) context.moveTo(px, py);
    else context.lineTo(px, py);
  }
  context.stroke();

  const cx = x(rpm);
  const cy = y(mapRate(mapping, rpm));
  context.fillStyle = "#ffd479";
  context.beginPath();
  context.arc(cx, cy, 4, 0, Math.PI * 2);
  context.fill();

  context.fillStyle = "#7d8996";
  context.font = "10px system-ui";
  context.fillText(`${Math.round(mapping.rpmLow)}`, pad, height - 1);
  context.fillText(`${Math.round(xMax)} rpm`, width - 62, height - 1);
  context.fillText(`${yMax.toFixed(2)}x`, 2, pad + 8);
}

function updateLive(rpm: number, speed: number, gear: number): void {
  $("rate-readout").textContent = `rpm ${Math.round(rpm)} → ${lastRate.toFixed(3)}x`;
  liveBox.innerHTML = `
    <div class="k">Speed</div><div class="v">${speed.toFixed(0)} km/h</div>
    <div class="k">Live rpm</div><div class="v">${Math.round(live.rpm)}</div>
    <div class="k">Live gear</div><div class="v">${live.gear}</div>
    <div class="k">Test rpm</div><div class="v">${Math.round(rpm)}</div>
    <div class="k">Playback rate</div><div class="v">${lastRate.toFixed(3)}×</div>
    <div class="k">Sample</div><div class="v">${SAMPLE_BY_ID.get(mapping.fileId)?.label ?? mapping.fileId}</div>
  `;
}

function frame(): void {
  const win = iframeWindow();
  if (win?.__driveViewer) {
    try {
      const telemetry = JSON.parse(win.__driveViewer.telemetry());
      live.rpm = Number(telemetry.rpm) || 0;
      live.speedKph = Number(telemetry.speedKph) || 0;
      live.gear = Number(telemetry.gear) || 0;
      live.ready = win.__driveViewerReady === true;
      live.error = win.__driveViewerError ?? "";
    } catch {
      // Ignore transient telemetry failures.
    }
  }
  if (modeSelect.value === "gear") updateGearRpm();
  const rpm = activeRpm();
  currentRpm = rpm;
  lastRate = mapRate(mapping, rpm);
  tuner.update(rpm);
  drawGraph(rpm);
  updateLive(rpm, live.speedKph, live.gear);
  requestAnimationFrame(frame);
}

function initialize(): void {
  carSelect.innerHTML = CAR_OPTIONS.map(
    (option) => `<option value="${option.id}">${option.name}</option>`,
  ).join("");
  sampleSelect.innerHTML = SAMPLES.map(
    (sample) => `<option value="${sample.id}">${sample.label}</option>`,
  ).join("");

  carSelect.addEventListener("change", () => {
    const next = CAR_OPTIONS.find((option) => option.id === carSelect.value);
    if (next) selectCar(next);
  });
  sampleSelect.addEventListener("change", () => {
    void loadSample(sampleSelect.value);
  });
  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    if (!file) return;
    const id = `custom:${file.name}`;
    const url = URL.createObjectURL(file);
    customUrls.set(id, url);
    sampleSelect.insertAdjacentHTML(
      "beforeend",
      `<option value="${id}">${file.name}</option>`,
    );
    sampleSelect.value = id;
    void loadSample(id);
  });
  $("car-sample").addEventListener("click", () => {
    const id = DEFAULT_SAMPLE_BY_CAR[car.id] ?? "v8-ears68";
    sampleSelect.value = id;
    void loadSample(id);
  });
  modeSelect.addEventListener("change", updateModeRows);
  manualRpm.addEventListener("input", () => {
    $("manual-rpm-value").textContent = `${manualRpm.value} rpm`;
  });
  speedInput.addEventListener("input", updateGearRpm);
  gearSelect.addEventListener("change", updateGearRpm);

  const mappingInputs = ["rpm-low", "rpm-high", "rate-low", "rate-high", "curve", "gain"];
  for (const id of mappingInputs) {
    $(id).addEventListener("input", readMappingFromUi);
  }

  $("audio").addEventListener("click", async () => {
    try {
      await tuner.start();
      await loadSample(mapping.fileId);
      $("audio").textContent = "Audio on";
      $("audio").classList.add("active");
      setStatus("Audio running.");
    } catch (error) {
      setStatus(`Audio failed: ${error}`);
    }
  });
  $("mute").addEventListener("click", () => {
    tunerMuted = !tunerMuted;
    tuner.setMuted(tunerMuted);
    $("mute").classList.toggle("active", tunerMuted);
  });
  $("copy").addEventListener("click", async () => {
    const text = JSON.stringify(store, null, 2);
    try {
      await navigator.clipboard.writeText(text);
      setStatus("Mappings copied.");
    } catch {
      setStatus(text.slice(0, 120));
    }
  });
  $("reset").addEventListener("click", () => {
    delete store[car.id];
    mapping = defaultMapping(car);
    saveStore();
    applyMappingToUi();
    void loadSample(mapping.fileId);
    setStatus("Car mapping reset.");
  });

  holdButton("throttle", "w");
  holdButton("brake", "s");
  $("camera").addEventListener("click", () => {
    sendKey("c", true);
    sendKey("c", false);
  });
  $("focus").addEventListener("click", () => {
    iframe.focus();
    iframeWindow()?.focus();
  });

  updateModeRows();
  $("manual-rpm-value").textContent = `${manualRpm.value} rpm`;
  $("speed-value").textContent = `${speedInput.value} km/h`;
  selectCar(car);
  requestAnimationFrame(frame);
}

initialize();
