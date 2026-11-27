// Engine audio tuning tool.
//
// Standalone dev page. A local drivetrain + road-load simulation drives the
// sample audio and a side-view car whose wheels turn. No track, no dyno, and
// no driving needed: target-speed and target-RPM modes auto-drive the model.

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

interface SimState {
  speedMps: number;
  rpm: number;
  gearIndex: number;
  throttle: number;
  brake: number;
  wheelAngle: number;
  distance: number;
  acceleration: number;
  integral: number;
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

const CAR_COLORS: Record<string, string> = {
  "mazda-rx7": "#d94a3d",
  "sc-deckard-conejo": "#b23a2e",
  "67gp-cavallo-123": "#c9a227",
  "67gp-guthrie-eagle": "#2f6b4f",
  "67gp-kuiper-t81": "#2f5f8f",
  "67gp-murasama-37a": "#d9d9d9",
  "67gp-mango-ms7": "#e07a2f",
  "67gp-upping-ut19": "#3f7f3f",
};

const STORE_KEY = "lapster.audioTune.v1";

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

const carSelect = $<HTMLSelectElement>("car");
const sampleSelect = $<HTMLSelectElement>("sample");
const fileInput = $<HTMLInputElement>("file");
const modeSelect = $<HTMLSelectElement>("mode");
const targetSpeedRow = $<HTMLDivElement>("target-speed-row");
const targetRpmRow = $<HTMLDivElement>("target-rpm-row");
const targetSpeed = $<HTMLInputElement>("target-speed");
const targetRpm = $<HTMLInputElement>("target-rpm");
const gearSelect = $<HTMLSelectElement>("gear");
const manualControls = $<HTMLDivElement>("manual-controls");
const throttleInput = $<HTMLInputElement>("throttle");
const brakeInput = $<HTMLInputElement>("brake");
const graph = $<HTMLCanvasElement>("graph");
const status = $<HTMLDivElement>("status");
const liveBox = $<HTMLDivElement>("live");
const metaBox = $<HTMLDivElement>("meta");
const carView = $<HTMLCanvasElement>("car-view");
const speedBig = $<HTMLDivElement>("speed-big");
const stageSub = $<HTMLDivElement>("stage-sub");
const gearNote = $<HTMLDivElement>("gear-note");

let store: Record<string, Mapping> = {};
try {
  store = JSON.parse(localStorage.getItem(STORE_KEY) ?? "{}") as Record<string, Mapping>;
} catch {
  store = {};
}

let car: CarOption = CAR_OPTIONS[0];
let mapping: Mapping = defaultMapping(car);
let customUrls = new Map<string, string>();
let tunerMuted = false;
let lastRate = 0;
const sim: SimState = {
  speedMps: 0,
  rpm: 900,
  gearIndex: 0,
  throttle: 0,
  brake: 0,
  wheelAngle: 0,
  distance: 0,
  acceleration: 0,
  integral: 0,
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function saveStore(): void {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(store));
  } catch {
    // Storage unavailable.
  }
}

function setStatus(text: string): void {
  status.textContent = text;
}

function engineMeta(target: CarOption): any {
  return (ENGINE_PROFILES as Record<string, any>)[target.engine] ?? {};
}

function vehicleMeta(target: CarOption): any {
  return (VEHICLE_PRESETS as Record<string, any>)[target.vehicle] ?? {};
}

function gearRatios(): number[] {
  const preset = vehicleMeta(car).gearRatios;
  return Array.isArray(preset) && preset.length > 0 ? preset : [1];
}

function finalDrive(): number {
  return Number(vehicleMeta(car).finalDrive) || 3.7;
}

function wheelRadius(): number {
  return Number(vehicleMeta(car).wheelRadius) || 0.33;
}

function massKg(): number {
  return Number(vehicleMeta(car).mass) || 1400;
}

function idleRpm(): number {
  return Number(engineMeta(car).idleRpm) || 900;
}

function redlineRpm(): number {
  return Number(engineMeta(car).redlineRpm) || 8000;
}

function selectedRatio(): number {
  const ratios = gearRatios();
  const index = clamp(Number(gearSelect.value) || 0, 0, ratios.length - 1);
  return ratios[index] || 1;
}

function rpmToSpeed(rpm: number): number {
  const wheelRpm = rpm / Math.max(0.1, selectedRatio() * finalDrive());
  return (wheelRpm / 60) * 2 * Math.PI * wheelRadius();
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
  const t = clamp((rpm - m.rpmLow) / span, 0, 1);
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
    const level = m.gain * (0.35 + 0.65 * clamp(load, 0, 1));
    this.gain.gain.setTargetAtTime(level, now, 0.05);
  }
}

const tuner = new TunerAudio();

function torqueFactor(rpm: number): number {
  const idle = idleRpm();
  const redline = redlineRpm();
  const peak = clamp((Number(engineMeta(car).peakTorqueRpm) || (idle + redline) / 2) - idle, 1, redline - idle) / Math.max(1, redline - idle);
  const x = clamp((rpm - idle) / Math.max(1, redline - idle), 0, 1.1);
  return 0.55 + 0.45 * Math.exp(-Math.pow((x - peak) / 0.4, 2));
}

function autoControls(targetSpeedMps: number, dt: number): { throttle: number; brake: number } {
  if (targetSpeedMps < 0.2 && sim.speedMps < 0.2) {
    sim.integral = 0;
    return { throttle: 0, brake: 0 };
  }
  const error = targetSpeedMps - sim.speedMps;
  sim.integral = clamp(sim.integral + error * dt, -3, 3);
  if (error > 0.2) {
    return { throttle: clamp(0.08 + error * 0.5 + sim.integral * 0.25, 0, 1), brake: 0 };
  }
  if (error < -0.2) {
    return { throttle: 0, brake: clamp(-error * 0.5 - sim.integral * 0.25, 0, 1) };
  }
  return { throttle: clamp(0.08 + sim.integral * 0.25, 0, 1), brake: 0 };
}

function simulate(dt: number): void {
  sim.gearIndex = clamp(Number(gearSelect.value) || 0, 0, gearRatios().length - 1);
  const mode = modeSelect.value;
  let throttle = 0;
  let brake = 0;
  if (mode === "manual") {
    throttle = Number(throttleInput.value) || 0;
    brake = Number(brakeInput.value) || 0;
  } else {
    const target = mode === "rpm"
      ? rpmToSpeed(Number(targetRpm.value) || idleRpm())
      : (Number(targetSpeed.value) || 0) / 3.6;
    const controls = autoControls(target, dt);
    throttle = controls.throttle;
    brake = controls.brake;
  }
  const ratio = selectedRatio();
  const fd = finalDrive();
  const radius = wheelRadius();
  const idle = idleRpm();
  const redline = redlineRpm();
  const rawRpm = (sim.speedMps / (2 * Math.PI * radius)) * ratio * fd * 60;
  sim.rpm = clamp(rawRpm, idle, redline);
  const engineTorque = (Number(engineMeta(car).peakTorque) || 300) * torqueFactor(sim.rpm) * throttle;
  const wheelForce = (engineTorque * ratio * fd * 0.9) / radius;
  const drag = 0.392 * sim.speedMps * sim.speedMps;
  const rolling = 0.012 * massKg() * 9.81;
  const brakeForce = sim.speedMps > 0.05 ? brake * massKg() * 9.0 : 0;
  const acceleration = (wheelForce - drag - rolling - brakeForce) / massKg();
  sim.speedMps = Math.max(0, sim.speedMps + acceleration * dt);
  sim.acceleration = acceleration;
  sim.throttle = throttle;
  sim.brake = brake;
  sim.wheelAngle += (sim.speedMps / Math.max(0.05, radius)) * dt;
  sim.distance += sim.speedMps * dt;
}

function carColor(): string {
  return CAR_COLORS[car.id] ?? "#d94a3d";
}

function shade(hex: string, amount: number): string {
  const value = hex.replace("#", "");
  const r = parseInt(value.slice(0, 2), 16);
  const g = parseInt(value.slice(2, 4), 16);
  const b = parseInt(value.slice(4, 6), 16);
  const mix = (channel: number): number =>
    amount >= 0 ? channel + (255 - channel) * amount : channel * (1 + amount);
  return `rgb(${Math.round(mix(r))}, ${Math.round(mix(g))}, ${Math.round(mix(b))})`;
}

function drawWheel(ctx: CanvasRenderingContext2D, x: number, y: number, radius: number, angle: number): void {
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(angle);
  ctx.fillStyle = "#0a0c0f";
  ctx.beginPath();
  ctx.arc(0, 0, radius, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = "#222a33";
  ctx.lineWidth = radius * 0.12;
  ctx.stroke();
  ctx.fillStyle = "#9aa6b2";
  ctx.beginPath();
  ctx.arc(0, 0, radius * 0.58, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = "#d5dde6";
  ctx.lineWidth = Math.max(1, radius * 0.06);
  for (let i = 0; i < 5; i += 1) {
    const a = (i * Math.PI * 2) / 5;
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(Math.cos(a) * radius * 0.52, Math.sin(a) * radius * 0.52);
    ctx.stroke();
  }
  ctx.fillStyle = "#2a333d";
  ctx.beginPath();
  ctx.arc(0, 0, radius * 0.15, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

function drawCar(): void {
  const dpr = window.devicePixelRatio || 1;
  const width = carView.clientWidth;
  const height = carView.clientHeight;
  if (carView.width !== Math.round(width * dpr) || carView.height !== Math.round(height * dpr)) {
    carView.width = Math.round(width * dpr);
    carView.height = Math.round(height * dpr);
  }
  const ctx = carView.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const sky = ctx.createLinearGradient(0, 0, 0, height);
  sky.addColorStop(0, "#0b1118");
  sky.addColorStop(0.62, "#121b24");
  sky.addColorStop(1, "#080b0f");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, width, height);

  const groundY = height * 0.73;
  ctx.fillStyle = "#090d12";
  ctx.fillRect(0, groundY, width, height - groundY);
  ctx.strokeStyle = "#1d2730";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, groundY);
  ctx.lineTo(width, groundY);
  ctx.stroke();

  const dashSpacing = Math.max(70, 110 * Math.min(1, width / 900));
  const offset = (sim.distance * 26) % dashSpacing;
  ctx.strokeStyle = "#2a3540";
  ctx.lineWidth = 4;
  ctx.setLineDash([dashSpacing * 0.42, dashSpacing * 0.58]);
  ctx.lineDashOffset = -offset;
  ctx.beginPath();
  ctx.moveTo(0, groundY + (height - groundY) * 0.55);
  ctx.lineTo(width, groundY + (height - groundY) * 0.55);
  ctx.stroke();
  ctx.setLineDash([]);

  const scale = Math.min(width / 1000, height / 620) * 1.15;
  const carWidth = 520 * scale;
  const carHeight = 150 * scale;
  const wheelRadiusPx = 44 * scale;
  const cx = width * 0.5;
  const baseY = groundY + 8;
  const bob = clamp(sim.acceleration * 1.5, -4, 4) * scale;
  const bodyY = baseY - wheelRadiusPx * 1.15 - carHeight * 0.62 + bob;
  const wheelY = baseY - wheelRadiusPx;
  const color = carColor();

  ctx.fillStyle = "rgba(0, 0, 0, 0.5)";
  ctx.beginPath();
  ctx.ellipse(cx, baseY + 8, carWidth * 0.56, 13 * scale, 0, 0, Math.PI * 2);
  ctx.fill();

  drawWheel(ctx, cx - carWidth * 0.31, wheelY, wheelRadiusPx, sim.wheelAngle);
  drawWheel(ctx, cx + carWidth * 0.31, wheelY, wheelRadiusPx, sim.wheelAngle);

  ctx.save();
  ctx.translate(cx, bodyY);
  ctx.rotate(clamp(-sim.acceleration * 0.006, -0.035, 0.035));
  const x0 = -carWidth / 2;
  const y0 = -carHeight / 2;
  const body = ctx.createLinearGradient(0, y0, 0, y0 + carHeight);
  body.addColorStop(0, shade(color, 0.25));
  body.addColorStop(0.5, color);
  body.addColorStop(1, shade(color, -0.35));
  ctx.fillStyle = body;
  ctx.beginPath();
  ctx.moveTo(x0 + carWidth * 0.02, y0 + carHeight * 0.8);
  ctx.lineTo(x0 + carWidth * 0.06, y0 + carHeight * 0.56);
  ctx.lineTo(x0 + carWidth * 0.2, y0 + carHeight * 0.4);
  ctx.lineTo(x0 + carWidth * 0.38, y0 + carHeight * 0.3);
  ctx.lineTo(x0 + carWidth * 0.62, y0 + carHeight * 0.28);
  ctx.lineTo(x0 + carWidth * 0.8, y0 + carHeight * 0.38);
  ctx.lineTo(x0 + carWidth * 0.95, y0 + carHeight * 0.54);
  ctx.lineTo(x0 + carWidth * 0.99, y0 + carHeight * 0.8);
  ctx.closePath();
  ctx.fill();

  ctx.fillStyle = "rgba(10, 16, 22, 0.86)";
  ctx.beginPath();
  ctx.moveTo(x0 + carWidth * 0.25, y0 + carHeight * 0.42);
  ctx.lineTo(x0 + carWidth * 0.38, y0 + carHeight * 0.32);
  ctx.lineTo(x0 + carWidth * 0.6, y0 + carHeight * 0.31);
  ctx.lineTo(x0 + carWidth * 0.72, y0 + carHeight * 0.44);
  ctx.lineTo(x0 + carWidth * 0.6, y0 + carHeight * 0.45);
  ctx.lineTo(x0 + carWidth * 0.4, y0 + carHeight * 0.46);
  ctx.closePath();
  ctx.fill();

  ctx.fillStyle = "#f5e6a8";
  ctx.beginPath();
  ctx.ellipse(x0 + carWidth * 0.93, y0 + carHeight * 0.58, carWidth * 0.025, carHeight * 0.07, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#7f1d1d";
  ctx.beginPath();
  ctx.ellipse(x0 + carWidth * 0.04, y0 + carHeight * 0.6, carWidth * 0.018, carHeight * 0.06, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = shade(color, -0.5);
  ctx.lineWidth = Math.max(1, 2 * scale);
  ctx.beginPath();
  ctx.moveTo(x0 + carWidth * 0.1, y0 + carHeight * 0.78);
  ctx.lineTo(x0 + carWidth * 0.9, y0 + carHeight * 0.78);
  ctx.stroke();
  ctx.restore();

  if (sim.speedMps > 4) {
    ctx.strokeStyle = "rgba(143, 211, 255, 0.16)";
    ctx.lineWidth = 2;
    for (let i = 0; i < 6; i += 1) {
      const lineY = groundY - 20 - i * 18 * scale;
      const drift = (sim.distance * 40 + i * 37) % (carWidth * 1.4);
      ctx.beginPath();
      ctx.moveTo(cx - carWidth * 0.75 + drift, lineY);
      ctx.lineTo(cx - carWidth * 0.75 + drift + 70 * scale, lineY);
      ctx.stroke();
    }
  }
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
    ["Idle / redline", `${engine.idleRpm ?? "?"} / ${engine.redlineRpm ?? "?"} rpm`],
    ["Peak torque", `${engine.peakTorque ?? "?"} Nm @ ${engine.peakTorqueRpm ?? "?"} rpm`],
    ["Gearbox preset", `${vehicle.label ?? car.vehicle}`],
    ["Gear ratios", ratios.map((r: number) => r.toFixed(2)).join(", ")],
    ["Final drive", `${finalDrive().toFixed(2)}:1`],
    ["Wheel radius", `${wheelRadius().toFixed(3)} m`],
    ["Mass", `${massKg()} kg`],
    ["Gearbox source", "catalog preset"],
  ];
  metaBox.innerHTML = `<table>${rows
    .map(([k, v]) => `<tr><td>${k}</td><td><code>${v}</code></td></tr>`)
    .join("")}</table>`;
}

function rebuildGearSelect(): void {
  const ratios = gearRatios();
  const current = Number(gearSelect.value) || 0;
  gearSelect.innerHTML = ratios
    .map((r: number, i: number) => `<option value="${i}">${i + 1} — ${r.toFixed(2)}</option>`)
    .join("");
  gearSelect.value = String(clamp(current, 0, ratios.length - 1));
}

function updateGearNote(): void {
  const ratio = selectedRatio();
  const maxSpeed = rpmToSpeed(redlineRpm()) * 3.6;
  const mode = modeSelect.value;
  const target = mode === "speed"
    ? Number(targetSpeed.value) || 0
    : mode === "rpm"
      ? rpmToSpeed(Number(targetRpm.value) || idleRpm()) * 3.6
      : null;
  const over = target !== null && target > maxSpeed + 1;
  gearNote.textContent = `Gear ${Number(gearSelect.value) + 1} · ${ratio.toFixed(2)}:1 · ${maxSpeed.toFixed(0)} km/h at redline${over ? " — target not reachable in this gear" : ""}`;
  gearNote.classList.toggle("warn", over);
}

function updateModeRows(): void {
  const mode = modeSelect.value;
  targetSpeedRow.classList.toggle("hidden", mode !== "speed");
  targetRpmRow.classList.toggle("hidden", mode !== "rpm");
  manualControls.classList.toggle("hidden", mode !== "manual");
  updateGearNote();
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
  targetRpm.min = String(idleRpm());
  targetRpm.max = String(redlineRpm());
  targetRpm.value = String(clamp(Number(targetRpm.value) || 3000, idleRpm(), redlineRpm()));
  $("target-rpm-value").textContent = `${Math.round(Number(targetRpm.value))} rpm`;
  $("target-speed-value").textContent = `${Math.round(Number(targetSpeed.value))} km/h`;
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
  sim.gearIndex = clamp(Number(gearSelect.value) || 0, 0, gearRatios().length - 1);
  sim.throttle = 0;
  sim.brake = 0;
  sim.wheelAngle = 0;
  sim.distance = 0;
  sim.acceleration = 0;
  sim.integral = 0;
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
  void loadSample(mapping.fileId);
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

function updateLive(): void {
  const speedKph = sim.speedMps * 3.6;
  const target = modeSelect.value === "speed"
    ? `${Math.round(Number(targetSpeed.value))} km/h`
    : modeSelect.value === "rpm"
      ? `${Math.round(Number(targetRpm.value))} rpm`
      : "manual";
  $("rate-readout").textContent = `rpm ${Math.round(sim.rpm)} → ${lastRate.toFixed(3)}x`;
  liveBox.innerHTML = `
    <div class="k">Speed</div><div class="v">${speedKph.toFixed(0)} km/h</div>
    <div class="k">Engine rpm</div><div class="v">${Math.round(sim.rpm)}</div>
    <div class="k">Gear</div><div class="v">${sim.gearIndex + 1}</div>
    <div class="k">Throttle</div><div class="v">${(sim.throttle * 100).toFixed(0)}%</div>
    <div class="k">Brake</div><div class="v">${(sim.brake * 100).toFixed(0)}%</div>
    <div class="k">Target</div><div class="v">${target}</div>
    <div class="k">Playback rate</div><div class="v">${lastRate.toFixed(3)}×</div>
    <div class="k">Sample</div><div class="v">${SAMPLE_BY_ID.get(mapping.fileId)?.label ?? mapping.fileId}</div>
  `;
  speedBig.innerHTML = `${Math.round(speedKph)}<span>km/h</span>`;
  stageSub.textContent = `gear ${sim.gearIndex + 1} · ${Math.round(sim.rpm)} rpm · throttle ${(sim.throttle * 100).toFixed(0)}%`;
}

let lastFrame = performance.now();
function frame(now: number): void {
  const dt = Math.min(0.05, Math.max(0, (now - lastFrame) / 1000));
  lastFrame = now;
  simulate(dt);
  lastRate = mapRate(mapping, sim.rpm);
  tuner.update(sim.rpm, sim.throttle);
  drawCar();
  drawGraph(sim.rpm);
  updateLive();
  requestAnimationFrame(frame);
}

function initialize(): void {
  carSelect.innerHTML = CAR_OPTIONS.map(
    (option) => `<option value="${option.id}">${option.name}</option>`,
  ).join("");
  sampleSelect.innerHTML = SAMPLES.map(
    (sample) => `<option value="${sample.id}">${sample.label}</option>`,
  ).join("");
  modeSelect.value = "speed";
  targetSpeed.value = "60";
  targetRpm.value = "3000";
  throttleInput.value = "0";
  brakeInput.value = "0";
  updateModeRows();

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
  modeSelect.addEventListener("change", () => {
    sim.integral = 0;
    updateModeRows();
  });
  targetSpeed.addEventListener("input", () => {
    sim.integral = 0;
    $("target-speed-value").textContent = `${Math.round(Number(targetSpeed.value))} km/h`;
    updateGearNote();
  });
  targetRpm.addEventListener("input", () => {
    sim.integral = 0;
    $("target-rpm-value").textContent = `${Math.round(Number(targetRpm.value))} rpm`;
    updateGearNote();
  });
  gearSelect.addEventListener("change", () => {
    sim.integral = 0;
    sim.gearIndex = clamp(Number(gearSelect.value) || 0, 0, gearRatios().length - 1);
    updateGearNote();
  });
  throttleInput.addEventListener("input", () => {
    $("throttle-value").textContent = `${(Number(throttleInput.value) * 100).toFixed(0)}%`;
  });
  brakeInput.addEventListener("input", () => {
    $("brake-value").textContent = `${(Number(brakeInput.value) * 100).toFixed(0)}%`;
  });
  for (const id of ["rpm-low", "rpm-high", "rate-low", "rate-high", "curve", "gain"]) {
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

  $("target-speed-value").textContent = `${Math.round(Number(targetSpeed.value))} km/h`;
  $("target-rpm-value").textContent = `${Math.round(Number(targetRpm.value))} rpm`;
  $("throttle-value").textContent = "0%";
  $("brake-value").textContent = "0%";
  selectCar(car);
  requestAnimationFrame(frame);
}

initialize();
