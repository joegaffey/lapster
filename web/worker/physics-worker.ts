import init, { Simulation } from "../pkg/worker/sim_worker.js";
import { HEADER_FIELDS, SNAPSHOT_STATE } from "../src/types/asset-layout.ts";
import {
  clearReset,
  getSabFlags,
  publishSnapshot,
  readInput,
  validateSab,
  writeFfb,
  type InputState,
} from "../src/sim/sab.ts";

const FIXED_STEP_MS = 1000 / 500;
const MAX_CATCHUP_STEPS = 32;
const MAX_LAG_MS = 250;
const PAUSED_WAIT_MS = 20;

let simulation: Simulation | null = null;
let sab: SharedArrayBuffer | null = null;
let waitWord: Int32Array | null = null;
let channel: MessageChannel | null = null;
let loopGeneration = 0;
let paused = false;
let lastInput: InputState = {
  steering: 0,
  throttle: 0,
  brake: 0,
  clutch: 0,
  gear: 1,
  buttons: 0,
  timestampSeconds: 0,
};
let nextDeadline = performance.now();
let droppedTicks = 0;
let catchupEvents = 0;
let stepTimes: number[] = [];
let lastDiagnosticsTime = performance.now();
let lastDiagnosticsTick = 0;

function post(type: string, payload: Record<string, unknown> = {}): void {
  self.postMessage({ type, ...payload });
}

function schedule(): void {
  channel?.port2.postMessage(0);
}

function stopLoop(): void {
  loopGeneration += 1;
  if (!channel) {
    return;
  }
  channel.port1.onmessage = null;
  channel.port1.close();
  channel.port2.close();
  channel = null;
}

function startLoop(): void {
  stopLoop();
  const generation = loopGeneration;
  const next = new MessageChannel();
  channel = next;
  next.port1.onmessage = () => {
    if (generation !== loopGeneration) {
      return;
    }
    runSlice(generation);
  };
  nextDeadline = performance.now();
  lastDiagnosticsTime = performance.now();
  lastDiagnosticsTick = simulation ? Number(simulation.tick()) : 0;
  schedule();
}

function wake(): void {
  if (waitWord) {
    Atomics.notify(waitWord, 0);
  }
}

function markDiagnosticsBaseline(): void {
  lastDiagnosticsTime = performance.now();
  lastDiagnosticsTick = simulation ? Number(simulation.tick()) : 0;
}

function publish(): void {
  if (!simulation || !sab) {
    return;
  }
  publishSnapshot(
    sab,
    Number(simulation.tick()),
    Date.now() / 1000,
    new Float32Array(simulation.snapshot()),
  );
}

function resetFromFlag(): void {
  if (!simulation || !sab || (getSabFlags(sab) & 2) === 0) {
    return;
  }
  simulation.reset();
  clearReset(sab);
  nextDeadline = performance.now();
  markDiagnosticsBaseline();
  publish();
}

function runSlice(generation: number): void {
  if (generation !== loopGeneration || !simulation || !sab || !waitWord) {
    return;
  }

  resetFromFlag();

  if (paused) {
    Atomics.wait(waitWord, 0, Atomics.load(waitWord, 0), PAUSED_WAIT_MS);
    emitDiagnostics(performance.now());
    schedule();
    return;
  }

  const now = performance.now();
  const waitMs = nextDeadline - now;
  if (waitMs > 0) {
    Atomics.wait(waitWord, 0, Atomics.load(waitWord, 0), waitMs);
  }

  const current = performance.now();
  let steps = 0;
  while (current >= nextDeadline && steps < MAX_CATCHUP_STEPS) {
    lastInput = readInput(sab, lastInput);
    const started = performance.now();
    simulation.step(
      lastInput.throttle,
      lastInput.brake,
      lastInput.steering,
      lastInput.gear,
      lastInput.clutch,
    );
    stepTimes.push(performance.now() - started);
    if (stepTimes.length > 250) {
      stepTimes.shift();
    }
    nextDeadline += FIXED_STEP_MS;
    steps += 1;
    const state = simulation.snapshot();
    publishSnapshot(
      sab,
      Number(simulation.tick()),
      Date.now() / 1000,
      new Float32Array(state),
    );
    writeFfb(sab, Math.max(-1, Math.min(1, state[SNAPSHOT_STATE.rackTorque] / 20)), true);
  }

  if (steps > 1) {
    catchupEvents += 1;
  }
  if (steps === MAX_CATCHUP_STEPS && current - nextDeadline > MAX_LAG_MS) {
    droppedTicks += Math.max(0, Math.round((current - nextDeadline) / FIXED_STEP_MS));
    nextDeadline = current;
  }

  emitDiagnostics(performance.now());
  schedule();
}

function emitDiagnostics(now: number): void {
  if (!simulation || now - lastDiagnosticsTime < 500) {
    return;
  }
  const tick = Number(simulation.tick());
  const elapsedSeconds = (now - lastDiagnosticsTime) / 1000;
  const physicsHz =
    elapsedSeconds > 0 ? (tick - lastDiagnosticsTick) / elapsedSeconds : 0;
  lastDiagnosticsTime = now;
  lastDiagnosticsTick = tick;
  const sorted = [...stepTimes].sort((a, b) => a - b);
  const p50 = sorted[Math.min(sorted.length - 1, Math.round(sorted.length * 0.5))] ?? 0;
  const p99 = sorted[Math.min(sorted.length - 1, Math.round(sorted.length * 0.99))] ?? 0;
  post("diagnostics", {
    tick,
    physicsHz,
    stepP50Ms: p50,
    stepP99Ms: p99,
    droppedTicks,
    catchupEvents,
  });
}

self.onmessage = async (event: MessageEvent) => {
  const message = event.data;
  if (message?.type === "init") {
    try {
      sab = message.sab as SharedArrayBuffer;
      validateSab(sab);
      waitWord = new Int32Array(sab, HEADER_FIELDS.flags, 1);
      await init();
      simulation = new Simulation(message.trackBytes, message.carBytes);
      post("ready", { setup: simulation.setup() });
      startLoop();
    } catch (error) {
      post("error", { message: error instanceof Error ? error.message : String(error) });
    }
    return;
  }

  if (!simulation) {
    if (message?.id !== undefined) {
      post("ack", { id: message.id, error: "worker is not initialized" });
    }
    return;
  }

  switch (message?.type) {
    case "pause":
      paused = true;
      wake();
      post("ack", { id: message.id });
      break;
    case "resume":
      paused = false;
      nextDeadline = performance.now();
      markDiagnosticsBaseline();
      wake();
      post("ack", { id: message.id });
      break;
    case "reset":
      simulation.reset();
      nextDeadline = performance.now();
      markDiagnosticsBaseline();
      publish();
      post("ack", { id: message.id, tick: Number(simulation.tick()) });
      break;
    case "simulate": {
      const tick = simulation.simulate(
        message.seconds,
        message.throttle,
        message.brake,
        message.steering,
        message.gear,
      );
      nextDeadline = performance.now();
      markDiagnosticsBaseline();
      publish();
      post("ack", { id: message.id, tick });
      break;
    }
    case "apply_physics_tuning": {
      const tick = simulation.apply_physics_tuning(message.tuning);
      nextDeadline = performance.now();
      markDiagnosticsBaseline();
      publish();
      post("ack", { id: message.id, tick });
      break;
    }
    case "apply_setup": {
      const setup = simulation.apply_setup(message.setup);
      post("ack", { id: message.id, setup });
      break;
    }
    case "reset_setup": {
      const setup = simulation.reset_setup();
      post("ack", { id: message.id, setup });
      break;
    }
    case "set_assists":
      simulation.set_traction_control(message.tc);
      simulation.set_traction_strength(message.tcStrength);
      simulation.set_abs_control(message.abs);
      simulation.set_stability_control(message.esc);
      simulation.set_stability_strength(message.escStrength);
      simulation.set_steering_sensitivity(message.steeringSensitivity);
      post("ack", { id: message.id });
      break;
    case "dispose":
      stopLoop();
      simulation = null;
      post("ack", { id: message.id });
      break;
    default:
      if (message?.id !== undefined) {
        post("ack", { id: message.id, error: `unknown command ${message.type}` });
      }
  }
};
