import { APP_VERSION } from "../version.generated.ts";
import { createSnapshotReader, writeInput, type InputState, type SnapshotReader } from "./sab.ts";

export interface PhysicsDiagnostics {
  tick: number;
  physicsHz: number;
  stepP50Ms: number;
  stepP99Ms: number;
  droppedTicks: number;
  catchupEvents: number;
}

interface Ack {
  type: "ack";
  id: number;
  tick?: number;
  setup?: string;
  error?: string;
}

export class PhysicsClient {
  readonly reader: SnapshotReader;
  diagnostics: PhysicsDiagnostics = {
    tick: 0,
    physicsHz: 0,
    stepP50Ms: 0,
    stepP99Ms: 0,
    droppedTicks: 0,
    catchupEvents: 0,
  };
  setup = "";
  onDiagnostics: ((diagnostics: PhysicsDiagnostics) => void) | null = null;
  onFatalError: ((message: string) => void) | null = null;

  private readonly worker: Worker;
  private readonly sab: SharedArrayBuffer;
  private readonly pending = new Map<number, { resolve: (ack: Ack) => void; reject: (error: Error) => void }>();
  private nextId = 1;
  private readyResolve: (() => void) | null = null;
  private readyReject: ((error: Error) => void) | null = null;
  private readyTimeout: ReturnType<typeof setTimeout> | null = null;

  private constructor(worker: Worker, sab: SharedArrayBuffer) {
    this.worker = worker;
    this.sab = sab;
    this.reader = createSnapshotReader(sab);
    this.worker.onmessage = (event) => this.handleMessage(event.data);
    this.worker.onerror = (event) => {
      const message = event.message || "physics worker failed";
      const error = new Error(message);
      if (this.readyReject) {
        if (this.readyTimeout !== null) {
          clearTimeout(this.readyTimeout);
          this.readyTimeout = null;
        }
        this.readyReject(error);
        this.readyResolve = null;
        this.readyReject = null;
      } else {
        this.onFatalError?.(message);
      }
      this.rejectAll(error);
    };
  }

  static async create(
    sab: SharedArrayBuffer,
    trackBytes: Uint8Array,
    carBytes: Uint8Array,
  ): Promise<PhysicsClient> {
    const url = new URL(`../worker/physics-worker.js?v=${encodeURIComponent(APP_VERSION)}`, import.meta.url);
    const worker = new Worker(url, { type: "module" });
    const client = new PhysicsClient(worker, sab);
    const trackCopy = trackBytes.slice();
    const carCopy = carBytes.slice();
    const ready = new Promise<void>((resolve, reject) => {
      client.readyResolve = resolve;
      client.readyReject = reject;
      client.readyTimeout = setTimeout(
        () => reject(new Error("physics worker timed out")),
        15_000,
      );
    });
    worker.postMessage(
      {
        type: "init",
        sab,
        trackBytes: trackCopy,
        carBytes: carCopy,
      },
      [trackCopy.buffer, carCopy.buffer],
    );
    try {
      await ready;
    } catch (error) {
      worker.terminate();
      throw error;
    }
    return client;
  }

  writeInput(input: Omit<InputState, "timestampSeconds">): void {
    writeInput(this.sab, { ...input, timestampSeconds: performance.now() / 1000 });
  }

  simulate(
    seconds: number,
    throttle: number,
    brake: number,
    steering: number,
    gear: number,
  ): Promise<number> {
    return this.command("simulate", { seconds, throttle, brake, steering, gear }).then(
      (ack) => ack.tick ?? 0,
    );
  }

  pause(): Promise<void> {
    return this.command("pause", {}).then(() => undefined);
  }

  resume(): Promise<void> {
    return this.command("resume", {}).then(() => undefined);
  }

  reset(): Promise<void> {
    return this.command("reset", {}).then(() => undefined);
  }

  applySetup(setup: string): void {
    void this.command("apply_setup", { setup });
  }

  resetSetup(): void {
    void this.command("reset_setup", {});
  }

  setAssists(assists: {
    tc: boolean;
    tcStrength: number;
    abs: boolean;
    esc: boolean;
    escStrength: number;
    steeringSensitivity: number;
  }): void {
    void this.command("set_assists", assists);
  }

  dispose(): void {
    void this.command("dispose", {});
    this.worker.terminate();
  }

  private command(type: string, payload: Record<string, unknown>): Promise<Ack> {
    const id = this.nextId;
    this.nextId += 1;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ type, id, ...payload });
    });
  }

  private handleMessage(data: any): void {
    if (data?.type === "ready") {
      if (this.readyTimeout !== null) {
        clearTimeout(this.readyTimeout);
        this.readyTimeout = null;
      }
      this.setup = data.setup ?? "";
      this.readyResolve?.();
      this.readyResolve = null;
      this.readyReject = null;
      return;
    }
    if (data?.type === "diagnostics") {
      this.diagnostics = {
        tick: data.tick ?? 0,
        physicsHz: data.physicsHz ?? 0,
        stepP50Ms: data.stepP50Ms ?? 0,
        stepP99Ms: data.stepP99Ms ?? 0,
        droppedTicks: data.droppedTicks ?? 0,
        catchupEvents: data.catchupEvents ?? 0,
      };
      this.onDiagnostics?.(this.diagnostics);
      return;
    }
    if (data?.type === "ack") {
      const pending = this.pending.get(data.id);
      if (!pending) {
        return;
      }
      this.pending.delete(data.id);
      if (data.error) {
        pending.reject(new Error(data.error));
      } else {
        if (typeof data.setup === "string") {
          this.setup = data.setup;
        }
        pending.resolve(data);
      }
      return;
    }
    if (data?.type === "error") {
      const error = new Error(data.message ?? "physics worker failed");
      if (this.readyReject) {
        if (this.readyTimeout !== null) {
          clearTimeout(this.readyTimeout);
          this.readyTimeout = null;
        }
        this.readyReject(error);
        this.readyResolve = null;
        this.readyReject = null;
      } else {
        this.onFatalError?.(error.message);
      }
    }
  }

  private rejectAll(error: Error): void {
    for (const { reject } of this.pending.values()) {
      reject(error);
    }
    this.pending.clear();
  }
}
