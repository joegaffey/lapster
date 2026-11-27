// Engine audio tuning tool.
//
// Real garage car models in a studio. A longitudinal road-load model drives the
// wheels and engine audio. Throttle/brake only; no steering. Audio is a toggle;
// auto gears and Neutral are available.

import {
  CAR_OPTIONS,
  carsInCategory,
  findCar,
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
  loadGain: number;
}

interface SimState {
  speedMps: number;
  rpm: number;
  gearIndex: number;
  throttle: number;
  brake: number;
  wheelAngle: number;
  acceleration: number;
  shiftCooldown: number;
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

interface PackMeta {
  idleRpm: number;
  redlineRpm: number;
  gearRatios: number[];
  finalDrive: number;
  peakTorque: number;
  peakTorqueRpm: number;
}

// Engine/gearbox values read from the actual car packs (car_setup()).
const PACK_META: Record<string, PackMeta> = {
  "mazda-rx7": {
    idleRpm: 900, redlineRpm: 8000,
    gearRatios: [3.483, 2.015, 1.391, 1, 0.719, 0.6], finalDrive: 4.1,
    peakTorque: 314, peakTorqueRpm: 4500,
  },
  "sc-deckard-conejo": {
    idleRpm: 1000, redlineRpm: 6930,
    gearRatios: [3.01, 2.07, 1.43, 1, 0.84, 0.57], finalDrive: 3.8,
    peakTorque: 567.2, peakTorqueRpm: 4620,
  },
  "67gp-cavallo-123": {
    idleRpm: 1000, redlineRpm: 10000,
    gearRatios: [3.5, 2.4, 1.795, 1.224, 0.95], finalDrive: 3.73,
    peakTorque: 287.9, peakTorqueRpm: 9250,
  },
  "67gp-guthrie-eagle": {
    idleRpm: 1000, redlineRpm: 9500,
    gearRatios: [3.26, 2.28, 1.5, 1.01, 0.87], finalDrive: 3.73,
    peakTorque: 389.6, peakTorqueRpm: 6500,
  },
  "67gp-kuiper-t81": {
    idleRpm: 1000, redlineRpm: 10000,
    gearRatios: [2.66, 1.78, 1.35, 1.01, 0.87], finalDrive: 3.73,
    peakTorque: 312, peakTorqueRpm: 8600,
  },
  "67gp-murasama-37a": {
    idleRpm: 1000, redlineRpm: 10000,
    gearRatios: [3.5, 2.4, 1.795, 1.224, 0.97], finalDrive: 3.73,
    peakTorque: 323.6, peakTorqueRpm: 8250,
  },
  "67gp-mango-ms7": {
    idleRpm: 1000, redlineRpm: 10000,
    gearRatios: [2.66, 1.78, 1.35, 1.01, 0.86], finalDrive: 3.73,
    peakTorque: 176.8, peakTorqueRpm: 7000,
  },
  "67gp-upping-ut19": {
    idleRpm: 1000, redlineRpm: 9000,
    gearRatios: [2.66, 1.78, 1.35, 1.01, 0.87], finalDrive: 3.73,
    peakTorque: 348.2, peakTorqueRpm: 6000,
  },
};

// Tuned per-car defaults. The Mazda anchors follow the pack: 900/8000.
const DEFAULT_PACK_MAPPINGS: Record<string, Mapping> = {
  "mazda-rx7": {
    fileId: "rotary2",
    rpmLow: 900,
    rpmHigh: 8000,
    rateLow: 0.3,
    rateHigh: 0.8,
    curve: 2.4,
    gain: 0.26,
    loadGain: 0,
  },
};

const STORE_KEY = "lapster.audioTune.v1";

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

const carSelect = $<HTMLSelectElement>("car");
const sampleSelect = $<HTMLSelectElement>("sample");
const fileInput = $<HTMLInputElement>("file");
const throttleInput = $<HTMLInputElement>("throttle");
const brakeInput = $<HTMLInputElement>("brake");
const autoGearsToggle = $<HTMLInputElement>("auto-gears");
const gearSelect = $<HTMLSelectElement>("gear");
const graph = $<HTMLCanvasElement>("graph");
const status = $<HTMLDivElement>("status");
const mappingJson = $<HTMLTextAreaElement>("mapping-json");
const liveBox = $<HTMLDivElement>("live");
const metaBox = $<HTMLDivElement>("meta");
const speedBig = $<HTMLDivElement>("speed-big");
const stageSub = $<HTMLDivElement>("stage-sub");
const rpmBig = $<HTMLDivElement>("rpm-big");
const manualRpmToggle = $<HTMLInputElement>("manual-rpm-toggle");
const manualRpm = $<HTMLInputElement>("manual-rpm");
const manualRpmValue = $<HTMLOutputElement>("manual-rpm-value");
const gearNote = $<HTMLDivElement>("gear-note");
const garageFrame = $<HTMLIFrameElement>("garage-frame");

let store: Record<string, Mapping> = {};
try {
  store = JSON.parse(localStorage.getItem(STORE_KEY) ?? "{}") as Record<string, Mapping>;
} catch {
  store = {};
}

let car: CarOption = CAR_OPTIONS[0];
let mapping: Mapping = defaultMapping(car);
let customUrls = new Map<string, string>();
let lastRate = 0;
let garageReady = false;
let garageCategory = "";
let garageKeysAttached = false;
let audioOn = false;
let keyThrottle = false;
let keyBrake = false;
let autoGears = true;
let manualRpmOn = false;
const sim: SimState = {
  speedMps: 0,
  rpm: 900,
  gearIndex: 0,
  throttle: 0,
  brake: 0,
  wheelAngle: 0,
  acceleration: 0,
  shiftCooldown: 0.5,
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function setStatus(text: string): void {
  status.textContent = text;
}

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

function gearRatios(): number[] {
  const pack = PACK_META[car.id];
  if (pack) return pack.gearRatios;
  const preset = vehicleMeta(car).gearRatios;
  return Array.isArray(preset) && preset.length > 0 ? preset : [1];
}

function finalDrive(): number {
  const pack = PACK_META[car.id];
  if (pack) return pack.finalDrive;
  return Number(vehicleMeta(car).finalDrive) || 3.7;
}

function wheelRadius(): number {
  return Number(vehicleMeta(car).wheelRadius) || 0.33;
}

function massKg(): number {
  return Number(vehicleMeta(car).mass) || 1400;
}

function idleRpm(): number {
  return PACK_META[car.id]?.idleRpm ?? (Number(engineMeta(car).idleRpm) || 900);
}

function redlineRpm(): number {
  return PACK_META[car.id]?.redlineRpm ?? (Number(engineMeta(car).redlineRpm) || 8000);
}

function selectedRatio(): number {
  const index = Number(gearSelect.value);
  if (index < 0) return 0;
  const ratios = gearRatios();
  return ratios[clamp(index, 0, ratios.length - 1)] || 1;
}

function rpmToSpeed(rpm: number): number {
  const ratio = selectedRatio();
  if (ratio <= 0) return 0;
  const wheelRpm = rpm / Math.max(0.1, ratio * finalDrive());
  return (wheelRpm / 60) * 2 * Math.PI * wheelRadius();
}

function defaultMapping(target: CarOption): Mapping {
  const tuned = DEFAULT_PACK_MAPPINGS[target.id];
  if (tuned) return { ...tuned };
  const engine = engineMeta(target);
  const pack = PACK_META[target.id];
  const rates = DEFAULT_RATES[target.engine] ?? [0.75, 1.45];
  return {
    fileId: DEFAULT_SAMPLE_BY_CAR[target.id] ?? "v8-ears68",
    rpmLow: pack?.idleRpm ?? (Number(engine.idleRpm) || 900),
    rpmHigh: pack?.redlineRpm ?? (Number(engine.redlineRpm) || 8000),
    rateLow: rates[0],
    rateHigh: rates[1],
    curve: 1,
    gain: 0.55,
    loadGain: 0.35,
  };
}

function mappingFor(target: CarOption): Mapping {
  return { ...defaultMapping(target), ...(store[target.id] ?? {}) };
}

function sampleUrl(fileId: string): string {
  if (fileId.startsWith("custom:")) {
    return customUrls.get(fileId) ?? (SAMPLE_BY_ID.get(DEFAULT_SAMPLE_BY_CAR[car.id] ?? "")?.url ?? SAMPLES[1].url);
  }
  return SAMPLE_BY_ID.get(fileId)?.url ?? SAMPLE_BY_ID.get("v8-ears68")?.url ?? SAMPLES[1].url;
}

function mapRate(m: Mapping, rpm: number): number {
  const span = Math.max(1, m.rpmHigh - m.rpmLow);
  const t = clamp((rpm - m.rpmLow) / span, 0, 1);
  return m.rateLow + (m.rateHigh - m.rateLow) * Math.pow(t, m.curve);
}

function normalizeMapping(target: CarOption, value: any): Mapping {
  const base = defaultMapping(target);
  const num = (v: any, fallback: number): number => {
    const n = Number(v);
    return Number.isFinite(n) ? n : fallback;
  };
  return {
    fileId: typeof value?.fileId === "string" ? value.fileId : base.fileId,
    rpmLow: num(value?.rpmLow, base.rpmLow),
    rpmHigh: num(value?.rpmHigh, base.rpmHigh),
    rateLow: num(value?.rateLow, base.rateLow),
    rateHigh: num(value?.rateHigh, base.rateHigh),
    curve: num(value?.curve, base.curve),
    gain: num(value?.gain, base.gain),
    loadGain: num(value?.loadGain, base.loadGain),
  };
}

function updateMappingJson(): void {
  mappingJson.value = JSON.stringify({ [car.id]: mapping }, null, 2);
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
  private lastLoad = 0;
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
    if (this.mapping) this.apply(this.mapping, this.lastRpm, this.lastLoad);
  }

  setMapping(m: Mapping): void {
    this.mapping = m;
    if (this.context) this.apply(m, this.lastRpm, this.lastLoad);
  }

  update(rpm: number, load: number): void {
    this.lastRpm = rpm;
    this.lastLoad = load;
    if (this.mapping) this.apply(this.mapping, rpm, load);
  }

  setMuted(muted: boolean): void {
    if (!this.master || !this.context) return;
    this.master.gain.setTargetAtTime(muted ? 0 : 0.5, this.context.currentTime, 0.03);
  }

  private apply(m: Mapping, rpm: number, load: number): void {
    if (!this.context || !this.source || !this.gain) return;
    const now = this.context.currentTime;
    this.source.playbackRate.setTargetAtTime(mapRate(m, rpm), now, 0.04);
    const loadGain = clamp(m.loadGain ?? 0.35, 0, 1);
    const throttleFactor = 0.35 + 0.65 * clamp(load, 0, 1);
    const gainFactor = 1 + loadGain * (throttleFactor - 1);
    const level = m.gain * clamp(gainFactor, 0.2, 1.2);
    this.gain.gain.setTargetAtTime(level, now, 0.05);
  }
}

const tuner = new TunerAudio();

function torqueFactor(rpm: number): number {
  const idle = idleRpm();
  const redline = redlineRpm();
  const pack = PACK_META[car.id];
  const peak = clamp(
    ((pack?.peakTorqueRpm ?? Number(engineMeta(car).peakTorqueRpm)) || (idle + redline) / 2) - idle,
    1,
    redline - idle,
  ) / Math.max(1, redline - idle);
  const x = clamp((rpm - idle) / Math.max(1, redline - idle), 0, 1.1);
  return 0.55 + 0.45 * Math.exp(-Math.pow((x - peak) / 0.4, 2));
}

function simulate(dt: number): void {
  sim.shiftCooldown = Math.max(0, sim.shiftCooldown - dt);
  sim.gearIndex = Number(gearSelect.value);
  if (manualRpmOn) {
    const idle = idleRpm();
    const redline = redlineRpm();
    sim.rpm = clamp(Number(manualRpm.value) || idle, idle, redline);
    const ratio = selectedRatio();
    sim.speedMps = ratio > 0 ? rpmToSpeed(sim.rpm) : 0;
    sim.throttle = 1;
    sim.brake = 0;
    sim.acceleration = 0;
    sim.wheelAngle += (sim.speedMps / Math.max(0.05, wheelRadius())) * dt;
    return;
  }
  const throttle = clamp(Number(throttleInput.value) || 0, 0, 1);
  const brake = clamp(Number(brakeInput.value) || 0, 0, 1);
  const idle = idleRpm();
  const redline = redlineRpm();
  const radius = wheelRadius();
  const mass = massKg();
  const drag = 0.392 * sim.speedMps * sim.speedMps;
  const rolling = 0.012 * mass * 9.81;
  const brakeForce = sim.speedMps > 0.05 ? brake * mass * 9.0 : 0;
  const neutral = sim.gearIndex < 0;

  if (neutral) {
    const target = idle + throttle * (redline - idle);
    sim.rpm += (target - sim.rpm) * Math.min(1, dt / 0.18);
    sim.rpm = clamp(sim.rpm, idle, redline);
    const acceleration = (-drag - rolling - brakeForce) / mass;
    sim.speedMps = Math.max(0, sim.speedMps + acceleration * dt);
    sim.acceleration = acceleration;
  } else {
    const ratio = selectedRatio();
    const fd = finalDrive();
    const rawRpm = (sim.speedMps / (2 * Math.PI * radius)) * ratio * fd * 60;
    sim.rpm = clamp(rawRpm, idle, redline);
    const pack = PACK_META[car.id];
    const peakTorque = pack?.peakTorque ?? (Number(engineMeta(car).peakTorque) || 300);
    const combustionTorque = peakTorque * torqueFactor(sim.rpm) * throttle;
    const engineBrakeTorque =
      peakTorque * 0.3 * (0.3 + 0.7 * (sim.rpm / redline)) * (1 - 0.85 * throttle);
    const wheelForce = ((combustionTorque - engineBrakeTorque) * ratio * fd * 0.9) / radius;
    const acceleration = (wheelForce - drag - rolling - brakeForce) / mass;
    sim.speedMps = Math.max(0, sim.speedMps + acceleration * dt);
    sim.acceleration = acceleration;
    if (autoGears && sim.shiftCooldown <= 0) {
      const ratios = gearRatios();
      if (sim.rpm >= redline * 0.93 && sim.gearIndex < ratios.length - 1) {
        sim.gearIndex += 1;
        gearSelect.value = String(sim.gearIndex);
        sim.shiftCooldown = 0.35;
      } else if (sim.rpm <= idle + 280 && sim.gearIndex > 0) {
        sim.gearIndex -= 1;
        gearSelect.value = String(sim.gearIndex);
        sim.shiftCooldown = 0.6;
      }
    }
  }

  sim.throttle = throttle;
  sim.brake = brake;
  sim.wheelAngle += (sim.speedMps / Math.max(0.05, radius)) * dt;
}

function garageWindow(): any | null {
  try {
    return garageFrame.contentWindow;
  } catch {
    return null;
  }
}

function setGarageSelection(): void {
  try {
    localStorage.setItem("lapster.car.v1", car.id);
    localStorage.setItem("lapster.category.v1", car.category);
  } catch {
    // Storage unavailable.
  }
}

function hideGarageHud(): void {
  try {
    const doc = garageFrame.contentDocument;
    if (!doc) return;
    const style = doc.createElement("style");
    style.textContent =
      "#hud,#viewer-controls,#car-nav,#menu-open,#app-menu,#paint-panel,#viewer-loading,#car-loading{display:none!important}";
    doc.head.append(style);
  } catch {
    // Cross-origin or not ready.
  }
}

function syncGarageWheels(): void {
  const viewer = garageWindow()?.__garageViewer;
  if (!viewer) return;
  try {
    viewer.set_wheel_angles(0, sim.wheelAngle);
  } catch {
    // Viewer not ready.
  }
}

function attachKeys(win: Window): void {
  win.addEventListener("keydown", onKeyDown);
  win.addEventListener("keyup", onKeyUp);
  win.addEventListener("blur", releaseKeys);
}

function onKeyDown(event: KeyboardEvent): void {
  if (manualRpmOn) return;
  const key = event.key.toLowerCase();
  if (key === "w" || key === "arrowup") {
    event.preventDefault();
    if (!keyThrottle) setThrottleKey(true);
  } else if (key === "s" || key === "arrowdown") {
    event.preventDefault();
    if (!keyBrake) setBrakeKey(true);
  }
}

function onKeyUp(event: KeyboardEvent): void {
  if (manualRpmOn) return;
  const key = event.key.toLowerCase();
  if (key === "w" || key === "arrowup") {
    event.preventDefault();
    if (keyThrottle) setThrottleKey(false);
  } else if (key === "s" || key === "arrowdown") {
    event.preventDefault();
    if (keyBrake) setBrakeKey(false);
  }
}

function releaseKeys(): void {
  if (keyThrottle) setThrottleKey(false);
  if (keyBrake) setBrakeKey(false);
}

function setThrottleKey(active: boolean): void {
  keyThrottle = active;
  throttleInput.value = active ? "1" : "0";
  updateControlLabels();
}

function setBrakeKey(active: boolean): void {
  keyBrake = active;
  brakeInput.value = active ? "1" : "0";
  updateControlLabels();
}

function selectGarageBay(): void {
  const viewer = garageWindow()?.__garageViewer;
  if (!viewer) return;
  setGarageSelection();
  const index = carsInCategory(car.category).findIndex((option) => option.id === car.id);
  if (index >= 0) {
    try {
      viewer.select_bay(index);
    } catch {
      // Ignore.
    }
  }
  syncGarageWheels();
}

function waitForGarage(): void {
  const started = Date.now();
  const tick = (): void => {
    const win = garageWindow();
    if (win?.__garageViewerReady || win?.__garageViewerError) {
      garageReady = win.__garageViewerReady === true;
      hideGarageHud();
      selectGarageBay();
      if (!garageKeysAttached && win) {
        attachKeys(win);
        garageKeysAttached = true;
      }
      setStatus(
        win.__garageViewerError
          ? `Garage failed: ${win.__garageViewerError}`
          : "Car ready. Use throttle/brake.",
      );
      return;
    }
    if (Date.now() - started > 60000) {
      setStatus("Garage view timed out.");
      return;
    }
    setTimeout(tick, 150);
  };
  tick();
}

function loadGarage(category: string): void {
  garageCategory = category;
  garageReady = false;
  garageKeysAttached = false;
  setGarageSelection();
  garageFrame.src = `../garage/?category=${encodeURIComponent(category)}&v=audio-tune-${Date.now()}`;
  waitForGarage();
}

function renderMeta(): void {
  const engine = engineMeta(car);
  const vehicle = vehicleMeta(car);
  const ratios = gearRatios();
  const rows: Array<[string, string]> = [
    ["Car", `${car.name} (${car.id})`],
    ["Engine", `${car.engine} · ${engine.label ?? "unknown"}`],
    ["Firing geometry", `${engine.cylinders ?? "?"} firing events`],
    ["Engine gears", `${engine.gears ?? "?"} forward`],
    ["Idle / redline", `${idleRpm()} / ${redlineRpm()} rpm`],
    ["Peak torque", `${PACK_META[car.id]?.peakTorque ?? engine.peakTorque ?? "?"} Nm @ ${PACK_META[car.id]?.peakTorqueRpm ?? engine.peakTorqueRpm ?? "?"} rpm`],
    ["Gearbox preset", `${vehicle.label ?? car.vehicle}`],
    ["Gearbox gears", `${ratios.length} forward`],
    ["Gear ratios", ratios.map((r: number) => r.toFixed(2)).join(", ")],
    ["Final drive", `${finalDrive().toFixed(2)}:1`],
    ["Gearbox source", PACK_META[car.id] ? "car pack" : "catalog preset"],
    ["Wheel radius", `${wheelRadius().toFixed(3)} m`],
    ["Mass", `${massKg()} kg`],
  ];
  metaBox.innerHTML = `<table>${rows
    .map(([k, v]) => `<tr><td>${k}</td><td><code>${v}</code></td></tr>`)
    .join("")}</table>`;
}

function rebuildGearSelect(): void {
  const ratios = gearRatios();
  const current = Number(gearSelect.value);
  gearSelect.innerHTML =
    `<option value="-1">N</option>` +
    ratios
      .map((r: number, i: number) => `<option value="${i}">${i + 1} — ${r.toFixed(2)}</option>`)
      .join("");
  gearSelect.value = current >= 0 && current < ratios.length ? String(current) : "0";
}

function updateGearNote(): void {
  const index = Number(gearSelect.value);
  if (index < 0) {
    gearNote.textContent = "Neutral — engine free revs; car coasts.";
    gearNote.classList.remove("warn");
    return;
  }
  const ratio = selectedRatio();
  const maxSpeed = rpmToSpeed(redlineRpm()) * 3.6;
  const over = sim.speedMps * 3.6 > maxSpeed + 1;
  gearNote.textContent = `Gear ${index + 1} · ${ratio.toFixed(2)}:1 · ${maxSpeed.toFixed(0)} km/h at redline${over ? " — over redline, shift up" : ""}`;
  gearNote.classList.toggle("warn", over);
}

function updateControlLabels(): void {
  $("throttle-value").textContent = `${(Number(throttleInput.value) * 100).toFixed(0)}%`;
  $("brake-value").textContent = `${(Number(brakeInput.value) * 100).toFixed(0)}%`;
}

function updateManualMode(): void {
  manualRpmOn = manualRpmToggle.checked;
  throttleInput.disabled = manualRpmOn;
  brakeInput.disabled = manualRpmOn;
  autoGearsToggle.disabled = manualRpmOn;
  manualRpm.disabled = !manualRpmOn;
  const idle = idleRpm();
  const redline = redlineRpm();
  manualRpm.min = String(idle);
  manualRpm.max = String(redline);
  manualRpm.value = String(clamp(Number(manualRpm.value) || 3000, idle, redline));
  manualRpmValue.textContent = `${Math.round(Number(manualRpm.value))} rpm`;
  updateGearNote();
}

function updateAudioButton(): void {
  const button = $("audio");
  button.textContent = audioOn ? "Audio: On" : "Audio: Off";
  button.classList.toggle("active", audioOn);
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
  set("load-gain", mapping.loadGain ?? 0.35, 2);
  const hasOption = [...sampleSelect.options].some(
    (option) => option.value === mapping.fileId,
  );
  sampleSelect.value = hasOption ? mapping.fileId : "";
  tuner.setMapping(mapping);
  updateMappingJson();
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
    loadGain: Number($<HTMLInputElement>("load-gain").value),
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
  updateMappingJson();
  try {
    await tuner.load(sampleUrl(fileId));
    setStatus(
      tuner.running
        ? `Sample loaded: ${SAMPLE_BY_ID.get(fileId)?.label ?? fileId}`
        : `Sample selected: ${SAMPLE_BY_ID.get(fileId)?.label ?? fileId}`,
    );
  } catch (error) {
    setStatus(`Sample failed: ${error}`);
  }
}

function resetSim(): void {
  sim.speedMps = 0;
  sim.rpm = idleRpm();
  sim.gearIndex = 0;
  sim.throttle = 0;
  sim.brake = 0;
  sim.wheelAngle = 0;
  sim.acceleration = 0;
  sim.shiftCooldown = 0.5;
  gearSelect.value = "0";
  setThrottleKey(false);
  setBrakeKey(false);
  updateControlLabels();
  syncGarageWheels();
}

function selectCar(target: CarOption): void {
  car = target;
  mapping = { ...mappingFor(car) };
  if (mapping.fileId.startsWith("custom:") && !customUrls.has(mapping.fileId)) {
    mapping.fileId = DEFAULT_SAMPLE_BY_CAR[car.id] ?? "v8-ears68";
  }
  rebuildGearSelect();
  applyMappingToUi();
  renderMeta();
  resetSim();
  updateGearNote();
  updateManualMode();
  void loadSample(mapping.fileId);
  loadGarage(car.category);
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
  const x = (value: number): number => pad + (clamp(value, 0, xMax) / xMax) * (width - pad * 2);
  const y = (value: number): number => height - pad - ((value - yMin) / Math.max(0.001, yMax - yMin)) * (height - pad * 2);
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
  context.fillStyle = "#ffd479";
  context.beginPath();
  context.arc(x(rpm), y(mapRate(mapping, rpm)), 4, 0, Math.PI * 2);
  context.fill();
  context.fillStyle = "#7d8996";
  context.font = "10px system-ui";
  context.fillText(`${Math.round(mapping.rpmLow)}`, pad, height - 1);
  context.fillText(`${Math.round(xMax)} rpm`, width - 62, height - 1);
  context.fillText(`${yMax.toFixed(2)}x`, 2, pad + 8);
}

function updateLive(): void {
  const speedKph = sim.speedMps * 3.6;
  const gear = sim.gearIndex < 0 ? "N" : String(sim.gearIndex + 1);
  const throttleText = manualRpmOn ? "manual" : `${(sim.throttle * 100).toFixed(0)}%`;
  $("rate-readout").textContent = `rpm ${Math.round(sim.rpm)} → ${lastRate.toFixed(3)}x`;
  liveBox.innerHTML = `
    <div class="k">Speed</div><div class="v">${speedKph.toFixed(0)} km/h</div>
    <div class="k">Engine rpm</div><div class="v">${Math.round(sim.rpm)}</div>
    <div class="k">Gear</div><div class="v">${gear}${autoGears ? " (auto)" : ""}</div>
    <div class="k">Throttle</div><div class="v">${throttleText}</div>
    <div class="k">Brake</div><div class="v">${(sim.brake * 100).toFixed(0)}%</div>
    <div class="k">Playback rate</div><div class="v">${lastRate.toFixed(3)}×</div>
    <div class="k">Sample</div><div class="v">${SAMPLE_BY_ID.get(mapping.fileId)?.label ?? mapping.fileId}</div>
  `;
  speedBig.innerHTML = `${Math.round(speedKph)}<span>km/h</span>`;
  rpmBig.innerHTML = `${Math.round(sim.rpm)}<span>rpm</span>`;
  stageSub.textContent = manualRpmOn
    ? `gear ${gear} · manual rpm`
    : `gear ${gear} · ${Math.round(sim.rpm)} rpm · throttle ${(sim.throttle * 100).toFixed(0)}%`;
}

let lastFrame = performance.now();
let simAccumulator = 0;
function frame(now: number): void {
  const delta = Math.min(0.25, Math.max(0, (now - lastFrame) / 1000));
  lastFrame = now;
  simAccumulator += delta;
  const step = 1 / 60;
  let steps = 0;
  while (simAccumulator >= step && steps < 30) {
    simulate(step);
    simAccumulator -= step;
    steps += 1;
  }
  syncGarageWheels();
  lastRate = mapRate(mapping, sim.rpm);
  tuner.update(sim.rpm, sim.throttle);
  drawGraph(sim.rpm);
  updateLive();
  updateGearNote();
  requestAnimationFrame(frame);
}

function initialize(): void {
  carSelect.innerHTML = CAR_OPTIONS.map(
    (option) => `<option value="${option.id}">${option.name}</option>`,
  ).join("");
  sampleSelect.innerHTML = SAMPLES.map(
    (sample) => `<option value="${sample.id}">${sample.label}</option>`,
  ).join("");
  throttleInput.value = "0";
  brakeInput.value = "0";
  autoGearsToggle.checked = true;
  autoGears = true;
  updateControlLabels();
  updateAudioButton();
  attachKeys(window);
  updateManualMode();

  carSelect.addEventListener("change", () => {
    const next = CAR_OPTIONS.find((option) => option.id === carSelect.value);
    if (next) selectCar(next);
  });
  sampleSelect.addEventListener("change", () => void loadSample(sampleSelect.value));
  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    if (!file) return;
    const id = `custom:${file.name}`;
    customUrls.set(id, URL.createObjectURL(file));
    sampleSelect.insertAdjacentHTML("beforeend", `<option value="${id}">${file.name}</option>`);
    sampleSelect.value = id;
    void loadSample(id);
  });
  $("car-sample").addEventListener("click", () => {
    const id = DEFAULT_SAMPLE_BY_CAR[car.id] ?? "v8-ears68";
    sampleSelect.value = id;
    void loadSample(id);
  });
  throttleInput.addEventListener("input", updateControlLabels);
  brakeInput.addEventListener("input", updateControlLabels);
  manualRpmToggle.addEventListener("change", () => {
    updateManualMode();
    manualRpmValue.textContent = `${Math.round(Number(manualRpm.value))} rpm`;
  });
  manualRpm.addEventListener("input", () => {
    manualRpmValue.textContent = `${Math.round(Number(manualRpm.value))} rpm`;
  });
  autoGearsToggle.addEventListener("change", () => {
    autoGears = autoGearsToggle.checked;
    if (autoGears && sim.gearIndex < 0) {
      sim.gearIndex = 0;
      gearSelect.value = "0";
    }
    sim.shiftCooldown = 0.5;
    updateGearNote();
  });
  gearSelect.addEventListener("change", () => {
    const value = Number(gearSelect.value);
    sim.gearIndex = value;
    if (value < 0 || autoGears) {
      autoGears = false;
      autoGearsToggle.checked = false;
    }
    sim.shiftCooldown = 0.4;
    updateGearNote();
  });
  $("reset-sim").addEventListener("click", () => {
    resetSim();
    updateGearNote();
  });
  for (const id of ["rpm-low", "rpm-high", "rate-low", "rate-high", "curve", "gain", "load-gain"]) {
    $(id).addEventListener("input", readMappingFromUi);
  }
  $("audio").addEventListener("click", async () => {
    try {
      if (!tuner.running) {
        await tuner.start();
        await loadSample(mapping.fileId);
        audioOn = true;
        tuner.setMuted(false);
      } else {
        audioOn = !audioOn;
        tuner.setMuted(!audioOn);
      }
      updateAudioButton();
      setStatus(audioOn ? "Audio on." : "Audio off.");
    } catch (error) {
      setStatus(`Audio failed: ${error}`);
    }
  });
  $("copy").addEventListener("click", async () => {
    updateMappingJson();
    const text = mappingJson.value;
    try {
      await navigator.clipboard.writeText(text);
      setStatus(`Copied ${car.name} mapping.`);
    } catch {
      mappingJson.focus();
      mappingJson.select();
      setStatus("Clipboard unavailable; selected the JSON for manual copy.");
    }
  });
  $("load-json").addEventListener("click", () => {
    try {
      const parsed = JSON.parse(mappingJson.value);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        throw new Error("expected a mapping object or a mapping set");
      }
      if ("fileId" in parsed || "rpmLow" in parsed) {
        store[car.id] = normalizeMapping(car, parsed);
        mapping = { ...store[car.id] };
        saveStore();
        applyMappingToUi();
        void loadSample(mapping.fileId);
      } else {
        let applied = false;
        for (const [id, value] of Object.entries(parsed)) {
          const option = findCar(id);
          if (!option || !value || typeof value !== "object") continue;
          store[option.id] = normalizeMapping(option, value);
          if (option.id === car.id) applied = true;
        }
        saveStore();
        if (applied) {
          mapping = { ...mappingFor(car) };
          applyMappingToUi();
          void loadSample(mapping.fileId);
        }
      }
      updateMappingJson();
      setStatus("JSON loaded.");
    } catch (error) {
      setStatus(`JSON error: ${error}`);
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

  selectCar(car);
  requestAnimationFrame(frame);
}

initialize();
