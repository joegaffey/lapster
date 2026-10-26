import test from "node:test";
import assert from "node:assert/strict";

import {
  FFB_MAILBOX_FIELDS,
  FFB_MAILBOX_OFFSET,
  FFB_MAILBOX_SIZE,
  HEADER_FIELDS,
  HEADER_OFFSET,
  HEADER_SIZE,
  INPUT_MAILBOX_FIELDS,
  INPUT_MAILBOX_OFFSET,
  INPUT_MAILBOX_SIZE,
  RESERVED_OFFSET,
  RESERVED_SIZE,
  SAB_LAYOUT_VERSION,
  SAB_MAGIC,
  SAB_TOTAL_SIZE,
  SNAPSHOT_COUNT,
  SNAPSHOT_FIELDS,
  SNAPSHOT_OFFSET,
  SNAPSHOT_STATE,
  SNAPSHOT_STATE_FLOATS,
  SNAPSHOT_REGION_SIZE,
  SNAPSHOT_STRIDE,
  snapshotSlotOffset,
} from "./asset-layout.ts";

test("layout identity", () => {
  assert.equal(SAB_MAGIC, "LSIM");
  assert.equal(SAB_LAYOUT_VERSION, 2);
});

test("regions fit the SAB", () => {
  assert.equal(HEADER_OFFSET + HEADER_SIZE, SNAPSHOT_OFFSET);
  assert.equal(SNAPSHOT_OFFSET + SNAPSHOT_REGION_SIZE, INPUT_MAILBOX_OFFSET);
  assert.equal(INPUT_MAILBOX_OFFSET + INPUT_MAILBOX_SIZE, FFB_MAILBOX_OFFSET);
  assert.equal(FFB_MAILBOX_OFFSET + FFB_MAILBOX_SIZE, RESERVED_OFFSET);
  assert.equal(RESERVED_OFFSET + RESERVED_SIZE, SAB_TOTAL_SIZE);
  assert.equal(SNAPSHOT_COUNT * SNAPSHOT_STRIDE, SNAPSHOT_REGION_SIZE);
});

test("regions are aligned", () => {
  assert.equal(HEADER_OFFSET % 8, 0);
  assert.equal(SNAPSHOT_OFFSET % 16, 0);
  assert.equal(SNAPSHOT_STRIDE % 16, 0);
  assert.equal(INPUT_MAILBOX_OFFSET % 16, 0);
  assert.equal(FFB_MAILBOX_OFFSET % 16, 0);
});

test("header fields match the Rust ABI", () => {
  assert.deepEqual(HEADER_FIELDS, {
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
  });
});

test("snapshot fields match the Rust ABI", () => {
  assert.deepEqual(SNAPSHOT_FIELDS, {
    seq: 0,
    reserved0: 4,
    tick: 8,
    timestampSeconds: 16,
    reserved1: 24,
    state: 32,
  });
  assert.equal(SNAPSHOT_STATE_FLOATS, 64);
  assert.equal(SNAPSHOT_STATE.tick, 62);
  assert.equal(SNAPSHOT_FIELDS.state + SNAPSHOT_STATE_FLOATS * 4, SNAPSHOT_STRIDE);
});

test("mailbox fields match the Rust ABI", () => {
  assert.deepEqual(INPUT_MAILBOX_FIELDS, {
    seq: 0,
    buttons: 4,
    timestampSeconds: 8,
    steering: 16,
    throttle: 20,
    brake: 24,
    clutch: 28,
    gear: 32,
    reserved: 36,
  });
  assert.equal(INPUT_MAILBOX_FIELDS.reserved + 28, INPUT_MAILBOX_SIZE);

  assert.deepEqual(FFB_MAILBOX_FIELDS, {
    seq: 0,
    flags: 4,
    timestampSeconds: 8,
    torque: 16,
    reserved: 20,
  });
  assert.equal(FFB_MAILBOX_FIELDS.reserved + 44, FFB_MAILBOX_SIZE);
});

test("snapshot slot offsets are bounded", () => {
  assert.equal(snapshotSlotOffset(0), SNAPSHOT_OFFSET);
  assert.equal(snapshotSlotOffset(3), SNAPSHOT_OFFSET + 3 * SNAPSHOT_STRIDE);
  assert.throws(() => snapshotSlotOffset(-1), RangeError);
  assert.throws(() => snapshotSlotOffset(SNAPSHOT_COUNT), RangeError);
  assert.throws(() => snapshotSlotOffset(1.5), RangeError);
});
