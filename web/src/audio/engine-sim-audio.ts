import { APP_VERSION } from "../version.generated.ts";
import { EngineSim } from "./engine-sim/engine-sim.js";

export type AudioStatus = "off" | "starting" | "running" | "error";

const AUDIO_RPM_TAU_S = 0.06;
const AUDIO_LOAD_ATTACK_S = 0.04;
const AUDIO_LOAD_RELEASE_S = 0.32;
const MAX_AUDIO_RPM_SLEW = 12_000;
const MAX_AUDIO_RPM = 9_000;

export interface TireAudioWheel {
  slipRatio: number;
  slipAngle: number;
  load: number;
  contact: boolean;
}

export class EngineSimAudio {
  status: AudioStatus = "off";
  error = "";
  muted = false;

  private context: AudioContext | null = null;
  private sim: EngineSim | null = null;
  private gain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private samples: Float32Array | null = null;
  private tireSource: AudioBufferSourceNode | null = null;
  private tireBand1: BiquadFilterNode | null = null;
  private tireBand2: BiquadFilterNode | null = null;
  private tireGain: GainNode | null = null;
  private roadAsphalt: BiquadFilterNode | null = null;
  private roadConcrete: BiquadFilterNode | null = null;
  private roadKerb: BiquadFilterNode | null = null;
  private roadGrass: BiquadFilterNode | null = null;
  private roadOuterGrit: BiquadFilterNode | null = null;
  private roadGrassRustle: BiquadFilterNode | null = null;
  private roadAsphaltGain: GainNode | null = null;
  private roadConcreteGain: GainNode | null = null;
  private roadKerbGain: GainNode | null = null;
  private roadGrassGain: GainNode | null = null;
  private roadOuterGritGain: GainNode | null = null;
  private roadGrassRustleGain: GainNode | null = null;
  private roadSurface = {
    asphalt: 0,
    concrete: 0,
    kerb: 0,
    grass: 0,
    gain: 0,
  };
  private tireSlip = 0;
  private starting: Promise<void> | null = null;
  private started = false;
  private audioRpm = 900;
  private audioLoad = 0;
  private lastRpm = 900;
  private lastGear = 1;
  private shiftTimer = 0;
  private hasRpm = false;
  private hasLoad = false;

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

      const noise = context.createBuffer(1, context.sampleRate * 2, context.sampleRate);
      const data = noise.getChannelData(0);
      for (let index = 0; index < data.length; index += 1) {
        data[index] = Math.random() * 2 - 1;
      }
      const source = context.createBufferSource();
      source.buffer = noise;
      source.loop = true;
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
      source.connect(tireBand1);
      source.connect(tireBand2);
      tireBand1.connect(tireGain);
      tireBand2.connect(tireGain);
      tireGain.connect(gain);

      const roadAsphalt = context.createBiquadFilter();
      roadAsphalt.type = "lowpass";
      roadAsphalt.frequency.value = 850;
      roadAsphalt.Q.value = 0.7;
      const roadConcrete = context.createBiquadFilter();
      roadConcrete.type = "lowpass";
      roadConcrete.frequency.value = 260;
      roadConcrete.Q.value = 0.7;
      const roadOuterGrit = context.createBiquadFilter();
      roadOuterGrit.type = "bandpass";
      roadOuterGrit.frequency.value = 900;
      roadOuterGrit.Q.value = 0.8;
      const roadKerb = context.createBiquadFilter();
      roadKerb.type = "bandpass";
      roadKerb.frequency.value = 620;
      roadKerb.Q.value = 2.2;
      const roadGrass = context.createBiquadFilter();
      roadGrass.type = "lowpass";
      roadGrass.frequency.value = 260;
      roadGrass.Q.value = 0.55;
      const roadGrassRustle = context.createBiquadFilter();
      roadGrassRustle.type = "bandpass";
      roadGrassRustle.frequency.value = 1_650;
      roadGrassRustle.Q.value = 0.9;
      const roadAsphaltGain = context.createGain();
      const roadConcreteGain = context.createGain();
      const roadKerbGain = context.createGain();
      const roadGrassGain = context.createGain();
      const roadOuterGritGain = context.createGain();
      const roadGrassRustleGain = context.createGain();
      roadAsphaltGain.gain.value = 0;
      roadConcreteGain.gain.value = 0;
      roadKerbGain.gain.value = 0;
      roadGrassGain.gain.value = 0;
      roadOuterGritGain.gain.value = 0;
      roadGrassRustleGain.gain.value = 0;
      source.connect(roadAsphalt);
      source.connect(roadConcrete);
      source.connect(roadOuterGrit);
      source.connect(roadKerb);
      source.connect(roadGrass);
      source.connect(roadGrassRustle);
      roadAsphalt.connect(roadAsphaltGain);
      roadConcrete.connect(roadConcreteGain);
      roadOuterGrit.connect(roadOuterGritGain);
      roadKerb.connect(roadKerbGain);
      roadGrass.connect(roadGrassGain);
      roadGrassRustle.connect(roadGrassRustleGain);
      roadAsphaltGain.connect(gain);
      roadConcreteGain.connect(gain);
      roadOuterGritGain.connect(gain);
      roadKerbGain.connect(gain);
      roadGrassGain.connect(gain);
      roadGrassRustleGain.connect(gain);
      source.start();
      this.roadAsphalt = roadAsphalt;
      this.roadConcrete = roadConcrete;
      this.roadKerb = roadKerb;
      this.roadGrass = roadGrass;
      this.roadOuterGrit = roadOuterGrit;
      this.roadGrassRustle = roadGrassRustle;
      this.roadAsphaltGain = roadAsphaltGain;
      this.roadConcreteGain = roadConcreteGain;
      this.roadKerbGain = roadKerbGain;
      this.roadGrassGain = roadGrassGain;
      this.roadOuterGritGain = roadOuterGritGain;
      this.roadGrassRustleGain = roadGrassRustleGain;
      this.tireSource = source;
      this.tireBand1 = tireBand1;
      this.tireBand2 = tireBand2;
      this.tireGain = tireGain;
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
      this.tireSource?.stop();
      this.tireSource?.disconnect();
      this.tireBand1?.disconnect();
      this.tireBand2?.disconnect();
      this.tireGain?.disconnect();
      this.tireSource = null;
      this.tireBand1 = null;
      this.tireBand2 = null;
      this.tireGain = null;
      this.roadAsphalt = null;
      this.roadConcrete = null;
      this.roadKerb = null;
      this.roadGrass = null;
      this.roadOuterGrit = null;
      this.roadGrassRustle = null;
      this.roadAsphaltGain = null;
      this.roadConcreteGain = null;
      this.roadKerbGain = null;
      this.roadGrassGain = null;
      this.roadOuterGritGain = null;
      this.roadGrassRustleGain = null;
      this.sim?.dispose();
      this.sim = null;
      await this.context?.close();
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
    if (!this.started || !this.sim) {
      return;
    }
    try {
      const dt = Math.max(0.001, Math.min(deltaSeconds, 0.1));
      this.shiftTimer = Math.max(0, this.shiftTimer - dt);
      const gameGear = Math.round(gear);
      if (gameGear >= 1 && this.lastGear >= 1 && gameGear !== this.lastGear) {
        this.shiftTimer = 0.12;
        this.sim.playShift(this.lastGear, gameGear);
      }
      this.lastGear = gameGear;

      const targetRpm = Math.max(0, Math.min(MAX_AUDIO_RPM, rpm));
      if (!this.hasRpm) {
        this.audioRpm = targetRpm;
        this.lastRpm = targetRpm;
        this.hasRpm = true;
      } else {
        const tau = this.shiftTimer > 0 ? 0.015 : AUDIO_RPM_TAU_S;
        const alpha = 1 - Math.exp(-dt / tau);
        const smoothed = this.audioRpm + (targetRpm - this.audioRpm) * alpha;
        const maxStep = MAX_AUDIO_RPM_SLEW * (this.shiftTimer > 0 ? 4 : 1) * dt;
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
        const tau = targetLoad > this.audioLoad ? AUDIO_LOAD_ATTACK_S : AUDIO_LOAD_RELEASE_S;
        this.audioLoad +=
          (targetLoad - this.audioLoad) * (1 - Math.exp(-dt / tau));
      }
      this.sim.updateExternal(dt, {
        rpm: this.audioRpm,
        speed: speedMps,
        gear,
        throttle,
        brake,
        load: this.audioLoad,
        prevRpm: this.lastRpm,
      });
      this.lastRpm = this.audioRpm;
      this.updateTires(wheels, speedMps);
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      this.status = "error";
      console.warn("engine audio update failed", error);
    }
  }

  private updateTires(wheels: TireAudioWheel[], speedMps: number): void {
    if (!this.tireGain || !this.context || !this.tireBand1 || !this.tireBand2) {
      return;
    }
    let slip = 0;
    let load = 0;
    let asphalt = 0;
    let concrete = 0;
    let kerb = 0;
    let grass = 0;
    for (const wheel of wheels) {
      if (!wheel.contact) {
        continue;
      }
      const friction = wheel.surfaceFriction;
      if (friction < 0.5) {
        grass += 1;
      } else if (friction < 0.95) {
        kerb += 1;
      } else if (friction < 0.985) {
        asphalt += 1;
      } else {
        concrete += 1;
      }
      const lateral = Math.abs(wheel.slipAngle);
      const longitudinal = Math.abs(wheel.slipRatio);
      const skid = Math.max(
        Math.max(0, lateral - 0.09) * 1.4,
        Math.max(0, longitudinal - 0.14) * 1.0,
      );
      if (skid > slip) {
        slip = skid;
        load = wheel.load;
      }
    }
    this.tireSlip = slip;
    const loadFactor = Math.max(0.2, Math.min(1, load / 4_500));
    const speedFactor = Math.max(0.35, Math.min(1, 0.35 + speedMps / 8));
    const surfaceScreech = grass > asphalt && grass > concrete ? 0.15 : 1;
    const squeal =
      Math.max(0, Math.min(1, slip * 4.2)) *
      loadFactor *
      speedFactor *
      surfaceScreech;
    const now = this.context.currentTime;
    this.tireGain.gain.setTargetAtTime(squeal * 0.9, now, 0.03);
    this.tireBand1.frequency.setTargetAtTime(
      900 + 650 * Math.max(0, Math.min(1, slip)),
      now,
      0.05,
    );
    this.tireBand2.frequency.setTargetAtTime(
      2_000 + 900 * Math.max(0, Math.min(1, slip)),
      now,
      0.05,
    );

    const asphaltWeight = asphalt / 4;
    const concreteWeight = concrete / 4;
    const kerbWeight = kerb / 4;
    const grassWeight = grass / 4;
    const rolling = Math.max(0, Math.min(1, speedMps / 30));
    const grassRolling = Math.max(0, Math.min(1, speedMps / 12));
    const asphaltGain = rolling * 0.16 * asphaltWeight;
    const concreteGain = rolling * 0.34 * concreteWeight;
    const outerGritGain = rolling * 0.14 * concreteWeight;
    const kerbGain = rolling * 0.22 * kerbWeight;
    const grassGain = grassRolling * 0.42 * grassWeight;
    const grassRustleGain = grassRolling * 0.10 * grassWeight;
    this.roadSurface = {
      asphalt: asphaltWeight,
      concrete: concreteWeight,
      kerb: kerbWeight,
      grass: grassWeight,
      gain:
        asphaltGain +
        concreteGain +
        outerGritGain +
        kerbGain +
        grassGain +
        grassRustleGain,
    };
    this.roadAsphaltGain?.gain.setTargetAtTime(asphaltGain, now, 0.08);
    this.roadConcreteGain?.gain.setTargetAtTime(concreteGain, now, 0.08);
    this.roadOuterGritGain?.gain.setTargetAtTime(outerGritGain, now, 0.08);
    this.roadKerbGain?.gain.setTargetAtTime(kerbGain, now, 0.08);
    this.roadGrassGain?.gain.setTargetAtTime(grassGain, now, 0.08);
    this.roadGrassRustleGain?.gain.setTargetAtTime(grassRustleGain, now, 0.08);
  }

  tireState(): { gain: number; slip: number } {
    return {
      gain: this.tireGain?.gain.value ?? 0,
      slip: this.tireSlip,
    };
  }

  roadState(): {
    asphalt: number;
    concrete: number;
    kerb: number;
    grass: number;
    gain: number;
  } {
    return { ...this.roadSurface };
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
    this.tireSource?.stop();
    this.tireSource?.disconnect();
    this.tireBand1?.disconnect();
    this.tireBand2?.disconnect();
    this.tireGain?.disconnect();
    this.tireSource = null;
    this.tireBand1 = null;
    this.tireBand2 = null;
    this.tireGain = null;
    this.roadAsphalt?.disconnect();
    this.roadConcrete?.disconnect();
    this.roadKerb?.disconnect();
    this.roadGrass?.disconnect();
    this.roadOuterGrit?.disconnect();
    this.roadGrassRustle?.disconnect();
    this.roadAsphaltGain?.disconnect();
    this.roadConcreteGain?.disconnect();
    this.roadKerbGain?.disconnect();
    this.roadGrassGain?.disconnect();
    this.roadOuterGritGain?.disconnect();
    this.roadGrassRustleGain?.disconnect();
    this.roadAsphalt = null;
    this.roadConcrete = null;
    this.roadKerb = null;
    this.roadGrass = null;
    this.roadOuterGrit = null;
    this.roadGrassRustle = null;
    this.roadAsphaltGain = null;
    this.roadConcreteGain = null;
    this.roadKerbGain = null;
    this.roadGrassGain = null;
    this.roadOuterGritGain = null;
    this.roadGrassRustleGain = null;
    this.gain?.disconnect();
    this.analyser?.disconnect();
    this.gain = null;
    this.analyser = null;
    this.samples = null;
    void this.context?.close();
    this.context = null;
  }
}
