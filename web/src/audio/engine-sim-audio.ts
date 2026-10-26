import { APP_VERSION } from "../version.generated.ts";
import { EngineSim } from "./engine-sim/engine-sim.js";

export type AudioStatus = "off" | "starting" | "running" | "error";

export class EngineSimAudio {
  status: AudioStatus = "off";
  error = "";
  muted = false;

  private context: AudioContext | null = null;
  private sim: EngineSim | null = null;
  private gain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private samples: Float32Array | null = null;
  private starting: Promise<void> | null = null;
  private started = false;
  private lastGear = 1;

  get contextState(): string {
    return this.context?.state ?? "closed";
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
      const context = new AudioContext({ latencyHint: "interactive" });
      this.context = context;
      EngineSim.workletURL = new URL(
        `../audio/combustion-worklet.js?v=${encodeURIComponent(APP_VERSION)}`,
        import.meta.url,
      );
      const sim = new EngineSim(context, {
        engine: "rotary2",
        vehicle: "sports",
        volume: 1,
      });
      sim.setAutoShift(false);
      sim.setGear(1);
      const gain = context.createGain();
      const analyser = context.createAnalyser();
      analyser.fftSize = 1024;
      gain.gain.value = this.muted ? 0 : 0.35;
      sim.connect(gain);
      gain.connect(analyser);
      analyser.connect(context.destination);
      await sim.start();
      this.sim = sim;
      this.gain = gain;
      this.analyser = analyser;
      this.samples = new Float32Array(analyser.fftSize);
      this.started = true;
      this.status = "running";
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      this.status = "error";
      this.sim?.dispose();
      this.sim = null;
      await this.context?.close();
      this.context = null;
      this.gain = null;
      this.analyser = null;
      throw error;
    }
  }

  update(deltaSeconds: number, throttle: number, brake: number, gear: number): void {
    if (!this.started || !this.sim) {
      return;
    }
    try {
      this.sim.setThrottle(throttle);
      this.sim.setBrake(brake);
      const targetGear = Math.max(1, Math.min(6, gear));
      if (targetGear !== this.lastGear) {
        this.sim.setGear(targetGear);
        this.lastGear = targetGear;
      }
      this.sim.update(deltaSeconds);
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      this.status = "error";
      console.warn("engine audio update failed", error);
    }
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    if (!this.context || !this.gain) {
      return;
    }
    this.gain.gain.setTargetAtTime(
      muted ? 0 : 0.35,
      this.context.currentTime,
      0.02,
    );
  }

  toggleMuted(): boolean {
    this.setMuted(!this.muted);
    return this.muted;
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
    this.sim?.dispose();
    this.sim = null;
    this.gain?.disconnect();
    this.analyser?.disconnect();
    this.gain = null;
    this.analyser = null;
    this.samples = null;
    void this.context?.close();
    this.context = null;
  }
}
