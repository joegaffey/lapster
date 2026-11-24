import test from "node:test";
import assert from "node:assert/strict";

import { resolveSpectatorSlot } from "../src/race/spectator.ts";

test("no spectate flag leaves normal races untouched", () => {
  assert.equal(resolveSpectatorSlot(null, 3, false, false), null);
});

test("spectating is unavailable without AI cars", () => {
  assert.equal(resolveSpectatorSlot("1", 0, false, false), null);
  assert.equal(resolveSpectatorSlot("1", Number.NaN, false, false), null);
});

test("spectating is unavailable in multiplayer and solo", () => {
  assert.equal(resolveSpectatorSlot("1", 3, true, false), null);
  assert.equal(resolveSpectatorSlot("1", 3, false, true), null);
});

test("a present flag defaults to slot 1", () => {
  assert.equal(resolveSpectatorSlot("", 3, false, false), 1);
  assert.equal(resolveSpectatorSlot("abc", 3, false, false), 1);
  assert.equal(resolveSpectatorSlot("1.9", 3, false, false), 1);
});

test("valid slots are passed through", () => {
  assert.equal(resolveSpectatorSlot("1", 3, false, false), 1);
  assert.equal(resolveSpectatorSlot("2", 3, false, false), 2);
  assert.equal(resolveSpectatorSlot("3", 3, false, false), 3);
});

test("slots are clamped to the available AI field", () => {
  assert.equal(resolveSpectatorSlot("0", 3, false, false), 1);
  assert.equal(resolveSpectatorSlot("-2", 2, false, false), 1);
  assert.equal(resolveSpectatorSlot("3", 2, false, false), 2);
  assert.equal(resolveSpectatorSlot("9", 1, false, false), 1);
});
