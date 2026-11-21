export const RACE_DATA_VERSION = 1;

export type Vec3 = [number, number, number];
export type Quat = [number, number, number, number];
export type LayoutDirection = "forward" | "reverse";

export interface StartFinish {
  section_id: string;
  fraction: number;
  position: Vec3;
  rotation_xyzw: Quat;
}

export interface GridSlot {
  name: string;
  position: Vec3;
  rotation_xyzw: Quat;
}

export interface Sector {
  name: string;
  start_fraction: number;
  end_fraction: number;
}

export interface Layout {
  layout_id: string;
  closed: boolean;
  direction: LayoutDirection;
  points: Vec3[];
  half_width: number;
}

export interface RaceData {
  version: number;
  layout: Layout;
  start_finish: StartFinish;
  grid: GridSlot[];
  sectors: Sector[];
}

export type RaceView =
  | { available: false }
  | ({ available: true } & RaceData);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isVec3(value: unknown): value is Vec3 {
  return (
    Array.isArray(value) &&
    value.length === 3 &&
    value.every(isFiniteNumber)
  );
}

function isQuat(value: unknown): value is Quat {
  return (
    Array.isArray(value) &&
    value.length === 4 &&
    value.every(isFiniteNumber)
  );
}

function isSpawn(value: unknown): value is GridSlot {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.name === "string" &&
    isVec3(value.position) &&
    isQuat(value.rotation_xyzw)
  );
}

function isSector(value: unknown): value is Sector {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.name === "string" &&
    isFiniteNumber(value.start_fraction) &&
    isFiniteNumber(value.end_fraction)
  );
}

export function isRaceData(value: unknown): value is RaceData {
  if (!isRecord(value) || !isRecord(value.layout)) {
    return false;
  }
  const layout = value.layout;
  if (
    !isFiniteNumber(value.version) ||
    typeof layout.layout_id !== "string" ||
    layout.layout_id.length === 0 ||
    typeof layout.closed !== "boolean" ||
    (layout.direction !== "forward" && layout.direction !== "reverse") ||
    !Array.isArray(layout.points) ||
    !layout.points.every(isVec3) ||
    !isFiniteNumber(layout.half_width)
  ) {
    return false;
  }

  const start = value.start_finish;
  if (
    !isRecord(start) ||
    typeof start.section_id !== "string" ||
    start.section_id.length === 0 ||
    !isFiniteNumber(start.fraction) ||
    !isVec3(start.position) ||
    !isQuat(start.rotation_xyzw)
  ) {
    return false;
  }

  if (!Array.isArray(value.grid) || !value.grid.every(isSpawn)) {
    return false;
  }
  if (!Array.isArray(value.sectors) || !value.sectors.every(isSector)) {
    return false;
  }

  return true;
}

export function isRaceView(value: unknown): value is RaceView {
  if (!isRecord(value) || typeof value.available !== "boolean") {
    return false;
  }
  if (!value.available) {
    return true;
  }
  return isRaceData(value);
}

export function parseRaceView(json: string): RaceView {
  const value: unknown = JSON.parse(json);
  if (!isRaceView(value)) {
    throw new Error("invalid race view");
  }
  return value;
}
