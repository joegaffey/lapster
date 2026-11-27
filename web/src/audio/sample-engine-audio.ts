// Lightweight recorded-sample engine audio for the sample-engine test branch.
//
// Replaces the runtime Engine_Sim voices with looping recorded Speed Dreams
// samples. If a recorded sample is missing or fails to decode, it falls back to
// a loop baked offline from Engine_Sim. The synth is never instantiated at
// runtime on this path.
//
// Test knobs (URL query):
//   ?sampleSource=fallback   prefer the baked fallback
//   ?sampleRefRpm=3500       override the recorded sample's reference rpm
//   ?sampleGain=0.3          override the master gain

import { distanceGain } from "./remote-engine-mix.ts";

export const NEAREST_SLOT = 0;
export type RemoteAudioMode = "lite" | "full";

export interface RemoteAudioOptions {
  engine?: string;
  vehicle?: string;
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
}

const SAMPLE_DEFS: Record<string, SampleDef> = {
  rotary2: {
    recorded: "speed-dreams/rotary2.wav",
    fallback: "fallback/rotary2.wav",
    refRpm: 3500,
  },
  v8cross: {
    recorded: "speed-dreams/v8-ears68.wav",
    fallback: "fallback/v8cross.wav",
    refRpm: 3000,
  },
  v12: {
    recorded: "speed-dreams/v12_smooth.wav",
    fallback: "fallback/v12.wav",
    refRpm: 3000,
  },
};

const DEFAULT_SAMPLE = SAMPLE_DEFS.v8cross;
const ASSET_BASE = new URL("../assets/audio/", import.meta.url);

const query = new URLSearchParams(globalThis.location?.search ?? "");
const SOURCE = query.get("sampleSource") === "fallback" ? "fallback" : "recorded";
const refOverride = Number(query.get("sampleRefRpm"));
const gainOverride = Number(query.get("sampleGain"));
const MASTER_GAIN =
  Number.isFinite(gainOverride) && gainOverride > 0 ? gainOverride : 0.3;

const sampleCache = new WeakMap<
  AudioContext,
  Map<string, Promise<AudioBuffer | null>>
>();

function sampleDef(engine: string | undefined): SampleDef {
  return (engine && SAMPLE_DEFS[engine]) || DEFAULT_SAMPLE;
}

function refRpmFor(engine: string | undefined): number {
  if (Number.isFinite(refOverride) && refOverride > 0) return refOverride;
  return sampleDef(engine).refRpm;
}

async function decode(
  context: AudioContext,
  relative: string,
): Promise<AudioBuffer> {
  const url = new URL(relative, ASSET_BASE);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${response.status} ${url}`);
  return context.decodeAudioData(await response.arrayBuffer());
}

function loadSample(
  context: AudioContext,
  engine: string | undefined,
): Promise<AudioBuffer | null> {
  let cache = sampleCache.get(context);
  if (!cache) {
    cache = new Map();
    sampleCache.set(context, cache);
  }
  const key = `${engine ?? "default"}:${SOURCE}`;
  const existing = cache.get(key);
  if (existing) return existing;
  const def = sampleDef(engine);
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

export class SampleEngineAudio {
  status: "off" | "starting" | "running" | "error" = "off";
  error = "";
  muted = false;

  private readonly engineId: string;
  private context: AudioContext | null = null;
  private master: GainNode | null = null;
  private gain: GainNode | null = null;
  private source: AudioBufferSourceNode | null = null;
  private analyser: AnalyserNode | null = null;
  private starting: Promise<void> | null = null;
  private refRpm = DEFAULT_SAMPLE.refRpm;
  private lastRpm = 900;
  private lastSpeedMps = 0;
  private lastGear = 1;
  private lastLoad = 0;

  constructor(options: { engine?: string; vehicle?: string } = {}) {
    this.engineId = options.engine ?? "rotary2";
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
      const buffer = await loadSample(context, this.engineId);
      if (!buffer) throw new Error("no engine sample available");
      this.refRpm = refRpmFor(this.engineId);

      const master = context.createGain();
      master.gain.value = MASTER_GAIN;
      master.connect(context.destination);
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
      master.connect(analyser);

      this.master = master;
      this.gain = gain;
      this.source = source;
      this.analyser = analyser;
      this.status = "running";
    } catch (error) {
      this.status = "error";
      this.error = String(error);
      console.warn("sample engine audio unavailable", error);
      await this.context?.close().catch(() => {});
      this.context = null;
    }
  }

  update(
    _delta: number,
    throttle: number,
    _brake: number,
    gear: number,
    rpm: number,
    speedMps: number,
    _wheels: unknown,
  ): void {
    if (this.status !== "running" || !this.context) return;
    const now = this.context.currentTime;
    const safeRpm = Math.max(0, Number(rpm) || 0);
    const rate = clamp(safeRpm / this.refRpm, 0.25, 3);
    if (this.source) {
      this.source.playbackRate.setTargetAtTime(rate, now, 0.04);
    }
    const load = clamp(Number(throttle) || 0, 0, 1);
    const target = this.muted ? 0 : 0.45 + 0.55 * load;
    if (this.gain) this.gain.gain.setTargetAtTime(target, now, 0.06);
    this.lastRpm = safeRpm;
    this.lastSpeedMps = Number(speedMps) || 0;
    this.lastGear = Number(gear) || 1;
    this.lastLoad = load;
  }

  state(): Record<string, number | string> {
    return {
      rpm: this.lastRpm,
      speedKph: this.lastSpeedMps * 3.6,
      gear: this.lastGear,
      clutch: 1,
      clutchSlip: 0,
      load: this.lastLoad,
      engine: this.engineId,
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
    return { asphalt: 0, concrete: 0, kerb: 0, grass: 0, dirt: 0, gain: 0 };
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
    const target = muted ? 0 : 0.45 + 0.55 * this.lastLoad;
    this.gain.gain.setTargetAtTime(target, this.context.currentTime, 0.03);
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

  playImpact(_speed: number): void {
    // Impact one-shots are out of scope for the light sample test.
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
  ready: boolean;
}

export class SampleRemoteEngineAudio {
  status: "off" | "starting" | "running" | "error" = "off";
  muted = false;
  readonly mode: RemoteAudioMode;

  private readonly engineId: string;
  private readonly vehicleId: string;
  private readonly contextProvider: (() => AudioContext | null) | null;
  private context: AudioContext | null = null;
  private ownsContext = true;
  private master: GainNode | null = null;
  private voices = new Map<number, RemoteVoice>();
  private profiles = new Map<number, { engine?: string; vehicle?: string }>();
  private startingVoices = new Set<number>();
  private starting: Promise<void> | null = null;

  constructor(options: RemoteAudioOptions = {}) {
    this.engineId = options.engine ?? "rotary2";
    this.vehicleId = options.vehicle ?? "sports";
    this.mode = options.mode ?? "full";
    this.contextProvider = options.contextProvider ?? null;
  }

  get sharesPlayerContext(): boolean {
    return !this.ownsContext;
  }

  get voiceCount(): number {
    return this.voices.size;
  }

  profileForSlot(slot: number): { engine: string; vehicle: string } {
    const profile = this.profiles.get(slot);
    return {
      engine: profile?.engine ?? this.engineId,
      vehicle: profile?.vehicle ?? this.vehicleId,
    };
  }

  setProfile(slot: number, engine: string, vehicle: string): void {
    const before = this.profileForSlot(slot);
    this.profiles.set(slot, { engine, vehicle });
    const after = this.profileForSlot(slot);
    if (before.engine !== after.engine) this.remove(slot);
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
      const master = context.createGain();
      master.gain.value = 1;
      master.connect(context.destination);
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
    const rate = clamp(
      (Number(state.rpm) || 0) / refRpmFor(voice.engine),
      0.25,
      3,
    );
    voice.source.playbackRate.setTargetAtTime(rate, now, 0.04);
    const load = clamp(Number(state.throttle) || 0, 0, 1);
    const target = this.muted
      ? 0
      : distanceGain(distance) * (0.55 + 0.45 * load);
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
      const buffer = await loadSample(context, profile.engine);
      if (!buffer) return;
      const current = this.profileForSlot(slot);
      if (current.engine !== profile.engine || this.voices.has(slot)) return;
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
        ready: true,
      });
    } catch (error) {
      console.warn("sample opponent voice unavailable", slot, error);
    } finally {
      this.startingVoices.delete(slot);
    }
  }
}
