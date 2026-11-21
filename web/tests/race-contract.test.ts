import test from "node:test";
import assert from "node:assert/strict";

import {
  RACE_DATA_VERSION,
  isRaceView,
  parseRaceView,
  type RaceData,
} from "../src/contracts/race.ts";

function validRaceData(): RaceData {
  return {
    version: RACE_DATA_VERSION,
    layout: {
      layout_id: "main",
      closed: true,
      direction: "forward",
      points: [
        [0, 0, 0],
        [10, 0, 0],
        [10, 0, 10],
      ],
      half_width: 5,
    },
    start_finish: {
      section_id: "main",
      fraction: 0,
      position: [0, 0, 0],
      rotation_xyzw: [0, 0, 0, 1],
    },
    grid: [
      {
        name: "grid_0",
        position: [0, 0, 2],
        rotation_xyzw: [0, 0, 0, 1],
      },
    ],
    sectors: [
      { name: "sector_1", start_fraction: 0, end_fraction: 1 },
    ],
  };
}

function validRaceView(): { available: true } & RaceData {
  return { available: true, ...validRaceData() };
}

test("accepts a race view that is unavailable", () => {
  assert.equal(isRaceView({ available: false }), true);
});

test("accepts a valid available race view", () => {
  assert.equal(isRaceView(validRaceView()), true);
});

test("rejects a race view with a missing or invalid layout", () => {
  const view = validRaceView();

  assert.equal(isRaceView({ ...view, layout: undefined }), false);
  assert.equal(isRaceView({ ...view, layout: { ...view.layout, layout_id: "" } }), false);
  assert.equal(
    isRaceView({ ...view, layout: { ...view.layout, direction: "sideways" } }),
    false,
  );
  assert.equal(isRaceView({ ...view, layout: { ...view.layout, points: [[0, 0]] } }), false);
});

test("rejects a race view with missing or invalid sectors", () => {
  const view = validRaceView();

  assert.equal(isRaceView({ ...view, sectors: undefined }), false);
  assert.equal(isRaceView({ ...view, sectors: "sectors" }), false);
  assert.equal(
    isRaceView({ ...view, sectors: [{ name: "sector_1", start_fraction: 0 }] }),
    false,
  );
});

test("parseRaceView round-trips a valid available view", () => {
  const view = validRaceView();

  assert.deepEqual(parseRaceView(JSON.stringify(view)), view);
  assert.deepEqual(parseRaceView(JSON.stringify({ available: false })), {
    available: false,
  });
});

test("parseRaceView rejects invalid JSON and invalid views", () => {
  assert.throws(() => parseRaceView("not json"));
  assert.throws(() => parseRaceView(JSON.stringify({ available: true })), /invalid race view/);
  assert.throws(
    () =>
      parseRaceView(
        JSON.stringify({
          ...validRaceView(),
          sectors: [{ name: "sector_1", start_fraction: 0 }],
        }),
      ),
    /invalid race view/,
  );
});
