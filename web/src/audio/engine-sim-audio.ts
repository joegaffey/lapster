import { APP_VERSION } from "../version.generated.ts";
import { EngineSim } from "./engine-sim/engine-sim.js";
import {
  DEFAULT_SURFACE_TUNING,
  SurfaceAudio,
  type SurfaceTireWheel,
  type SurfaceTuning,
} from "./surface-audio.ts";

export type AudioStatus = "off" | "starting" | "running" | "error";

export interface AudioTuning extends SurfaceTuning {
  masterGain: number;
  transmissionMix: number;
  rpmTau: number;
  loadAttack: number;
  loadRelease: number;
  maxRpmSlew: number;
  maxAudioRpm: number;
}

const DEFAULT_AUDIO_TUNING: AudioTuning = {
  ...DEFAULT_SURFACE_TUNING,
  masterGain: 0.35,
  transmissionMix: 0,
  rpmTau: 0.06,
  loadAttack: 0.04,
  loadRelease: 0.32,
  maxRpmSlew: 12_000,
  maxAudioRpm: 9_000,
};

export type TireAudioWheel = SurfaceTireWheel;

export interface EngineSimAudioOptions {
  engine?: string;
  vehicle?: string;
  surfaceOnly?: boolean;
  context?: AudioContext;
}

export class EngineSimAudio {
  status: AudioStatus = "off";
  error = "";
  muted = false;
  tuning: AudioTuning = { ...DEFAULT_AUDIO_TUNING };

  private readonly engineId: string;
  private readonly vehicleId: string;
  private readonly surfaceOnly: boolean;
  private readonly sharedContext: AudioContext | null;
  private ownsContext = true;

  private context: AudioContext | null = null;
  private sim: EngineSim | null = null;
  private gain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private samples: Float32Array | null = null;
  private surface: SurfaceAudio | null = null;
  private starting: Promise<void> | null = null;
  private started = false;
  private audioRpm = 900;
  private audioLoad = 0;
  private lastRpm = 900;
  private lastGear = 1;
  private shiftTimer = 0;
  private hasRpm = false;
  private hasLoad = false;
  private impactNoise: AudioBuffer | null = null;
  private lastImpactTime = Number.NEGATIVE_INFINITY;

  constructor(options: EngineSimAudioOptions = {}) {
    this.engineId = options.engine ?? "rotary2";
    this.vehicleId = options.vehicle ?? "sports";
    this.surfaceOnly = options.surfaceOnly === true;
    this.sharedContext = options.context ?? null;
  }

  get contextState(): string {
    return this.context?.state ?? "closed";
  }

  /** The player context, exposed so remote voices can optionally share it. */
  get audioContext(): AudioContext | null {
    return this.context;
  }

  async start(): Promise<void> {
    if (this.started) {
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

  private async startInternal(): Promise<void> {
    this.status = "starting";
    this.error = "";
    try {
      const context = this.sharedContext ?? new AudioContext({ latencyHint: "interactive" });
      this.context = context;
      this.ownsContext = this.sharedContext === null;
      // Resume inside the same user gesture before any await. Mobile Chrome and
      // Samsung Internet can reject a resume that happens after other awaits.
      const resume = context.resume();
      let sim: EngineSim | null = null;
      const gain = context.createGain();
      const analyser = context.createAnalyser();
      analyser.fftSize = 1024;
      gain.gain.value = this.muted ? 0 : this.tuning.masterGain;
      if (!this.surfaceOnly) {
        EngineSim.workletURL = new URL(
          `../audio/combustion-worklet.js?v=${encodeURIComponent(APP_VERSION)}`,
          import.meta.url,
        );
        sim = new EngineSim(context, {
          engine: this.engineId,
          vehicle: this.vehicleId,
          volume: 1,
        });
        sim.setAutoShift(false);
        sim.setGear(1);
        sim.setMix({ transmission: this.tuning.transmissionMix });
        sim.connect(gain);
      }
      gain.connect(analyser);
      analyser.connect(context.destination);

      this.surface = new SurfaceAudio(context, gain, this.tuning);
      try {
        await resume;
      } catch (error) {
        throw new Error(
          `audio context could not resume: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
      if (sim) {
        await sim.start();
        this.sim = sim;
      }
      this.gain = gain;
      this.analyser = analyser;
      this.samples = new Float32Array(analyser.fftSize);
      this.started = true;
      this.status = "running";
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      this.status = "error";
      this.surface?.dispose();
      this.surface = null;
      this.sim?.dispose();
      this.sim = null;
      if (this.ownsContext) await this.context?.close();
      this.context = null;
      this.gain = null;
      this.analyser = null;
      throw error;
    }
  }

  update(
    deltaSeconds: number,
    throttle: number,
    brake: number,
    gear: number,
    rpm: number,
    speedMps: number,
    wheels: TireAudioWheel[] = [],
  ): void {
    if (!this.started) {
      return;
    }
    try {
      const dt = Math.max(0.001, Math.min(deltaSeconds, 0.1));
      const sim = this.sim;
      if (sim) {
        this.shiftTimer = Math.max(0, this.shiftTimer - dt);
        const gameGear = Math.round(gear);
        if (gameGear >= 1 && this.lastGear >= 1 && gameGear !== this.lastGear) {
          this.shiftTimer = 0.12;
          sim.playShift(this.lastGear, gameGear);
        }
        this.lastGear = gameGear;

        const targetRpm = Math.max(0, Math.min(this.tuning.maxAudioRpm, rpm));
        if (!this.hasRpm) {
          this.audioRpm = targetRpm;
          this.lastRpm = targetRpm;
          this.hasRpm = true;
        } else {
          const tau = this.shiftTimer > 0 ? 0.015 : this.tuning.rpmTau;
          const alpha = 1 - Math.exp(-dt / tau);
          const smoothed = this.audioRpm + (targetRpm - this.audioRpm) * alpha;
          const maxStep = this.tuning.maxRpmSlew * (this.shiftTimer > 0 ? 4 : 1) * dt;
          this.audioRpm = Math.max(
            this.audioRpm - maxStep,
            Math.min(this.audioRpm + maxStep, smoothed),
          );
        }
        const movingUnderPower = speedMps > 2 && rpm > 1_500 && throttle < 0.05;
        const targetLoad = Math.max(
          throttle,
          movingUnderPower ? 0.38 : 0,
        );
        if (!this.hasLoad) {
          this.audioLoad = targetLoad;
          this.hasLoad = true;
        } else {
          const tau =
            targetLoad > this.audioLoad
              ? this.tuning.loadAttack
              : this.tuning.loadRelease;
          this.audioLoad +=
            (targetLoad - this.audioLoad) * (1 - Math.exp(-dt / tau));
        }
        sim.updateExternal(dt, {
          rpm: this.audioRpm,
          speed: speedMps,
          gear,
          throttle,
          brake,
          load: this.audioLoad,
          prevRpm: this.lastRpm,
        });
        this.lastRpm = this.audioRpm;
      }
      this.surface?.update(wheels, speedMps, this.tuning);
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      this.status = "error";
      console.warn("engine audio update failed", error);
    }
  }

  tireState(): { gain: number; slip: number } {
    return this.surface?.tireState() ?? { gain: 0, slip: 0 };
  }

  roadState(): {
    asphalt: number;
    concrete: number;
    kerb: number;
    grass: number;
    dirt: number;
    gain: number;
  } {
    return this.surface?.roadState() ?? {
      asphalt: 0,
      concrete: 0,
      kerb: 0,
      grass: 0,
      dirt: 0,
      gain: 0,
    };
  }

  /**
   * Play a short, bounded car-to-car impact burst.
   *
   * No-op while audio is off, muted, or not running. The noise buffer is built
   * once and shared; only small per-impact nodes are allocated, and a short
   * global cooldown keeps simultaneous contacts bounded.
   */
  playImpact(speed: number): void {
    if (
      this.muted ||
      !this.started ||
      !this.context ||
      !this.gain ||
      !Number.isFinite(speed)
    ) {
      return;
    }
    const intensity = Math.max(0, Math.min(1, speed / 20));
    if (intensity < 0.04) {
      return;
    }
    const context = this.context;
    const now = context.currentTime;
    if (now - this.lastImpactTime < 0.05) {
      return;
    }
    this.lastImpactTime = now;
    try {
      if (!this.impactNoise) {
        const length = Math.max(1, Math.floor(context.sampleRate * 0.2));
        const buffer = context.createBuffer(1, length, context.sampleRate);
        const data = buffer.getChannelData(0);
        let low = 0;
        for (let index = 0; index < length; index += 1) {
          const white = Math.random() * 2 - 1;
          low += 0.35 * (white - low);
          const decay = 1 - index / length;
          data[index] = low * decay * decay;
        }
        this.impactNoise = buffer;
      }
      const source = context.createBufferSource();
      source.buffer = this.impactNoise;
      source.playbackRate.value = 0.7 + intensity * 0.6;
      const filter = context.createBiquadFilter();
      filter.type = "lowpass";
      filter.frequency.value = 500 + 1_500 * intensity;
      filter.Q.value = 0.7;
      const gain = context.createGain();
      const peak = 0.18 + 0.5 * intensity;
      const duration = 0.08 + 0.12 * intensity;
      gain.gain.setValueAtTime(0.0001, now);
      gain.gain.exponentialRampToValueAtTime(peak, now + 0.006);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);
      source.connect(filter);
      filter.connect(gain);
      gain.connect(this.gain);
      source.onended = () => {
        source.disconnect();
        filter.disconnect();
        gain.disconnect();
      };
      source.start(now);
      source.stop(now + duration + 0.02);
    } catch {
      // Impact audio is best-effort and must never break Drive.
    }
  }

  state(): {
    rpm: number;
    speedKph: number;
    gear: number;
    clutch: number;
    clutchSlip: number;
  } | null {
    const state = this.sim?.getState();
    if (!state) {
      return null;
    }
    return {
      rpm: state.rpm,
      speedKph: state.speedKmh,
      gear: state.gear,
      clutch: state.clutch,
      clutchSlip: state.clutchSlip,
    };
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    if (!this.context || !this.gain) {
      return;
    }
    this.gain.gain.setTargetAtTime(
      muted ? 0 : this.tuning.masterGain,
      this.context.currentTime,
      0.02,
    );
  }

  toggleMuted(): boolean {
    this.setMuted(!this.muted);
    return this.muted;
  }

  applyTuning(patch: Partial<AudioTuning>): void {
    this.tuning = { ...this.tuning, ...patch };
    if (!this.context || !this.gain) {
      return;
    }
    this.gain.gain.setTargetAtTime(
      this.muted ? 0 : this.tuning.masterGain,
      this.context.currentTime,
      0.02,
    );
    this.sim?.setMix({ transmission: this.tuning.transmissionMix });
  }

  async restart(): Promise<void> {
    if (!this.started) {
      return;
    }
    const muted = this.muted;
    this.dispose();
    this.muted = muted;
    await this.start();
  }

  suspend(): void {
    if (this.context?.state === "running") {
      void this.context.suspend();
    }
  }

  async resume(): Promise<void> {
    if (this.started && this.context) {
      await this.context.resume();
    }
  }

  level(): number {
    if (!this.analyser || !this.samples) {
      return 0;
    }
    this.analyser.getFloatTimeDomainData(this.samples);
    let sum = 0;
    for (const sample of this.samples) {
      sum += sample * sample;
    }
    return Math.sqrt(sum / this.samples.length);
  }

  dispose(): void {
    this.started = false;
    this.status = "off";
    this.impactNoise = null;
    this.lastImpactTime = Number.NEGATIVE_INFINITY;
    this.sim?.dispose();
    this.sim = null;
    this.surface?.dispose();
    this.surface = null;
    this.gain?.disconnect();
    this.analyser?.disconnect();
    this.gain = null;
    this.analyser = null;
    this.samples = null;
    if (this.ownsContext) void this.context?.close();
    this.context = null;
  }
}
