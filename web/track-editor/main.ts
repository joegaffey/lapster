import init, { TrackViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { EDITOR_TRACK_ID, EDITOR_TRACK_CACHE_KEY } from "../src/assets/editor-track.ts";
import { setupTrackCamera, type OrbitCameraController } from "../src/ui/track-camera.ts";
import { setupTrackDevMenu } from "../src/ui/dev-menu.ts";
import { setupHud } from "../src/ui/hud.ts";
import { recordFrame } from "../src/ui/fps.ts";
import { hideLoading, showLoadingFailure } from "../src/ui/loading.ts";

declare global {
  interface Window {
    __trackEditorReady?: boolean;
    __trackEditorError?: string;
    __trackEditor?: TrackViewer;
  }
}

type Vec3 = [number, number, number];

interface CameraView {
  eye: number[];
  target: number[];
  fovY: number;
}

interface LoopPointData {
  id: string;
  position: [number, number];
  kind: string;
  degree: number;
}

interface LoopPoint {
  id: string;
  position: Vec3;
  kind: string;
  degree: number;
}

interface ProjectJunction {
  id: string;
  position: [number, number];
  kind?: string;
}

interface ProjectSegment {
  id: string;
  from: string;
  to: string;
}

interface ProjectLayout {
  id: string;
  route?: string[];
}

interface TrackProjectDocument {
  schema?: string;
  name?: string;
  junctions?: ProjectJunction[];
  segments?: ProjectSegment[];
  layouts?: ProjectLayout[];
}

const MAX_CAMERA_HEIGHT = 140;
const CONTROL_POINT_RADIUS = 9;
const CONTROL_POINT_HIT_RADIUS = 18;
const PROJECT_STORAGE_KEY = "lapster.track-editor.project.v5";
const PROJECT_SCHEMA = "lapster/track-project@5";
const SURFACE_COLORS: Record<number, string> = {
  0: "#33a6ff",
  1: "#33ff59",
  2: "#b35933",
};

const devMode =
  new URLSearchParams(window.location.search).has("dev") &&
  new URLSearchParams(window.location.search).get("dev") !== "0";

const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
const overlay = document.querySelector<HTMLCanvasElement>("#control-points")!;
const overlayContext = overlay.getContext("2d")!;
const status = document.querySelector<HTMLElement>("#status")!;
const stats = document.querySelector<HTMLElement>("#stats")!;
const bounds = document.querySelector<HTMLElement>("#bounds")!;
const surfaces = document.querySelector<HTMLElement>("#surfaces")!;
const meshes = document.querySelector<HTMLElement>("#meshes")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const frameLabel = document.querySelector<HTMLElement>("#frame-ms")!;
const drawsLabel = document.querySelector<HTMLElement>("#draws")!;
const treeDetailButton = document.querySelector<HTMLButtonElement>("#tree-detail")!;
const overlayToggle = document.querySelector<HTMLButtonElement>("#toggle-overlay")!;
const cameraViewButton = document.querySelector<HTMLButtonElement>("#camera-view")!;
const testDriveButton = document.querySelector<HTMLButtonElement>("#test-drive")!;
const saveProjectButton =
  document.querySelector<HTMLButtonElement>("#save-project")!;
const loadProjectButton =
  document.querySelector<HTMLButtonElement>("#load-project")!;
const exportProjectButton =
  document.querySelector<HTMLButtonElement>("#export-project")!;
const importProjectButton =
  document.querySelector<HTMLButtonElement>("#import-project")!;
const importProjectInput =
  document.querySelector<HTMLInputElement>("#import-project-file")!;
const resetProjectButton =
  document.querySelector<HTMLButtonElement>("#reset-project")!;

let editorViewer: TrackViewer | undefined;
let cameraController: OrbitCameraController | undefined;
let project: TrackProjectDocument = {};
let activeLayoutId = "";
let loopPoints: LoopPoint[] = [];
let centerline: Vec3[] = [];
let defaultProjectJson = "";
let dragIndex: number | null = null;
let dragPointerId: number | null = null;
let dragPlaneY = 0;
let rebuilding = false;
let suspended = false;
let cameraDragging = false;
let lastPointer: [number, number] | null = null;
const TREE_DETAIL_MODES = ["high", "low", "cards"] as const;
type TreeDetailMode = (typeof TREE_DETAIL_MODES)[number];
const TREE_DETAIL_LABELS: Record<TreeDetailMode, string> = {
  high: "High",
  low: "Low",
  cards: "Cards",
};
let treeDetailMode: TreeDetailMode = "high";
let overlayVisible = true;
const CAMERA_VIEWS = ["front", "back", "left", "right", "car"] as const;
type CameraViewMode = (typeof CAMERA_VIEWS)[number];
const CAMERA_VIEW_LABELS: Record<CameraViewMode, string> = {
  front: "Front",
  back: "Back",
  left: "Left",
  right: "Right",
  car: "Car",
};
const CAR_MARKER_HEIGHT = 3.2;
const CAR_MARKER_HIT_RADIUS = 16;
const CAR_CAMERA_HEIGHT = 1.4;
const CAR_LOOK_AHEAD = 7;
const CAR_ARROW_STEP = 1;
let cameraViewMode: CameraViewMode = "front";
let carIndex = 0;
let carDragPointerId: number | null = null;

setupHud();

const add = (a: Vec3, b: Vec3): Vec3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (value: Vec3, amount: number): Vec3 => [
  value[0] * amount,
  value[1] * amount,
  value[2] * amount,
];
const dot = (a: Vec3, b: Vec3): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const toVec3 = (value: number[]): Vec3 => [value[0] ?? 0, value[1] ?? 0, value[2] ?? 0];
const normalize = (value: Vec3): Vec3 => {
  const length = Math.hypot(value[0], value[1], value[2]);
  return length > 1e-8 ? scale(value, 1 / length) : [0, 0, 0];
};

const lerp = (a: Vec3, b: Vec3, t: number): Vec3 => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t,
];

function centerlinePoint(index: number): Vec3 {
  const count = centerline.length;
  if (count === 0) {
    return [0, 0, 0];
  }
  const wrapped = ((index % count) + count) % count;
  const base = Math.floor(wrapped) % count;
  const next = (base + 1) % count;
  return lerp(centerline[base], centerline[next], wrapped - Math.floor(wrapped));
}

function centerlineTangent(index: number): Vec3 {
  const count = centerline.length;
  if (count < 2) {
    return [0, 0, -1];
  }
  const wrapped = ((index % count) + count) % count;
  const base = Math.floor(wrapped) % count;
  const t = wrapped - Math.floor(wrapped);
  const previous = centerline[(base + count - 1) % count];
  const current = centerline[base];
  const next = centerline[(base + 1) % count];
  const incoming = normalize(sub(current, previous));
  const outgoing = normalize(sub(next, current));
  return normalize(lerp(incoming, outgoing, t));
}

function carMarkerPoint(): Vec3 {
  const point = centerlinePoint(carIndex);
  return [point[0], point[1] + CAR_MARKER_HEIGHT, point[2]];
}

function carMarkerScreen(): [number, number] | null {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  return projectPoint(
    carMarkerPoint(),
    camera,
    canvas.clientWidth,
    canvas.clientHeight,
  );
}

function hitTestCarMarker(x: number, y: number): boolean {
  const projected = carMarkerScreen();
  return (
    !!projected &&
    Math.hypot(projected[0] - x, projected[1] - y) <= CAR_MARKER_HIT_RADIUS
  );
}

function nearestCenterlineIndex(x: number, y: number): number | null {
  const camera = currentCamera();
  if (!camera || centerline.length < 2) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const projected = centerline.map((point) =>
    projectPoint(point, camera, width, height),
  );
  let bestDistance = Infinity;
  let bestIndex: number | null = null;
  for (let index = 0; index < projected.length; index += 1) {
    const start = projected[index];
    const end = projected[(index + 1) % projected.length];
    if (!start || !end) {
      continue;
    }
    const segmentX = end[0] - start[0];
    const segmentY = end[1] - start[1];
    const lengthSquared = segmentX * segmentX + segmentY * segmentY;
    const t =
      lengthSquared > 1e-6
        ? Math.max(
            0,
            Math.min(
              1,
              ((x - start[0]) * segmentX + (y - start[1]) * segmentY) /
                lengthSquared,
            ),
          )
        : 0;
    const hitX = start[0] + segmentX * t;
    const hitY = start[1] + segmentY * t;
    const distance = Math.hypot(x - hitX, y - hitY);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = index + t;
    }
  }
  return bestIndex;
}

function applyCameraView(): void {
  if (!cameraController || centerline.length === 0) {
    return;
  }
  if (cameraViewMode === "car") {
    const point = centerlinePoint(carIndex);
    const tangent = centerlineTangent(carIndex);
    const eye = add(point, [0, CAR_CAMERA_HEIGHT, 0]);
    const target = add(
      add(point, scale(tangent, CAR_LOOK_AHEAD)),
      [0, CAR_CAMERA_HEIGHT, 0],
    );
    cameraController.setView(eye, target);
    return;
  }
  let minX = Infinity;
  let minY = Infinity;
  let minZ = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  let maxZ = -Infinity;
  for (const point of centerline) {
    minX = Math.min(minX, point[0]);
    minY = Math.min(minY, point[1]);
    minZ = Math.min(minZ, point[2]);
    maxX = Math.max(maxX, point[0]);
    maxY = Math.max(maxY, point[1]);
    maxZ = Math.max(maxZ, point[2]);
  }
  const center: Vec3 = [
    (minX + maxX) * 0.5,
    (minY + maxY) * 0.5,
    (minZ + maxZ) * 0.5,
  ];
  const span = Math.max(maxX - minX, maxZ - minZ);
  const distance = Math.max(span * 1.25, 55);
  const height = Math.max(span, 40);
  let eye: Vec3 = [center[0], center[1] + height, center[2] + distance];
  if (cameraViewMode === "back") {
    eye = [center[0], center[1] + height, center[2] - distance];
  } else if (cameraViewMode === "left") {
    eye = [center[0] - distance, center[1] + height, center[2]];
  } else if (cameraViewMode === "right") {
    eye = [center[0] + distance, center[1] + height, center[2]];
  }
  cameraController.setView(eye, center);
}

function syncCameraViewButton(): void {
  cameraViewButton.textContent = `View: ${CAMERA_VIEW_LABELS[cameraViewMode]}`;
}

function cycleCameraView(): void {
  const index = CAMERA_VIEWS.indexOf(cameraViewMode);
  cameraViewMode = CAMERA_VIEWS[(index + 1) % CAMERA_VIEWS.length];
  syncCameraViewButton();
  applyCameraView();
  drawControlPoints();
}

function moveCar(delta: number): void {
  const count = centerline.length;
  if (count === 0) {
    return;
  }
  carIndex = (((carIndex + delta) % count) + count) % count;
  if (cameraViewMode === "car") {
    applyCameraView();
  }
  drawControlPoints();
}

function currentCamera(): CameraView | null {
  if (!editorViewer) {
    return null;
  }
  try {
    return JSON.parse(editorViewer.camera_info()) as CameraView;
  } catch {
    return null;
  }
}

function cameraBasis(camera: CameraView) {
  const eye = toVec3(camera.eye);
  const target = toVec3(camera.target);
  const forward = normalize(sub(target, eye));
  const right = normalize(cross(forward, [0, 1, 0]));
  const up = normalize(cross(right, forward));
  return { eye, forward, right, up };
}

function projectPoint(
  point: Vec3,
  camera: CameraView,
  width: number,
  height: number,
): [number, number] | null {
  const { eye, forward, right, up } = cameraBasis(camera);
  const offset = sub(point, eye);
  const depth = dot(offset, forward);
  if (depth <= 0.01) {
    return null;
  }
  const tanHalf = Math.tan(camera.fovY * 0.5);
  const aspect = Math.max(width, 1) / Math.max(height, 1);
  const ndcX = dot(offset, right) / (depth * tanHalf * aspect);
  const ndcY = dot(offset, up) / (depth * tanHalf);
  return [(ndcX * 0.5 + 0.5) * width, (0.5 - ndcY * 0.5) * height];
}

function screenRay(
  x: number,
  y: number,
  camera: CameraView,
  width: number,
  height: number,
): { origin: Vec3; direction: Vec3 } {
  const { eye, forward, right, up } = cameraBasis(camera);
  const tanHalf = Math.tan(camera.fovY * 0.5);
  const aspect = Math.max(width, 1) / Math.max(height, 1);
  const ndcX = (x / Math.max(width, 1)) * 2 - 1;
  const ndcY = 1 - (y / Math.max(height, 1)) * 2;
  const direction = normalize(
    add(
      forward,
      add(scale(right, ndcX * tanHalf * aspect), scale(up, ndcY * tanHalf)),
    ),
  );
  return { origin: eye, direction };
}

function pointerPosition(event: PointerEvent): [number, number] {
  const rect = canvas.getBoundingClientRect();
  return [event.clientX - rect.left, event.clientY - rect.top];
}

function hitTestControlPoint(x: number, y: number): number {
  const camera = currentCamera();
  if (!camera) {
    return -1;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best = -1;
  let bestDistance = CONTROL_POINT_HIT_RADIUS;
  loopPoints.forEach((point, index) => {
    const projected = projectPoint(point.position, camera, width, height);
    if (!projected) {
      return;
    }
    const distance = Math.hypot(projected[0] - x, projected[1] - y);
    if (distance <= bestDistance) {
      bestDistance = distance;
      best = index;
    }
  });
  return best;
}

function updateCursor(): void {
  if (
    !overlayVisible ||
    !lastPointer ||
    dragIndex !== null ||
    carDragPointerId !== null ||
    rebuilding ||
    cameraDragging
  ) {
    return;
  }
  if (
    hitTestCarMarker(lastPointer[0], lastPointer[1]) ||
    hitTestControlPoint(lastPointer[0], lastPointer[1]) >= 0
  ) {
    canvas.style.cursor = "grab";
    return;
  }
  canvas.style.cursor = "";
}

function setOverlayVisible(visible: boolean): void {
  overlayVisible = visible;
  overlay.hidden = !visible;
  overlayToggle.textContent = visible ? "Hide overlay" : "Show overlay";
  overlayToggle.setAttribute("aria-pressed", String(visible));
  if (!visible) {
    canvas.style.cursor = "";
  }
  drawControlPoints();
}

function setStatus(message: string, state: "" | "ready" | "error" = ""): void {
  status.textContent = message;
  status.className = `status${state ? ` ${state}` : ""}`;
}

function reportProjectError(prefix: string, error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  setStatus(`${prefix}: ${message}`, "error");
}

function drawControlPoints(): void {
  if (!overlayVisible) {
    return;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  if (width <= 0 || height <= 0) {
    return;
  }
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const pixelWidth = Math.max(1, Math.floor(width * ratio));
  const pixelHeight = Math.max(1, Math.floor(height * ratio));
  if (overlay.width !== pixelWidth || overlay.height !== pixelHeight) {
    overlay.width = pixelWidth;
    overlay.height = pixelHeight;
  }
  overlayContext.setTransform(ratio, 0, 0, ratio, 0, 0);
  overlayContext.clearRect(0, 0, width, height);
  const camera = currentCamera();
  if (!camera) {
    return;
  }
  updateCursor();
  if (centerline.length > 1) {
    overlayContext.beginPath();
    let drawing = false;
    for (let index = 0; index <= centerline.length; index += 1) {
      const projected = projectPoint(
        centerline[index % centerline.length],
        camera,
        width,
        height,
      );
      if (!projected) {
        drawing = false;
        continue;
      }
      if (!drawing) {
        overlayContext.moveTo(projected[0], projected[1]);
        drawing = true;
      } else {
        overlayContext.lineTo(projected[0], projected[1]);
      }
    }
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle = "rgba(56, 189, 248, 0.9)";
    overlayContext.stroke();
  }

  const carMarker = carMarkerPoint();
  const carProjected = projectPoint(carMarker, camera, width, height);
  const carAheadProjected = projectPoint(
    add(carMarker, scale(centerlineTangent(carIndex), 4)),
    camera,
    width,
    height,
  );
  if (carProjected) {
    const [carX, carY] = carProjected;
    const selected = cameraViewMode === "car";
    overlayContext.save();
    overlayContext.beginPath();
    overlayContext.arc(carX, carY, 11, 0, Math.PI * 2);
    overlayContext.fillStyle = selected ? "#0ea5e9" : "#155e75";
    overlayContext.fill();
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle = selected ? "#e0f2fe" : "#67e8f9";
    overlayContext.stroke();
    if (carAheadProjected) {
      const dx = carAheadProjected[0] - carX;
      const dy = carAheadProjected[1] - carY;
      const headingLength = Math.hypot(dx, dy) || 1;
      const ux = dx / headingLength;
      const uy = dy / headingLength;
      const px = -uy;
      const py = ux;
      overlayContext.beginPath();
      overlayContext.moveTo(carX + ux * 7, carY + uy * 7);
      overlayContext.lineTo(
        carX - ux * 3 + px * 4,
        carY - uy * 3 + py * 4,
      );
      overlayContext.lineTo(
        carX - ux * 3 - px * 4,
        carY - uy * 3 - py * 4,
      );
      overlayContext.closePath();
      overlayContext.fillStyle = "#082f49";
      overlayContext.fill();
    }
    overlayContext.restore();
  }

  loopPoints.forEach((point, index) => {
    const projected = projectPoint(point.position, camera, width, height);
    if (!projected) {
      return;
    }
    overlayContext.beginPath();
    overlayContext.arc(
      projected[0],
      projected[1],
      CONTROL_POINT_RADIUS,
      0,
      Math.PI * 2,
    );
    overlayContext.fillStyle = index === dragIndex ? "#38bdf8" : "#fbbf24";
    overlayContext.fill();
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle = "#080a0d";
    overlayContext.stroke();
    overlayContext.fillStyle = "#080a0d";
    overlayContext.font = "600 10px ui-monospace, monospace";
    overlayContext.textAlign = "center";
    overlayContext.textBaseline = "middle";
    overlayContext.fillText(String(index + 1), projected[0], projected[1] + 0.5);
  });
}

function moveControlPoint(index: number, x: number, y: number): void {
  const camera = currentCamera();
  if (!camera) {
    return;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const ray = screenRay(x, y, camera, width, height);
  if (Math.abs(ray.direction[1]) < 1e-5) {
    return;
  }
  const distance = (dragPlaneY - ray.origin[1]) / ray.direction[1];
  if (distance <= 0) {
    return;
  }
  const world = add(ray.origin, scale(ray.direction, distance));
  const current = loopPoints[index];
  if (!current) {
    return;
  }
  current.position = [
    Math.max(-140, Math.min(140, world[0])),
    current.position[1],
    Math.max(-140, Math.min(140, world[2])),
  ];
}

function junctionHeights(
  document: TrackProjectDocument,
  segmentCenterlines: Record<string, Vec3[]>,
): Map<string, number> {
  const heights = new Map<string, number>();
  for (const segment of document.segments ?? []) {
    const points = segmentCenterlines[segment.id];
    if (!points || points.length === 0) {
      continue;
    }
    if (!heights.has(segment.from)) {
      heights.set(segment.from, points[0][1]);
    }
    if (!heights.has(segment.to)) {
      heights.set(segment.to, points[points.length - 1][1]);
    }
  }
  return heights;
}

function refreshEditorState(): void {
  if (!editorViewer) {
    return;
  }
  project = JSON.parse(editorViewer.editor_project()) as TrackProjectDocument;
  activeLayoutId = editorViewer.editor_active_layout();
  const loopData = JSON.parse(editorViewer.editor_loop_points()) as LoopPointData[];
  const segmentCenterlines = JSON.parse(
    editorViewer.editor_segment_centerlines(),
  ) as Record<string, Vec3[]>;
  const heights = junctionHeights(project, segmentCenterlines);
  loopPoints = loopData.map((point) => ({
    id: point.id,
    position: [
      point.position[0],
      heights.get(point.id) ?? 0,
      point.position[1],
    ],
    kind: point.kind,
    degree: point.degree,
  }));
  centerline = JSON.parse(editorViewer.editor_centerline()) as Vec3[];
  const debug = JSON.parse(editorViewer.debug_info());
  renderStats(debug);
  drawsLabel.textContent = String(debug.stats.drawCalls);
}

async function testDrive(): Promise<void> {
  if (!editorViewer || rebuilding) {
    return;
  }
  testDriveButton.disabled = true;
  setStatus("exporting track");
  try {
    const bytes = editorViewer.editor_track_pack();
    await new IndexedDbPackCache().put(EDITOR_TRACK_CACHE_KEY, bytes);
    const target = new URL("../drive/", window.location.href);
    target.searchParams.set("track", EDITOR_TRACK_ID);
    const car = new URLSearchParams(window.location.search).get("car");
    if (car) {
      target.searchParams.set("car", car);
    }
    target.searchParams.set("trees", treeDetailMode);
    window.location.href = target.toString();
  } catch (error) {
    testDriveButton.disabled = false;
    fail(error);
  }
}

testDriveButton.addEventListener("click", () => {
  void testDrive();
});

async function rebuildEditor(
  message: string,
  action: () => Promise<void>,
): Promise<void> {
  if (!editorViewer || rebuilding) {
    return;
  }
  rebuilding = true;
  suspended = true;
  testDriveButton.disabled = true;
  cameraController?.setEnabled(false);
  setStatus(message);
  try {
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await action();
    refreshEditorState();
    testDriveButton.disabled = false;
    setStatus("ready", "ready");
  } catch (error) {
    reportProjectError("edit failed", error);
    testDriveButton.disabled = false;
  } finally {
    suspended = false;
    rebuilding = false;
    cameraController?.setEnabled(true);
    drawControlPoints();
  }
}

async function moveJunction(point: LoopPoint): Promise<void> {
  await rebuildEditor("rebuilding track", () =>
    editorViewer!.move_editor_control_point(
      point.id,
      point.position[0],
      point.position[2],
    ),
  );
}

function projectJson(): string {
  return editorViewer?.editor_project() ?? "{}";
}

function projectFilename(): string {
  const name = (project.name ?? "track-project")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${name || "track-project"}.json`;
}

function downloadProject(): void {
  if (!editorViewer) {
    return;
  }
  const blob = new Blob([projectJson()], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = projectFilename();
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

function readStoredProject(): string | null {
  try {
    return localStorage.getItem(PROJECT_STORAGE_KEY);
  } catch {
    return null;
  }
}

function saveProject(): void {
  if (!editorViewer) {
    return;
  }
  try {
    localStorage.setItem(PROJECT_STORAGE_KEY, projectJson());
    setStatus("ready", "ready");
  } catch (error) {
    reportProjectError("save failed", error);
  }
}

async function loadProject(source: string, label: string): Promise<boolean> {
  if (!editorViewer || rebuilding) {
    return false;
  }
  rebuilding = true;
  suspended = true;
  testDriveButton.disabled = true;
  cameraController?.setEnabled(false);
  setStatus(label);
  try {
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await editorViewer.set_editor_project(source);
    refreshEditorState();
    testDriveButton.disabled = false;
    setStatus("ready", "ready");
    return true;
  } catch (error) {
    reportProjectError("import failed", error);
    testDriveButton.disabled = false;
    return false;
  } finally {
    suspended = false;
    rebuilding = false;
    cameraController?.setEnabled(true);
    drawControlPoints();
  }
}

async function importProjectFile(file: File): Promise<void> {
  const source = await file.text();
  if (!source.trim()) {
    reportProjectError("import failed", "the selected file is empty");
    return;
  }
  await loadProject(source, "importing project");
}

function loadStoredProject(): void {
  const stored = readStoredProject();
  if (!stored) {
    reportProjectError("load failed", "no saved project in this browser");
    return;
  }
  void loadProject(stored, "loading project");
}

function resetProject(): void {
  if (!editorViewer) {
    return;
  }
  void loadProject(defaultProjectJson, "resetting project").then((loaded) => {
    if (loaded) {
      try {
        localStorage.removeItem(PROJECT_STORAGE_KEY);
      } catch {
        // Storage may be unavailable; the in-memory project is still reset.
      }
    }
  });
}

saveProjectButton.addEventListener("click", saveProject);
loadProjectButton.addEventListener("click", loadStoredProject);
exportProjectButton.addEventListener("click", downloadProject);
resetProjectButton.addEventListener("click", resetProject);
importProjectButton.addEventListener("click", () => {
  importProjectInput.value = "";
  importProjectInput.click();
});
importProjectInput.addEventListener("change", () => {
  const file = importProjectInput.files?.[0];
  if (file) {
    void importProjectFile(file);
  }
});

canvas.addEventListener(
  "pointerdown",
  (event) => {
    if (event.button !== 0 || dragIndex !== null || rebuilding) {
      return;
    }
    const [x, y] = pointerPosition(event);
    if (
      overlayVisible &&
      carDragPointerId === null &&
      hitTestCarMarker(x, y)
    ) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      carDragPointerId = event.pointerId;
      cameraController?.setEnabled(false);
      try {
        canvas.setPointerCapture(event.pointerId);
      } catch {
        // Synthetic pointers may not support capture.
      }
      canvas.style.cursor = "grabbing";
      drawControlPoints();
      return;
    }
    const index = overlayVisible ? hitTestControlPoint(x, y) : -1;
    if (index >= 0) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      dragIndex = index;
      dragPointerId = event.pointerId;
      dragPlaneY = loopPoints[index].position[1];
      cameraController?.setEnabled(false);
      try {
        canvas.setPointerCapture(event.pointerId);
      } catch {
        // Synthetic pointers may not support capture.
      }
      drawControlPoints();
      return;
    }
    drawControlPoints();
    if (event.button === 0) {
      cameraDragging = true;
    }
  },
  { capture: true },
);

window.addEventListener(
  "pointermove",
  (event) => {
    if (carDragPointerId !== null && event.pointerId === carDragPointerId) {
      event.preventDefault();
      const [x, y] = pointerPosition(event);
      const index = nearestCenterlineIndex(x, y);
      if (index !== null) {
        carIndex = index;
        if (cameraViewMode === "car") {
          applyCameraView();
        }
        drawControlPoints();
      }
      return;
    }
    if (dragIndex === null || event.pointerId !== dragPointerId) {
      return;
    }
    event.preventDefault();
    const [x, y] = pointerPosition(event);
    moveControlPoint(dragIndex, x, y);
    drawControlPoints();
  },
  { capture: true, passive: false },
);

const endDrag = (event: PointerEvent) => {
  if (dragIndex === null || event.pointerId !== dragPointerId) {
    return;
  }
  const point = loopPoints[dragIndex];
  dragIndex = null;
  dragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  drawControlPoints();
  if (point) {
    void moveJunction(point);
  }
};

const endCarDrag = (event: PointerEvent) => {
  if (carDragPointerId === null || event.pointerId !== carDragPointerId) {
    return;
  }
  carDragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  cameraController?.setEnabled(true);
  canvas.style.cursor = "";
  if (cameraViewMode === "car") {
    applyCameraView();
  }
  drawControlPoints();
};

canvas.addEventListener("pointermove", (event) => {
  lastPointer = pointerPosition(event);
  drawControlPoints();
});

canvas.addEventListener("pointerleave", () => {
  lastPointer = null;
  canvas.style.cursor = "";
  drawControlPoints();
});

window.addEventListener("keydown", (event) => {
  const target = event.target;
  if (
    target instanceof HTMLElement &&
    (target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable)
  ) {
    return;
  }
  if (cameraViewMode === "car" && carDragPointerId === null) {
    if (event.key === "ArrowUp" || event.key === "ArrowRight") {
      event.preventDefault();
      moveCar(CAR_ARROW_STEP);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
      event.preventDefault();
      moveCar(-CAR_ARROW_STEP);
      return;
    }
  }
});

treeDetailButton.addEventListener("click", () => {
  const index = TREE_DETAIL_MODES.indexOf(treeDetailMode);
  treeDetailMode = TREE_DETAIL_MODES[(index + 1) % TREE_DETAIL_MODES.length];
  editorViewer?.set_tree_detail(treeDetailMode);
  syncTreeDetailButton();
});

cameraViewButton.addEventListener("click", () => {
  cycleCameraView();
});

overlayToggle.addEventListener("click", () => {
  setOverlayVisible(!overlayVisible);
});

const endCameraDrag = () => {
  cameraDragging = false;
};

window.addEventListener("pointerup", endDrag, { capture: true });
window.addEventListener("pointercancel", endDrag, { capture: true });
window.addEventListener("pointerup", endCarDrag, { capture: true });
window.addEventListener("pointercancel", endCarDrag, { capture: true });
window.addEventListener("pointerup", endCameraDrag, { capture: true });
window.addEventListener("pointercancel", endCameraDrag, { capture: true });

function syncTreeDetailButton(): void {
  treeDetailButton.textContent = `Trees: ${TREE_DETAIL_LABELS[treeDetailMode]}`;
}

function resizeCanvas(): boolean {
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
  stats.replaceChildren();
  const entries: Array<[string, string]> = [
    ["vertices", String(debug.stats.vertices)],
    ["triangles", String(debug.stats.triangles)],
    ["draw calls", String(debug.stats.drawCalls)],
    ["collision tris", String(debug.stats.collisionTriangles)],
    ["grass blades", String(debug.stats.grassBlades ?? 0)],
    ["spawns", String(debug.spawns.length)],
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
  setStatus("loading wasm");
  await init();

  resizeCanvas();
  setStatus("generating track");
  const viewer = await TrackViewer.create_editor(canvas);
  editorViewer = viewer;
  window.__trackEditor = viewer;
  defaultProjectJson = viewer.editor_project();
  const stored = readStoredProject();
  if (stored) {
    try {
      await viewer.set_editor_project(stored);
    } catch (error) {
      console.warn("ignoring invalid saved track project", error);
    }
  }
  if (devMode) {
    setupTrackDevMenu(viewer);
  }
  refreshEditorState();
  cameraController = setupTrackCamera(canvas, viewer, {
    maxHeight: MAX_CAMERA_HEIGHT,
    attachResetButton: false,
  });
  syncCameraViewButton();
  syncTreeDetailButton();
  applyCameraView();

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
      if (!suspended) {
        viewer.render();
        drawControlPoints();
      }
      requestAnimationFrame(frame);
    } catch (error) {
      fail(error);
    }
  };

  testDriveButton.disabled = false;
  setStatus("ready", "ready");
  window.__trackEditorReady = true;
  requestAnimationFrame(frame);
  requestAnimationFrame(hideLoading);
}

function fail(error: unknown) {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__trackEditorError = message;
  setStatus(message, "error");
  console.error(error);
}

boot().catch(fail);

export { PROJECT_SCHEMA, PROJECT_STORAGE_KEY, type TrackProjectDocument };
