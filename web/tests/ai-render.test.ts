import test from "node:test";
import assert from "node:assert/strict";

import {
  aiRenderSource,
  resolveAiRenderFrame,
} from "../src/sim/ai-render.ts";
import { SNAPSHOT_STATE } from "../src/types/asset-layout.ts";

const POSITION = SNAPSHOT_STATE.position;

function pose(x: number): Float32Array {
  const values = new Float32Array(64);
  values[POSITION] = x;
  return values;
}

test("raw wins over no-smooth, and no-smooth wins over the default filter", () => {
  assert.equal(aiRenderSource(false, false), "smooth");
  assert.equal(aiRenderSource(true, false), "interpolated");
  assert.equal(aiRenderSource(false, true), "raw");
  assert.equal(aiRenderSource(true, true), "raw");
});

test("default smooth path filters the buffer frame", () => {
  const bufferFrame = pose(10);
  const render = resolveAiRenderFrame(
    "smooth",
    bufferFrame,
    null,
    pose(0),
    0.05,
  );
  if (!render) {
    throw new Error("expected a render frame");
  }

  assert.equal(render.source, bufferFrame);
  assert.notEqual(render.drawn, bufferFrame);
  assert.ok(render.drawn[POSITION] > 0, "position did not advance");
  assert.ok(render.drawn[POSITION] < 10, "position overshot");
});

test("no-smooth path copies the buffer frame without filtering", () => {
  const bufferFrame = pose(7);
  const render = resolveAiRenderFrame(
    "interpolated",
    bufferFrame,
    pose(99),
    pose(0),
    0.05,
  );
  if (!render) {
    throw new Error("expected a render frame");
  }

  assert.equal(render.source, bufferFrame);
  assert.notEqual(render.drawn, bufferFrame);
  assert.deepEqual(Array.from(render.drawn), Array.from(bufferFrame));

  // Callers may freeze wheels in place without touching the buffer history.
  render.drawn[POSITION] = 123;
  assert.equal(bufferFrame[POSITION], 7);
  assert.equal(render.drawn[POSITION], 123);
});

test("raw path uses the latest raw frame and ignores the buffer", () => {
  const raw = pose(42);
  const render = resolveAiRenderFrame("raw", pose(3), raw, pose(0), 0.05);
  if (!render) {
    throw new Error("expected a render frame");
  }

  assert.equal(render.source, raw);
  assert.notEqual(render.drawn, raw);
  assert.equal(render.drawn[POSITION], 42);

  render.drawn[POSITION] = 5;
  assert.equal(raw[POSITION], 42);
});

test("raw path waits for a raw frame instead of using the buffer", () => {
  assert.equal(resolveAiRenderFrame("raw", pose(3), null, null, 0.05), null);
});

test("missing source frames yield no render", () => {
  assert.equal(resolveAiRenderFrame("smooth", null, null, null, 0.05), null);
  assert.equal(
    resolveAiRenderFrame("interpolated", null, pose(1), null, 0.05),
    null,
  );
});
