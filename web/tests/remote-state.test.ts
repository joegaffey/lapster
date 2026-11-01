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

test("remote states interpolate and reject stale ticks", () => {
  const buffer = new RemoteStateBuffer(100);
  assert.equal(buffer.frame(1000), null);
  assert.equal(buffer.push(1, 10, state(0), 1000), true);
  assert.equal(buffer.push(2, 20, state(20), 1100), true);
  assert.equal(buffer.push(1, 10, state(-20), 1200), false);

  assert.equal(buffer.frame(1100)?.[SNAPSHOT_STATE.position], 0);
  assert.equal(buffer.frame(1200)?.[SNAPSHOT_STATE.position], 20);
  assert.equal(buffer.frame(1150)?.[SNAPSHOT_STATE.position], 10);
  assert.equal(buffer.age(1300), 200);
});
