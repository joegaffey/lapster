import test from "node:test";
import assert from "node:assert/strict";

import { distanceGain, liteGain } from "../src/audio/remote-engine-mix.ts";

test("close idle lite voice stays audible", () => {
  const gain = liteGain(distanceGain(2), 900, 0);
  assert.ok(gain >= 0.15, `expected close idle gain >= 0.15, got ${gain}`);
  assert.ok(gain <= 1);
});

test("throttle and rpm raise the lite voice but stay clamped", () => {
  const idle = liteGain(distanceGain(2), 900, 0);
  const loaded = liteGain(distanceGain(2), 7_000, 1);
  assert.ok(loaded > idle);
  assert.ok(loaded <= 1);
});

test("lite voice is silent at and beyond the audible radius", () => {
  assert.equal(liteGain(distanceGain(250), 900, 1), 0);
  assert.equal(liteGain(distanceGain(1_000), 900, 1), 0);
  assert.equal(distanceGain(Number.POSITIVE_INFINITY), 0);
});
