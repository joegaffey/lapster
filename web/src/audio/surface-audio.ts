export interface SurfaceTireWheel {
  slipRatio: number;
  slipAngle: number;
  load: number;
  contact: boolean;
  surface?: string;
}

export interface SurfaceTuning {
  tireLateralThreshold: number;
  tireLongitudinalThreshold: number;
  tireLateralScale: number;
  tireLongitudinalScale: number;
  tireLoadReference: number;
  tireSquealScale: number;
  tireGain: number;
}

export const DEFAULT_SURFACE_TUNING: SurfaceTuning = {
  tireLateralThreshold: 0.09,
  tireLongitudinalThreshold: 0.14,
  tireLateralScale: 1.4,
  tireLongitudinalScale: 1.0,
  tireLoadReference: 4_500,
  tireSquealScale: 4.2,
  tireGain: 0.9,
};

export class SurfaceAudio {
  private readonly context: AudioContext;
  readonly tuning: SurfaceTuning;

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
  private roadDirt: BiquadFilterNode | null = null;
  private roadAsphaltGain: GainNode | null = null;
  private roadConcreteGain: GainNode | null = null;
  private roadKerbGain: GainNode | null = null;
  private roadGrassGain: GainNode | null = null;
  private roadOuterGritGain: GainNode | null = null;
  private roadGrassRustleGain: GainNode | null = null;
  private roadDirtGain: GainNode | null = null;
  private outerTremolo: GainNode | null = null;
  private grassTremolo: GainNode | null = null;
  private dirtLfos: OscillatorNode[] = [];
  private dirtLfoGains: GainNode[] = [];
  private roadSurface = {
    asphalt: 0,
    concrete: 0,
    kerb: 0,
    grass: 0,
    dirt: 0,
    gain: 0,
  };
  private tireSlip = 0;

  constructor(
    context: AudioContext,
    destination: AudioNode,
    tuning = DEFAULT_SURFACE_TUNING,
  ) {
    this.context = context;
    this.tuning = tuning;

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
    tireGain.connect(destination);

    const roadAsphalt = context.createBiquadFilter();
    roadAsphalt.type = "lowpass";
    roadAsphalt.frequency.value = 850;
    roadAsphalt.Q.value = 0.7;
    const roadConcrete = context.createBiquadFilter();
    roadConcrete.type = "lowpass";
    roadConcrete.frequency.value = 320;
    roadConcrete.Q.value = 0.9;
    const roadOuterGrit = context.createBiquadFilter();
    roadOuterGrit.type = "bandpass";
    roadOuterGrit.frequency.value = 650;
    roadOuterGrit.Q.value = 0.8;
    const roadKerb = context.createBiquadFilter();
    roadKerb.type = "bandpass";
    roadKerb.frequency.value = 520;
    roadKerb.Q.value = 2.0;
    const roadGrass = context.createBiquadFilter();
    roadGrass.type = "lowpass";
    roadGrass.frequency.value = 220;
    roadGrass.Q.value = 0.7;
    const roadGrassRustle = context.createBiquadFilter();
    roadGrassRustle.type = "bandpass";
    roadGrassRustle.frequency.value = 600;
    roadGrassRustle.Q.value = 0.7;
    const roadDirt = context.createBiquadFilter();
    roadDirt.type = "lowpass";
    roadDirt.frequency.value = 320;
    roadDirt.Q.value = 0.9;
    const roadAsphaltGain = context.createGain();
    const roadConcreteGain = context.createGain();
    const roadKerbGain = context.createGain();
    const roadGrassGain = context.createGain();
    const roadOuterGritGain = context.createGain();
    const roadGrassRustleGain = context.createGain();
    const roadDirtGain = context.createGain();
    roadAsphaltGain.gain.value = 0;
    roadConcreteGain.gain.value = 0;
    roadKerbGain.gain.value = 0;
    roadGrassGain.gain.value = 0;
    roadOuterGritGain.gain.value = 0;
    roadGrassRustleGain.gain.value = 0;
    roadDirtGain.gain.value = 0;
    source.connect(roadAsphalt);
    source.connect(roadConcrete);
    source.connect(roadOuterGrit);
    source.connect(roadKerb);
    source.connect(roadGrass);
    source.connect(roadGrassRustle);
    source.connect(roadDirt);
    roadAsphalt.connect(roadAsphaltGain);
    roadConcrete.connect(roadConcreteGain);
    roadOuterGrit.connect(roadOuterGritGain);
    roadKerb.connect(roadKerbGain);
    roadGrass.connect(roadGrassGain);
    roadGrassRustle.connect(roadGrassRustleGain);
    roadDirt.connect(roadDirtGain);
    roadAsphaltGain.connect(destination);
    roadKerbGain.connect(destination);
    const outerTremolo = context.createGain();
    const grassTremolo = context.createGain();
    outerTremolo.gain.value = 1;
    grassTremolo.gain.value = 1;
    roadConcreteGain.connect(outerTremolo);
    roadOuterGritGain.connect(outerTremolo);
    roadGrassGain.connect(grassTremolo);
    roadGrassRustleGain.connect(grassTremolo);
    roadDirtGain.connect(grassTremolo);
    outerTremolo.connect(destination);
    grassTremolo.connect(destination);
    const dirtLfos: OscillatorNode[] = [];
    const dirtLfoGains: GainNode[] = [];
    for (const [frequency, depth, target] of [
      [6.3, 0.22, outerTremolo],
      [10.7, 0.14, outerTremolo],
      [4.7, 0.26, grassTremolo],
      [8.9, 0.12, grassTremolo],
    ] as Array<[number, number, GainNode]>) {
      const lfo = context.createOscillator();
      lfo.type = "sine";
      lfo.frequency.value = frequency;
      const depthGain = context.createGain();
      depthGain.gain.value = depth;
      lfo.connect(depthGain);
      depthGain.connect(target.gain);
      lfo.start();
      dirtLfos.push(lfo);
      dirtLfoGains.push(depthGain);
    }
    this.outerTremolo = outerTremolo;
    this.grassTremolo = grassTremolo;
    this.dirtLfos = dirtLfos;
    this.dirtLfoGains = dirtLfoGains;
    source.start();
    this.roadAsphalt = roadAsphalt;
    this.roadConcrete = roadConcrete;
    this.roadKerb = roadKerb;
    this.roadGrass = roadGrass;
    this.roadOuterGrit = roadOuterGrit;
    this.roadGrassRustle = roadGrassRustle;
    this.roadDirt = roadDirt;
    this.roadAsphaltGain = roadAsphaltGain;
    this.roadConcreteGain = roadConcreteGain;
    this.roadKerbGain = roadKerbGain;
    this.roadGrassGain = roadGrassGain;
    this.roadOuterGritGain = roadOuterGritGain;
    this.roadGrassRustleGain = roadGrassRustleGain;
    this.roadDirtGain = roadDirtGain;
    this.tireSource = source;
    this.tireBand1 = tireBand1;
    this.tireBand2 = tireBand2;
    this.tireGain = tireGain;
  }

  update(
    wheels: SurfaceTireWheel[],
    speedMps: number,
    tuning = this.tuning,
  ): void {
    if (!this.tireGain || !this.context || !this.tireBand1 || !this.tireBand2) {
      return;
    }
    let slip = 0;
    let load = 0;
    let asphalt = 0;
    let concrete = 0;
    let kerb = 0;
    let grass = 0;
    let dirt = 0;
    for (const wheel of wheels) {
      if (!wheel.contact) {
        continue;
      }
      const surface = (wheel.surface ?? "").toUpperCase();
      if (surface.includes("GRASS")) {
        grass += 1;
      } else if (surface.includes("DIRT")) {
        dirt += 1;
      } else if (surface.includes("KERB")) {
        kerb += 1;
      } else if (surface.includes("ANELLO")) {
        concrete += 1;
      } else {
        asphalt += 1;
      }
      const lateral = Math.abs(wheel.slipAngle);
      const longitudinal = Math.abs(wheel.slipRatio);
      const skid = Math.max(
        Math.max(0, lateral - tuning.tireLateralThreshold) *
          tuning.tireLateralScale,
        Math.max(0, longitudinal - tuning.tireLongitudinalThreshold) *
          tuning.tireLongitudinalScale,
      );
      if (skid > slip) {
        slip = skid;
        load = wheel.load;
      }
    }
    this.tireSlip = slip;
    const loadFactor = Math.max(
      0.2,
      Math.min(1, load / Math.max(1, tuning.tireLoadReference)),
    );
    const speedFactor = Math.max(0.35, Math.min(1, 0.35 + speedMps / 8));
    const loose = grass + dirt;
    const surfaceScreech = loose > asphalt && loose > concrete ? 0.15 : 1;
    const squeal =
      Math.max(0, Math.min(1, slip * tuning.tireSquealScale)) *
      loadFactor *
      speedFactor *
      surfaceScreech;
    const now = this.context.currentTime;
    this.tireGain.gain.setTargetAtTime(squeal * tuning.tireGain, now, 0.03);
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
    const dirtWeight = dirt / 4;
    const rolling = Math.max(0, Math.min(1, speedMps / 30));
    const grassRolling = Math.max(0, Math.min(1, speedMps / 12));
    const dirtRolling = Math.max(0, Math.min(1, speedMps / 14));
    const asphaltGain = rolling * 0.18 * asphaltWeight;
    const concreteGain = rolling * 2.2 * concreteWeight;
    const outerGritGain = rolling * 0.35 * concreteWeight;
    const kerbGain = rolling * 0.5 * kerbWeight;
    const grassGain = grassRolling * 1.5 * grassWeight;
    const grassRustleGain = grassRolling * 0.2 * grassWeight;
    const dirtGain = dirtRolling * 1.2 * dirtWeight;
    this.roadSurface = {
      asphalt: asphaltWeight,
      concrete: concreteWeight,
      kerb: kerbWeight,
      grass: grassWeight,
      dirt: dirtWeight,
      gain:
        asphaltGain +
        concreteGain +
        outerGritGain +
        kerbGain +
        grassGain +
        grassRustleGain +
        dirtGain,
    };
    this.roadAsphaltGain?.gain.setTargetAtTime(asphaltGain, now, 0.08);
    this.roadConcreteGain?.gain.setTargetAtTime(concreteGain, now, 0.08);
    this.roadOuterGritGain?.gain.setTargetAtTime(outerGritGain, now, 0.08);
    this.roadKerbGain?.gain.setTargetAtTime(kerbGain, now, 0.08);
    this.roadGrassGain?.gain.setTargetAtTime(grassGain, now, 0.08);
    this.roadGrassRustleGain?.gain.setTargetAtTime(grassRustleGain, now, 0.08);
    this.roadDirtGain?.gain.setTargetAtTime(dirtGain, now, 0.08);
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
    dirt: number;
    gain: number;
  } {
    return { ...this.roadSurface };
  }

  dispose(): void {
    try {
      this.tireSource?.stop();
    } catch {
      // Already stopped.
    }
    this.tireSource?.disconnect();
    this.tireBand1?.disconnect();
    this.tireBand2?.disconnect();
    this.tireGain?.disconnect();
    this.roadAsphalt?.disconnect();
    this.roadConcrete?.disconnect();
    this.roadKerb?.disconnect();
    this.roadGrass?.disconnect();
    this.roadOuterGrit?.disconnect();
    this.roadGrassRustle?.disconnect();
    this.roadDirt?.disconnect();
    this.roadAsphaltGain?.disconnect();
    this.roadConcreteGain?.disconnect();
    this.roadKerbGain?.disconnect();
    this.roadGrassGain?.disconnect();
    this.roadOuterGritGain?.disconnect();
    this.roadGrassRustleGain?.disconnect();
    this.roadDirtGain?.disconnect();
    for (const lfo of this.dirtLfos) {
      try {
        lfo.stop();
      } catch {
        // Already stopped.
      }
      lfo.disconnect();
    }
    for (const gain of this.dirtLfoGains) gain.disconnect();
    this.outerTremolo?.disconnect();
    this.grassTremolo?.disconnect();
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
    this.roadDirt = null;
    this.roadAsphaltGain = null;
    this.roadConcreteGain = null;
    this.roadKerbGain = null;
    this.roadGrassGain = null;
    this.roadOuterGritGain = null;
    this.roadGrassRustleGain = null;
    this.roadDirtGain = null;
    this.outerTremolo = null;
    this.grassTremolo = null;
    this.dirtLfos = [];
    this.dirtLfoGains = [];
  }
}
