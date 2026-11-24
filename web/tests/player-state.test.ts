import test from "node:test";
import assert from "node:assert/strict";

import {
  PLAYER_INTERPOLATION_MS,
  PlayerStateBuffer,
} from "../src/sim/player-state.ts";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";

const MS_PER_TICK = 2; // 500 Hz worker
const TRUE_SPEED = 10; // world units per second

function stateAt(tick: number): Float32Array {
  const state = new Float32Array(64);
  state[SNAPSHOT_STATE.position] =
    tick * ((TRUE_SPEED * MS_PER_TICK) / 1000);
  state[SNAPSHOT_STATE.rotation + 3] = 1;
  return state;
}

test("irregular 60 Hz polling of a 500 Hz stream renders smooth constant velocity", () => {
  const frameDeltasMs = [14, 24, 16, 18, 14, 14];
  const buffer = new PlayerStateBuffer();
  let now = 1000;
  let tick = 500; // offset starts at now - tick * MS_PER_TICK = 0
  let previous: number | null = null;
  let backwards = 0;
  let worstDeviation = 0;

  // 160 frames (~2.7 s) of worker catch-up: tick jumps 3-16 per poll while
  // wall-clock frame intervals vary 14-24 ms. The alternating +/-4 tick jitter
  // models catch-up without drifting the tick timeline away from wall time.
  for (let index = 0; index < 160; index += 1) {
    let deltaMs = 0;
    if (index > 0) {
      deltaMs = frameDeltasMs[(index - 1) % frameDeltasMs.length];
      tick += Math.round(deltaMs / 2) + (index % 2 === 0 ? 4 : -4);
      now += deltaMs;
    }
    assert.equal(
      buffer.push({ tick, state: stateAt(tick) }, now),
      true,
      `push failed at frame ${index}`,
    );

    const frame = buffer.frame(now);
    assert.ok(frame, `no frame at ${now}`);
    const position = frame[SNAPSHOT_STATE.position];
    if (index >= 40 && previous !== null) {
      if (position < previous) {
        backwards += 1;
      }
      const speed = (position - previous) / (deltaMs / 1000);
      worstDeviation = Math.max(
        worstDeviation,
        Math.abs(speed - TRUE_SPEED) / TRUE_SPEED,
      );
    }
    previous = position;
  }

  assert.equal(backwards, 0, "rendered position moved backwards");
  assert.ok(
    worstDeviation <= 0.05,
    `per-frame speed deviated ${(worstDeviation * 100).toFixed(2)}% from ${TRUE_SPEED} u/s`,
  );
});

test("rendered pose lags the newest snapshot by the configured delay", () => {
  const buffer = new PlayerStateBuffer();
  const lastTick = 400;
  for (let tick = 0; tick <= lastTick; tick += 1) {
    buffer.push({ tick, state: stateAt(tick) }, tick * MS_PER_TICK);
  }

  const now = lastTick * MS_PER_TICK;
  const newest = buffer.latest();
  const frame = buffer.frame(now);
  assert.ok(newest && frame);

  const expectedLag =
    (TRUE_SPEED * PLAYER_INTERPOLATION_MS) / 1000;
  const lag =
    newest[SNAPSHOT_STATE.position] - frame[SNAPSHOT_STATE.position];
  assert.ok(
    Math.abs(lag - expectedLag) < 0.005,
    `expected ${expectedLag} units of lag, got ${lag}`,
  );
});

test("duplicate ticks do not stall or advance the rendered pose", () => {
  const buffer = new PlayerStateBuffer();
  buffer.push({ tick: 1000, state: stateAt(1000) }, 2000);
  buffer.push({ tick: 1020, state: stateAt(1020) }, 2040);

  const depthBefore = buffer.depth;
  const latestBefore = buffer.latest()?.[SNAPSHOT_STATE.position];
  const frameBefore = buffer.frame(2060)?.[SNAPSHOT_STATE.position];
  assert.notEqual(frameBefore, undefined);

  const conflicting = stateAt(1020);
  conflicting[SNAPSHOT_STATE.position] = 999;
  assert.equal(
    buffer.push({ tick: 1020, state: conflicting }, 2070),
    false,
  );
  assert.equal(buffer.depth, depthBefore);
  assert.equal(
    buffer.frame(2060)?.[SNAPSHOT_STATE.position],
    frameBefore,
  );
  assert.equal(
    buffer.latest()?.[SNAPSHOT_STATE.position],
    latestBefore,
  );

  // A newer tick still advances the rendered pose normally.
  buffer.push({ tick: 1040, state: stateAt(1040) }, 2080);
  assert.ok(
    (buffer.frame(2100)?.[SNAPSHOT_STATE.position] ?? 0) >
      (frameBefore ?? 0),
  );
});

test("tick regression and clear reset history to the new timeline", () => {
  const buffer = new PlayerStateBuffer();
  buffer.push({ tick: 1000, state: stateAt(1000) }, 3000);
  buffer.push({ tick: 1020, state: stateAt(1020) }, 3040);
  assert.equal(buffer.depth, 2);

  buffer.clear();
  assert.equal(buffer.depth, 0);
  assert.equal(buffer.latest(), null);
  assert.equal(buffer.frame(3040), null);

  // A reset drops back to tick 0, which must snap to the new sample instead of
  // interpolating from the old ~20-unit position.
  buffer.push({ tick: 0, state: stateAt(0) }, 4000);
  assert.equal(buffer.depth, 1);
  assert.equal(buffer.latest()?.[SNAPSHOT_STATE.position], 0);
  assert.equal(buffer.frame(4000)?.[SNAPSHOT_STATE.position], 0);
});

test("rotation interpolation takes the short arc across a negated quaternion", () => {
  const buffer = new PlayerStateBuffer(0);
  const half = 0.05; // 0.1 rad total
  const previous = new Float32Array(64);
  previous[SNAPSHOT_STATE.rotation + 1] = Math.sin(half);
  previous[SNAPSHOT_STATE.rotation + 3] = Math.cos(half);
  // Same physical rotation as the inverse angle, but negated so the raw dot
  // product is negative. The short arc still passes through the identity.
  const current = new Float32Array(64);
  current[SNAPSHOT_STATE.rotation + 1] = Math.sin(half);
  current[SNAPSHOT_STATE.rotation + 3] = -Math.cos(half);

  buffer.push({ tick: 0, state: previous }, 1000);
  buffer.push({ tick: 100, state: current }, 1200);

  const frame = buffer.frame(1100);
  assert.ok(frame);
  const y = frame[SNAPSHOT_STATE.rotation + 1];
  const w = frame[SNAPSHOT_STATE.rotation + 3];
  assert.ok(Math.abs(y) < 1e-3, `expected short-arc identity, got y=${y}`);
  assert.ok(
    Math.abs(Math.abs(w) - 1) < 1e-3,
    `expected unit |w|, got w=${w}`,
  );
});
