import test from "node:test";
import assert from "node:assert/strict";

import {
  COCKPIT_RPM_OFFSETS,
  cockpitRpmOffset,
} from "../src/ui/cockpit-offsets.ts";

test("67GP cockpit RPM offsets match the tuned values", () => {
  assert.deepEqual(COCKPIT_RPM_OFFSETS, {
    "67gp-cavallo-123": [0.002, -0.024, -0.01],
    "67gp-kuiper-t81": [0, -0.05, -0.01],
    "67gp-mango-ms7": [0, -0.05, -0.01],
    "67gp-murasama-37a": [0, 0.02, -0.12],
    "67gp-upping-ut19": [0, -0.1, -0.06],
  });
  assert.deepEqual(cockpitRpmOffset("67gp-cavallo-123"), [0.002, -0.024, -0.01]);
  assert.deepEqual(cockpitRpmOffset("67gp-kuiper-t81"), [0, -0.05, -0.01]);
  assert.deepEqual(cockpitRpmOffset("67gp-mango-ms7"), [0, -0.05, -0.01]);
  assert.deepEqual(cockpitRpmOffset("67gp-murasama-37a"), [0, 0.02, -0.12]);
  assert.deepEqual(cockpitRpmOffset("67gp-upping-ut19"), [0, -0.1, -0.06]);
});

test("unknown car ids have no cockpit RPM offset", () => {
  assert.equal(cockpitRpmOffset("mazda-rx7"), undefined);
  assert.equal(cockpitRpmOffset(""), undefined);
});
