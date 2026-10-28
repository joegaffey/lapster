import { readFfb } from "../sim/sab.ts";
import { WebHIDWheel } from "../vendor/webhid-wheel/webhid-wheel.js";
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

export interface WheelFfbStatus {
  available: boolean;
  connected: boolean;
  name: string;
  parser: string;
  ffb: boolean;
  error: string;
  steering: number;
  base: number;
  output: number;
  frontSlip: number;
  rearSlip: number;
}

export class WheelFfb {
  readonly wheel: WebHIDWheel;
  private loop: number | null = null;
  private steeringValue = 0;
  private steeringAt = 0;
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
  private testTorque = 0;
  private testUntil = 0;
  private stopped = true;
  private invertSteering = false;
  private manualStop = false;
  private readonly ffbSab: SharedArrayBuffer | null;
  error = "";

  constructor(ffbSab?: SharedArrayBuffer) {
    this.ffbSab = ffbSab ?? null;
    this.wheel = new WebHIDWheel({
      maxRotation: 900,
      ffbStrength: 0.5,
      ffbInvert: false,
      deadzone: 0.02,
    });
    this.wheel.on("input", (state) => {
      const rotation = state.rotation ?? 0;
      const half = this.wheel.maxRotation / 2;
      const normalized = clamp(rotation / Math.max(1, half), -1, 1);
      this.steeringValue = this.invertSteering ? -normalized : normalized;
      this.steeringAt = performance.now();
    });
    this.wheel.on("disconnect", () => {
      this.steeringValue = 0;
      this.stop();
    });
    this.wheel.on("error", (error) => {
      this.error = error instanceof Error ? error.message : String(error);
    });
  }

  get available(): boolean {
    return (
      typeof navigator !== "undefined" &&
      typeof (navigator as any).hid?.requestDevice === "function"
    );
  }

  get connected(): boolean {
    return this.wheel.isConnected;
  }

  get steering(): number {
    if (!this.connected || performance.now() - this.steeringAt > STEERING_STALE_MS) {
      return 0;
    }
    return this.steeringValue;
  }

  get ffbAvailable(): boolean {
    return this.wheel.ffbCaps?.constant === true;
  }

  status(): WheelFfbStatus {
    return {
      available: this.available,
      connected: this.connected,
      name: this.wheel.name ?? "",
      parser: this.wheel.parserName ?? "",
      ffb: this.ffbAvailable,
      error: this.error,
      steering: this.steering,
      base: this.lastBase,
      output: this.lastOutput,
      frontSlip: this.frontSlip,
      rearSlip: this.rearSlip,
    };
  }

  async connect(): Promise<void> {
    this.error = "";
    try {
      await this.wheel.connect();
      if (this.connected) {
        this.stopped = true;
        this.manualStop = false;
        this.startLoop();
      }
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    this.stopLoop();
    await this.wheel.disconnect();
    this.steeringValue = 0;
    this.torque = 0;
    this.manualStop = false;
  }

  setStrength(strength: number): void {
    this.wheel.ffbStrength = clamp(strength, 0, 1);
  }

  setInvertForce(invert: boolean): void {
    this.wheel.ffbInvert = invert;
  }

  setInvertSteering(invert: boolean): void {
    this.invertSteering = invert;
    this.steeringValue = -this.steeringValue;
  }

  setDeadzone(deadzone: number): void {
    this.wheel.deadzone = clamp(deadzone, 0, 0.5);
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
    if (this.ffbSab || performance.now() < this.testUntil) {
      return;
    }
    this.baseTorque = clamp(torqueNm / TORQUE_REFERENCE_NM, -1, 1);
    this.torqueAt = performance.now();
  }

  testForce(value: number): void {
    if (!this.wheel.ffb?.supported.constant) {
      return;
    }
    this.manualStop = false;
    this.testTorque = clamp(value, -1, 1);
    this.testUntil = performance.now() + 1500;
  }

  stop(latch = false): void {
    if (latch) {
      this.manualStop = true;
    }
    this.stopped = true;
    this.testUntil = 0;
    this.lastSent = Number.NaN;
    this.lastOutput = 0;
    if (this.wheel.ffb) {
      void this.wheel.setConstantForce(0).catch(() => undefined);
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
    if (
      this.manualStop ||
      !this.connected ||
      !this.wheel.ffb?.supported.constant
    ) {
      return;
    }
    const now = performance.now();
    let output = 0;
    if (now < this.testUntil) {
      output = this.testTorque;
      this.lastBase = output;
      this.torqueAt = now;
    } else {
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
      output = computeFfbOutput(
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
    }
    this.stopped = false;
    this.lastOutput = output;
    if (output === this.lastSent && now - this.lastSentAt < 20) {
      return;
    }
    this.lastSent = output;
    this.lastSentAt = now;
    void this.wheel.setConstantForceFast(output).catch((error) => {
      this.error = error instanceof Error ? error.message : String(error);
    });
  }
}
