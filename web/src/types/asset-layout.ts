export const SAB_MAGIC = "LSIM";
export const SAB_LAYOUT_VERSION = 1;
export const SAB_TOTAL_SIZE = 4096;

export const HEADER_OFFSET = 0;
export const HEADER_SIZE = 64;

export const SNAPSHOT_OFFSET = 64;
export const SNAPSHOT_COUNT = 4;
export const SNAPSHOT_STRIDE = 256;
export const SNAPSHOT_REGION_SIZE = SNAPSHOT_COUNT * SNAPSHOT_STRIDE;

export const INPUT_MAILBOX_OFFSET = 1088;
export const INPUT_MAILBOX_SIZE = 64;

export const FFB_MAILBOX_OFFSET = 1152;
export const FFB_MAILBOX_SIZE = 64;

export const RESERVED_OFFSET = 1216;
export const RESERVED_SIZE = 2880;

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
  position: 32,
  rotationXyzw: 48,
  velocity: 64,
  angularVelocity: 80,
  wheelAngularSpeeds: 96,
  suspensionTravel: 112,
  engineRpmRackTorqueThrottleBrake: 128,
  slipRatio: 144,
  slipAngle: 160,
  tireLoad: 176,
  contactFlags: 192,
  surfaceFriction: 208,
  reserved2: 224,
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
