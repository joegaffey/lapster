import { readFfb } from "../sim/sab.ts";
import {
  WheelManager,
  type WheelManagerSession,
} from "../vendor/webhid-wheel/webhid-wheel.js";
import {
  computeFfbOutput,
  DEFAULT_FEEL,
  type WheelFeelSettings,
} from "./wheel-feel.ts";

const TORQUE_REFERENCE_NM = 20;
const FFB_INTERVAL_MS = 5;
const FFB_STALE_MS = 50;
const STEERING_STALE_MS = 250;

const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

const numberChannel = (channels: Record<string, number>, name: string) => {
  const value = channels[name];
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
};

export interface WheelFfbStatus {
  available: boolean;
  connected: boolean;
  name: string;
  parser: string;
  ffb: boolean;
  ffbTarget: string | null;
  ffbDevice: string;
  ffbInterfaces: number;
  ffbGain: number;
  error: string;
  steering: number;
  base: number;
  output: number;
  frontSlip: number;
  rearSlip: number;
  devices: number;
}

export class WheelFfb {
  readonly manager: WheelManager;
  private loop: number | null = null;
  private baseTorque = 0;
  private torqueAt = 0;
  private feel: WheelFeelSettings = { ...DEFAULT_FEEL };
  private speedMps = 0;
  private frontSlip = 0;
  private rearSlip = 0;
  private noise = 0;
  private lastBase = 0;
  private lastOutput = 0;
  private lastSent = Number.NaN;
  private lastSentAt = 0;
  private lastFfbTarget: string | null = null;
  private stopped = true;
  private manualStop = false;
  private readonly ffbSab: SharedArrayBuffer | null;
  error = "";

  constructor(ffbSab?: SharedArrayBuffer) {
    this.ffbSab = ffbSab ?? null;
    this.manager = new WheelManager({
      maxRotation: 900,
      ffbStrength: 0.5,
      ffbInvert: false,
      deadzone: 0.02,
      saturation: 1,
    });
    this.manager.setCalibration({ deadzone: 0.02, maxRotation: 900, saturation: 1 });

    const managerStop = this.manager.stop.bind(this.manager);
    this.manager.stop = () => {
      this.manualStop = true;
      this.stopped = true;
      this.lastSent = Number.NaN;
      this.lastOutput = 0;
      return managerStop();
    };
    const managerSetFfb = this.manager.setFfb.bind(this.manager);
    this.manager.setFfb = (ffb = {}) => {
      if (ffb.gain !== undefined || ffb.invert !== undefined) {
        this.manualStop = false;
      }
      return managerSetFfb(ffb);
    };

    this.manager.on("connect", () => {
      this.error = "";
      this.stopped = true;
      this.manualStop = false;
      this.startLoop();
    });
    this.manager.on("disconnect", () => {
      if (this.manager.isConnected) {
        return;
      }
      this.stopLoop();
      this.stopped = true;
      this.lastOutput = 0;
      this.lastBase = 0;
    });
    this.manager.on("error", (error) => {
      this.error = error instanceof Error ? error.message : String(error);
    });
    this.manager.on("setup-applied", () => {
      this.manualStop = false;
    });
  }

  get wheel() {
    return this.manager.wheel;
  }

  get available(): boolean {
    return (
      typeof navigator !== "undefined" &&
      typeof (navigator as any).hid?.requestDevice === "function"
    );
  }

  get connected(): boolean {
    return this.manager.isConnected;
  }

  get channels(): Record<string, number> {
    return this.manager.channels;
  }

  get hasSteering(): boolean {
    return this.steeringSession() !== null;
  }

  get steering(): number {
    const entry = this.steeringSession();
    if (!entry || !entry.session.isConnected) {
      return 0;
    }
    const timestamp = (entry.session as any).timestamp;
    if (typeof timestamp !== "number" || performance.now() - timestamp > STEERING_STALE_MS) {
      return 0;
    }
    return clamp(numberChannel(this.manager.channels, "steering"), -1, 1);
  }

  get ffbAvailable(): boolean {
    const entry = this.ffbSession();
    return !!(entry && entry.capabilities.ffb && entry.session.isConnected);
  }

  status(): WheelFfbStatus {
    const connected = this.manager.sessionList.filter(
      (entry) => entry.session && entry.session.isConnected,
    );
    const steering = this.steeringSession();
    const ffb = this.ffbSession();
    const ffbTarget = this.manager.getFfb();
    const primary = steering ?? connected[0] ?? null;
    return {
      available: this.available,
      connected: connected.length > 0,
      name: connected.map((entry) => entry.name).filter(Boolean).join(", "),
      parser: primary ? primary.capabilities.profile || primary.driver : "",
      ffb: this.ffbAvailable,
      ffbTarget: ffbTarget.device,
      ffbDevice: ffb ? (ffb.session as any).ffbDevice?.productName || "" : "",
      ffbInterfaces: ffb ? ffb.interfaces?.length ?? 0 : 0,
      ffbGain: ffbTarget.gain,
      error: this.error,
      steering: this.steering,
      base: this.lastBase,
      output: this.lastOutput,
      frontSlip: this.frontSlip,
      rearSlip: this.rearSlip,
      devices: connected.length,
    };
  }

  async connect(): Promise<void> {
    this.error = "";
    try {
      const session = await this.manager.connect();
      if (!session) {
        return;
      }
      this.stopped = true;
      this.manualStop = false;
      this.startLoop();
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    this.stopLoop();
    await this.manager.disconnectAll();
    this.manualStop = false;
    this.lastFfbTarget = null;
    this.lastOutput = 0;
    this.lastBase = 0;
  }

  setStrength(strength: number): void {
    this.manager.setFfb({ gain: clamp(strength, 0, 1) });
  }

  setInvertForce(invert: boolean): void {
    this.manager.setFfb({ invert });
  }

  setInvertSteering(invert: boolean): void {
    this.manager.setCalibration({ invert });
  }

  setDeadzone(deadzone: number): void {
    this.manager.setCalibration({ deadzone: clamp(deadzone, 0, 0.5) });
  }

  setSaturation(saturation: number): void {
    this.manager.setCalibration({ saturation: clamp(saturation, 0.05, 1) });
  }

  setFeel(feel: WheelFeelSettings): void {
    this.feel = {
      ...DEFAULT_FEEL,
      ...feel,
      shudderHz: clamp(feel.shudderHz, 4, 80),
      roughnessHz: clamp(feel.roughnessHz, 4, 120),
    };
  }

  updateTelemetry(telemetry: any): void {
    this.speedMps = Number(telemetry?.speedMps ?? 0);
    let frontSlip = 0;
    let rearSlip = 0;
    const wheels = Array.isArray(telemetry?.wheels) ? telemetry.wheels : [];
    for (let index = 0; index < wheels.length; index += 1) {
      const wheel = wheels[index];
      if (wheel?.contact === false) continue;
      const slip = Math.abs(Number(wheel?.slipAngle ?? 0));
      if (index < 2) frontSlip = Math.max(frontSlip, slip);
      else rearSlip = Math.max(rearSlip, slip);
    }
    this.frontSlip = frontSlip;
    this.rearSlip = rearSlip;
    this.setTorque(Number(telemetry?.steeringRackTorque ?? 0));
  }

  setTorque(torqueNm: number): void {
    if (this.ffbSab) {
      return;
    }
    this.baseTorque = clamp(torqueNm / TORQUE_REFERENCE_NM, -1, 1);
    this.torqueAt = performance.now();
  }

  stop(latch = false): void {
    if (latch) {
      this.manualStop = true;
    }
    this.stopped = true;
    this.lastSent = Number.NaN;
    this.lastOutput = 0;
    this.stopAllEffects();
  }

  private steeringSession(): WheelManagerSession | null {
    const id = this.manager.getCalibration().device;
    const entry = id ? this.manager.sessions.get(id) : null;
    return entry && entry.capabilities.steering ? entry : null;
  }

  private ffbSession(): WheelManagerSession | null {
    const id = this.manager.getFfb().device;
    const entry = id ? this.manager.sessions.get(id) : null;
    return entry && entry.capabilities.ffb ? entry : null;
  }

  private stopSession(id: string | null): void {
    if (!id) {
      return;
    }
    const entry = this.manager.sessions.get(id);
    const session = entry?.session as any;
    if (session && typeof session.stopEffects === "function") {
      void session.stopEffects().catch(() => undefined);
    }
  }

  private stopAllEffects(): void {
    for (const entry of this.manager.sessionList) {
      if (!entry.capabilities.ffb) continue;
      const session = entry.session as any;
      if (session && typeof session.stopEffects === "function") {
        void session.stopEffects().catch(() => undefined);
      }
    }
  }

  private startLoop(): void {
    this.stopLoop();
    this.loop = window.setInterval(() => this.tick(), FFB_INTERVAL_MS);
  }

  private stopLoop(): void {
    if (this.loop !== null) {
      window.clearInterval(this.loop);
      this.loop = null;
    }
  }

  private tick(): void {
    if (this.manualStop || !this.connected) {
      return;
    }
    const target = this.ffbSession();
    if (this.lastFfbTarget !== (target?.id ?? null)) {
      this.stopSession(this.lastFfbTarget);
      this.lastFfbTarget = target?.id ?? null;
    }
    if (!target) {
      return;
    }
    const now = performance.now();
    let base = this.baseTorque;
    if (this.ffbSab) {
      const ffb = readFfb(this.ffbSab);
      const ageSeconds = ffb ? Date.now() / 1000 - ffb.timestampSeconds : Infinity;
      if (!ffb || !ffb.active || ageSeconds > FFB_STALE_MS / 1000) {
        if (!this.stopped) {
          this.stop();
        }
        return;
      }
      base = clamp(ffb.torque, -1, 1);
      this.torqueAt = now;
    }
    if (now - this.torqueAt > FFB_STALE_MS) {
      if (!this.stopped) {
        this.stop();
      }
      return;
    }
    this.noise = this.noise * 0.65 + (Math.random() * 2 - 1) * 0.35;
    this.lastBase = base;
    const output = computeFfbOutput(
      base,
      {
        speedMps: this.speedMps,
        frontSlip: this.frontSlip,
        rearSlip: this.rearSlip,
      },
      this.feel,
      now / 1000,
      this.noise,
    );
    this.stopped = false;
    this.lastOutput = output;
    if (output === this.lastSent && now - this.lastSentAt < 20) {
      return;
    }
    this.lastSent = output;
    this.lastSentAt = now;
    void this.manager.constantFast(output).catch((error) => {
      this.error = error instanceof Error ? error.message : String(error);
    });
  }
}
