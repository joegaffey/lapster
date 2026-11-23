import type { Layout } from "../contracts/race.ts";
import type { RaceStateView } from "../contracts/race-state.ts";

export interface RaceCarState {
  slot: number;
  isPlayer: boolean;
  lap: number;
  sector: number;
  progress: number;
  distanceM: number;
  lastLapTime: number | null;
  bestLapTime: number | null;
  finished: boolean;
  finishTime: number | null;
}

export interface RaceStanding extends RaceCarState {
  position: number;
  totalDistanceM: number;
}

export interface RaceView {
  totalLaps: number;
  elapsed: number;
  cars: RaceStanding[];
  playerPosition: number;
  playerFinished: boolean;
}

export interface RaceDirectorOptions {
  totalLaps: number;
  layoutLength: number;
  aiSlots?: number[];
  now?: () => number;
}

interface InternalCar extends RaceCarState {
  finished: boolean;
  finishTime: number | null;
  gridOffset: number | null;
}

const PLAYER_SLOT = 0;

/** Total centreline length of a race layout in metres. */
export function raceLayoutLength(layout: Layout): number {
  const points = layout.points;
  if (!Array.isArray(points) || points.length < 2) {
    return 0;
  }
  const segments = layout.closed ? points.length : points.length - 1;
  let total = 0;
  for (let index = 0; index < segments; index += 1) {
    const start = points[index];
    const end = points[(index + 1) % points.length];
    const dx = end[0] - start[0];
    const dy = end[1] - start[1];
    const dz = end[2] - start[2];
    total += Math.hypot(dx, dy, dz);
  }
  return total;
}

function initialCar(slot: number, isPlayer: boolean): InternalCar {
  return {
    slot,
    isPlayer,
    lap: 1,
    sector: 1,
    progress: 0,
    distanceM: 0,
    lastLapTime: null,
    bestLapTime: null,
    finished: false,
    finishTime: null,
    gridOffset: null,
  };
}

/**
 * Pure race standings model built from the local player's and AI cars'
 * `RaceState` views.
 *
 * Time is injected through `now` so tests and callers can sample it
 * deterministically. Call `start()` at GO; positions are available before the
 * start so the HUD can show the current field.
 */
export class RaceDirector {
  readonly totalLaps: number;
  readonly layoutLength: number;

  private readonly now: () => number;
  private readonly aiSlots: number[];
  private readonly cars = new Map<number, InternalCar>();
  private startTime: number | null = null;

  constructor(options: RaceDirectorOptions) {
    this.totalLaps = Math.max(1, Math.trunc(options.totalLaps));
    this.layoutLength = Math.max(0, options.layoutLength);
    this.now = options.now ?? (() => performance.now());
    this.aiSlots = [...(options.aiSlots ?? [1, 2, 3])];
    this.reset();
  }

  /** Starts the race clock. Calling again keeps the first start time. */
  start(now: number = this.now()): void {
    if (this.startTime === null) {
      this.startTime = now;
    }
  }

  /** Clears lap progress, finish records, and the race clock. */
  reset(): void {
    this.startTime = null;
    this.cars.clear();
    this.cars.set(PLAYER_SLOT, initialCar(PLAYER_SLOT, true));
    for (const slot of this.aiSlots) {
      if (slot === PLAYER_SLOT) {
        continue;
      }
      this.cars.set(slot, initialCar(slot, false));
    }
  }

  get started(): boolean {
    return this.startTime !== null;
  }

  elapsedSeconds(now: number = this.now()): number {
    if (this.startTime === null) {
      return 0;
    }
    return Math.max(0, (now - this.startTime) / 1000);
  }

  updatePlayer(view: RaceStateView, now: number = this.now()): void {
    this.updateCar(PLAYER_SLOT, view, now);
  }

  updateAi(slot: number, view: RaceStateView, now: number = this.now()): void {
    if (slot === PLAYER_SLOT || slot < 0) {
      return;
    }
    this.updateCar(slot, view, now);
  }

  view(now: number = this.now()): RaceView {
    const standings = [...this.cars.values()].map((car) =>
      this.standing(car),
    );
    standings.sort(compareStandings);
    standings.forEach((car, index) => {
      car.position = index + 1;
    });
    const player = standings.find((car) => car.isPlayer);
    return {
      totalLaps: this.totalLaps,
      elapsed: this.elapsedSeconds(now),
      cars: standings,
      playerPosition: player?.position ?? 1,
      playerFinished: player?.finished ?? false,
    };
  }

  private updateCar(
    slot: number,
    view: RaceStateView,
    now: number,
  ): void {
    if (!view.available) {
      return;
    }
    let car = this.cars.get(slot);
    if (!car) {
      car = initialCar(slot, slot === PLAYER_SLOT);
      this.cars.set(slot, car);
    }
    car.lap = view.lap;
    car.sector = view.sector;
    car.progress = view.progress;
    car.distanceM = view.distance_m;
    car.lastLapTime = view.last_lap_time;
    car.bestLapTime = view.best_lap_time;
    car.started = view.started;
    if (car.gridOffset === null) {
      car.gridOffset = view.distance_m;
    }
    if (!car.finished && view.lap > this.totalLaps) {
      car.finished = true;
      car.finishTime = this.elapsedSeconds(now);
    }
  }

  private standing(car: InternalCar): RaceStanding {
    return {
      ...car,
      position: 0,
      totalDistanceM: this.totalDistance(car),
    };
  }

  private totalDistance(car: InternalCar): number {
    const completedLaps = Math.max(0, car.lap - 1);
    const offset = car.gridOffset ?? 0;
    const behindStartLine = offset > this.layoutLength / 2;
    let total: number;
    if (behindStartLine) {
      if (!car.started) {
        return 0;
      }
      total =
        this.layoutLength -
        offset +
        completedLaps * this.layoutLength +
        car.distanceM;
    } else {
      total =
        completedLaps * this.layoutLength + car.distanceM - offset;
    }
    return Math.max(0, total);
  }
}

function compareStandings(a: RaceStanding, b: RaceStanding): number {
  if (a.finished && b.finished) {
    const timeA = a.finishTime ?? Number.POSITIVE_INFINITY;
    const timeB = b.finishTime ?? Number.POSITIVE_INFINITY;
    return timeA - timeB || a.slot - b.slot;
  }
  if (a.finished) {
    return -1;
  }
  if (b.finished) {
    return 1;
  }
  return b.totalDistanceM - a.totalDistanceM || a.slot - b.slot;
}
