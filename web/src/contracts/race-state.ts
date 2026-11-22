export interface RaceState {
  lap: number;
  sector: number;
  progress: number;
  distance_m: number;
  wrong_way: boolean;
  started: boolean;
  current_lap_time: number;
  last_lap_time: number | null;
  best_lap_time: number | null;
}

export type RaceStateView =
  | { available: false }
  | ({ available: true } & RaceState);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isNonNegativeNumber(value: unknown): value is number {
  return isFiniteNumber(value) && value >= 0;
}

function isLapNumber(value: unknown): value is number {
  return Number.isInteger(value) && (value as number) >= 1;
}

function isOptionalTime(value: unknown): value is number | null {
  return value === null || isNonNegativeNumber(value);
}

export function isRaceState(value: unknown): value is RaceState {
  if (!isRecord(value)) {
    return false;
  }
  return (
    isLapNumber(value.lap) &&
    isLapNumber(value.sector) &&
    isFiniteNumber(value.progress) &&
    value.progress >= 0 &&
    value.progress <= 1 &&
    isNonNegativeNumber(value.distance_m) &&
    typeof value.wrong_way === "boolean" &&
    typeof value.started === "boolean" &&
    isNonNegativeNumber(value.current_lap_time) &&
    isOptionalTime(value.last_lap_time) &&
    isOptionalTime(value.best_lap_time)
  );
}

export function isRaceStateView(value: unknown): value is RaceStateView {
  if (!isRecord(value) || typeof value.available !== "boolean") {
    return false;
  }
  if (!value.available) {
    return true;
  }
  return isRaceState(value);
}

export function parseRaceStateView(json: string): RaceStateView {
  const value: unknown = JSON.parse(json);
  if (!isRaceStateView(value)) {
    throw new Error("invalid race state view");
  }
  return value;
}
