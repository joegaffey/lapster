import init, { TrackViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { setupTrackCamera } from "../src/ui/track-camera.ts";
import { APP_VERSION } from "../src/version.generated.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";
import { setupAssistsDialog } from "../src/ui/assists.ts";
import { recordFrame } from "../src/ui/fps.ts";

declare global {
  interface Window {
    __trackViewerReady?: boolean;
    __trackViewerError?: string;
    __trackViewer?: TrackViewer;
  }
}

const MAX_CAMERA_HEIGHT = 60;
const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
const status = document.querySelector<HTMLElement>("#status")!;
const asset = document.querySelector<HTMLElement>("#asset")!;
const stats = document.querySelector<HTMLElement>("#stats")!;
const bounds = document.querySelector<HTMLElement>("#bounds")!;
const surfaces = document.querySelector<HTMLElement>("#surfaces")!;
const spawn = document.querySelector<HTMLElement>("#spawn")!;
const meshes = document.querySelector<HTMLElement>("#meshes")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const frameLabel = document.querySelector<HTMLElement>("#frame-ms")!;
const drawsLabel = document.querySelector<HTMLElement>("#draws")!;

setupHud();
checkForUpdate();

const params = new URLSearchParams(window.location.search);
const TRACK_URL =
  params.get("track") ??
  `../../fixtures/tracks/skidpad05/pack.sima?v=${encodeURIComponent(APP_VERSION)}`;

const SURFACE_COLORS: Record<number, string> = {
  0: "#33a6ff",
  1: "#ff4d33",
  2: "#ffe633",
  3: "#cc4dff",
  4: "#33ffb3",
};

async function loadPack() {
  try {
    return await loadPackFromUrl(TRACK_URL, new IndexedDbPackCache());
  } catch {
    return await loadPackFromUrl(TRACK_URL);
  }
}

function resizeCanvas() {
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
  const height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    return true;
  }
  return false;
}

function renderStats(debug: {
  asset: { id: string; kind: string };
  stats: Record<string, number>;
  bounds: { min: number[]; max: number[] };
  surfaces: Array<{ id: number; name: string; friction: number; roughness: number }>;
  spawns: Array<{ name: string; position: number[] }>;
  meshes: Array<{ name: string; vertices: number; triangles: number; material: number }>;
}) {
  asset.textContent = `${debug.asset.id} (${debug.asset.kind})`;

  stats.replaceChildren();
  const entries: Array<[string, string]> = [
    ["vertices", String(debug.stats.vertices)],
    ["triangles", String(debug.stats.triangles)],
    ["draw calls", String(debug.stats.drawCalls)],
    ["textures", String(debug.stats.textures)],
    ["collision tris", String(debug.stats.collisionTriangles)],
  ];
  for (const [label, value] of entries) {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = value;
    stats.append(dt, dd);
  }

  bounds.textContent = `${debug.bounds.min.map((value) => value.toFixed(1)).join(", ")} → ${debug.bounds.max
    .map((value) => value.toFixed(1))
    .join(", ")}`;

  surfaces.replaceChildren();
  for (const surface of debug.surfaces) {
    const item = document.createElement("li");
    const swatch = document.createElement("span");
    swatch.className = "swatch";
    swatch.style.background = SURFACE_COLORS[surface.id] ?? "#e5e7eb";
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${surface.id}: ${surface.name}`;
    const friction = document.createElement("span");
    friction.className = "muted";
    friction.textContent = `μ ${surface.friction.toFixed(2)}`;
    item.append(swatch, name, friction);
    surfaces.append(item);
  }

  const firstSpawn = debug.spawns[0];
  spawn.textContent = firstSpawn
    ? `${firstSpawn.name} @ ${firstSpawn.position.map((value) => value.toFixed(1)).join(", ")}`
    : "none";

  meshes.replaceChildren();
  for (const mesh of debug.meshes) {
    const item = document.createElement("li");
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = mesh.name;
    const detail = document.createElement("span");
    detail.className = "muted";
    detail.textContent = `${mesh.vertices}v ${mesh.triangles}t m${mesh.material}`;
    item.append(name, detail);
    meshes.append(item);
  }
}

async function boot() {
  status.textContent = "starting";
  if (await setupPwa()) {
    return;
  }
  status.textContent = "loading wasm";
  await init();

  status.textContent = "loading pack";
  const pack = await loadPack();

  resizeCanvas();
  status.textContent = "creating renderer";
  const viewer = await TrackViewer.create(canvas, pack.bytes);
  window.__trackViewer = viewer;
  setupAssistsDialog(viewer, { gameplay: false });
  setupTrackCamera(canvas, viewer, { maxHeight: MAX_CAMERA_HEIGHT });

  const debug = JSON.parse(viewer.debug_info());
  renderStats(debug);
  drawsLabel.textContent = String(debug.stats.drawCalls);

  const observer = new ResizeObserver(() => {
    if (resizeCanvas()) {
      viewer.resize(canvas.width, canvas.height);
    }
  });
  observer.observe(canvas);

  let last = performance.now();
  let frames = 0;
  let accumulated = 0;
  let frameTime = 0;

  const frame = (now: number) => {
    const delta = now - last;
    last = now;
    frames += 1;
    accumulated += delta;
    frameTime += delta;
    recordFrame(delta);
    if (accumulated >= 500) {
      fpsLabel.textContent = ((frames * 1000) / accumulated).toFixed(1);
      frameLabel.textContent = `${(frameTime / frames).toFixed(2)} ms`;
      frames = 0;
      accumulated = 0;
      frameTime = 0;
    }
    try {
      viewer.render();
      requestAnimationFrame(frame);
    } catch (error) {
      fail(error);
    }
  };

  status.textContent = "ready";
  status.className = "status ready";
  window.__trackViewerReady = true;
  requestAnimationFrame(frame);
}

function fail(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  window.__trackViewerError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
