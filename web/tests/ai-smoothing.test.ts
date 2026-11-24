import test from "node:test";
import assert from "node:assert/strict";

import {
  AI_FROZEN_SUSPENSION_TRAVEL,
  freezeAiWheels,
  smoothAiPose,
} from "../src/sim/ai-smoothing.ts";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";

const POSITION = SNAPSHOT_STATE.position;
const ROTATION = SNAPSHOT_STATE.rotation;

function pose(position: number, quaternion: [number, number, number, number] = [0, 0, 0, 1]): Float32Array {
  const values = new Float32Array(64);
  values[POSITION] = position;
  values[POSITION + 1] = position * 2;
  values[POSITION + 2] = -position;
  values[ROTATION] = quaternion[0];
  values[ROTATION + 1] = quaternion[1];
  values[ROTATION + 2] = quaternion[2];
  values[ROTATION + 3] = quaternion[3];
  // Non-pose fields should always come from the incoming frame.
  values[SNAPSHOT_STATE.velocity] = position + 100;
  values[SNAPSHOT_STATE.engineRpm] = 1234 + position;
  return values;
}

function quaternionNorm(state: Float32Array): number {
  return Math.hypot(
    state[ROTATION],
    state[ROTATION + 1],
    state[ROTATION + 2],
    state[ROTATION + 3],
  );
}

test("missing previous frame returns a copy of the incoming frame", () => {
  const frame = pose(5);
  const smoothed = smoothAiPose(null, frame, 0.016);

  assert.notEqual(smoothed, frame);
  assert.deepEqual(Array.from(smoothed), Array.from(frame));

  smoothed[POSITION] = 99;
  assert.equal(frame[POSITION], 5);
});

test("position moves toward the incoming frame and other fields pass through", () => {
  const previous = pose(0);
  const frame = pose(10);
  // Pass tau explicitly (dt == tau) so this asserts a fixed factor of
  // 1 - exp(-1) ~= 0.632, independent of the production default.
  const smoothed = smoothAiPose(previous, frame, 0.05, 0.05);

  assert.ok(smoothed[POSITION] > 0, "position did not advance");
  assert.ok(smoothed[POSITION] < 10, "position overshot");
  assert.ok(Math.abs(smoothed[POSITION] - 6.321) < 0.01);
  assert.ok(Math.abs(smoothed[POSITION + 1] - 12.642) < 0.02);
  assert.ok(Math.abs(smoothed[POSITION + 2] + 6.321) < 0.01);

  // Non-pose fields are taken verbatim from the incoming frame.
  assert.equal(smoothed[SNAPSHOT_STATE.velocity], frame[SNAPSHOT_STATE.velocity]);
  assert.equal(smoothed[SNAPSHOT_STATE.engineRpm], frame[SNAPSHOT_STATE.engineRpm]);
});

test("default time constant applies the shorter 20 ms blend", () => {
  const previous = pose(0);
  const frame = pose(10);
  // Production default tau is 0.02 s, so dt == tau gives ~0.632 again.
  const smoothed = smoothAiPose(previous, frame, 0.02);

  assert.ok(smoothed[POSITION] > 0, "position did not advance");
  assert.ok(smoothed[POSITION] < 10, "position overshot");
  assert.ok(Math.abs(smoothed[POSITION] - 6.321) < 0.01);
});

test("quaternion sign correction blends along the short arc", () => {
  const previous = pose(0, [0, 0, 0, 1]);
  // -identity encodes the same rotation; without sign correction the blend
  // would collapse toward zero before normalising.
  const frame = pose(0, [0, 0, 0, -1]);
  const smoothed = smoothAiPose(previous, frame, 0.05);

  assert.ok(quaternionNorm(smoothed) > 0.999, "quaternion not normalised");
  assert.ok(Math.abs(smoothed[ROTATION + 3]) > 0.999, "unexpected sign flip");
  assert.ok(Math.abs(smoothed[ROTATION]) < 1e-6);
  assert.ok(Math.abs(smoothed[ROTATION + 1]) < 1e-6);
  assert.ok(Math.abs(smoothed[ROTATION + 2]) < 1e-6);
});

test("invalid or non-positive dt is sanitised instead of producing NaN", () => {
  const previous = pose(0);
  const frame = pose(10);
  for (const dt of [Number.NaN, Number.POSITIVE_INFINITY, -1, 0]) {
    const smoothed = smoothAiPose(previous, frame, dt);
    for (let index = 0; index < smoothed.length; index += 1) {
      assert.ok(Number.isFinite(smoothed[index]), `non-finite at ${index} for dt ${dt}`);
    }
    assert.ok(smoothed[POSITION] >= 0 && smoothed[POSITION] <= 10);
    assert.ok(Math.abs(quaternionNorm(smoothed) - 1) < 1e-6);
  }
});

function wheelFrame(): Float32Array {
  const values = pose(3);
  const groups: Array<[number, number]> = [
    [SNAPSHOT_STATE.suspensionTravel, 0.2],
    [SNAPSHOT_STATE.wheelSteerAngle, 0.4],
    [SNAPSHOT_STATE.wheelSpinAngle, 1.2],
  ];
  for (const [start, base] of groups) {
    for (let index = 0; index < 4; index += 1) {
      values[start + index] = base + index * 0.1;
    }
  }
  return values;
}

test("freezeAiWheels zeroes wheel fields and pins suspension travel", () => {
  const frame = wheelFrame();
  // Snapshot the non-wheel fields to prove they are left untouched.
  const before = Float32Array.from(frame);

  const result = freezeAiWheels(frame);

  assert.equal(result, frame);
  for (let wheel = 0; wheel < 4; wheel += 1) {
    assert.equal(
      frame[SNAPSHOT_STATE.suspensionTravel + wheel],
      Math.fround(AI_FROZEN_SUSPENSION_TRAVEL),
    );
    assert.equal(frame[SNAPSHOT_STATE.wheelSteerAngle + wheel], 0);
    assert.equal(frame[SNAPSHOT_STATE.wheelSpinAngle + wheel], 0);
  }

  const frozen = new Set([
    ...Array.from({ length: 4 }, (_, i) => SNAPSHOT_STATE.suspensionTravel + i),
    ...Array.from({ length: 4 }, (_, i) => SNAPSHOT_STATE.wheelSteerAngle + i),
    ...Array.from({ length: 4 }, (_, i) => SNAPSHOT_STATE.wheelSpinAngle + i),
  ]);
  for (let index = 0; index < frame.length; index += 1) {
    if (!frozen.has(index)) {
      assert.equal(frame[index], before[index], `field ${index} changed`);
    }
  }
});

test("freezeAiWheels accepts a custom suspension constant", () => {
  const frame = wheelFrame();
  freezeAiWheels(frame, 0.12);
  for (let wheel = 0; wheel < 4; wheel += 1) {
    assert.equal(frame[SNAPSHOT_STATE.suspensionTravel + wheel], Math.fround(0.12));
  }
});
