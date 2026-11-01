import { APP_VERSION } from "../version.generated.ts";
import { EngineSim } from "./engine-sim/engine-sim.js";

export interface RemoteAudioOptions {
  engine?: string;
  vehicle?: string;
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
  sim: EngineSim;
  gain: GainNode;
  ready: boolean;
  prevRpm: number;
  tireBand1: BiquadFilterNode;
  tireBand2: BiquadFilterNode;
  tireGain: GainNode;
  roadFilter: BiquadFilterNode;
  roadGain: GainNode;
}

const REFERENCE_DISTANCE = 12;
const MAX_DISTANCE = 90;
const REMOTE_MASTER_GAIN = 0.45;

export class RemoteEngineAudio {
  status: "off" | "starting" | "running" | "error" = "off";
  muted = false;

  private readonly engineId: string;
  private readonly vehicleId: string;
  private context: AudioContext | null = null;
  private noiseSource: AudioBufferSourceNode | null = null;
  private voices = new Map<number, Voice>();
  private starting: Promise<void> | null = null;
  private startingVoices = new Set<number>();

  constructor(options: RemoteAudioOptions = {}) {
    this.engineId = options.engine ?? "rotary2";
    this.vehicleId = options.vehicle ?? "sports";
  }

  get voiceCount(): number {
    return this.voices.size;
  }

  async start(): Promise<void> {
    if (this.status === "running") {
      await this.context?.resume();
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
    if (!voice) {
      if (!this.startingVoices.has(slot)) {
        void this.createVoice(slot);
      }
      return;
    }
    if (!voice.ready) {
      return;
    }
    const target = this.muted ? 0 : distanceGain(distance);
    voice.gain.gain.setTargetAtTime(target, this.context.currentTime, 0.06);
    voice.sim.updateExternal(Math.max(0.001, Math.min(deltaSeconds, 0.1)), {
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
    voice.sim.dispose();
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
    if (this.context?.state === "running") {
      void this.context.suspend();
    }
  }

  async resume(): Promise<void> {
    if (this.status === "running" && this.context) {
      await this.context.resume();
    }
  }

  dispose(): void {
    for (const slot of [...this.voices.keys()]) {
      this.remove(slot);
    }
    this.voices.clear();
    this.startingVoices.clear();
    this.status = "off";
    this.noiseSource?.stop();
    this.noiseSource?.disconnect();
    this.noiseSource = null;
    void this.context?.close();
    this.context = null;
  }

  private async startInternal(): Promise<void> {
    this.status = "starting";
    try {
      const context = new AudioContext({ latencyHint: "interactive" });
      this.context = context;
      EngineSim.workletURL = new URL(
        `../audio/combustion-worklet.js?v=${encodeURIComponent(APP_VERSION)}`,
        import.meta.url,
      );
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
      await context.resume();
      this.status = "running";
    } catch (error) {
      this.status = "error";
      console.warn("remote engine audio unavailable", error);
      await this.context?.close();
      this.context = null;
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

  private async createVoice(slot: number): Promise<void> {
    const context = this.context;
    if (!context || this.voices.has(slot)) {
      return;
    }
    this.startingVoices.add(slot);
    const sim = new EngineSim(context, {
      engine: this.engineId,
      vehicle: this.vehicleId,
      volume: 1,
    });
    sim.setAutoShift(false);
    sim.setGear(1);
    sim.setMix({ transmission: 0 });
    const gain = context.createGain();
    gain.gain.value = 0;
    sim.connect(gain);
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
    const voice: Voice = {
      sim,
      gain,
      ready: false,
      prevRpm: 900,
      tireBand1,
      tireBand2,
      tireGain,
      roadFilter,
      roadGain,
    };
    this.voices.set(slot, voice);
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

function distanceGain(distance: number): number {
  if (!Number.isFinite(distance) || distance >= MAX_DISTANCE) {
    return 0;
  }
  const falloff = Math.max(0, 1 - distance / MAX_DISTANCE);
  const inverse = 1 / (1 + (distance / REFERENCE_DISTANCE) ** 2);
  return REMOTE_MASTER_GAIN * inverse * falloff;
}
