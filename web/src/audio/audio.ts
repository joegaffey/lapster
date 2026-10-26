import { APP_VERSION } from "../version.generated.ts";

export interface AudioProfile {
  pulsesPerRev: number;
  idleRpm: number;
  redlineRpm: number;
}

export type AudioStatus = "off" | "starting" | "running" | "error";

export class DriveAudio {
  status: AudioStatus = "off";
  error = "";
  muted = false;

  private context: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  private gain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private samples: Float32Array | null = null;
  private starting: Promise<void> | null = null;
  private started = false;

  constructor(
    private readonly sab: SharedArrayBuffer,
    private readonly profile: AudioProfile,
  ) {}

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
      const worklet = new URL(
        `../audio/engine-worklet.js?v=${encodeURIComponent(APP_VERSION)}`,
        import.meta.url,
      );
      await context.audioWorklet.addModule(worklet);
      const node = new AudioWorkletNode(context, "lapster-engine", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [2],
        processorOptions: { sab: this.sab, profile: this.profile },
      });
      const gain = context.createGain();
      const analyser = context.createAnalyser();
      analyser.fftSize = 1024;
      gain.gain.value = this.muted ? 0 : 0.35;
      node.connect(gain);
      gain.connect(analyser);
      analyser.connect(context.destination);
      this.node = node;
      this.gain = gain;
      this.analyser = analyser;
      this.samples = new Float32Array(analyser.fftSize);
      await context.resume();
      this.started = true;
      this.status = "running";
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      this.status = "error";
      await this.context?.close();
      this.context = null;
      this.node = null;
      this.gain = null;
      this.analyser = null;
      throw error;
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
    this.node?.disconnect();
    this.gain?.disconnect();
    this.analyser?.disconnect();
    this.node = null;
    this.gain = null;
    this.analyser = null;
    this.samples = null;
    void this.context?.close();
    this.context = null;
  }
}
