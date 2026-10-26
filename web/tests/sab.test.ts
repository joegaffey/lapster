import test from "node:test";
import assert from "node:assert/strict";

import {
  SAB_LAYOUT_VERSION,
  SAB_TOTAL_SIZE,
  SNAPSHOT_FIELDS,
  SNAPSHOT_STATE,
  SNAPSHOT_STATE_FLOATS,
  snapshotSlotOffset,
} from "../src/types/asset-layout.ts";
import {
  clearReset,
  createSnapshotReader,
  getSabFlags,
  initializeSab,
  publishSnapshot,
  readInput,
  requestReset,
  setPaused,
  validateSab,
  writeFfb,
  writeInput,
  type InputState,
} from "../src/sim/sab.ts";

function makeSab(): SharedArrayBuffer {
  const sab = new SharedArrayBuffer(SAB_TOTAL_SIZE);
  initializeSab(sab);
  validateSab(sab);
  return sab;
}

function state(): Float32Array {
  return new Float32Array(SNAPSHOT_STATE_FLOATS);
}

test("initializes and validates the SAB header", () => {
  const sab = makeSab();
  const view = new DataView(sab);
  assert.equal(String.fromCharCode(...new Uint8Array(sab, 0, 4)), "LSIM");
  assert.equal(view.getUint16(4, true), SAB_LAYOUT_VERSION);
  assert.equal(view.getUint16(6, true), 64);
  assert.equal(createSnapshotReader(sab).readLatest(), null);
});

test("publishes and reads snapshots", () => {
  const sab = makeSab();
  const reader = createSnapshotReader(sab);
  const payload = state();
  payload[SNAPSHOT_STATE.position] = 1.25;
  payload[SNAPSHOT_STATE.position + 2] = -3.5;
  payload[SNAPSHOT_STATE.gear] = -1;
  publishSnapshot(sab, 7, 123.5, payload);

  const snapshot = reader.readLatest();
  assert.ok(snapshot);
  assert.equal(snapshot.tick, 7);
  assert.equal(snapshot.timestampSeconds, 123.5);
  assert.equal(snapshot.state[SNAPSHOT_STATE.position], 1.25);
  assert.equal(snapshot.state[SNAPSHOT_STATE.position + 2], -3.5);
  assert.equal(snapshot.state[SNAPSHOT_STATE.gear], -1);
});

test("ignores a snapshot while its seqlock is odd", () => {
  const sab = makeSab();
  const reader = createSnapshotReader(sab);
  const payload = state();
  publishSnapshot(sab, 3, 1, payload);
  const slot = snapshotSlotOffset(3);
  const seq = new Int32Array(sab, slot, 1);
  Atomics.store(seq, 0, 1);
  assert.equal(reader.readLatest(), null);
  Atomics.store(seq, 0, 2);
  assert.equal(reader.readLatest()?.tick, 3);
});

test("writes and reads the input mailbox", () => {
  const sab = makeSab();
  const previous: InputState = {
    steering: 0,
    throttle: 0,
    brake: 0,
    clutch: 0,
    gear: 1,
    buttons: 0,
    timestampSeconds: 0,
  };
  writeInput(sab, {
    steering: 0.5,
    throttle: 1,
    brake: 0.25,
    clutch: 0,
    gear: -1,
    buttons: 3,
    timestampSeconds: 42,
  });
  const input = readInput(sab, previous);
  assert.equal(input.steering, 0.5);
  assert.equal(input.throttle, 1);
  assert.equal(input.brake, 0.25);
  assert.equal(input.gear, -1);
  assert.equal(input.buttons, 3);
  assert.equal(input.timestampSeconds, 42);
});

test("updates pause and reset flags", () => {
  const sab = makeSab();
  setPaused(sab, true);
  assert.equal(getSabFlags(sab) & 1, 1);
  setPaused(sab, false);
  assert.equal(getSabFlags(sab) & 1, 0);
  requestReset(sab);
  assert.equal(getSabFlags(sab) & 2, 2);
  clearReset(sab);
  assert.equal(getSabFlags(sab) & 2, 0);
});

test("writes the FFB mailbox without exposing a torn sequence", () => {
  const sab = makeSab();
  writeFfb(sab, 0.4, true);
  const view = new DataView(sab);
  assert.equal(view.getFloat32(1280 + 16, true), Math.fround(0.4));
  assert.equal(view.getUint32(1280 + 4, true), 1);
});

test("snapshot slot fields match the payload offset", () => {
  const sab = makeSab();
  publishSnapshot(sab, 1, 1, state());
  const slot = snapshotSlotOffset(1);
  assert.equal(new Float32Array(sab, slot + SNAPSHOT_FIELDS.state, SNAPSHOT_STATE_FLOATS).length, 64);
});
