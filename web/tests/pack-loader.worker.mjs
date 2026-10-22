import { parentPort, workerData } from "node:worker_threads";
import { loadPackFromBytes } from "../src/assets/loader.ts";

const bytes = new Uint8Array(workerData.bytes);
const pack = await loadPackFromBytes(bytes);
parentPort?.postMessage({
  id: pack.manifest.id,
  kind: pack.manifest.kind,
  chunks: pack.manifest.chunks.length,
  physicsBytes: pack.chunkBytes("Physics").length,
});
