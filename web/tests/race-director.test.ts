import test from "node:test";
import assert from "node:assert/strict";

import {
  RaceDirector,
  raceLayoutLength,
} from "../src/race/race-director.ts";
import type { RaceState } from "../src/contracts/race-state.ts";
import type { Layout } from "../src/contracts/race.ts";

function state(overrides: Partial<RaceState> = {}): RaceState {
  return {
    lap: 1,
    sector: 1,
    progress: 0,
    distance_m: 0,
    wrong_way: false,
    started: true,
    current_lap_time: 0,
    last_lap_time: null,
    best_lap_time: null,
    ...overrides,
  };
}

function gridState(overrides: Partial<RaceState> = {}): RaceState {
  return state({ started: false, ...overrides });
}

function director(totalLaps = 3, aiSlots = [1, 2, 3], now = () => 0) {
  return new RaceDirector({
    totalLaps,
    layoutLength: 100,
    aiSlots,
    now,
  });
}

function placeOnGrid(race: RaceDirector, distance = 0): void {
  race.updatePlayer({ available: true, ...gridState({ distance_m: distance }) });
  for (const slot of [1, 2, 3]) {
    race.updateAi(slot, {
      available: true,
      ...gridState({ distance_m: distance }),
    });
  }
}

test("orders unfinished cars by total distance including completed laps", () => {
  const race = director();
  placeOnGrid(race);
  race.updatePlayer({ available: true, ...state({ lap: 1, distance_m: 10 }) });
  race.updateAi(1, { available: true, ...state({ lap: 2, distance_m: 5 }) });
  race.updateAi(2, { available: true, ...state({ lap: 1, distance_m: 30 }) });
  race.updateAi(3, { available: true, ...state({ lap: 1, distance_m: 20 }) });

  const view = race.view();
  assert.deepEqual(
    view.cars.map((car) => car.slot),
    [1, 2, 3, 0],
  );
  assert.deepEqual(
    view.cars.map((car) => car.totalDistanceM),
    [105, 30, 20, 10],
  );
  assert.equal(view.playerPosition, 4);
});

test("grid states rank by slot before movement", () => {
  const race = director();
  race.updatePlayer({
    available: true,
    ...gridState({ distance_m: 95 }),
  });
  race.updateAi(1, { available: true, ...gridState({ distance_m: 90 }) });
  race.updateAi(2, { available: true, ...gridState({ distance_m: 85 }) });
  race.updateAi(3, { available: true, ...gridState({ distance_m: 80 }) });

  const view = race.view();
  assert.deepEqual(
    view.cars.map((car) => car.slot),
    [0, 1, 2, 3],
  );
  assert.deepEqual(
    view.cars.map((car) => car.totalDistanceM),
    [0, 0, 0, 0],
  );
});

test("a car behind the line gains distance after the start crossing", () => {
  const race = director();
  race.updatePlayer({
    available: true,
    ...gridState({ distance_m: 95 }),
  });
  assert.equal(race.view().cars[0].totalDistanceM, 0);

  race.updatePlayer({
    available: true,
    ...state({ lap: 1, distance_m: 2, started: true }),
  });
  const distance = race.view().cars[0].totalDistanceM;
  assert.ok(distance > 0, `expected positive distance, got ${distance}`);
  assert.ok(distance < 20, `expected a small distance, got ${distance}`);
});

test("an ahead-of-line grid car starts at zero relative distance", () => {
  const race = director();
  race.updatePlayer({
    available: true,
    ...gridState({ distance_m: 20 }),
  });
  assert.equal(race.view().cars[0].totalDistanceM, 0);

  race.updatePlayer({
    available: true,
    ...state({ lap: 1, distance_m: 25, started: true }),
  });
  assert.equal(race.view().cars[0].totalDistanceM, 5);
});

test("completed laps continue monotonically", () => {
  const race = director(3, []);
  const samples = [
    gridState({ distance_m: 95 }),
    state({ lap: 1, distance_m: 2 }),
    state({ lap: 1, distance_m: 50 }),
    state({ lap: 2, distance_m: 2 }),
    state({ lap: 2, distance_m: 50 }),
    state({ lap: 3, distance_m: 2 }),
  ];
  let previous = -1;
  for (const sample of samples) {
    race.updatePlayer({ available: true, ...sample });
    const distance = race.view().cars[0].totalDistanceM;
    assert.ok(
      distance > previous,
      `expected distance ${distance} to exceed ${previous}`,
    );
    previous = distance;
  }
});

test("finished cars sort ahead of unfinished cars by finish time", () => {
  let clock = 0;
  const race = director(2, [1, 2, 3], () => clock);
  placeOnGrid(race);
  race.start(0);
  clock = 50_000;
  race.updateAi(1, { available: true, ...state({ lap: 2, distance_m: 90 }) });
  clock = 60_000;
  race.updateAi(2, { available: true, ...state({ lap: 3, distance_m: 5 }) }, 60_000);
  clock = 70_000;
  race.updatePlayer(
    { available: true, ...state({ lap: 3, distance_m: 10 }) },
    70_000,
  );
  clock = 80_000;
  race.updateAi(3, { available: true, ...state({ lap: 3, distance_m: 90 }) }, 80_000);

  const view = race.view();
  assert.deepEqual(
    view.cars.map((car) => car.slot),
    [2, 0, 3, 1],
  );
  assert.deepEqual(
    view.cars.map((car) => car.finishTime),
    [60, 70, 80, null],
  );
  assert.equal(view.playerFinished, true);
  assert.equal(view.playerPosition, 2);
  assert.equal(view.elapsed, 80);
});

test("records finish once and keeps the first observed finish time", () => {
  const race = director(1, [], () => 0);
  race.updatePlayer({ available: true, ...gridState() }, 1_000);
  race.start(1_000);
  race.updatePlayer({ available: true, ...state({ lap: 2, distance_m: 0 }) }, 4_000);
  race.updatePlayer({ available: true, ...state({ lap: 3, distance_m: 5 }) }, 9_000);

  const view = race.view(10_000);
  const player = view.cars[0];
  assert.equal(player.finished, true);
  assert.equal(player.finishTime, 3);
  assert.equal(view.playerFinished, true);
});

test("ignores unavailable states and keeps the field size", () => {
  const race = director(3, [1, 2, 3]);
  race.updatePlayer({ available: true, ...state() });
  race.updateAi(1, { available: false });

  const view = race.view();
  assert.equal(view.cars.length, 4);
  assert.equal(view.playerPosition, 1);
  assert.equal(view.cars.filter((car) => car.finished).length, 0);
});

test("reset clears progress and the race clock", () => {
  let clock = 0;
  const race = director(2, [1, 2], () => clock);
  placeOnGrid(race);
  race.start(0);
  clock = 5_000;
  race.updatePlayer({ available: true, ...state({ lap: 2, distance_m: 40 }) }, 5_000);
  clock = 7_000;
  race.reset();

  const view = race.view(7_000);
  assert.equal(view.elapsed, 0);
  assert.equal(view.playerPosition, 1);
  assert.equal(view.playerFinished, false);
  assert.equal(view.cars.length, 3);
  assert.ok(view.cars.every((car) => car.lap === 1 && car.totalDistanceM === 0));
});

test("raceLayoutLength follows closed and open layouts", () => {
  const closed: Layout = {
    layout_id: "test",
    closed: true,
    direction: "forward",
    points: [
      [0, 0, 0],
      [3, 0, 0],
      [3, 0, 4],
    ],
    half_width: 2,
  };
  const open: Layout = { ...closed, closed: false };
  assert.equal(raceLayoutLength(closed), 12);
  assert.equal(raceLayoutLength(open), 7);
});
