import {
  HEADER_FIELDS,
  SNAPSHOT_COUNT,
  SNAPSHOT_FIELDS,
  SNAPSHOT_STATE,
  SNAPSHOT_STATE_FLOATS,
  snapshotSlotOffset,
} from "../types/asset-layout.ts";

declare const sampleRate: number;

declare abstract class AudioWorkletProcessor {
  readonly port: MessagePort;
  constructor(options?: unknown);
  abstract process(
    inputs: Float32Array[][],
    outputs: Float32Array[][],
    parameters: Record<string, Float32Array>,
  ): boolean;
}

declare function registerProcessor(
  name: string,
  processor: new (options?: unknown) => AudioWorkletProcessor,
): void;

interface AudioProfile {
  pulsesPerRev: number;
  idleRpm: number;
  redlineRpm: number;
}

const TAU = Math.PI * 2;

class SnapshotReader {
  private readonly header: Int32Array;
  private readonly sequences: Int32Array[];
  private readonly slots: Float32Array[];
  readonly state = new Float32Array(SNAPSHOT_STATE_FLOATS);
  hasSnapshot = false;

  constructor(sab: SharedArrayBuffer) {
    this.header = new Int32Array(sab, 0, HEADER_FIELDS.latestSlot / 4 + 1);
    this.sequences = [];
    this.slots = [];
    for (let index = 0; index < SNAPSHOT_COUNT; index += 1) {
      const offset = snapshotSlotOffset(index);
      this.sequences.push(new Int32Array(sab, offset, 1));
      this.slots.push(
        new Float32Array(
          sab,
          offset + SNAPSHOT_FIELDS.state,
          SNAPSHOT_STATE_FLOATS,
        ),
      );
    }
  }

  read(): void {
    const latest = Atomics.load(this.header, HEADER_FIELDS.latestSlot / 4);
    if (latest < 0 || latest >= SNAPSHOT_COUNT) {
      return;
    }
    const sequence = this.sequences[latest];
    const before = Atomics.load(sequence, 0);
    if (before & 1) {
      return;
    }
    this.state.set(this.slots[latest]);
    const after = Atomics.load(sequence, 0);
    if (before === after) {
      this.hasSnapshot = true;
    }
  }
}

class EngineProcessor extends AudioWorkletProcessor {
  private readonly reader: SnapshotReader | null;
  private readonly profile: AudioProfile;
  private readonly harmonicPhase = new Float32Array(10);
  private phase = 0;
  private noiseState = 0x12345678;
  private engineNoise = 0;
  private tireNoise = 0;
  private windNoise = 0;
  private roadNoise = 0;
  private rpm = 0;
  private throttle = 0;
  private speed = 0;
  private slip = 0;
  private contact = 0;

  constructor(options?: { processorOptions?: { sab?: SharedArrayBuffer; profile?: AudioProfile } }) {
    super(options);
    const processorOptions = options?.processorOptions;
    this.reader = processorOptions?.sab ? new SnapshotReader(processorOptions.sab) : null;
    this.profile = processorOptions?.profile ?? {
      pulsesPerRev: 2,
      idleRpm: 900,
      redlineRpm: 8000,
    };
    for (let index = 0; index < this.harmonicPhase.length; index += 1) {
      this.harmonicPhase[index] = this.noise() * TAU;
    }
  }

  private noise(): number {
    this.noiseState = (Math.imul(this.noiseState, 1664525) + 1013904223) >>> 0;
    return (this.noiseState / 0xffffffff) * 2 - 1;
  }

  private smooth(current: number, target: number, alpha: number): number {
    return current + (target - current) * alpha;
  }

  process(
    _inputs: Float32Array[][],
    outputs: Float32Array[][],
    _parameters: Record<string, Float32Array>,
  ): boolean {
    const channels = outputs[0];
    if (!channels || channels.length === 0) {
      return true;
    }
    const left = channels[0];
    const right = channels[1] ?? left;

    if (this.reader) {
      this.reader.read();
    }
    const state = this.reader?.state;
    const alpha = 1 - Math.exp(-(left.length / sampleRate) / 0.035);
    if (state && this.reader?.hasSnapshot) {
      const targetRpm = state[SNAPSHOT_STATE.engineRpm];
      const targetThrottle = state[SNAPSHOT_STATE.throttle];
      const vx = state[SNAPSHOT_STATE.velocity];
      const vy = state[SNAPSHOT_STATE.velocity + 1];
      const vz = state[SNAPSHOT_STATE.velocity + 2];
      const targetSpeed = Math.sqrt(vx * vx + vy * vy + vz * vz);
      let targetSlip = 0;
      let targetContact = 0;
      for (let wheel = 0; wheel < 4; wheel += 1) {
        const slipRatio = Math.abs(state[SNAPSHOT_STATE.slipRatio + wheel]);
        const slipAngle = Math.abs(state[SNAPSHOT_STATE.slipAngle + wheel]);
        targetSlip = Math.max(targetSlip, slipRatio * 0.18 + slipAngle * 0.35);
        targetContact += state[SNAPSHOT_STATE.contactFlags + wheel] > 0.5 ? 1 : 0;
      }
      this.rpm = this.smooth(this.rpm, targetRpm, alpha);
      this.throttle = this.smooth(this.throttle, targetThrottle, alpha);
      this.speed = this.smooth(this.speed, targetSpeed, alpha);
      this.slip = this.smooth(this.slip, targetSlip, alpha);
      this.contact = this.smooth(this.contact, targetContact / 4, alpha);
    }

    const firing = (Math.max(this.rpm, this.profile.idleRpm) / 60) * this.profile.pulsesPerRev;
    const rpmNorm = Math.max(
      0,
      Math.min(
        1,
        (this.rpm - this.profile.idleRpm) /
          Math.max(1, this.profile.redlineRpm - this.profile.idleRpm),
      ),
    );
    const load = Math.max(0, Math.min(1, this.throttle));
    const limiter =
      this.rpm > this.profile.redlineRpm - 120 && load > 0.4
        ? Math.sin(this.phase * 0.5) > -0.35
          ? 1
          : 0.2
        : 1;
    const engineGain = (0.1 + 0.3 * load + 0.05 * rpmNorm) * limiter;
    const engineNoiseGain = 0.025 + 0.11 * load + 0.04 * rpmNorm;
    const tireGain = Math.min(0.45, this.slip * 0.7) * Math.min(1, this.speed / 8);
    const windGain = Math.min(0.16, (this.speed / 55) ** 2 * 0.16);
    const roadGain = Math.min(0.08, this.contact * Math.min(1, this.speed / 30) * 0.06);

    for (let frame = 0; frame < left.length; frame += 1) {
      this.phase += (TAU * firing) / sampleRate;
      if (this.phase >= TAU) {
        this.phase -= TAU;
      }

      let engine = 0;
      for (let harmonic = 1; harmonic <= this.harmonicPhase.length; harmonic += 1) {
        engine +=
          Math.sin(this.phase * harmonic + this.harmonicPhase[harmonic - 1]) /
          harmonic ** 1.25;
      }
      engine *= engineGain;

      const noise = this.noise();
      this.engineNoise += (noise - this.engineNoise) * 0.22;
      this.tireNoise += (noise - this.tireNoise) * 0.12;
      this.windNoise += (noise - this.windNoise) * 0.015;
      this.roadNoise += (noise - this.roadNoise) * 0.04;

      let sample =
        engine +
        this.engineNoise * engineNoiseGain +
        this.tireNoise * tireGain +
        this.windNoise * windGain +
        this.roadNoise * roadGain;
      sample = sample / (1 + Math.abs(sample));
      left[frame] = sample;
      right[frame] = sample;
    }
    return true;
  }
}

registerProcessor("lapster-engine", EngineProcessor);
