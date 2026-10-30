import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { Worker } from "node:worker_threads";

import {
  loadPackFromBytes,
  loadPackFromUrl,
  parsePack,
  withPackVersion,
  type PackCache,
} from "../src/assets/loader.ts";

const carUrl = new URL("../../fixtures/cars/minimal/pack.sima", import.meta.url);
const trackUrl = new URL("../../fixtures/tracks/minimal/pack.sima", import.meta.url);

async function readBytes(url: URL): Promise<Uint8Array> {
  return new Uint8Array(await readFile(url));
}

class MemoryCache implements PackCache {
  values = new Map<string, Uint8Array>();

  async get(key: string): Promise<Uint8Array | undefined> {
    return this.values.get(key);
  }

  async put(key: string, bytes: Uint8Array): Promise<void> {
    this.values.set(key, bytes);
  }

  async delete(key: string): Promise<void> {
    this.values.delete(key);
  }
}

test("loads and verifies the car pack", async () => {
  const pack = await loadPackFromBytes(await readBytes(carUrl));
  assert.equal(pack.manifest.kind, "Car");
  assert.equal(pack.manifest.id, "fixtures/car/minimal");
  assert.equal(pack.manifest.chunks.length, 2);
  assert.equal(pack.manifest.entrypoints.physics, 0);
  assert.equal(pack.manifest.entrypoints.visual, 1);
  assert.ok(pack.manifest.stats.triangles > 0);
  assert.ok(pack.chunkBytes("Physics").length > 0);
  assert.ok(pack.chunkBytes("Visual").length > 0);
});

test("loads and verifies the track pack", async () => {
  const pack = await loadPackFromBytes(await readBytes(trackUrl));
  assert.equal(pack.manifest.kind, "Track");
  assert.equal(pack.manifest.id, "fixtures/track/minimal");
  assert.equal(pack.manifest.chunks.length, 3);
  assert.equal(pack.manifest.entrypoints.collision, 1);
  assert.equal(pack.manifest.entrypoints.metadata, 2);
  assert.ok(pack.chunkBytes("Collision").length > 0);
  assert.ok(pack.chunkBytes("Metadata").length > 0);
});

test("loads a pack inside a worker", async () => {
  const bytes = await readBytes(carUrl);
  const result = await new Promise<{
    id: string;
    kind: string;
    chunks: number;
    physicsBytes: number;
  }>((resolve, reject) => {
    const worker = new Worker(new URL("./pack-loader.worker.mjs", import.meta.url), {
      workerData: { bytes },
    });
    worker.once("message", resolve);
    worker.once("error", reject);
    worker.once("exit", (code) => {
      if (code !== 0) {
        reject(new Error(`worker exited with code ${code}`));
      }
    });
  });

  assert.equal(result.id, "fixtures/car/minimal");
  assert.equal(result.kind, "Car");
  assert.equal(result.chunks, 2);
  assert.ok(result.physicsBytes > 0);
});

test("adds the build version to pack URLs without one", () => {
  const url = new URL(
    withPackVersion("../../fixtures/tracks/minimal/pack.sima", "build-123", "https://example.com/web/"),
  );
  assert.equal(url.pathname, "/fixtures/tracks/minimal/pack.sima");
  assert.equal(url.searchParams.get("v"), "build-123");
});

test("preserves an explicit pack cache version", () => {
  const url = new URL(
    withPackVersion("../../fixtures/tracks/minimal/pack.sima?v=asset-7", "build-123", "https://example.com/web/"),
  );
  assert.equal(url.searchParams.get("v"), "asset-7");
});

test("loads through a cacheable URL", async () => {
  const bytes = await readBytes(carUrl);
  const url = `data:application/octet-stream;base64,${Buffer.from(bytes).toString("base64")}`;
  const cache = new MemoryCache();
  const pack = await loadPackFromUrl(url, cache);
  assert.equal(pack.manifest.kind, "Car");
  assert.equal(cache.values.size, 1);
});

test("rejects corrupt chunks", async () => {
  const bytes = await readBytes(carUrl);
  const parsed = parsePack(bytes);
  const offset = parsed.manifest.chunks[0].offset;
  bytes[offset] ^= 0xff;
  await assert.rejects(loadPackFromBytes(bytes), /hash does not match/);
});

test("rejects bad magic and truncated packs", async () => {
  const bytes = await readBytes(carUrl);
  const badMagic = bytes.slice();
  badMagic[0] ^= 0xff;
  await assert.rejects(loadPackFromBytes(badMagic), /magic/);

  const truncated = bytes.slice(0, bytes.length - 1);
  await assert.rejects(loadPackFromBytes(truncated), /length mismatch/);
});
