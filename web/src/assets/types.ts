export type AssetKind = "Car" | "Track";

export type ChunkKind =
  | "Physics"
  | "Visual"
  | "Collision"
  | "Metadata"
  | "Audio"
  | "Texture";

export type Compression = "None";

export interface UnitSystem {
  length: "Meter";
  mass: "Kilogram";
  time: "Second";
  angle: "Radian";
}

export interface Frame {
  handedness: "Right";
  up: "Y";
  forward: "NegZ";
}

export interface Aabb {
  min: [number, number, number];
  max: [number, number, number];
}

export interface ChunkRef {
  kind: ChunkKind;
  offset: number;
  len: number;
  align: number;
  hash: number[];
  compression: Compression;
  schema_version: number;
}

export interface EntryPoints {
  physics: number | null;
  visual: number | null;
  collision: number | null;
  metadata: number | null;
  audio: number | null;
  texture: number | null;
}

export interface AssetStats {
  vertices: number;
  triangles: number;
  materials: number;
  textures: number;
  draw_calls: number;
}

export interface AssetManifest {
  magic: number[];
  schema_version: number;
  kind: AssetKind;
  id: string;
  source_hash: number[];
  units: UnitSystem;
  frame: Frame;
  bounds: Aabb;
  chunks: ChunkRef[];
  entrypoints: EntryPoints;
  stats: AssetStats;
  warnings: string[];
}

export interface PackHeader {
  formatVersion: number;
  headerSize: number;
  manifestOffset: number;
  manifestLen: number;
  packLen: number;
}

export interface LoadedPack {
  header: PackHeader;
  manifest: AssetManifest;
  bytes: Uint8Array;
  chunkBytes(kind: ChunkKind): Uint8Array;
}
