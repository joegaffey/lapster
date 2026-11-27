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
//   ?sampleNoiseGain=1.6       tyre/road trim

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
    rpmLow: 900,
    rpmHigh: 8000,
    rateLow: 0.3,
    rateHigh: 0.8,
    curve: 2.4,
    gain: 0.26,
    loadGain: 0,
  },
  "sc-deckard-conejo": {
    rpmLow: 1000,
    rpmHigh: 6930,
    rateLow: 0.6,
    rateHigh: 2,
    curve: 1.75,
    gain: 0.4,
    loadGain: 0.35,
  },
  "67gp-cavallo-123": {
    rpmLow: 1000,
    rpmHigh: 10000,
    rateLow: 0.5,
    rateHigh: 2,
    curve: 2.05,
    gain: 0.25,
    loadGain: 0.35,
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
const noiseGainOverride = Number(query.get("sampleNoiseGain"));
const MASTER_GAIN =
  Number.isFinite(gainOverride) && gainOverride > 0 ? gainOverride : 0.32;
const ENGINE_TRIM =
  Number.isFinite(engineGainOverride) && engineGainOverride > 0
    ? engineGainOverride
    : 0.55;
const NOISE_TRIM =
  Number.isFinite(noiseGainOverride) && noiseGainOverride > 0
    ? noiseGainOverride
    : 1.6;
const REMOTE_MASTER = 0.55;

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

function surfaceRoadFactor(surface: string | undefined): number {
  const name = (surface ?? "").toUpperCase();
  if (name.includes("GRASS")) return 1.6;
  if (name.includes("KERB")) return 1.3;
  if (name.includes("ANELLO")) return 1.2;
  return 1;
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
  private readonly carId: string | undefined;
  private context: AudioContext | null = null;
  private master: GainNode | null = null;
  private gain: GainNode | null = null;
  private source: AudioBufferSourceNode | null = null;
  private analyser: AnalyserNode | null = null;
  private noise: AudioBufferSourceNode | null = null;
  private roadGain: GainNode | null = null;
  private roadFilter: BiquadFilterNode | null = null;
  private rollingGain: GainNode | null = null;
  private rollingFilter: BiquadFilterNode | null = null;
  private tireGain: GainNode | null = null;
  private tireFilter: BiquadFilterNode | null = null;
  private starting: Promise<void> | null = null;
  private refRpm = DEFAULT_SAMPLE.refRpm;
  private lastRpm = 900;
  private audioRpm = 900;
  private lastSpeedMps = 0;
  private lastGear = 1;
  private lastLoad = 0;
  private lastSkid = 0;
  private lastRoad = 0;
  private lastSurface = "ASPHALT";

  constructor(options: { engine?: string; vehicle?: string; car?: string } = {}) {
    this.engineId = options.engine ?? "rotary2";
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

      // Shared noise bed for road, rolling tyre, and squeal layers.
      const noiseBuffer = context.createBuffer(1, context.sampleRate * 2, context.sampleRate);
      const noiseData = noiseBuffer.getChannelData(0);
      for (let i = 0; i < noiseData.length; i += 1) {
        noiseData[i] = Math.random() * 2 - 1;
      }
      const noise = context.createBufferSource();
      noise.buffer = noiseBuffer;
      noise.loop = true;
      noise.start();
      const roadFilter = context.createBiquadFilter();
      roadFilter.type = "lowpass";
      roadFilter.frequency.value = 850;
      roadFilter.Q.value = 0.7;
      const roadGain = context.createGain();
      roadGain.gain.value = 0;
      noise.connect(roadFilter);
      roadFilter.connect(roadGain);
      roadGain.connect(master);
      const rollingFilter = context.createBiquadFilter();
      rollingFilter.type = "bandpass";
      rollingFilter.frequency.value = 520;
      rollingFilter.Q.value = 1.2;
      const rollingGain = context.createGain();
      rollingGain.gain.value = 0;
      noise.connect(rollingFilter);
      rollingFilter.connect(rollingGain);
      rollingGain.connect(master);
      const tireFilter = context.createBiquadFilter();
      tireFilter.type = "bandpass";
      tireFilter.frequency.value = 1100;
      tireFilter.Q.value = 9;
      const tireGain = context.createGain();
      tireGain.gain.value = 0;
      noise.connect(tireFilter);
      tireFilter.connect(tireGain);
      tireGain.connect(master);

      this.master = master;
      this.gain = gain;
      this.source = source;
      this.analyser = analyser;
      this.noise = noise;
      this.roadFilter = roadFilter;
      this.roadGain = roadGain;
      this.rollingFilter = rollingFilter;
      this.rollingGain = rollingGain;
      this.tireFilter = tireFilter;
      this.tireGain = tireGain;
      this.status = "running";
    } catch (error) {
      this.status = "error";
      this.error = String(error);
      console.warn("sample engine audio unavailable", error);
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
    wheels: unknown,
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

    // Tyre and road layers from the same wheel telemetry the synth used.
    let skid = 0;
    let wheelLoad = 0;
    let surface = this.lastSurface;
    if (Array.isArray(wheels)) {
      for (const wheel of wheels as Array<{
        slipRatio?: number;
        slipAngle?: number;
        load?: number;
        contact?: boolean;
        surface?: string;
      }>) {
        if (wheel?.contact === false) continue;
        const lateral = Math.max(0, Math.abs(wheel?.slipAngle ?? 0) - 0.09) * 1.4;
        const longitudinal = Math.max(0, Math.abs(wheel?.slipRatio ?? 0) - 0.14);
        const slip = Math.max(lateral, longitudinal);
        if (slip > skid) {
          skid = slip;
          wheelLoad = Number(wheel?.load) || 0;
          if (wheel?.surface) surface = wheel.surface;
        }
      }
    }
    const speed = Number(speedMps) || 0;
    const speedFactor = clamp(speed / 30, 0, 1);
    const roadTarget = this.muted
      ? 0
      : clamp(
          0.16 * speedFactor * surfaceRoadFactor(surface) * NOISE_TRIM,
          0,
          0.4,
        );
    const rollingTarget = this.muted
      ? 0
      : clamp(0.12 * speedFactor * NOISE_TRIM, 0, 0.3);
    const loadFactor = clamp(wheelLoad / 4500, 0.2, 1);
    const tireTarget = this.muted
      ? 0
      : clamp(
          skid * 2.2 * loadFactor * (0.35 + speedFactor) * NOISE_TRIM,
          0,
          0.55,
        );
    if (this.roadGain) {
      this.roadGain.gain.setTargetAtTime(roadTarget, now, 0.08);
    }
    if (this.roadFilter) {
      this.roadFilter.frequency.setTargetAtTime(
        750 + 450 * speedFactor,
        now,
        0.08,
      );
    }
    if (this.rollingGain) {
      this.rollingGain.gain.setTargetAtTime(rollingTarget, now, 0.1);
    }
    if (this.rollingFilter) {
      this.rollingFilter.frequency.setTargetAtTime(
        420 + 260 * speedFactor,
        now,
        0.1,
      );
    }
    if (this.tireGain) {
      this.tireGain.gain.setTargetAtTime(tireTarget, now, 0.05);
    }
    if (this.tireFilter) {
      this.tireFilter.frequency.setTargetAtTime(
        900 + 800 * clamp(skid, 0, 1),
        now,
        0.05,
      );
    }

    this.lastRpm = this.audioRpm;
    this.lastSpeedMps = speed;
    this.lastGear = Number(gear) || 1;
    this.lastLoad = load;
    this.lastSkid = skid;
    this.lastRoad = roadTarget;
    this.lastSurface = surface;
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
    const name = this.lastSurface.toUpperCase();
    const gainValue = this.lastRoad;
    return {
      asphalt: name.includes("ASPHALT") || name === "" ? gainValue : 0,
      concrete: name.includes("ANELLO") ? gainValue : 0,
      kerb: name.includes("KERB") ? gainValue : 0,
      grass: name.includes("GRASS") ? gainValue : 0,
      dirt: name.includes("DIRT") ? gainValue : 0,
      gain: gainValue,
    };
  }

  tireState(): { gain: number; slip: number } {
    return {
      gain: this.tireGain?.gain.value ?? 0,
      slip: this.lastSkid,
    };
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
    this.roadGain?.gain.setTargetAtTime(0, now, 0.03);
    this.rollingGain?.gain.setTargetAtTime(0, now, 0.03);
    this.tireGain?.gain.setTargetAtTime(0, now, 0.03);
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
      this.noise?.stop();
    } catch {
      // Already stopped.
    }
    this.source?.disconnect();
    this.noise?.disconnect();
    this.gain?.disconnect();
    this.master?.disconnect();
    this.analyser?.disconnect();
    this.roadFilter?.disconnect();
    this.roadGain?.disconnect();
    this.rollingFilter?.disconnect();
    this.rollingGain?.disconnect();
    this.tireFilter?.disconnect();
    this.tireGain?.disconnect();
    this.source = null;
    this.noise = null;
    this.gain = null;
    this.master = null;
    this.analyser = null;
    this.roadFilter = null;
    this.roadGain = null;
    this.rollingFilter = null;
    this.rollingGain = null;
    this.tireFilter = null;
    this.tireGain = null;
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
