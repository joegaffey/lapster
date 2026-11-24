import test from "node:test";
import assert from "node:assert/strict";

import {
  AI_DEBUG_HISTORY_LIMIT,
  createAiDebugHistory,
  pushBounded,
  sampleRawAiFrame,
  sampleRenderedAiPose,
} from "../src/sim/ai-debug.ts";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";

function frame(
  x: number,
  vx: number,
  vz: number,
  tick: number,
  yaw = 0,
): Float32Array {
  const values = new Float32Array(64);
  values[SNAPSHOT_STATE.position] = x;
  values[SNAPSHOT_STATE.position + 1] = x + 1;
  values[SNAPSHOT_STATE.position + 2] = x + 2;
  values[SNAPSHOT_STATE.velocity] = vx;
  values[SNAPSHOT_STATE.velocity + 2] = vz;
  values[SNAPSHOT_STATE.tick] = tick;

  // Rotation around the world Y axis so the derived yaw is easy to predict.
  const half = yaw / 2;
  values[SNAPSHOT_STATE.rotation] = 0;
  values[SNAPSHOT_STATE.rotation + 1] = Math.sin(half);
  values[SNAPSHOT_STATE.rotation + 2] = 0;
  values[SNAPSHOT_STATE.rotation + 3] = Math.cos(half);

  // Integer fields keep the Float32Array values exact for deep comparisons.
  const groups: Array<[number, number, number]> = [
    [SNAPSHOT_STATE.angularVelocity, 3, 0],
    [SNAPSHOT_STATE.suspensionTravel, 4, 10],
    [SNAPSHOT_STATE.wheelSteerAngle, 4, 20],
    [SNAPSHOT_STATE.wheelSpinAngle, 4, 30],
  ];
  for (const [start, length, base] of groups) {
    for (let index = 0; index < length; index += 1) {
      values[start + index] = x + base + index;
    }
  }
  return values;
}

function assertClose(actual: number, expected: number, label: string): void {
  assert.ok(
    Math.abs(actual - expected) < 1e-6,
    `${label}: ${actual} != ${expected}`,
  );
}

test("raw sample reads pose, wheel fields and derived yaw", () => {
  const sample = sampleRawAiFrame(
    2,
    frame(5, 3, 4, 120, Math.PI / 2),
    1000,
    120,
  );

  assert.equal(sample.t, 1000);
  assert.equal(sample.slot, 2);
  assert.equal(sample.tick, 120);
  assert.deepEqual(sample.position, [5, 6, 7]);
  assert.equal(sample.speed, 5);
  assert.deepEqual(sample.angularVelocity, [5, 6, 7]);
  assert.deepEqual(sample.suspensionTravel, [15, 16, 17, 18]);
  assert.deepEqual(sample.wheelSteerAngle, [25, 26, 27, 28]);
  assert.deepEqual(sample.wheelSpinAngle, [35, 36, 37, 38]);
  assertClose(sample.rotation[0], 0, "rotation.x");
  assertClose(sample.rotation[1], Math.SQRT1_2, "rotation.y");
  assertClose(sample.rotation[2], 0, "rotation.z");
  assertClose(sample.rotation[3], Math.SQRT1_2, "rotation.w");
  assertClose(sample.yaw, Math.PI / 2, "yaw");
});

test("rendered sample keeps interpolated and smoothed poses apart", () => {
  const interpolated = frame(10, 0, 0, 200, Math.PI / 2);
  const smoothed = frame(8, 0, 0, 200, Math.PI / 4);
  const sample = sampleRenderedAiPose(1, interpolated, smoothed, 2000, 200, {
    alpha: 0.25,
    depth: 12,
    jitter: 1.5,
    offsetMs: -40,
    age: 16,
  });

  assert.deepEqual(sample.position, [10, 11, 12]);
  assert.deepEqual(sample.smoothed, [8, 9, 10]);
  assert.equal(sample.alpha, 0.25);
  assert.equal(sample.depth, 12);
  assert.equal(sample.jitter, 1.5);
  assert.equal(sample.offsetMs, -40);
  assert.equal(sample.age, 16);
  // Pose and wheel fields describe the rendered (filtered) pose.
  assert.deepEqual(sample.angularVelocity, [8, 9, 10]);
  assert.deepEqual(sample.suspensionTravel, [18, 19, 20, 21]);
  assert.deepEqual(sample.wheelSteerAngle, [28, 29, 30, 31]);
  assert.deepEqual(sample.wheelSpinAngle, [38, 39, 40, 41]);
  assertClose(sample.rotation[1], Math.sin(Math.PI / 8), "rotation.y");
  assertClose(sample.rotation[3], Math.cos(Math.PI / 8), "rotation.w");
  assertClose(sample.yaw, Math.PI / 4, "yaw");
});

test("rendered sample falls back to the raw pose without a filtered frame", () => {
  const input = frame(3, 0, 0, 9, Math.PI / 3);
  const sample = sampleRenderedAiPose(0, input, null, 10, 9, {
    alpha: 1,
    depth: 1,
    jitter: 0,
    offsetMs: 0,
    age: 0,
  });

  assert.deepEqual(sample.smoothed, sample.position);
  assert.deepEqual(sample.angularVelocity, [3, 4, 5]);
  assert.deepEqual(sample.suspensionTravel, [13, 14, 15, 16]);
  assert.deepEqual(sample.wheelSteerAngle, [23, 24, 25, 26]);
  assert.deepEqual(sample.wheelSpinAngle, [33, 34, 35, 36]);
  assertClose(sample.yaw, Math.PI / 3, "yaw");
});

test("derived yaw is unchanged when the quaternion is negated", () => {
  const positive = frame(0, 0, 0, 1, Math.PI / 5);
  const negative = Float32Array.from(positive);
  for (let index = 0; index < 4; index += 1) {
    negative[SNAPSHOT_STATE.rotation + index] = -negative[
      SNAPSHOT_STATE.rotation + index
    ];
  }

  const a = sampleRawAiFrame(0, positive, 0, 1);
  const b = sampleRawAiFrame(0, negative, 0, 1);
  assertClose(b.yaw, a.yaw, "negated yaw");
  assertClose(a.yaw, Math.PI / 5, "yaw");
});

test("pushBounded keeps only the newest samples", () => {
  const history = createAiDebugHistory(3);
  assert.equal(history.limit, 3);
  const raw: number[] = [];
  for (let index = 0; index < 5; index += 1) {
    pushBounded(raw, index, history.limit);
  }

  assert.deepEqual(raw, [2, 3, 4]);
});

test("default history limit is bounded", () => {
  const history = createAiDebugHistory();
  assert.equal(history.limit, AI_DEBUG_HISTORY_LIMIT);
  const rendered: number[] = [];
  for (let index = 0; index < AI_DEBUG_HISTORY_LIMIT + 50; index += 1) {
    pushBounded(rendered, index, history.limit);
  }
  assert.equal(rendered.length, AI_DEBUG_HISTORY_LIMIT);
});
