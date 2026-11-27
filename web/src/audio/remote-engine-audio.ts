import { APP_VERSION } from "../version.generated.ts";
import { EngineSim } from "./engine-sim/engine-sim.js";
import {
  distanceGain,
  liteGain,
  REMOTE_MAX_DISTANCE as MAX_DISTANCE,
} from "./remote-engine-mix.ts";

export type RemoteAudioMode = "lite" | "full";

/**
 * Dedicated slot for the single nearest-AI `full` voice. AI field slots use
 * 1..9, so slot 0 never collides with a per-car voice.
 */
export const NEAREST_SLOT = 0;

export interface RemoteAudioOptions {
  engine?: string;
  vehicle?: string;
  /**
   * `lite` uses plain Web Audio oscillators on the player's context. `full`
   * keeps the EngineSim AudioWorklet voice path.
   */
  mode?: RemoteAudioMode;
  /**
   * Optional source for a context shared with the player audio. Evaluated at
   * start time so the player has a chance to create its context first. Used by
   * both `lite` and `full` voices when provided; otherwise a private context is
   * created.
   */
  contextProvider?: () => AudioContext | null;
}

export interface RemoteAudioProfile {
  engine: string;
  vehicle: string;
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

interface Voice {
  mode: RemoteAudioMode;
  sim: EngineSim | null;
  osc1: OscillatorNode | null;
  osc2: OscillatorNode | null;
  filter: BiquadFilterNode | null;
  gain: GainNode;
  ready: boolean;
  distance: number;
  profile: RemoteAudioProfile;
  prevRpm: number;
  tireBand1: BiquadFilterNode;
  tireBand2: BiquadFilterNode;
  tireGain: GainNode;
  roadFilter: BiquadFilterNode;
  roadGain: GainNode;
}

// Firing-frequency cylinder counts for the lite oscillator voice. The full
// EngineSim profiles keep their own geometry; rotary2 is treated as a two-pulse
// source here per the lite approximation.
const CYLINDERS_BY_ENGINE: Record<string, number> = {
  v12: 12,
  v8cross: 8,
  v8: 8,
  rotary2: 2,
};

function cylindersForEngine(engine: string): number {
  return CYLINDERS_BY_ENGINE[engine] ?? 4;
}

function firingHz(engine: string, rpm: number): number {
  return (Math.max(0, rpm) / 60) * (cylindersForEngine(engine) / 2);
}

function liteCutoff(firing: number, throttle: number): number {
  return Math.max(200, Math.min(6_000, 300 + firing * 4 + throttle * 1_500));
}

export class RemoteEngineAudio {
  status: "off" | "starting" | "running" | "error" = "off";
  muted = false;
  readonly mode: RemoteAudioMode;

  private readonly engineId: string;
  private readonly vehicleId: string;
  private readonly contextProvider: (() => AudioContext | null) | null;
  private context: AudioContext | null = null;
  private sharedContext = false;
  private noiseSource: AudioBufferSourceNode | null = null;
  private voices = new Map<number, Voice>();
  private profiles = new Map<number, { engine?: string; vehicle?: string }>();
  private starting: Promise<void> | null = null;
  private startingVoices = new Set<number>();

  constructor(options: RemoteAudioOptions = {}) {
    this.engineId = options.engine ?? "rotary2";
    this.vehicleId = options.vehicle ?? "sports";
    this.mode = options.mode ?? "lite";
    this.contextProvider = options.contextProvider ?? null;
  }

  /** True when voices play through the player's AudioContext. */
  get sharesPlayerContext(): boolean {
    return this.sharedContext;
  }

  get voiceCount(): number {
    return this.voices.size;
  }

  voiceSummary(): Array<{
    slot: number;
    mode: RemoteAudioMode;
    ready: boolean;
    gain: number;
    distance: number;
    profile: RemoteAudioProfile;
  }> {
    return [...this.voices.entries()]
      .sort(([a], [b]) => a - b)
      .map(([slot, voice]) => ({
        slot,
        mode: voice.mode,
        ready: voice.ready,
        gain: voice.gain.gain.value,
        distance: voice.distance,
        profile: { ...voice.profile },
      }));
  }

  /**
   * Assign the engine/vehicle profile for one slot. Existing voices created
   * with a different profile are removed and recreated on the next update.
   */
  setProfile(slot: number, engine?: string, vehicle?: string): void {
    const before = this.profileForSlot(slot);
    this.profiles.set(slot, { engine, vehicle });
    const after = this.profileForSlot(slot);
    if (before.engine === after.engine && before.vehicle === after.vehicle) {
      return;
    }
    if (this.voices.has(slot)) {
      this.remove(slot);
    }
  }

  /** Resolved profile for a slot, falling back to the constructor defaults. */
  profileForSlot(slot: number): RemoteAudioProfile {
    const profile = this.profiles.get(slot);
    return {
      engine: profile?.engine ?? this.engineId,
      vehicle: profile?.vehicle ?? this.vehicleId,
    };
  }

  async start(): Promise<void> {
    if (this.status === "running") {
      await this.resume();
      return;
    }
    if (this.starting) {
      return this.starting;
    }
    this.starting = this.startInternal();
    try {
      await this.starting;
    } finally {
      this.starting = null;
    }
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    if (!this.context) {
      return;
    }
    const now = this.context.currentTime;
    for (const voice of this.voices.values()) {
      voice.gain.gain.setTargetAtTime(0, now, 0.02);
    }
  }

  update(
    slot: number,
    state: RemoteEngineState | null,
    distance: number,
    deltaSeconds: number,
  ): void {
    if (!this.context || this.status !== "running") {
      return;
    }
    const voice = this.voices.get(slot);
    if (!state) {
      if (voice) {
        voice.gain.gain.setTargetAtTime(0, this.context.currentTime, 0.04);
      }
      return;
    }
    // Remember the last distance, including cars outside the audible
    // radius, so the debug summary can explain a silent voice.
    if (voice) {
      voice.distance = distance;
    }
    // Never allocate a voice for far-away cars, and stop simulating an
    // existing one once it leaves the audible radius.
    if (distance >= MAX_DISTANCE) {
      if (voice) {
        voice.gain.gain.setTargetAtTime(0, this.context.currentTime, 0.04);
      }
      return;
    }
    if (!voice) {
      if (!this.startingVoices.has(slot)) {
        void this.createVoice(slot, distance);
      }
      return;
    }
    if (!voice.ready) {
      return;
    }
    if (this.mode === "lite") {
      const now = this.context.currentTime;
      const throttle = Math.max(0, Math.min(1, state.throttle));
      const firing = firingHz(voice.profile.engine, state.rpm);
      const target = this.muted
        ? 0
        : liteGain(distanceGain(distance), state.rpm, throttle);
      voice.gain.gain.setTargetAtTime(target, now, 0.06);
      voice.osc1?.frequency.setTargetAtTime(firing, now, 0.06);
      voice.osc2?.frequency.setTargetAtTime(firing * 2.02, now, 0.06);
      voice.filter?.frequency.setTargetAtTime(
        liteCutoff(firing, throttle),
        now,
        0.06,
      );
      this.updateTires(voice, state);
      return;
    }
    const target = this.muted ? 0 : distanceGain(distance);
    voice.gain.gain.setTargetAtTime(target, this.context.currentTime, 0.06);
    voice.sim?.updateExternal(Math.max(0.001, Math.min(deltaSeconds, 0.1)), {
      rpm: state.rpm,
      speed: state.speedMps,
      gear: state.gear,
      throttle: state.throttle,
      brake: state.brake,
      load: state.throttle,
      prevRpm: voice.prevRpm,
    });
    voice.prevRpm = state.rpm;
    this.updateTires(voice, state);
  }

  silence(slot: number): void {
    const voice = this.voices.get(slot);
    if (!voice || !this.context) {
      return;
    }
    voice.gain.gain.setTargetAtTime(0, this.context.currentTime, 0.04);
  }

  remove(slot: number): void {
    const voice = this.voices.get(slot);
    if (!voice) {
      return;
    }
    this.voices.delete(slot);
    voice.gain.gain.value = 0;
    voice.sim?.dispose();
    try {
      voice.osc1?.stop();
      voice.osc2?.stop();
    } catch {
      // Oscillators may already be stopped during teardown.
    }
    voice.osc1?.disconnect();
    voice.osc2?.disconnect();
    voice.filter?.disconnect();
    try {
      this.noiseSource?.disconnect(voice.tireBand1);
      this.noiseSource?.disconnect(voice.tireBand2);
      this.noiseSource?.disconnect(voice.roadFilter);
    } catch {
      // The source may already be disconnected during teardown.
    }
    voice.tireBand1.disconnect();
    voice.tireBand2.disconnect();
    voice.tireGain.disconnect();
    voice.roadFilter.disconnect();
    voice.roadGain.disconnect();
    voice.gain.disconnect();
  }

  suspend(): void {
    // The player owns a shared context and handles its own suspend/resume.
    if (this.sharedContext) {
      return;
    }
    if (this.context?.state === "running") {
      void this.context.suspend();
    }
  }

  async resume(): Promise<void> {
    // The player owns a shared context and handles its own suspend/resume.
    if (this.sharedContext) {
      return;
    }
    if (this.status === "running" && this.context) {
      await this.context.resume();
    }
  }

  dispose(): void {
    for (const slot of [...this.voices.keys()]) {
      this.remove(slot);
    }
    this.voices.clear();
    this.profiles.clear();
    this.startingVoices.clear();
    this.status = "off";
    this.noiseSource?.stop();
    this.noiseSource?.disconnect();
    this.noiseSource = null;
    const context = this.context;
    this.context = null;
    // Never close a context owned by the player.
    if (!this.sharedContext) {
      void context?.close();
    }
    this.sharedContext = false;
  }

  private async startInternal(): Promise<void> {
    this.status = "starting";
    try {
      // Both modes can share the player's context when a provider is given.
      // Otherwise the remote engine keeps its own private context.
      const shared = this.contextProvider?.() ?? null;
      const context =
        shared ?? new AudioContext({ latencyHint: "interactive" });
      this.sharedContext = shared !== null;
      this.context = context;
      if (this.mode === "full") {
        EngineSim.workletURL = new URL(
          `../audio/combustion-worklet.js?v=${encodeURIComponent(APP_VERSION)}`,
          import.meta.url,
        );
      }
      const noise = context.createBuffer(1, context.sampleRate * 2, context.sampleRate);
      const data = noise.getChannelData(0);
      for (let index = 0; index < data.length; index += 1) {
        data[index] = Math.random() * 2 - 1;
      }
      const noiseSource = context.createBufferSource();
      noiseSource.buffer = noise;
      noiseSource.loop = true;
      noiseSource.start();
      this.noiseSource = noiseSource;
      if (!this.sharedContext) {
        await context.resume();
      }
      this.status = "running";
    } catch (error) {
      this.status = "error";
      console.warn("remote engine audio unavailable", error);
      if (!this.sharedContext) {
        await this.context?.close();
      }
      this.context = null;
      this.sharedContext = false;
      throw error;
    }
  }

  private updateTires(voice: Voice, state: RemoteEngineState): void {
    if (!this.context) {
      return;
    }
    let slip = 0;
    let load = 0;
    for (const wheel of state.wheels) {
      if (!wheel.contact) {
        continue;
      }
      const lateral = Math.max(0, Math.abs(wheel.slipAngle) - 0.09) * 1.4;
      const longitudinal = Math.max(0, Math.abs(wheel.slipRatio) - 0.14);
      const skid = Math.max(lateral, longitudinal);
      if (skid > slip) {
        slip = skid;
        load = wheel.load;
      }
    }
    const loadFactor = Math.max(0.2, Math.min(1, load / 4_500));
    const speedFactor = Math.max(0.35, Math.min(1, 0.35 + state.speedMps / 8));
    const squeal = Math.max(0, Math.min(1, slip * 4.2)) * loadFactor * speedFactor;
    const rolling = Math.max(0, Math.min(1, state.speedMps / 30)) * 0.16;
    const now = this.context.currentTime;
    voice.tireGain.gain.setTargetAtTime(squeal * 0.9, now, 0.03);
    voice.tireBand1.frequency.setTargetAtTime(
      900 + 650 * Math.max(0, Math.min(1, slip)),
      now,
      0.05,
    );
    voice.tireBand2.frequency.setTargetAtTime(
      2_000 + 900 * Math.max(0, Math.min(1, slip)),
      now,
      0.05,
    );
    voice.roadGain.gain.setTargetAtTime(rolling, now, 0.08);
  }

  private async createVoice(slot: number, distance: number): Promise<void> {
    const context = this.context;
    if (!context || this.voices.has(slot)) {
      return;
    }
    const profile = this.profileForSlot(slot);
    const full = this.mode === "full";
    if (full) {
      this.startingVoices.add(slot);
    }
    const gain = context.createGain();
    gain.gain.value = 0;
    gain.connect(context.destination);
    const noiseSource = this.noiseSource;
    const tireBand1 = context.createBiquadFilter();
    tireBand1.type = "bandpass";
    tireBand1.frequency.value = 1_100;
    tireBand1.Q.value = 9;
    const tireBand2 = context.createBiquadFilter();
    tireBand2.type = "bandpass";
    tireBand2.frequency.value = 2_400;
    tireBand2.Q.value = 14;
    const tireGain = context.createGain();
    tireGain.gain.value = 0;
    const roadFilter = context.createBiquadFilter();
    roadFilter.type = "lowpass";
    roadFilter.frequency.value = 850;
    roadFilter.Q.value = 0.7;
    const roadGain = context.createGain();
    roadGain.gain.value = 0;
    if (noiseSource) {
      noiseSource.connect(tireBand1);
      noiseSource.connect(tireBand2);
      noiseSource.connect(roadFilter);
    }
    tireBand1.connect(tireGain);
    tireBand2.connect(tireGain);
    roadFilter.connect(roadGain);
    tireGain.connect(gain);
    roadGain.connect(gain);

    let sim: EngineSim | null = null;
    let osc1: OscillatorNode | null = null;
    let osc2: OscillatorNode | null = null;
    let filter: BiquadFilterNode | null = null;
    if (full) {
      sim = new EngineSim(context, {
        engine: profile.engine,
        vehicle: profile.vehicle,
        volume: 1,
      });
      sim.setAutoShift(false);
      sim.setGear(1);
      sim.setMix({ transmission: 0 });
      sim.connect(gain);
    } else {
      const firing = firingHz(profile.engine, 900);
      filter = context.createBiquadFilter();
      filter.type = "lowpass";
      filter.frequency.value = liteCutoff(firing, 0);
      filter.Q.value = 1;
      osc1 = context.createOscillator();
      osc1.type = "sawtooth";
      osc1.frequency.value = firing;
      osc2 = context.createOscillator();
      osc2.type = "square";
      osc2.frequency.value = firing * 2.02;
      osc1.connect(filter);
      osc2.connect(filter);
      filter.connect(gain);
      osc1.start();
      osc2.start();
    }

    const voice: Voice = {
      mode: this.mode,
      sim,
      osc1,
      osc2,
      filter,
      gain,
      ready: !full,
      distance,
      profile,
      prevRpm: 900,
      tireBand1,
      tireBand2,
      tireGain,
      roadFilter,
      roadGain,
    };
    this.voices.set(slot, voice);
    if (!full || !sim) {
      return;
    }
    try {
      await sim.start();
      if (this.voices.get(slot) !== voice) {
        sim.dispose();
        gain.disconnect();
        return;
      }
      voice.ready = true;
    } catch (error) {
      console.warn("remote engine voice unavailable", error);
      this.remove(slot);
    } finally {
      this.startingVoices.delete(slot);
    }
  }
}
