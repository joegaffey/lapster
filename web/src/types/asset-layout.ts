export const SAB_MAGIC = "LSIM";
export const SAB_LAYOUT_VERSION = 2;
export const SAB_TOTAL_SIZE = 4096;

export const HEADER_OFFSET = 0;
export const HEADER_SIZE = 64;

export const SNAPSHOT_OFFSET = 64;
export const SNAPSHOT_COUNT = 4;
export const SNAPSHOT_STRIDE = 288;
export const SNAPSHOT_REGION_SIZE = SNAPSHOT_COUNT * SNAPSHOT_STRIDE;

export const INPUT_MAILBOX_OFFSET = 1216;
export const INPUT_MAILBOX_SIZE = 64;

export const FFB_MAILBOX_OFFSET = 1280;
export const FFB_MAILBOX_SIZE = 64;

export const RESERVED_OFFSET = 1344;
export const RESERVED_SIZE = 2752;

export const HEADER_FLAG_PAUSED = 1 << 0;
export const HEADER_FLAG_RESET = 1 << 1;
export const FFB_FLAG_ACTIVE = 1 << 0;
export const FFB_FLAG_STOP = 1 << 1;

export const HEADER_FIELDS = {
  magic: 0,
  layoutVersion: 4,
  headerSize: 6,
  slotCount: 8,
  slotStride: 12,
  latestSlot: 16,
  flags: 20,
  writerTick: 24,
  reserved0: 28,
  timeBaseSeconds: 32,
  reserved1: 40,
} as const;

export const SNAPSHOT_FIELDS = {
  seq: 0,
  reserved0: 4,
  tick: 8,
  timestampSeconds: 16,
  reserved1: 24,
  state: 32,
} as const;

export const SNAPSHOT_STATE_FLOATS = 64;

export const SNAPSHOT_STATE = {
  position: 0,
  rotation: 3,
  velocity: 7,
  angularVelocity: 10,
  wheelAngularSpeeds: 13,
  suspensionTravel: 17,
  engineRpm: 21,
  rackTorque: 22,
  throttle: 23,
  brake: 24,
  slipRatio: 25,
  slipAngle: 29,
  tireLoad: 33,
  contactFlags: 37,
  surfaceFriction: 41,
  wheelSteerAngle: 45,
  wheelSpinAngle: 49,
  contactCenter: 53,
  contactNormal: 56,
  gear: 59,
  absActive: 60,
  escActivity: 61,
  tick: 62,
  surfaceIds: 63,
} as const;

export const INPUT_MAILBOX_FIELDS = {
  seq: 0,
  buttons: 4,
  timestampSeconds: 8,
  steering: 16,
  throttle: 20,
  brake: 24,
  clutch: 28,
  gear: 32,
  reserved: 36,
} as const;

export const FFB_MAILBOX_FIELDS = {
  seq: 0,
  flags: 4,
  timestampSeconds: 8,
  torque: 16,
  reserved: 20,
} as const;

export function snapshotSlotOffset(index: number): number {
  if (!Number.isInteger(index) || index < 0 || index >= SNAPSHOT_COUNT) {
    throw new RangeError(`snapshot index ${index} is out of range`);
  }
  return SNAPSHOT_OFFSET + index * SNAPSHOT_STRIDE;
}
