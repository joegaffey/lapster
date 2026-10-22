import type {
  AssetManifest,
  ChunkKind,
  ChunkRef,
  LoadedPack,
  PackHeader,
} from "./types.ts";

export const PACK_MAGIC = "LSIM";
export const PACK_FORMAT_VERSION = 1;
export const PACK_HEADER_SIZE = 64;

const REQUIRED_ENTRYPOINTS: Record<string, ChunkKind[]> = {
  Car: ["Physics", "Visual"],
  Track: ["Visual", "Collision", "Metadata"],
};

export class PackLoadError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PackLoadError";
  }
}

export interface PackCache {
  get(key: string): Promise<Uint8Array | undefined>;
  put(key: string, bytes: Uint8Array): Promise<void>;
  delete(key: string): Promise<void>;
}

export function parsePack(bytes: Uint8Array): LoadedPack {
  if (bytes.length < PACK_HEADER_SIZE) {
    throw new PackLoadError("pack is shorter than the header");
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const magic = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
  if (magic !== PACK_MAGIC) {
    throw new PackLoadError("pack magic does not match LSIM");
  }

  const header: PackHeader = {
    formatVersion: view.getUint16(4, true),
    headerSize: view.getUint16(6, true),
    manifestOffset: readU64(view, 8),
    manifestLen: readU64(view, 16),
    packLen: readU64(view, 24),
  };

  if (header.formatVersion !== PACK_FORMAT_VERSION) {
    throw new PackLoadError(`unsupported pack format version ${header.formatVersion}`);
  }
  if (header.headerSize !== PACK_HEADER_SIZE) {
    throw new PackLoadError(`unsupported header size ${header.headerSize}`);
  }
  if (header.packLen !== bytes.length) {
    throw new PackLoadError(
      `pack length mismatch: expected ${header.packLen}, got ${bytes.length}`,
    );
  }
  if (header.manifestOffset % 16 !== 0) {
    throw new PackLoadError(`manifest offset ${header.manifestOffset} is not 16-byte aligned`);
  }

  const manifestEnd = header.manifestOffset + header.manifestLen;
  if (manifestEnd > bytes.length) {
    throw new PackLoadError("manifest is outside the pack");
  }

  const manifestText = new TextDecoder().decode(
    bytes.subarray(header.manifestOffset, manifestEnd),
  );
  let manifest: AssetManifest;
  try {
    manifest = JSON.parse(manifestText) as AssetManifest;
  } catch (error) {
    throw new PackLoadError(`manifest is not valid JSON: ${String(error)}`);
  }
  validateManifest(manifest, bytes.length);

  const chunkBytes = (kind: ChunkKind): Uint8Array => {
    const index = manifest.entrypoints[entrypointKey(kind)];
    if (index === null) {
      throw new PackLoadError(`missing entrypoint ${kind}`);
    }
    const chunk = manifest.chunks[index];
    if (!chunk) {
      throw new PackLoadError(`missing chunk ${index}`);
    }
    return bytes.subarray(chunk.offset, chunk.offset + chunk.len);
  };

  return { header, manifest, bytes, chunkBytes };
}

export async function verifyPackHashes(pack: LoadedPack): Promise<void> {
  for (const chunk of pack.manifest.chunks) {
    const bytes = pack.bytes.subarray(chunk.offset, chunk.offset + chunk.len);
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
    if (!equalBytes(digest, chunk.hash)) {
      throw new PackLoadError(`chunk ${chunk.kind} hash does not match`);
    }
  }
}

export async function loadPackFromBytes(bytes: Uint8Array): Promise<LoadedPack> {
  const pack = parsePack(bytes);
  await verifyPackHashes(pack);
  return pack;
}

export async function loadPackFromUrl(
  url: string,
  cache?: PackCache,
): Promise<LoadedPack> {
  if (cache) {
    const cached = await cache.get(cacheKey(url));
    if (cached) {
      try {
        return await loadPackFromBytes(cached);
      } catch {
        await cache.delete(cacheKey(url));
      }
    }
  }

  const response = await fetch(url);
  if (!response.ok) {
    throw new PackLoadError(`failed to fetch ${url}: ${response.status}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  const pack = await loadPackFromBytes(bytes);
  if (cache) {
    await cache.put(cacheKey(url), bytes);
  }
  return pack;
}

function cacheKey(url: string): string {
  return `${url}@pack-v${PACK_FORMAT_VERSION}`;
}

function validateManifest(manifest: AssetManifest, packLen: number): void {
  if (!Array.isArray(manifest.magic) || !equalBytes(manifest.magic, [76, 83, 73, 77])) {
    throw new PackLoadError("manifest magic does not match LSIM");
  }
  if (manifest.schema_version !== 1) {
    throw new PackLoadError(`unsupported manifest version ${manifest.schema_version}`);
  }
  if (manifest.kind !== "Car" && manifest.kind !== "Track") {
    throw new PackLoadError(`unknown asset kind ${manifest.kind}`);
  }
  if (typeof manifest.id !== "string" || manifest.id.length === 0) {
    throw new PackLoadError("asset id is empty");
  }
  if (!Array.isArray(manifest.source_hash) || manifest.source_hash.length !== 32) {
    throw new PackLoadError("source hash must contain 32 bytes");
  }
  if (!manifest.bounds || !validBounds(manifest.bounds.min, manifest.bounds.max)) {
    throw new PackLoadError("asset bounds are invalid");
  }
  if (!Array.isArray(manifest.chunks) || manifest.chunks.length === 0) {
    throw new PackLoadError("manifest has no chunks");
  }

  for (const [index, chunk] of manifest.chunks.entries()) {
    validateChunk(chunk, index, packLen);
  }

  const entrypoints = manifest.entrypoints as unknown as Record<string, number | null>;
  for (const [key, entry] of Object.entries(entrypoints)) {
    if (entry === null) {
      continue;
    }
    const kind = chunkKindForEntrypoint(key);
    if (!kind) {
      throw new PackLoadError(`unknown entrypoint ${key}`);
    }
    const chunk = manifest.chunks[entry];
    if (!chunk) {
      throw new PackLoadError(`entrypoint ${kind} references missing chunk ${entry}`);
    }
    if (chunk.kind !== kind) {
      throw new PackLoadError(
        `entrypoint ${kind} references chunk ${entry} of kind ${chunk.kind}`,
      );
    }
  }

  for (const kind of REQUIRED_ENTRYPOINTS[manifest.kind]) {
    if (entrypoints[entrypointKey(kind)] === null) {
      throw new PackLoadError(`required entrypoint ${kind} is missing`);
    }
  }
}

function validateChunk(chunk: ChunkRef, index: number, packLen: number): void {
  if (!chunk || typeof chunk.kind !== "string") {
    throw new PackLoadError(`chunk ${index} is invalid`);
  }
  if (!Number.isInteger(chunk.offset) || chunk.offset < 0) {
    throw new PackLoadError(`chunk ${index} offset is invalid`);
  }
  if (!Number.isInteger(chunk.len) || chunk.len <= 0) {
    throw new PackLoadError(`chunk ${index} length is invalid`);
  }
  if (!Number.isInteger(chunk.align) || chunk.align <= 0 || (chunk.align & (chunk.align - 1)) !== 0) {
    throw new PackLoadError(`chunk ${index} alignment is invalid`);
  }
  if (chunk.offset % chunk.align !== 0) {
    throw new PackLoadError(`chunk ${index} is misaligned`);
  }
  if (chunk.offset + chunk.len > packLen) {
    throw new PackLoadError(`chunk ${index} is outside the pack`);
  }
  if (!Array.isArray(chunk.hash) || chunk.hash.length !== 32) {
    throw new PackLoadError(`chunk ${index} hash must contain 32 bytes`);
  }
  if (chunk.schema_version !== 1) {
    throw new PackLoadError(`chunk ${index} has unsupported version ${chunk.schema_version}`);
  }
  if (chunk.compression !== "None") {
    throw new PackLoadError(`chunk ${index} uses unsupported compression ${chunk.compression}`);
  }
}

function chunkKindForEntrypoint(key: string): ChunkKind | undefined {
  switch (key) {
    case "physics":
      return "Physics";
    case "visual":
      return "Visual";
    case "collision":
      return "Collision";
    case "metadata":
      return "Metadata";
    case "audio":
      return "Audio";
    case "texture":
      return "Texture";
    default:
      return undefined;
  }
}

function entrypointKey(kind: ChunkKind): keyof AssetManifest["entrypoints"] {
  switch (kind) {
    case "Physics":
      return "physics";
    case "Visual":
      return "visual";
    case "Collision":
      return "collision";
    case "Metadata":
      return "metadata";
    case "Audio":
      return "audio";
    case "Texture":
      return "texture";
  }
}

function validBounds(min: number[], max: number[]): boolean {
  return (
    Array.isArray(min) &&
    Array.isArray(max) &&
    min.length === 3 &&
    max.length === 3 &&
    min.every((value, index) => Number.isFinite(value) && value <= max[index]) &&
    max.every(Number.isFinite)
  );
}

function equalBytes(left: Uint8Array | number[], right: Uint8Array | number[]): boolean {
  if (left.length !== right.length) {
    return false;
  }
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
}

function readU64(view: DataView, offset: number): number {
  const value = view.getBigUint64(offset, true);
  if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new PackLoadError(`value at offset ${offset} is too large`);
  }
  return Number(value);
}
