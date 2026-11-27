// Lightweight recorded-sample engine audio for the sample-engine test branch.
//
// Each car now prefers its own Speed Dreams source sample where one exists,
// falling back to the engine-profile sample, then to a loop baked offline from
// Engine_Sim. The synth is never instantiated at runtime on this path.
//
// Test knobs (URL query):
//   ?sampleSource=fallback     prefer the baked fallback
//   ?sampleEngine=v8cross      force one sample for every voice
//   ?sampleRefRpm=3500         override the sample reference rpm
//   ?sampleRateMin=0.7         override the lowest playback rate
//   ?sampleRateMax=1.5         override the highest playback rate
//   ?sampleGain=0.3            override the master gain
//   ?sampleEngineGain=0.55     engine trim against the noise bus

import { EngineSimAudio, type TireAudioWheel } from "./engine-sim-audio.ts";
import { distanceGain } from "./remote-engine-mix.ts";

export const NEAREST_SLOT = 0;
export type RemoteAudioMode = "lite" | "full";

export interface RemoteAudioOptions {
  engine?: string;
  vehicle?: string;
  car?: string;
  mode?: RemoteAudioMode;
  contextProvider?: () => AudioContext | null;
}

export interface RemoteWheelState {
  slipRatio: number;
  slipAngle: number;
  load: number;
  contact: boolean;
}

export interface RemoteEngineState {
  rpm: number;
  speedMps: number;
  gear: number;
  throttle: number;
  brake: number;
  wheels: RemoteWheelState[];
}

interface SampleDef {
  recorded: string;
  fallback: string;
  refRpm: number;
  rateMin: number;
  rateMax: number;
}

interface CarDef {
  recorded: string;
  refRpm?: number;
  rateMin?: number;
  rateMax?: number;
}

interface SampleMapping {
  rpmLow: number;
  rpmHigh: number;
  rateLow: number;
  rateHigh: number;
  curve: number;
  gain: number;
  loadGain: number;
}

const CAR_MAPPINGS: Record<string, SampleMapping> = {
  "mazda-rx7": {
    rpmLow: 900, rpmHigh: 8000,
    rateLow: 0.3, rateHigh: 0.8, curve: 2.4,
    gain: 0.28, loadGain: 0.35,
  },
  "sc-deckard-conejo": {
    rpmLow: 1000, rpmHigh: 6930,
    rateLow: 0.6, rateHigh: 2, curve: 1.75,
    gain: 0.28, loadGain: 0.35,
  },
  "67gp-cavallo-123": {
    rpmLow: 1000, rpmHigh: 10000,
    rateLow: 0.5, rateHigh: 2, curve: 2.05,
    gain: 0.28, loadGain: 0.35,
  },
  "67gp-guthrie-eagle": {
    rpmLow: 1000, rpmHigh: 9500,
    rateLow: 0.5, rateHigh: 1.4, curve: 2.45,
    gain: 0.28, loadGain: 0.35,
  },
  "67gp-kuiper-t81": {
    rpmLow: 1000, rpmHigh: 10000,
    rateLow: 0.5, rateHigh: 1.4, curve: 2.45,
    gain: 0.28, loadGain: 0.35,
  },
  "67gp-murasama-37a": {
    rpmLow: 1000, rpmHigh: 10000,
    rateLow: 0.5, rateHigh: 2, curve: 2.05,
    gain: 0.28, loadGain: 0.35,
  },
  "67gp-mango-ms7": {
    rpmLow: 1000, rpmHigh: 10000,
    rateLow: 0.6, rateHigh: 2, curve: 1.75,
    gain: 0.28, loadGain: 0.35,
  },
  "67gp-upping-ut19": {
    rpmLow: 1000, rpmHigh: 9000,
    rateLow: 0.5, rateHigh: 1.4, curve: 2.45,
    gain: 0.28, loadGain: 0.35,
  },
};

function mappingFor(car: string | undefined): SampleMapping | undefined {
  return car ? CAR_MAPPINGS[car] : undefined;
}

function mappedRate(mapping: SampleMapping, rpm: number): number {
  const span = Math.max(1, mapping.rpmHigh - mapping.rpmLow);
  const t = clamp((rpm - mapping.rpmLow) / span, 0, 1);
  return mapping.rateLow + (mapping.rateHigh - mapping.rateLow) * Math.pow(t, mapping.curve);
}

// Narrow rate ranges: the first pass let rpm sweep 0.25x-3x, which made every
// car dive too low at idle and scream too high at redline.
const SAMPLE_DEFS: Record<string, SampleDef> = {
  rotary2: {
    recorded: "speed-dreams/rotary2.wav",
    fallback: "fallback/rotary2.wav",
    refRpm: 3500,
    rateMin: 0.7,
    rateMax: 1.4,
  },
  v8cross: {
    recorded: "speed-dreams/v8-ears68.wav",
    fallback: "fallback/v8cross.wav",
    refRpm: 3000,
    rateMin: 0.75,
    rateMax: 1.45,
  },
  v12: {
    recorded: "speed-dreams/v12_smooth.wav",
    fallback: "fallback/v12.wav",
    refRpm: 3000,
    rateMin: 0.7,
    rateMax: 1.4,
  },
};

// Per-car source loops. The 67GP cars have their own Speed Dreams recordings;
// the engine-profile loop is only a fallback for unknown packs.
const CAR_DEFS: Record<string, CarDef> = {
  "sc-deckard-conejo": { recorded: "speed-dreams/v8-ears68.wav" },
  "mazda-rx7": { recorded: "speed-dreams/rotary2.wav" },
  "67gp-cavallo-123": { recorded: "speed-dreams/cars/f123bis.wav" },
  "67gp-guthrie-eagle": { recorded: "speed-dreams/cars/GT3O.wav" },
  "67gp-kuiper-t81": { recorded: "speed-dreams/cars/GT3O.wav" },
  "67gp-murasama-37a": { recorded: "speed-dreams/cars/037a.wav" },
  "67gp-mango-ms7": { recorded: "speed-dreams/cars/formulaford.wav" },
  "67gp-upping-ut19": { recorded: "speed-dreams/cars/GT3O.wav" },
};

const DEFAULT_SAMPLE = SAMPLE_DEFS.v8cross;
const ASSET_BASE = new URL("../assets/audio/", import.meta.url);

const query = new URLSearchParams(globalThis.location?.search ?? "");
const SOURCE = query.get("sampleSource") === "fallback" ? "fallback" : "recorded";
const forcedEngine = query.get("sampleEngine");
const refOverride = Number(query.get("sampleRefRpm"));
const rateMinOverride = Number(query.get("sampleRateMin"));
const rateMaxOverride = Number(query.get("sampleRateMax"));
const gainOverride = Number(query.get("sampleGain"));
const engineGainOverride = Number(query.get("sampleEngineGain"));
const MASTER_GAIN =
  Number.isFinite(gainOverride) && gainOverride > 0 ? gainOverride : 0.22;
const ENGINE_TRIM =
  Number.isFinite(engineGainOverride) && engineGainOverride > 0
    ? engineGainOverride
    : 0.55;
const REMOTE_MASTER = 1.0;

const sampleCache = new WeakMap<
  AudioContext,
  Map<string, Promise<AudioBuffer | null>>
>();

function sampleDef(car: string | undefined, engine: string | undefined): SampleDef {
  if (forcedEngine && SAMPLE_DEFS[forcedEngine]) return SAMPLE_DEFS[forcedEngine];
  const base = (engine && SAMPLE_DEFS[engine]) || DEFAULT_SAMPLE;
  const carDef = car ? CAR_DEFS[car] : undefined;
  if (!carDef) return base;
  return {
    ...base,
    recorded: carDef.recorded,
    refRpm: carDef.refRpm ?? base.refRpm,
    rateMin: carDef.rateMin ?? base.rateMin,
    rateMax: carDef.rateMax ?? base.rateMax,
  };
}

function refRpmFor(car: string | undefined, engine: string | undefined): number {
  if (Number.isFinite(refOverride) && refOverride > 0) return refOverride;
  return sampleDef(car, engine).refRpm;
}

function rateRangeFor(
  car: string | undefined,
  engine: string | undefined,
): [number, number] {
  const def = sampleDef(car, engine);
  const min =
    Number.isFinite(rateMinOverride) && rateMinOverride > 0
      ? rateMinOverride
      : def.rateMin;
  const max =
    Number.isFinite(rateMaxOverride) && rateMaxOverride > 0
      ? rateMaxOverride
      : def.rateMax;
  return [Math.min(min, max), Math.max(min, max)];
}

function playbackRate(
  car: string | undefined,
  engine: string | undefined,
  rpm: number,
): number {
  const mapping = mappingFor(car);
  if (mapping) return mappedRate(mapping, rpm);
  const [min, max] = rateRangeFor(car, engine);
  return clamp(rpm / refRpmFor(car, engine), min, max);
}

async function decode(
  context: AudioContext,
  relative: string,
): Promise<AudioBuffer> {
  const url = new URL(relative, ASSET_BASE);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${response.status} ${url}`);
  return normalizeBuffer(context, await context.decodeAudioData(await response.arrayBuffer()));
}

// Recorded loops have very different levels. Normalise each decoded loop to a
// consistent RMS (with a peak ceiling) so a V12 car does not dominate a V8.
function normalizeBuffer(context: AudioContext, buffer: AudioBuffer): AudioBuffer {
  const data = buffer.getChannelData(0);
  let sum = 0;
  let peak = 0;
  for (let i = 0; i < data.length; i += 1) {
    const value = data[i];
    sum += value * value;
    peak = Math.max(peak, Math.abs(value));
  }
  const rms = Math.sqrt(sum / data.length);
  if (rms < 1e-6 || peak < 1e-6) return buffer;
  let gain = 0.11 / rms;
  gain = Math.min(gain, 0.92 / peak);
  gain = clamp(gain, 0.25, 4);
  for (let i = 0; i < data.length; i += 1) data[i] *= gain;
  void context;
  return buffer;
}

function loadSample(
  context: AudioContext,
  car: string | undefined,
  engine: string | undefined,
): Promise<AudioBuffer | null> {
  let cache = sampleCache.get(context);
  if (!cache) {
    cache = new Map();
    sampleCache.set(context, cache);
  }
  const key = `${car ?? ""}:${engine ?? "default"}:${forcedEngine ?? ""}:${SOURCE}`;
  const existing = cache.get(key);
  if (existing) return existing;
  const def = sampleDef(car, engine);
  const order =
    SOURCE === "fallback"
      ? [def.fallback, def.recorded]
      : [def.recorded, def.fallback];
  const loading = (async () => {
    for (const relative of order) {
      try {
        return await decode(context, relative);
      } catch (error) {
        console.warn("engine sample unavailable", relative, error);
      }
    }
    return null;
  })();
  cache.set(key, loading);
  return loading;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function limiter(context: AudioContext): DynamicsCompressorNode {
  const node = context.createDynamicsCompressor();
  node.threshold.value = -14;
  node.knee.value = 12;
  node.ratio.value = 6;
  node.attack.value = 0.003;
  node.release.value = 0.25;
  return node;
}

export class SampleEngineAudio {
  status: "off" | "starting" | "running" | "error" = "off";
  error = "";
  muted = false;

  private readonly engineId: string;
  private readonly vehicleId: string;
  private readonly carId: string | undefined;
  private context: AudioContext | null = null;
  private master: GainNode | null = null;
  private gain: GainNode | null = null;
  private source: AudioBufferSourceNode | null = null;
  private analyser: AnalyserNode | null = null;
  private surfaceAudio: EngineSimAudio | null = null;
  private starting: Promise<void> | null = null;
  private refRpm = DEFAULT_SAMPLE.refRpm;
  private lastRpm = 900;
  private audioRpm = 900;
  private lastSpeedMps = 0;
  private lastGear = 1;
  private lastLoad = 0;
  private lastSkid = 0;

  constructor(options: { engine?: string; vehicle?: string; car?: string } = {}) {
    this.engineId = options.engine ?? "rotary2";
    this.vehicleId = options.vehicle ?? "sports";
    this.carId = options.car;
  }

  get contextState(): string {
    return this.context?.state ?? "closed";
  }

  async start(): Promise<void> {
    if (this.status === "running") {
      await this.context?.resume();
      return;
    }
    if (this.starting) return this.starting;
    this.starting = this.startInternal();
    try {
      await this.starting;
    } finally {
      this.starting = null;
    }
  }

  private async startInternal(): Promise<void> {
    this.status = "starting";
    try {
      const context = new AudioContext({ latencyHint: "interactive" });
      this.context = context;
      // Resume before the sample fetch so the user gesture still applies.
      await context.resume();
      const surface = new EngineSimAudio({
        engine: this.engineId,
        vehicle: this.vehicleId,
        surfaceOnly: true,
        context,
      });
      await surface.start();
      this.surfaceAudio = surface;
      const buffer = await loadSample(context, this.carId, this.engineId);
      if (!buffer) throw new Error("no engine sample available");
      this.refRpm = refRpmFor(this.carId, this.engineId);

      const comp = limiter(context);
      comp.connect(context.destination);
      const master = context.createGain();
      master.gain.value = MASTER_GAIN;
      master.connect(comp);
      const gain = context.createGain();
      gain.gain.value = 0;
      gain.connect(master);
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.loop = true;
      source.connect(gain);
      source.start();
      const analyser = context.createAnalyser();
      analyser.fftSize = 256;
      comp.connect(analyser);

      this.master = master;
      this.gain = gain;
      this.source = source;
      this.analyser = analyser;
      this.status = "running";
    } catch (error) {
      this.status = "error";
      this.error = String(error);
      console.warn("sample engine audio unavailable", error);
      this.surfaceAudio?.dispose();
      this.surfaceAudio = null;
      await this.context?.close().catch(() => {});
      this.context = null;
    }
  }

  private engineTarget(load: number): number {
    const mapping = mappingFor(this.carId);
    if (mapping) {
      const gainFactor = 1 + mapping.loadGain * ((0.35 + 0.65 * clamp(load, 0, 1)) - 1);
      return mapping.gain * clamp(gainFactor, 0.2, 1.2);
    }
    return ENGINE_TRIM * (0.45 + 0.55 * clamp(load, 0, 1));
  }

  update(
    _delta: number,
    throttle: number,
    _brake: number,
    gear: number,
    rpm: number,
    speedMps: number,
    wheels: TireAudioWheel[] = [],
  ): void {
    if (this.status !== "running" || !this.context) return;
    const now = this.context.currentTime;
    const safeRpm = Math.max(0, Number(rpm) || 0);
    const dt = Math.max(0, Math.min(0.1, Number(_delta) || 0));
    const previousRpm = this.audioRpm;
    const filtered = previousRpm + (safeRpm - previousRpm) * (1 - Math.exp(-dt / 0.06));
    const maxStep = 6_000 * dt;
    this.audioRpm = clamp(filtered, previousRpm - maxStep, previousRpm + maxStep);
    if (this.source) {
      this.source.playbackRate.setTargetAtTime(
        playbackRate(this.carId, this.engineId, this.audioRpm),
        now,
        0.04,
      );
    }
    const load = clamp(Number(throttle) || 0, 0, 1);
    const target = this.muted ? 0 : this.engineTarget(load);
    if (this.gain) this.gain.gain.setTargetAtTime(target, now, 0.06);

    this.surfaceAudio?.update(_delta, throttle, _brake, gear, rpm, speedMps, wheels);

    this.lastRpm = this.audioRpm;
    this.lastSpeedMps = Number(speedMps) || 0;
    this.lastGear = Number(gear) || 1;
    this.lastLoad = load;
    this.lastSkid = this.surfaceAudio?.tireState().slip ?? this.lastSkid;
  }

  state(): Record<string, number | string> {
    return {
      rpm: this.lastRpm,
      speedKph: this.lastSpeedMps * 3.6,
      gear: this.lastGear,
      clutch: 1,
      clutchSlip: 0,
      load: this.lastLoad,
      skid: this.lastSkid,
      engine: this.engineId,
      car: this.carId ?? "",
      refRpm: this.refRpm,
      source: SOURCE,
    };
  }

  roadState(): {
    asphalt: number;
    concrete: number;
    kerb: number;
    grass: number;
    dirt: number;
    gain: number;
  } {
    return this.surfaceAudio?.roadState() ?? {
      asphalt: 0,
      concrete: 0,
      kerb: 0,
      grass: 0,
      dirt: 0,
      gain: 0,
    };
  }

  tireState(): { gain: number; slip: number } {
    return this.surfaceAudio?.tireState() ?? { gain: 0, slip: 0 };
  }

  level(): number {
    if (!this.analyser) return 0;
    const data = new Float32Array(this.analyser.fftSize);
    this.analyser.getFloatTimeDomainData(data);
    let sum = 0;
    for (let i = 0; i < data.length; i += 1) sum += data[i] * data[i];
    return Math.sqrt(sum / data.length);
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    if (!this.context || !this.gain) return;
    const now = this.context.currentTime;
    const target = muted ? 0 : this.engineTarget(this.lastLoad);
    this.gain.gain.setTargetAtTime(target, now, 0.03);
    this.surfaceAudio?.setMuted(muted);
  }

  toggleMuted(): void {
    this.setMuted(!this.muted);
  }

  suspend(): void {
    if (this.context?.state === "running") void this.context.suspend();
  }

  async resume(): Promise<void> {
    if (this.status === "running") await this.context?.resume();
  }

  applyTuning(_tuning: unknown): void {
    // Test branch: no tuning surface yet.
  }

  playImpact(speed: number): void {
    const context = this.context;
    const master = this.master;
    if (!context || !master || this.muted) return;
    const amp = clamp(Number(speed) / 20, 0.1, 1) * 0.35;
    const frames = Math.floor(context.sampleRate * 0.12);
    const buffer = context.createBuffer(1, frames, context.sampleRate);
    const data = buffer.getChannelData(0);
    const decay = context.sampleRate * 0.03;
    for (let i = 0; i < frames; i += 1) {
      data[i] = (Math.random() * 2 - 1) * amp * Math.exp(-i / decay);
    }
    const source = context.createBufferSource();
    source.buffer = buffer;
    const filter = context.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = 600;
    source.connect(filter);
    filter.connect(master);
    source.start();
  }

  async restart(): Promise<void> {
    this.dispose();
    await this.start();
  }

  dispose(): void {
    try {
      this.source?.stop();
    } catch {
      // Already stopped.
    }
    this.surfaceAudio?.dispose();
    this.surfaceAudio = null;
    this.source?.disconnect();
    this.gain?.disconnect();
    this.master?.disconnect();
    this.analyser?.disconnect();
    this.source = null;
    this.gain = null;
    this.master = null;
    this.analyser = null;
    const context = this.context;
    this.context = null;
    this.status = "off";
    void context?.close().catch(() => {});
  }
}

interface RemoteVoice {
  source: AudioBufferSourceNode;
  gain: GainNode;
  engine: string;
  vehicle: string;
  car: string | undefined;
  ready: boolean;
}

export class SampleRemoteEngineAudio {
  status: "off" | "starting" | "running" | "error" = "off";
  muted = false;
  readonly mode: RemoteAudioMode;

  private readonly engineId: string;
  private readonly vehicleId: string;
  private readonly carId: string | undefined;
  private readonly contextProvider: (() => AudioContext | null) | null;
  private context: AudioContext | null = null;
  private ownsContext = true;
  private master: GainNode | null = null;
  private voices = new Map<number, RemoteVoice>();
  private profiles = new Map<
    number,
    { engine?: string; vehicle?: string; car?: string }
  >();
  private startingVoices = new Set<number>();
  private starting: Promise<void> | null = null;

  constructor(options: RemoteAudioOptions = {}) {
    this.engineId = options.engine ?? "rotary2";
    this.vehicleId = options.vehicle ?? "sports";
    this.carId = options.car;
    this.mode = options.mode ?? "full";
    this.contextProvider = options.contextProvider ?? null;
  }

  get sharesPlayerContext(): boolean {
    return !this.ownsContext;
  }

  get voiceCount(): number {
    return this.voices.size;
  }

  profileForSlot(slot: number): {
    engine: string;
    vehicle: string;
    car: string | undefined;
  } {
    const profile = this.profiles.get(slot);
    return {
      engine: profile?.engine ?? this.engineId,
      vehicle: profile?.vehicle ?? this.vehicleId,
      car: profile?.car ?? this.carId,
    };
  }

  setProfile(
    slot: number,
    engine: string,
    vehicle: string,
    car?: string,
  ): void {
    const before = this.profileForSlot(slot);
    this.profiles.set(slot, { engine, vehicle, car });
    const after = this.profileForSlot(slot);
    if (before.engine !== after.engine || before.car !== after.car) {
      this.remove(slot);
    }
  }

  async start(): Promise<void> {
    if (this.status === "running") {
      await this.context?.resume();
      return;
    }
    if (this.starting) return this.starting;
    this.starting = this.startInternal();
    try {
      await this.starting;
    } finally {
      this.starting = null;
    }
  }

  private async startInternal(): Promise<void> {
    this.status = "starting";
    try {
      const shared = this.contextProvider?.() ?? null;
      const context = shared ?? new AudioContext({ latencyHint: "interactive" });
      this.ownsContext = shared === null;
      this.context = context;
      await context.resume();
      const comp = limiter(context);
      comp.connect(context.destination);
      const master = context.createGain();
      master.gain.value = REMOTE_MASTER;
      master.connect(comp);
      this.master = master;
      this.status = "running";
    } catch (error) {
      this.status = "error";
      console.warn("sample opponent audio unavailable", error);
      if (this.ownsContext) await this.context?.close().catch(() => {});
      this.context = null;
    }
  }

  update(
    slot: number,
    state: RemoteEngineState | null,
    distance: number,
    _deltaSeconds: number,
  ): void {
    if (this.status !== "running" || !this.context) return;
    const voice = this.voices.get(slot);
    if (!state) {
      if (voice) {
        voice.gain.gain.setTargetAtTime(0, this.context.currentTime, 0.04);
      }
      return;
    }
    if (!voice) {
      if (!this.startingVoices.has(slot)) void this.createVoice(slot);
      return;
    }
    if (!voice.ready) return;
    const now = this.context.currentTime;
    voice.source.playbackRate.setTargetAtTime(
      playbackRate(voice.car, voice.engine, Number(state.rpm) || 0),
      now,
      0.04,
    );
    const load = clamp(Number(state.throttle) || 0, 0, 1);
    const mapping = mappingFor(voice.car);
    const gainFactor = mapping
      ? mapping.gain * clamp(1 + mapping.loadGain * ((0.35 + 0.65 * load) - 1), 0.2, 1.2)
      : 0.45 + 0.35 * load;
    const target = this.muted ? 0 : distanceGain(distance) * gainFactor;
    voice.gain.gain.setTargetAtTime(target, now, 0.06);
  }

  silence(slot: number): void {
    const voice = this.voices.get(slot);
    if (!voice || !this.context) return;
    voice.gain.gain.setTargetAtTime(0, this.context.currentTime, 0.04);
  }

  remove(slot: number): void {
    const voice = this.voices.get(slot);
    if (!voice) return;
    this.voices.delete(slot);
    voice.gain.gain.value = 0;
    try {
      voice.source.stop();
    } catch {
      // Already stopped.
    }
    voice.source.disconnect();
    voice.gain.disconnect();
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    if (!this.context) return;
    const now = this.context.currentTime;
    for (const voice of this.voices.values()) {
      voice.gain.gain.setTargetAtTime(0, now, 0.03);
    }
  }

  suspend(): void {
    if (this.context?.state === "running") void this.context.suspend();
  }

  async resume(): Promise<void> {
    if (this.status === "running") await this.context?.resume();
  }

  voiceSummary(): Array<Record<string, string | number | boolean>> {
    return [...this.voices.entries()].map(([slot, voice]) => ({
      slot,
      engine: voice.engine,
      vehicle: voice.vehicle,
      car: voice.car ?? "",
      mode: this.mode,
      ready: voice.ready,
      gain: voice.gain.gain.value,
    }));
  }

  dispose(): void {
    for (const slot of [...this.voices.keys()]) this.remove(slot);
    this.voices.clear();
    this.startingVoices.clear();
    this.profiles.clear();
    this.master?.disconnect();
    this.master = null;
    const context = this.context;
    this.context = null;
    this.status = "off";
    if (this.ownsContext) void context?.close().catch(() => {});
  }

  private async createVoice(slot: number): Promise<void> {
    const context = this.context;
    const master = this.master;
    if (
      !context ||
      !master ||
      this.voices.has(slot) ||
      this.startingVoices.has(slot)
    ) {
      return;
    }
    this.startingVoices.add(slot);
    const profile = this.profileForSlot(slot);
    try {
      const buffer = await loadSample(context, profile.car, profile.engine);
      if (!buffer) return;
      const current = this.profileForSlot(slot);
      if (
        current.engine !== profile.engine ||
        current.car !== profile.car ||
        this.voices.has(slot)
      ) {
        return;
      }
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.loop = true;
      const gain = context.createGain();
      gain.gain.value = 0;
      source.connect(gain);
      gain.connect(master);
      source.start();
      this.voices.set(slot, {
        source,
        gain,
        engine: profile.engine,
        vehicle: profile.vehicle,
        car: profile.car,
        ready: true,
      });
    } catch (error) {
      console.warn("sample opponent voice unavailable", slot, error);
    } finally {
      this.startingVoices.delete(slot);
    }
  }
}
