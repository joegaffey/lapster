import {
  FFB_MAILBOX_FIELDS,
  FFB_MAILBOX_OFFSET,
  HEADER_FIELDS,
  INPUT_MAILBOX_FIELDS,
  INPUT_MAILBOX_OFFSET,
  SAB_LAYOUT_VERSION,
  SAB_MAGIC,
  SAB_TOTAL_SIZE,
  SNAPSHOT_COUNT,
  SNAPSHOT_FIELDS,
  SNAPSHOT_STATE_FLOATS,
  SNAPSHOT_STRIDE,
  snapshotSlotOffset,
} from "../types/asset-layout.ts";

export interface InputState {
  steering: number;
  throttle: number;
  brake: number;
  clutch: number;
  gear: number;
  buttons: number;
  timestampSeconds: number;
}

export interface Snapshot {
  tick: number;
  timestampSeconds: number;
  state: Float32Array;
}

export interface SnapshotReader {
  readLatest(): Snapshot | null;
}

const MAGIC_BYTES = new TextEncoder().encode(SAB_MAGIC);

export function initializeSab(sab: SharedArrayBuffer): void {
  if (sab.byteLength < SAB_TOTAL_SIZE) {
    throw new Error(`SAB is ${sab.byteLength} bytes; expected ${SAB_TOTAL_SIZE}`);
  }
  new Uint8Array(sab).fill(0);
  const bytes = new Uint8Array(sab);
  bytes.set(MAGIC_BYTES, HEADER_FIELDS.magic);
  const view = new DataView(sab);
  view.setUint16(HEADER_FIELDS.layoutVersion, SAB_LAYOUT_VERSION, true);
  view.setUint16(HEADER_FIELDS.headerSize, 64, true);
  view.setUint32(HEADER_FIELDS.slotCount, SNAPSHOT_COUNT, true);
  view.setUint32(HEADER_FIELDS.slotStride, SNAPSHOT_STRIDE, true);
  view.setInt32(HEADER_FIELDS.latestSlot, -1, true);
  view.setFloat64(HEADER_FIELDS.timeBaseSeconds, Date.now() / 1000, true);
}

export function validateSab(sab: SharedArrayBuffer): void {
  if (sab.byteLength < SAB_TOTAL_SIZE) {
    throw new Error(`SAB is ${sab.byteLength} bytes; expected ${SAB_TOTAL_SIZE}`);
  }
  const bytes = new Uint8Array(sab);
  const magic = String.fromCharCode(...bytes.subarray(0, 4));
  if (magic !== SAB_MAGIC) {
    throw new Error(`SAB magic is ${magic}; expected ${SAB_MAGIC}`);
  }
  const view = new DataView(sab);
  const version = view.getUint16(HEADER_FIELDS.layoutVersion, true);
  if (version !== SAB_LAYOUT_VERSION) {
    throw new Error(`SAB layout ${version}; expected ${SAB_LAYOUT_VERSION}`);
  }
  const slotCount = view.getUint32(HEADER_FIELDS.slotCount, true);
  const slotStride = view.getUint32(HEADER_FIELDS.slotStride, true);
  if (slotCount !== SNAPSHOT_COUNT || slotStride !== SNAPSHOT_STRIDE) {
    throw new Error(`SAB snapshot layout is ${slotCount}x${slotStride}`);
  }
}

function headerInts(sab: SharedArrayBuffer): Int32Array {
  return new Int32Array(sab, 0, SAB_TOTAL_SIZE / 4);
}

export function setSabFlags(sab: SharedArrayBuffer, flags: number): void {
  AtomicStore(headerInts(sab), HEADER_FIELDS.flags / 4, flags);
}

export function getSabFlags(sab: SharedArrayBuffer): number {
  return AtomicLoad(headerInts(sab), HEADER_FIELDS.flags / 4);
}

export function setPaused(sab: SharedArrayBuffer, paused: boolean): void {
  const ints = headerInts(sab);
  let flags = AtomicLoad(ints, HEADER_FIELDS.flags / 4);
  flags = paused ? flags | 1 : flags & ~1;
  AtomicStore(ints, HEADER_FIELDS.flags / 4, flags);
}

export function requestReset(sab: SharedArrayBuffer): void {
  const ints = headerInts(sab);
  const index = HEADER_FIELDS.flags / 4;
  AtomicStore(ints, index, AtomicLoad(ints, index) | 2);
}

export function clearReset(sab: SharedArrayBuffer): void {
  const ints = headerInts(sab);
  const index = HEADER_FIELDS.flags / 4;
  AtomicStore(ints, index, AtomicLoad(ints, index) & ~2);
}

export function writeInput(sab: SharedArrayBuffer, input: InputState): void {
  const ints = new Int32Array(sab, INPUT_MAILBOX_OFFSET, INPUT_MAILBOX_FIELDS.reserved / 4 + 7);
  const view = new DataView(sab);
  const seqIndex = INPUT_MAILBOX_FIELDS.seq / 4;
  const seq = AtomicLoad(ints, seqIndex);
  AtomicStore(ints, seqIndex, seq + 1);
  view.setUint32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.buttons, input.buttons, true);
  view.setFloat64(
    INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.timestampSeconds,
    input.timestampSeconds,
    true,
  );
  view.setFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.steering, input.steering, true);
  view.setFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.throttle, input.throttle, true);
  view.setFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.brake, input.brake, true);
  view.setFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.clutch, input.clutch, true);
  view.setInt32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.gear, input.gear, true);
  AtomicStore(ints, seqIndex, seq + 2);
}

export function readInput(sab: SharedArrayBuffer, previous: InputState): InputState {
  const ints = new Int32Array(sab, INPUT_MAILBOX_OFFSET, 16);
  const view = new DataView(sab);
  const seqIndex = INPUT_MAILBOX_FIELDS.seq / 4;
  const before = AtomicLoad(ints, seqIndex);
  if (before & 1) {
    return previous;
  }
  const next: InputState = {
    buttons: view.getUint32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.buttons, true),
    timestampSeconds: view.getFloat64(
      INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.timestampSeconds,
      true,
    ),
    steering: view.getFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.steering, true),
    throttle: view.getFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.throttle, true),
    brake: view.getFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.brake, true),
    clutch: view.getFloat32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.clutch, true),
    gear: view.getInt32(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_FIELDS.gear, true),
  };
  const after = AtomicLoad(ints, seqIndex);
  return before === after ? next : previous;
}

export function publishSnapshot(
  sab: SharedArrayBuffer,
  tick: number,
  timestampSeconds: number,
  state: Float32Array,
): void {
  if (state.length !== SNAPSHOT_STATE_FLOATS) {
    throw new Error(`snapshot has ${state.length} floats; expected ${SNAPSHOT_STATE_FLOATS}`);
  }
  const slotIndex = tick % SNAPSHOT_COUNT;
  const slot = snapshotSlotOffset(slotIndex);
  const seq = new Int32Array(sab, slot, 1);
  const view = new DataView(sab);
  const current = AtomicLoad(seq, 0);
  AtomicStore(seq, 0, current + 1);
  view.setBigUint64(slot + SNAPSHOT_FIELDS.tick, BigInt(tick), true);
  view.setFloat64(slot + SNAPSHOT_FIELDS.timestampSeconds, timestampSeconds, true);
  new Float32Array(sab, slot + SNAPSHOT_FIELDS.state, SNAPSHOT_STATE_FLOATS).set(state);
  AtomicStore(seq, 0, current + 2);
  const header = headerInts(sab);
  AtomicStore(header, HEADER_FIELDS.latestSlot / 4, slotIndex);
  AtomicStore(header, HEADER_FIELDS.writerTick / 4, tick);
}

export function createSnapshotReader(sab: SharedArrayBuffer): SnapshotReader {
  let output = new Float32Array(SNAPSHOT_STATE_FLOATS);
  return {
    readLatest(): Snapshot | null {
      const header = headerInts(sab);
      const latest = AtomicLoad(header, HEADER_FIELDS.latestSlot / 4);
      if (latest < 0 || latest >= SNAPSHOT_COUNT) {
        return null;
      }
      const slot = snapshotSlotOffset(latest);
      const seq = new Int32Array(sab, slot, 1);
      const view = new DataView(sab);
      for (let attempt = 0; attempt < 2; attempt += 1) {
        const before = AtomicLoad(seq, 0);
        if (before & 1) {
          continue;
        }
        output.set(new Float32Array(sab, slot + SNAPSHOT_FIELDS.state, SNAPSHOT_STATE_FLOATS));
        const tick = Number(view.getBigUint64(slot + SNAPSHOT_FIELDS.tick, true));
        const timestampSeconds = view.getFloat64(slot + SNAPSHOT_FIELDS.timestampSeconds, true);
        const after = AtomicLoad(seq, 0);
        if (before === after) {
          return { tick, timestampSeconds, state: output };
        }
      }
      return null;
    },
  };
}

export function writeFfb(sab: SharedArrayBuffer, torque: number, active: boolean): void {
  const ints = new Int32Array(sab, FFB_MAILBOX_OFFSET, 16);
  const view = new DataView(sab);
  const seqIndex = FFB_MAILBOX_FIELDS.seq / 4;
  const seq = AtomicLoad(ints, seqIndex);
  AtomicStore(ints, seqIndex, seq + 1);
  view.setUint32(
    FFB_MAILBOX_OFFSET + FFB_MAILBOX_FIELDS.flags,
    active ? 1 : 0,
    true,
  );
  view.setFloat64(
    FFB_MAILBOX_OFFSET + FFB_MAILBOX_FIELDS.timestampSeconds,
    Date.now() / 1000,
    true,
  );
  view.setFloat32(FFB_MAILBOX_OFFSET + FFB_MAILBOX_FIELDS.torque, torque, true);
  AtomicStore(ints, seqIndex, seq + 2);
}

const AtomicLoad = (array: Int32Array, index: number): number => Atomics.load(array, index);
const AtomicStore = (array: Int32Array, index: number, value: number): void => {
  Atomics.store(array, index, value);
};
