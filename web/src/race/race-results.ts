import type { RaceView } from "./race-director.ts";

export interface RaceResultRow {
  position: number;
  name: string;
  laps: number;
  time: number | null;
  timeText: string;
}

/** Formats a race result time as `m:ss.ss`, or a dash when unavailable. */
export function formatRaceResultTime(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) {
    return "—";
  }
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds - minutes * 60;
  return `${minutes}:${remainder.toFixed(2).padStart(5, "0")}`;
}

/**
 * Builds ordered results rows from a race director view.
 *
 * Unfinished cars keep their current position and show a dash for time.
 */
export function buildRaceResultRows(view: RaceView): RaceResultRow[] {
  return [...view.cars]
    .sort((a, b) => a.position - b.position)
    .map((car) => {
      const finished = car.finished && car.finishTime !== null;
      const laps = Math.min(Math.max(1, car.lap), view.totalLaps);
      return {
        position: car.position,
        name: car.isPlayer ? "You" : `AI ${car.slot}`,
        laps,
        time: finished ? car.finishTime : null,
        timeText: formatRaceResultTime(finished ? car.finishTime : null),
      };
    });
}
