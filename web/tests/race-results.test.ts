import test from "node:test";
import assert from "node:assert/strict";

import {
  buildRaceResultRows,
  formatRaceResultTime,
} from "../src/race/race-results.ts";
import type { RaceStanding, RaceView } from "../src/race/race-director.ts";

function standing(
  overrides: Partial<RaceStanding> = {},
): RaceStanding {
  return {
    slot: 1,
    isPlayer: false,
    lap: 1,
    sector: 1,
    progress: 0,
    distanceM: 0,
    lastLapTime: null,
    bestLapTime: null,
    finished: false,
    finishTime: null,
    position: 1,
    totalDistanceM: 0,
    ...overrides,
  };
}

function view(cars: RaceStanding[]): RaceView {
  return {
    totalLaps: 3,
    elapsed: 120,
    cars,
    playerPosition: cars.find((car) => car.isPlayer)?.position ?? 1,
    playerFinished: cars.some((car) => car.isPlayer && car.finished),
  };
}

test("orders rows by position with player and AI names", () => {
  const rows = buildRaceResultRows(
    view([
      standing({ slot: 2, position: 3, lap: 2 }),
      standing({ slot: 0, isPlayer: true, position: 2, lap: 4, finished: true, finishTime: 95.5 }),
      standing({ slot: 1, position: 1, lap: 4, finished: true, finishTime: 92 }),
    ]),
  );

  assert.deepEqual(
    rows.map((row) => row.position),
    [1, 2, 3],
  );
  assert.deepEqual(
    rows.map((row) => row.name),
    ["AI 1", "You", "AI 2"],
  );
  assert.deepEqual(
    rows.map((row) => row.laps),
    [3, 3, 2],
  );
  assert.deepEqual(
    rows.map((row) => row.timeText),
    ["1:32.00", "1:35.50", "—"],
  );
});

test("unfinished cars keep their current position and dash time", () => {
  const rows = buildRaceResultRows(
    view([
      standing({ slot: 0, isPlayer: true, position: 2, lap: 3, finished: true, finishTime: 60 }),
      standing({ slot: 1, position: 1, lap: 2, finished: false, finishTime: null }),
      standing({ slot: 2, position: 3, lap: 1, finished: false, finishTime: null }),
    ]),
  );

  assert.equal(rows[0].name, "AI 1");
  assert.equal(rows[0].time, null);
  assert.equal(rows[0].timeText, "—");
  assert.equal(rows[1].name, "You");
  assert.equal(rows[1].time, 60);
  assert.equal(rows[1].timeText, "1:00.00");
  assert.equal(rows[2].timeText, "—");
});

test("formats result times and unavailable values", () => {
  assert.equal(formatRaceResultTime(0), "0:00.00");
  assert.equal(formatRaceResultTime(61.234), "1:01.23");
  assert.equal(formatRaceResultTime(null), "—");
  assert.equal(formatRaceResultTime(Number.NaN), "—");
  assert.equal(formatRaceResultTime(-1), "—");
});
