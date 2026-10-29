import test from "node:test";
import assert from "node:assert/strict";

import {
  computeFfbOutput,
  DEFAULT_FEEL,
  type WheelFeelSettings,
  type WheelFeelState,
} from "../src/input/wheel-feel.ts";

const settings: WheelFeelSettings = {
  ...DEFAULT_FEEL,
  roadNoise: 0,
  roughness: 0,
  looseness: 0,
  rearLightening: 0.75,
  frontShudder: 0.5,
};
const calm: WheelFeelState = { speedMps: 0, frontSlip: 0, rearSlip: 0 };

test("base torque passes through without slip or noise", () => {
  assert.equal(computeFfbOutput(0.4, calm, settings, 0, 0), 0.4);
});

test("rear slip lightens the steering force", () => {
  const output = computeFfbOutput(
    0.4,
    { speedMps: 20, frontSlip: 0, rearSlip: 0.4 },
    settings,
    0,
    0,
  );
  assert.ok(output < 0.4);
  assert.ok(output >= 0);
});

test("front grip loss adds a configurable shudder", () => {
  const state = { speedMps: 20, frontSlip: 0.4, rearSlip: 0 };
  const time = 1 / (4 * settings.shudderHz);
  const calmOrder = computeFfbOutput(0.4, state, { ...settings, frontShudder: 0 }, time, 0);
  const shuddering = computeFfbOutput(0.4, state, settings, time, 0);
  assert.ok(shuddering > calmOrder);
});

test("road noise and roughness are speed-scaled", () => {
  const fast = computeFfbOutput(
    0,
    { speedMps: 25, frontSlip: 0, rearSlip: 0 },
    { ...DEFAULT_FEEL, roadNoise: 1, roughness: 1 },
    0.01,
    1,
  );
  const stopped = computeFfbOutput(
    0,
    { speedMps: 0, frontSlip: 0, rearSlip: 0 },
    { ...DEFAULT_FEEL, roadNoise: 1, roughness: 1 },
    0.01,
    1,
  );
  assert.ok(Math.abs(fast) > 0);
  assert.equal(stopped, 0);
});

test("output is clamped", () => {
  assert.equal(computeFfbOutput(4, calm, settings, 0, 0), 1);
  assert.equal(computeFfbOutput(-4, calm, settings, 0, 0), -1);
});
