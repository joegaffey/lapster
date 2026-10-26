import init, { Simulation } from "../pkg/worker/sim_worker.js";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";
import {
  clearReset,
  getSabFlags,
  publishSnapshot,
  readInput,
  validateSab,
  writeFfb,
  type InputState,
} from "../src/sim/sab.ts";

const FIXED_DT = 1 / 500;
const MAX_STEPS = 4;

let simulation: Simulation | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
let sab: SharedArrayBuffer | null = null;
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
let lastTime = performance.now();
let accumulator = 0;
let droppedTicks = 0;
let catchupEvents = 0;
let stepTimes: number[] = [];
let lastDiagnostics = performance.now();

function post(type: string, payload: Record<string, unknown> = {}): void {
  self.postMessage({ type, ...payload });
}

function startLoop(): void {
  if (timer !== null) {
    clearTimeout(timer);
  }
  lastTime = performance.now();
  accumulator = 0;
  const loop = () => {
    if (!simulation || !sab) {
      timer = null;
      return;
    }
    const now = performance.now();
    const dt = Math.min((now - lastTime) / 1000, 0.05);
    lastTime = now;
    if (!paused) {
      accumulator += dt;
      let steps = 0;
      while (accumulator >= FIXED_DT && steps < MAX_STEPS) {
        lastInput = readInput(sab, lastInput);
        const started = performance.now();
        simulation.step(
          lastInput.throttle,
          lastInput.brake,
          lastInput.steering,
          lastInput.gear,
        );
        stepTimes.push(performance.now() - started);
        if (stepTimes.length > 250) {
          stepTimes.shift();
        }
        accumulator -= FIXED_DT;
        steps += 1;
        const state = simulation.snapshot();
        publishSnapshot(sab, Number(simulation.tick()), Date.now() / 1000, new Float32Array(state));
        writeFfb(sab, Math.max(-1, Math.min(1, state[SNAPSHOT_STATE.rackTorque] / 10)), true);
      }
      if (steps === MAX_STEPS && accumulator >= FIXED_DT) {
        accumulator = 0;
        droppedTicks += 1;
      }
      if (steps === MAX_STEPS) {
        catchupEvents += 1;
      }
      if ((getSabFlags(sab) & 2) !== 0) {
        simulation.reset();
        clearReset(sab);
        accumulator = 0;
        publishSnapshot(sab, Number(simulation.tick()), Date.now() / 1000, new Float32Array(simulation.snapshot()));
      }
    } else {
      accumulator = 0;
    }

    if (now - lastDiagnostics >= 500) {
      lastDiagnostics = now;
      const sorted = [...stepTimes].sort((a, b) => a - b);
      const p50 = sorted[Math.min(sorted.length - 1, Math.round(sorted.length * 0.5))] ?? 0;
      const p99 = sorted[Math.min(sorted.length - 1, Math.round(sorted.length * 0.99))] ?? 0;
      post("diagnostics", {
        tick: Number(simulation.tick()),
        stepP50Ms: p50,
        stepP99Ms: p99,
        droppedTicks,
        catchupEvents,
      });
    }
    timer = setTimeout(loop, 1);
  };
  timer = setTimeout(loop, 1);
}

function stopLoop(): void {
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
}

self.onmessage = async (event: MessageEvent) => {
  const message = event.data;
  if (message?.type === "init") {
    try {
      sab = message.sab as SharedArrayBuffer;
      validateSab(sab);
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
      stopLoop();
      startLoop();
      post("ack", { id: message.id });
      break;
    case "resume":
      paused = false;
      lastTime = performance.now();
      accumulator = 0;
      startLoop();
      post("ack", { id: message.id });
      break;
    case "reset":
      simulation.reset();
      accumulator = 0;
      if (sab) {
        publishSnapshot(sab, Number(simulation.tick()), Date.now() / 1000, new Float32Array(simulation.snapshot()));
      }
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
      if (sab) {
        publishSnapshot(sab, Number(simulation.tick()), Date.now() / 1000, new Float32Array(simulation.snapshot()));
      }
      lastTime = performance.now();
      accumulator = 0;
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
