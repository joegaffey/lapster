import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { Worker } from "node:worker_threads";

import {
  ASSET_SCHEMA_VERSION,
  PACK_FORMAT_VERSION,
  PACK_HEADER_SIZE,
  PACK_MAGIC,
  loadPackFromBytes,
  loadPackFromUrl,
  parsePack,
  withPackVersion,
  type PackCache,
} from "../src/assets/loader.ts";
import type {
  AssetManifest,
  ChunkKind,
  EntryPoints,
} from "../src/assets/types.ts";

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

test("loads packs when SubtleCrypto is unavailable", async () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, "crypto");
  const originalWarn = console.warn;
  Object.defineProperty(globalThis, "crypto", { value: {}, configurable: true });
  console.warn = () => {};
  try {
    const pack = await loadPackFromBytes(await readBytes(carUrl));
    assert.equal(pack.manifest.kind, "Car");
  } finally {
    console.warn = originalWarn;
    if (original) {
      Object.defineProperty(globalThis, "crypto", original);
    } else {
      delete (globalThis as { crypto?: unknown }).crypto;
    }
  }
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

function alignTo(value: number, align: number): number {
  return Math.ceil(value / align) * align;
}

function chunkData(seed: number, length: number): Uint8Array {
  return new Uint8Array(length).fill(seed);
}

interface ChunkData {
  kind: ChunkKind;
  data: Uint8Array;
}

function buildPack(manifest: AssetManifest, chunks: ChunkData[]): Uint8Array {
  let cursor = PACK_HEADER_SIZE;
  const layout = chunks.map((chunk) => {
    cursor = alignTo(cursor, 16);
    const offset = cursor;
    cursor += chunk.data.length;
    return { kind: chunk.kind, offset, len: chunk.data.length };
  });

  const manifestOffset = alignTo(cursor, 16);
  const withChunks: AssetManifest = {
    ...manifest,
    chunks: layout.map((chunk, index) => ({
      kind: chunk.kind,
      offset: chunk.offset,
      len: chunk.len,
      align: 16,
      hash: [...createHash("sha256").update(chunks[index].data).digest()],
      compression: "None",
      schema_version: ASSET_SCHEMA_VERSION,
    })),
  };
  const manifestBytes = new TextEncoder().encode(JSON.stringify(withChunks));
  const packLen = manifestOffset + manifestBytes.length;

  const bytes = new Uint8Array(packLen);
  const view = new DataView(bytes.buffer);
  bytes.set([...PACK_MAGIC].map((char) => char.charCodeAt(0)), 0);
  view.setUint16(4, PACK_FORMAT_VERSION, true);
  view.setUint16(6, PACK_HEADER_SIZE, true);
  view.setBigUint64(8, BigInt(manifestOffset), true);
  view.setBigUint64(16, BigInt(manifestBytes.length), true);
  view.setBigUint64(24, BigInt(packLen), true);

  for (const [index, chunk] of chunks.entries()) {
    bytes.set(chunk.data, layout[index].offset);
  }
  bytes.set(manifestBytes, manifestOffset);
  return bytes;
}

function trackEntrypoints(): EntryPoints {
  return {
    physics: null,
    visual: 0,
    collision: 1,
    metadata: 2,
    audio: null,
    texture: null,
  };
}

function trackManifest(entrypoints: EntryPoints): AssetManifest {
  return {
    magic: [76, 83, 73, 77],
    schema_version: ASSET_SCHEMA_VERSION,
    kind: "Track",
    id: "fixtures/track/race-entrypoint",
    source_hash: new Array(32).fill(0),
    units: { length: "Meter", mass: "Kilogram", time: "Second", angle: "Radian" },
    frame: { handedness: "Right", up: "Y", forward: "NegZ" },
    bounds: { min: [0, 0, 0], max: [1, 1, 1] },
    chunks: [],
    entrypoints,
    stats: { vertices: 0, triangles: 0, materials: 0, textures: 0, draw_calls: 0 },
    warnings: [],
  };
}

const trackChunks: ChunkData[] = [
  { kind: "Visual", data: chunkData(1, 16) },
  { kind: "Collision", data: chunkData(2, 16) },
  { kind: "Metadata", data: chunkData(3, 16) },
];

test("accepts a track manifest with a Race entrypoint", async () => {
  const chunks: ChunkData[] = [...trackChunks, { kind: "Race", data: chunkData(4, 32) }];
  const manifest = trackManifest({ ...trackEntrypoints(), race: 3 });

  const pack = await loadPackFromBytes(buildPack(manifest, chunks));

  assert.equal(pack.manifest.kind, "Track");
  assert.equal(pack.manifest.entrypoints.race, 3);
  assert.equal(pack.manifest.chunks[3].kind, "Race");
});

test("accepts a track manifest without a race entrypoint", async () => {
  const manifest = trackManifest(trackEntrypoints());

  const pack = await loadPackFromBytes(buildPack(manifest, trackChunks));

  assert.equal(pack.manifest.kind, "Track");
  assert.equal(pack.manifest.entrypoints.race, undefined);
});

test("rejects a Race entrypoint pointing at a chunk of another kind", async () => {
  const manifest = trackManifest({ ...trackEntrypoints(), race: 2 });

  await assert.rejects(
    loadPackFromBytes(buildPack(manifest, trackChunks)),
    /entrypoint Race references chunk 2 of kind Metadata/,
  );
});
