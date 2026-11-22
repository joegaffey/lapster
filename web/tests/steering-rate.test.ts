import test from "node:test";
import assert from "node:assert/strict";

import { rateLimitSteering } from "../src/input/steering-rate.ts";

test("rate limiter moves toward the target at the configured rate", () => {
  assert.ok(Math.abs(rateLimitSteering(0, 1, 6, 1 / 60) - 0.1) < 1e-9);
  assert.ok(Math.abs(rateLimitSteering(0, -1, 6, 1 / 60) + 0.1) < 1e-9);
  assert.equal(rateLimitSteering(0.5, 1, 6, 0.1), 1);
});

test("rate zero (and non-positive rates) applies the target directly", () => {
  assert.equal(rateLimitSteering(0, 1, 0, 1 / 60), 1);
  assert.equal(rateLimitSteering(0, -1, 0, 1 / 60), -1);
  assert.equal(rateLimitSteering(0.4, 0, -5, 1 / 60), 0);
});

test("the limiter does not overshoot and clamps to the steering range", () => {
  assert.equal(rateLimitSteering(0, 0.05, 6, 1 / 60), 0.05);
  assert.equal(rateLimitSteering(0.95, 1, 20, 0.1), 1);
  assert.equal(rateLimitSteering(-0.95, -1, 20, 0.1), -1);
  assert.equal(rateLimitSteering(0, 5, 20, 1), 1);
});

test("the step scales with delta time and ignores invalid elapsed time", () => {
  assert.ok(Math.abs(rateLimitSteering(0, 1, 6, 1 / 30) - 0.2) < 1e-9);
  assert.equal(rateLimitSteering(0.5, 1, 6, 0), 0.5);
  assert.equal(rateLimitSteering(0.5, 1, 6, -1), 0.5);
  assert.equal(rateLimitSteering(0.5, 1, 6, Number.NaN), 0.5);
});
