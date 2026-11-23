import test from "node:test";
import assert from "node:assert/strict";

import {
  isRaceState,
  isRaceStateView,
  parseRaceStateView,
  type RaceState,
} from "../src/contracts/race-state.ts";

function validRaceState(): RaceState {
  return {
    lap: 1,
    sector: 1,
    progress: 0.12,
    distance_m: 44.2,
    wrong_way: false,
    started: true,
    current_lap_time: 12.34,
    last_lap_time: null,
    best_lap_time: null,
  };
}

test("accepts an unavailable race state view", () => {
  assert.equal(isRaceStateView({ available: false }), true);
  assert.equal(isRaceStateView({ available: true }), false);
});

test("accepts a valid available race state view", () => {
  assert.equal(isRaceStateView({ available: true, ...validRaceState() }), true);
  assert.equal(
    isRaceStateView({
      available: true,
      ...validRaceState(),
      last_lap_time: 61.5,
      best_lap_time: 60.25,
    }),
    true,
  );
});

test("rejects invalid progress", () => {
  assert.equal(isRaceState({ ...validRaceState(), progress: -0.1 }), false);
  assert.equal(isRaceState({ ...validRaceState(), progress: 1.1 }), false);
  assert.equal(
    isRaceState({ ...validRaceState(), progress: Number.NaN }),
    false,
  );
  assert.equal(
    isRaceStateView({ available: true, ...validRaceState(), progress: 2 }),
    false,
  );
});

test("rejects missing required fields", () => {
  for (const field of [
    "lap",
    "sector",
    "progress",
    "distance_m",
    "wrong_way",
    "started",
    "current_lap_time",
    "last_lap_time",
    "best_lap_time",
  ] as const) {
    const view: Record<string, unknown> = { available: true, ...validRaceState() };
    delete view[field];
    assert.equal(isRaceStateView(view), false, `missing ${field}`);
  }
});

test("rejects negative and non-finite times", () => {
  assert.equal(isRaceState({ ...validRaceState(), current_lap_time: -1 }), false);
  assert.equal(isRaceState({ ...validRaceState(), distance_m: -1 }), false);
  assert.equal(isRaceState({ ...validRaceState(), last_lap_time: -1 }), false);
  assert.equal(isRaceState({ ...validRaceState(), best_lap_time: -1 }), false);
  assert.equal(
    isRaceState({ ...validRaceState(), current_lap_time: Number.POSITIVE_INFINITY }),
    false,
  );
});

test("rejects invalid lap and sector numbers", () => {
  assert.equal(isRaceState({ ...validRaceState(), lap: 0 }), false);
  assert.equal(isRaceState({ ...validRaceState(), lap: 1.5 }), false);
  assert.equal(isRaceState({ ...validRaceState(), sector: 0 }), false);
});

test("parseRaceStateView accepts valid views and rejects malformed input", () => {
  const view = { available: true, ...validRaceState() } as const;
  assert.deepEqual(parseRaceStateView(JSON.stringify(view)), view);
  assert.deepEqual(parseRaceStateView(JSON.stringify({ available: false })), {
    available: false,
  });

  assert.throws(() => parseRaceStateView("not json"));
  assert.throws(() => parseRaceStateView("null"));
  assert.throws(() => parseRaceStateView(JSON.stringify({ available: true })));
  assert.throws(
    () =>
      parseRaceStateView(
        JSON.stringify({ available: true, ...validRaceState(), progress: -1 }),
      ),
    /invalid race state view/,
  );
});
