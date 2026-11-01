import test from "node:test";
import assert from "node:assert/strict";

import { RemoteStateBuffer } from "../src/net/remote-state.ts";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";

function state(x: number): number[] {
  const values = Array.from({ length: 64 }, () => 0);
  values[SNAPSHOT_STATE.position] = x;
  values[SNAPSHOT_STATE.rotation + 3] = 1;
  return values;
}

test("remote states interpolate from a tick timeline", () => {
  const buffer = new RemoteStateBuffer(100);
  assert.equal(buffer.frame(1000), null);
  assert.equal(buffer.push(1, 0, state(0), 1000), true);
  assert.equal(buffer.push(2, 25, state(25), 1050), true);
  assert.equal(buffer.push(3, 50, state(50), 1100), true);

  assert.equal(buffer.frame(1100)?.[SNAPSHOT_STATE.position], 0);
  assert.equal(buffer.frame(1150)?.[SNAPSHOT_STATE.position], 25);
  assert.equal(buffer.frame(1200)?.[SNAPSHOT_STATE.position], 50);
  assert.equal(buffer.depth, 3);
});

test("irregular arrivals do not move the remote car backwards", () => {
  const buffer = new RemoteStateBuffer(100);
  const arrivals = [1000, 1085, 1110, 1180, 1205, 1290];
  for (let index = 0; index < arrivals.length; index += 1) {
    const tick = index * 25;
    buffer.push(index + 1, tick, state(tick), arrivals[index]);
  }

  let previous = -1;
  for (let now = 1000; now <= 1450; now += 16) {
    const frame = buffer.frame(now);
    if (!frame) {
      continue;
    }
    const position = frame[SNAPSHOT_STATE.position];
    assert.ok(position >= previous, `position moved backwards at ${now}`);
    assert.ok(position - previous < 20, `position jumped at ${now}`);
    previous = position;
  }
});

test("duplicate packets are ignored and tick resets clear history", () => {
  const buffer = new RemoteStateBuffer(100);
  buffer.push(1, 1000, state(1000), 3000);
  buffer.push(2, 1025, state(1025), 3050);
  assert.equal(buffer.push(1, 1025, state(900), 3100), false);
  assert.equal(buffer.depth, 2);

  assert.equal(buffer.push(3, 0, state(0), 4000), true);
  assert.equal(buffer.depth, 1);
  assert.equal(buffer.frame(4100)?.[SNAPSHOT_STATE.position], 0);
});

test("history is bounded", () => {
  const buffer = new RemoteStateBuffer(100);
  for (let index = 0; index < 20; index += 1) {
    buffer.push(index + 1, index * 25, state(index * 25), 1000 + index * 50);
  }
  assert.equal(buffer.depth, 6);
  assert.ok(buffer.jitter >= 0);
  assert.ok(buffer.interpolationAlpha >= 0 && buffer.interpolationAlpha <= 1);
});
