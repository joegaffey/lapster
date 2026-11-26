import { APP_VERSION } from "../version.generated.ts";
import {
  isRaceStateView,
  parseRaceStateView,
  type RaceStateView,
} from "../contracts/race-state.ts";
import { createSnapshotReader, writeInput, type InputState, type SnapshotReader } from "./sab.ts";
import { parseContacts, type ContactEvent } from "./contacts.ts";

export type { ContactEvent } from "./contacts.ts";

export interface PhysicsDiagnostics {
  tick: number;
  physicsHz: number;
  stepP50Ms: number;
  stepP99Ms: number;
  droppedTicks: number;
  catchupEvents: number;
  aiCount: number;
}

interface Ack {
  type: "ack";
  id: number;
  tick?: number;
  setup?: string;
  error?: string;
}

function parseAiRaceState(value: unknown): RaceStateView {
  if (typeof value === "string") {
    try {
      return parseRaceStateView(value);
    } catch {
      return { available: false };
    }
  }
  if (isRaceStateView(value)) {
    return value;
  }
  return { available: false };
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
    aiCount: 0,
  };
  setup = "";
  onDiagnostics: ((diagnostics: PhysicsDiagnostics) => void) | null = null;
  onFatalError: ((message: string) => void) | null = null;
  onAiFrame: ((slot: number, frame: Float32Array, race: RaceStateView) => void) | null = null;
  onContact: ((contact: ContactEvent) => void) | null = null;

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
    playerCarBytes: Uint8Array,
    aiCarBytes: Uint8Array[] = [],
    aiPace = 1,
  ): Promise<PhysicsClient> {
    const url = new URL(`../worker/physics-worker.js?v=${encodeURIComponent(APP_VERSION)}`, import.meta.url);
    const worker = new Worker(url, { type: "module" });
    const client = new PhysicsClient(worker, sab);
    client.diagnostics.aiCount = aiCarBytes.length;
    const trackCopy = trackBytes.slice();
    const carCopy = playerCarBytes.slice();
    const aiCopies = aiCarBytes.map((bytes) => bytes.slice());
    const ready = new Promise<void>((resolve, reject) => {
      client.readyResolve = resolve;
      client.readyReject = reject;
      client.readyTimeout = setTimeout(
        () => reject(new Error("physics worker timed out")),
        15_000,
      );
    });
    // Structured-clone the packs instead of transferring their buffers.
    // Mobile browsers have rejected multi-buffer transfer lists for AI grids,
    // which used to fail worker startup (and, before audio was decoupled, left
    // the whole session silent).
    worker.postMessage({
      type: "init",
      sab,
      trackBytes: trackCopy,
      carBytes: carCopy,
      aiCarBytes: aiCopies,
      aiPace,
    });
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

  placeOnGrid(slot: number): Promise<void> {
    return this.command("place_on_grid", { slot }).then(() => undefined);
  }

  placeOnRaceGrid(slot: number): Promise<void> {
    return this.command("place_on_race_grid", { slot }).then(() => undefined);
  }

  startRace(laps: number): Promise<void> {
    return this.command("start_race", { laps }).then(() => undefined);
  }

  applySetup(setup: string): void {
    void this.command("apply_setup", { setup });
  }

  resetSetup(): void {
    void this.command("reset_setup", {});
  }

  applyPhysicsTuning(tuning: string): Promise<number> {
    return this.command("apply_physics_tuning", { tuning }).then(
      (ack) => ack.tick ?? 0,
    );
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
    if (data?.type === "ai_states") {
      const states = Array.isArray(data.states) ? data.states : [];
      for (const state of states) {
        const frame =
          state?.frame instanceof Float32Array
            ? state.frame
            : new Float32Array(state?.frame ?? []);
        this.onAiFrame?.(
          Number(state?.slot),
          frame,
          parseAiRaceState(state?.race),
        );
      }
      return;
    }
    if (data?.type === "ai_state") {
      const frame =
        data.frame instanceof Float32Array
          ? data.frame
          : new Float32Array(data.frame ?? []);
      this.onAiFrame?.(
        Number(data.slot),
        frame,
        parseAiRaceState(data.race),
      );
      return;
    }
    if (data?.type === "contacts") {
      for (const contact of parseContacts(data.contacts)) {
        this.onContact?.(contact);
      }
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
        aiCount: data.aiCount ?? 0,
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
