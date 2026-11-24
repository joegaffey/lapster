import test from "node:test";
import assert from "node:assert/strict";

import { isServiceWorkerBypass } from "../src/pwa-bypass.ts";

test("sw=0 requests the service-worker bypass", () => {
  assert.equal(isServiceWorkerBypass("?sw=0"), true);
  assert.equal(isServiceWorkerBypass("?foo=1&sw=0"), true);
  assert.equal(isServiceWorkerBypass("sw=0"), true);
});

test("any other sw value keeps normal service-worker behavior", () => {
  assert.equal(isServiceWorkerBypass(""), false);
  assert.equal(isServiceWorkerBypass("?sw=1"), false);
  assert.equal(isServiceWorkerBypass("?sw=false"), false);
  assert.equal(isServiceWorkerBypass("?other=0"), false);
  assert.equal(isServiceWorkerBypass("?sw"), false);
});
