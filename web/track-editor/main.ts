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

interface KerbSpan {
  start: number;
  end: number;
  side: number;
}

type PropKind = "blocks" | "tyres" | "trees" | "off";

interface PropLine {
  start: number;
  end: number;
  side: number;
  offset: number;
  kind: PropKind;
}

interface ProjectPropSpan {
  spline: string;
  start: number;
  end: number;
  kind: PropKind;
}

interface GuideDef {
  kind: "kerb" | "prop";
  side: number;
  radius: number;
  color: string;
}

interface GuidePoint {
  guide: GuideDef;
  index: number;
  screen: [number, number];
}

type SegmentKind = "kerb" | "prop";

interface DecorationSelection {
  kind: SegmentKind;
  side: number;
  offset: number;
  start: number;
  end: number;
}

interface GuideDrag {
  kind: "kerb" | "prop";
  side: number;
  offset: number;
  edge: "start" | "end";
  pointerId: number;
}

interface SegmentPointDrag {
  segmentId: string;
  index: number;
  pointerId: number;
  planeY: number;
  position: Vec3;
  moved: boolean;
}

interface ProjectSpline {
  id: string;
  role?: string;
  base?: string | null;
  offset?: number;
  surface?: string | null;
  [key: string]: unknown;
}

interface VenueSettings {
  width?: number;
  splines?: ProjectSpline[];
  kerb: { width: number; depth: number; elementLength: number };
  pit?: unknown;
}

interface ProjectRoute {
  id: string;
  name: string;
  route: string[];
  startFinish?: { edge: string; fraction: number };
  sectors?: unknown[];
  cameras?: unknown[];
  spawns?: unknown[];
  [key: string]: unknown;
}

interface ProjectJunction {
  id: string;
  position: [number, number];
  kind?: string;
  profile?: unknown;
  branchTangents?: Record<string, [number, number]>;
  [key: string]: unknown;
}

interface ProjectEdge {
  id: string;
  from: string;
  to: string;
  points: [number, number][];
  fromTangent?: [number, number] | null;
  toTangent?: [number, number] | null;
  kerbs: unknown[];
  propSpans: unknown[];
  [key: string]: unknown;
}

interface ProjectPath {
  id: string;
  kind: string;
  route: string[];
  boxes: unknown[];
  [key: string]: unknown;
}

interface TrackProject {
  schema: string;
  name: string;
  settings: VenueSettings;
  controlPoints: ProjectJunction[];
  edges: ProjectEdge[];
  layouts: ProjectRoute[];
  paths: ProjectPath[];
  startFinish?: { edge: string; fraction: number };
  pitLane?: unknown;
  [key: string]: unknown;
}

interface RouteJunctionInfo {
  id: string;
  position: [number, number];
  kind: string;
  degree: number;
}

const TRACK_PROJECT_SCHEMA = "lapster/track-project@4";
const READABLE_TRACK_PROJECT_SCHEMAS = new Set([
  "lapster/track-project@2",
  "lapster/track-project@4",
]);
const TRACK_PROJECT_STORAGE_KEY = "lapster.track-project.v1";
const DEFAULT_TRACK_WIDTH = 12;
const MIN_TRACK_WIDTH = 6;
const MAX_TRACK_WIDTH = 30;

const MAX_CAMERA_HEIGHT = 140;
const CONTROL_POINT_RADIUS = 9;
const CONTROL_POINT_HIT_RADIUS = 18;
const CENTERLINE_HIT_RADIUS = 12;
const SEGMENT_CENTERLINE_HIT_RADIUS = 10;
const SEGMENT_POINT_HIT_RADIUS = 14;
const SEGMENT_SELECTION_COLOR = "#f97316";
const SEGMENT_SELECTION_WIDTH = 5;
const KERB_RADIUS = 6.4;
const PROP_OFFSET = 14;
const PROP_FAR_OFFSET = 19;
const GUIDE_HIT_RADIUS = 12;
const GUIDE_HANDLE_RADIUS = 7;
const GUIDE_HANDLE_HIT_RADIUS = 16;
const ROUTE_CENTERLINE_COLORS = [
  "#38bdf8",
  "#a78bfa",
  "#34d399",
  "#fbbf24",
  "#f472b6",
  "#fb923c",
  "#22d3ee",
  "#c084fc",
];
const DEFAULT_KERB_SEGMENT_LENGTH = 6;
const PROP_SEGMENT_LENGTH_METERS = 3;
const MIN_SEGMENT_LENGTH = 0.5;
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
const propToggle = document.querySelector<HTMLButtonElement>("#prop-toggle")!;
const treeDetailButton = document.querySelector<HTMLButtonElement>("#tree-detail")!;
const deleteSegmentButton =
  document.querySelector<HTMLButtonElement>("#delete-segment")!;
const overlayToggle = document.querySelector<HTMLButtonElement>("#toggle-overlay")!;
const cameraViewButton = document.querySelector<HTMLButtonElement>("#camera-view")!;
const testDriveButton = document.querySelector<HTMLButtonElement>("#test-drive")!;
const layoutSelect = document.querySelector<HTMLSelectElement>("#layout-select")!;
const addLayoutButton = document.querySelector<HTMLButtonElement>("#add-layout")!;
const duplicateLayoutButton =
  document.querySelector<HTMLButtonElement>("#duplicate-layout")!;
const deleteLayoutButton =
  document.querySelector<HTMLButtonElement>("#delete-layout")!;
const layoutNameInput = document.querySelector<HTMLInputElement>("#layout-name")!;
const layoutWidthInput =
  document.querySelector<HTMLInputElement>("#layout-width")!;
const layoutHud = document.querySelector<HTMLElement>("#layout-hud")!;
const segmentHud = document.querySelector<HTMLElement>("#segment-hud")!;
const segmentMakeUniqueButton =
  document.querySelector<HTMLButtonElement>("#segment-make-unique")!;
const segmentAddPointButton =
  document.querySelector<HTMLButtonElement>("#segment-add-point")!;
const segmentDeletePointButton =
  document.querySelector<HTMLButtonElement>("#segment-delete-point")!;
const saveProjectButton =
  document.querySelector<HTMLButtonElement>("#save-project")!;
const loadProjectButton =
  document.querySelector<HTMLButtonElement>("#load-project")!;
const exportProjectButton =
  document.querySelector<HTMLButtonElement>("#export-project")!;
const importProjectButton =
  document.querySelector<HTMLButtonElement>("#import-project")!;
const projectFileInput =
  document.querySelector<HTMLInputElement>("#project-file")!;
const pathSelect = document.querySelector<HTMLSelectElement>("#path-select")!;
const addPathButton = document.querySelector<HTMLButtonElement>("#add-path")!;
const deletePathButton =
  document.querySelector<HTMLButtonElement>("#delete-path")!;
const appendPathSegmentButton =
  document.querySelector<HTMLButtonElement>("#append-path-segment")!;
const removePathSegmentButton =
  document.querySelector<HTMLButtonElement>("#remove-path-segment")!;
const pathHud = document.querySelector<HTMLElement>("#path-hud")!;

let editorViewer: TrackViewer | undefined;
let project: TrackProject | null = null;
let projectJson: string | null = null;
let activeRouteId = "";
let syncingProjectControls = false;
let cameraController: OrbitCameraController | undefined;
let controlPoints: Vec3[] = [];
let routeJunctions: RouteJunctionInfo[] = [];
let selectedPointId: string | null = null;
let centerline: Vec3[] = [];
let routeCenterlines: Record<string, Vec3[]> = {};
let dragIndex: number | null = null;
let dragPointerId: number | null = null;
let dragPlaneY = 0;
let rebuilding = false;
let suspended = false;
let cameraDragging = false;
let deleteMode = false;
let hoverDeleteIndex: number | null = null;
let lastPointer: [number, number] | null = null;
let hoverInsert: { point: Vec3; screen: [number, number]; afterPointId: string } | null = null;
let kerbSpans: KerbSpan[] = [];
let propLines: PropLine[] = [];
let hoverPoint: GuidePoint | null = null;
let hoverHandleEdge: "start" | "end" | null = null;
let decorationSelection: DecorationSelection | null = null;
let decorationSelectionId: string | null = null;
let segmentCenterlines: Record<string, number[][]> = {};
let selectedSegmentId: string | null = null;
let activePathId = "";
let selectedSegmentPointIndex: number | null = null;
let segmentPointDrag: SegmentPointDrag | null = null;
let guideDrag: GuideDrag | null = null;
let paintKind: PropKind = "blocks";
const TREE_DETAIL_MODES = ["high", "low", "cards"] as const;
type TreeDetailMode = (typeof TREE_DETAIL_MODES)[number];
const TREE_DETAIL_LABELS: Record<TreeDetailMode, string> = {
  high: "High",
  low: "Low",
  cards: "Cards",
};
let treeDetailMode: TreeDetailMode = "high";
let overlayVisible = true;
const PROP_KINDS: PropKind[] = ["blocks", "tyres", "trees", "off"];
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

function groundPointFromScreen(
  x: number,
  y: number,
  planeY: number,
): Vec3 | null {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const ray = screenRay(x, y, camera, width, height);
  if (Math.abs(ray.direction[1]) < 1e-5) {
    return null;
  }
  const distance = (planeY - ray.origin[1]) / ray.direction[1];
  if (distance <= 0) {
    return null;
  }
  return add(ray.origin, scale(ray.direction, distance));
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
  controlPoints.forEach((point, index) => {
    const projected = projectPoint(point, camera, width, height);
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

function hitTestSegmentCenterline(x: number, y: number): string | null {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let bestDistance = SEGMENT_CENTERLINE_HIT_RADIUS;
  let bestId: string | null = null;
  for (const [id, points] of Object.entries(segmentCenterlines)) {
    if (points.length < 2) {
      continue;
    }
    const projected = points.map((point) =>
      projectPoint([point[0], point[1] ?? 0, point[2] ?? 0], camera, width, height),
    );
    for (let index = 0; index < projected.length - 1; index += 1) {
      const start = projected[index];
      const end = projected[index + 1];
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
      if (distance <= bestDistance) {
        bestDistance = distance;
        bestId = id;
      }
    }
  }
  return bestId;
}

function routeOutsideSide(): number {
  let area = 0;
  for (let index = 0; index < centerline.length; index += 1) {
    const point = centerline[index];
    const next = centerline[(index + 1) % centerline.length];
    area += point[0] * next[2] - next[0] * point[2];
  }
  return area > 0 ? -1 : 1;
}

function guideDefinitions(): GuideDef[] {
  const outside = routeOutsideSide();
  return [
    { kind: "kerb", side: -1, radius: KERB_RADIUS, color: "#fb7185" },
    { kind: "kerb", side: 1, radius: KERB_RADIUS, color: "#fb7185" },
    { kind: "prop", side: outside, radius: PROP_OFFSET, color: "#fb923c" },
    { kind: "prop", side: -outside, radius: PROP_OFFSET, color: "#2dd4bf" },
    { kind: "prop", side: outside, radius: PROP_FAR_OFFSET, color: "#f472b6" },
    { kind: "prop", side: -outside, radius: PROP_FAR_OFFSET, color: "#c084fc" },
  ];
}

function selectedSegmentPoints(): Vec3[] {
  if (!selectedSegmentId) {
    return [];
  }
  return (segmentCenterlines[selectedSegmentId] ?? []).map((point) => toVec3(point));
}

function paintCenterline(): Vec3[] {
  return selectedSegmentId ? selectedSegmentPoints() : centerline;
}

function paintIsClosed(): boolean {
  return !selectedSegmentId;
}

function paintUnits(): number {
  const count = paintCenterline().length;
  if (count < 2) {
    return 0;
  }
  return paintIsClosed() ? count : count - 1;
}

function binormalAt(sample: number): Vec3 {
  const points = paintCenterline();
  const count = points.length;
  if (count < 2) {
    return [1, 0, 0];
  }
  let index: number;
  let previous: number;
  let next: number;
  if (paintIsClosed()) {
    index = ((Math.round(sample) % count) + count) % count;
    previous = (index + count - 1) % count;
    next = (index + 1) % count;
  } else {
    index = Math.max(0, Math.min(count - 1, Math.round(sample)));
    previous = Math.max(0, index - 1);
    next = Math.min(count - 1, index + 1);
  }
  const incoming = normalize(sub(points[index], points[previous]));
  const outgoing = normalize(sub(points[next], points[index]));
  const tangent = normalize(add(incoming, outgoing));
  return [-tangent[2], 0, tangent[0]];
}

function offsetPoint(
  index: number,
  side: number,
  radius: number,
): Vec3 | null {
  const points = paintCenterline();
  const count = points.length;
  if (count < 2) {
    return null;
  }
  const units = paintUnits();
  const position = Math.max(0, Math.min(units, index));
  let base: number;
  let next: number;
  if (paintIsClosed()) {
    base = Math.floor(position) % count;
    next = (base + 1) % count;
  } else {
    base = Math.min(count - 1, Math.floor(position));
    next = Math.min(count - 1, base + 1);
  }
  const t = position - Math.floor(position);
  const point = lerp(points[base], points[next], t);
  const binormal = normalize(lerp(binormalAt(base), binormalAt(next), t));
  return add(point, scale(binormal, radius * side));
}

function guidePoint(guide: GuideDef, index: number): Vec3 | null {
  return offsetPoint(index, guide.side, guide.radius);
}

function guidePoints(guide: GuideDef): Vec3[] {
  const points: Vec3[] = [];
  const units = paintUnits();
  for (let index = 0; index <= units; index += 1) {
    const point = guidePoint(guide, index);
    if (point) {
      points.push(point);
    }
  }
  return points;
}

function hitGuidePoint(
  x: number,
  y: number,
  guides: GuideDef[] = guideDefinitions(),
): GuidePoint | null {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best: GuidePoint | null = null;
  let bestDistance = GUIDE_HIT_RADIUS;

  for (const guide of guides) {
    const projected = guidePoints(guide).map((point) =>
      projectPoint(point, camera, width, height),
    );
    for (let index = 0; index + 1 < projected.length; index += 1) {
      const start = projected[index];
      const end = projected[index + 1];
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
      if (distance >= bestDistance) {
        continue;
      }
      bestDistance = distance;
      best = {
        guide,
        index: Math.max(0, Math.min(paintUnits(), index + t)),
        screen: [hitX, hitY],
      };
    }
  }
  return best;
}

function sameOffset(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-4;
}

function guideFor(
  kind: SegmentKind,
  side: number,
  offset: number,
): GuideDef | null {
  return (
    guideDefinitions().find(
      (guide) =>
        guide.kind === kind &&
        guide.side === side &&
        (kind === "kerb" || sameOffset(guide.radius, offset)),
    ) ?? null
  );
}

function matchesLane(
  kind: SegmentKind,
  range: { side: number; offset?: number },
  side: number,
  offset: number,
): boolean {
  return (
    range.side === side &&
    (kind === "kerb" || sameOffset(range.offset ?? PROP_OFFSET, offset))
  );
}

function findRange(
  kind: SegmentKind,
  side: number,
  offset: number,
  index: number,
): { start: number; end: number; side: number; offset?: number } | null {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return (
    ranges.find(
      (range) =>
        matchesLane(kind, range, side, offset) &&
        index >= range.start - 1e-4 &&
        index <= range.end + 1e-4,
    ) ?? null
  );
}

function findRangeExact(
  kind: SegmentKind,
  side: number,
  offset: number,
  start: number,
  end: number,
): { start: number; end: number; side: number; offset?: number } | null {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return (
    ranges.find(
      (range) =>
        matchesLane(kind, range, side, offset) &&
        Math.abs(range.start - start) < 1e-4 &&
        Math.abs(range.end - end) < 1e-4,
    ) ?? null
  );
}

function selectedPropLine(): PropLine | undefined {
  if (!decorationSelection || decorationSelection.kind !== "prop") {
    return undefined;
  }
  return propLines.find(
    (line) =>
      matchesLane(
        "prop",
        line,
        decorationSelection!.side,
        decorationSelection!.offset,
      ) &&
      Math.abs(line.start - decorationSelection!.start) < 1e-4 &&
      Math.abs(line.end - decorationSelection!.end) < 1e-4,
  );
}

function resolveSelection(): void {
  if (!decorationSelection) {
    return;
  }
  const range =
    findRangeExact(
      decorationSelection.kind,
      decorationSelection.side,
      decorationSelection.offset,
      decorationSelection.start,
      decorationSelection.end,
    ) ??
    findRange(
      decorationSelection.kind,
      decorationSelection.side,
      decorationSelection.offset,
      (decorationSelection.start + decorationSelection.end) * 0.5,
    );
  if (range) {
    decorationSelection.start = range.start;
    decorationSelection.end = range.end;
    if (range.offset !== undefined) {
      decorationSelection.offset = range.offset;
    }
  } else {
    decorationSelection = null;
  }
}

function subtractRanges<
  T extends { start: number; end: number; side: number; offset?: number },
>(
  items: T[],
  kind: SegmentKind,
  side: number,
  offset: number,
  start: number,
  end: number,
): T[] {
  const result: T[] = [];
  for (const item of items) {
    if (
      !matchesLane(kind, item, side, offset) ||
      item.end <= start ||
      item.start >= end
    ) {
      result.push(item);
    } else {
      if (item.start < start) {
        result.push({ ...item, end: start });
      }
      if (item.end > end) {
        result.push({ ...item, start: end });
      }
    }
  }
  return result;
}

function selectSegment(
  kind: SegmentKind,
  side: number,
  offset: number,
  index: number,
): boolean {
  const range = findRange(kind, side, offset, index);
  if (!range) {
    return false;
  }
  decorationSelection = {
    kind,
    side,
    offset: range.offset ?? offset,
    start: range.start,
    end: range.end,
  };
  if (kind === "prop") {
    const line = propLines.find(
      (item) =>
        matchesLane("prop", item, range.side, offset) &&
        Math.abs(item.start - range.start) < 1e-4 &&
        Math.abs(item.end - range.end) < 1e-4,
    );
    if (line) {
      paintKind = line.kind;
    }
  }
  syncPaintControls();
  return true;
}

function propSegmentLength(): number {
  const points = paintCenterline();
  const count = points.length;
  const units = paintUnits();
  if (count < 2 || units < 2) {
    return MIN_SEGMENT_LENGTH;
  }
  let perimeter = 0;
  for (let index = 0; index < units; index += 1) {
    const point = points[index];
    const next = points[paintIsClosed() ? (index + 1) % count : index + 1];
    perimeter += Math.hypot(
      next[0] - point[0],
      next[1] - point[1],
      next[2] - point[2],
    );
  }
  const sampleSpacing = perimeter / units;
  return Math.max(
    MIN_SEGMENT_LENGTH,
    PROP_SEGMENT_LENGTH_METERS / Math.max(sampleSpacing, 1e-4),
  );
}

function createSegment(guide: GuideDef, index: number): void {
  const count = paintUnits();
  if (guide.kind === "prop" && paintKind === "off") {
    flashStatus("Choose a prop type to add props");
    return;
  }
  const defaultLength =
    guide.kind === "prop"
      ? propSegmentLength()
      : DEFAULT_KERB_SEGMENT_LENGTH;
  const length = Math.min(defaultLength, count);
  const start = Math.max(0, Math.min(count - length, index - length * 0.5));
  const end = Math.min(count, start + length);
  if (end - start < MIN_SEGMENT_LENGTH) {
    return;
  }

  if (guide.kind === "kerb") {
    kerbSpans = subtractRanges(
      kerbSpans,
      "kerb",
      guide.side,
      guide.radius,
      start,
      end,
    );
    kerbSpans.push({ start, end, side: guide.side });
    kerbSpans.sort((a, b) => a.side - b.side || a.start - b.start);
    decorationSelection = {
      kind: "kerb",
      side: guide.side,
      offset: guide.radius,
      start,
      end,
    };
    hoverPoint = null;
    syncPaintControls();
    drawControlPoints();
    void rebuildKerbSpans();
    return;
  }

  propLines = subtractRanges(
    propLines,
    "prop",
    guide.side,
    guide.radius,
    start,
    end,
  );
  propLines.push({
    start,
    end,
    side: guide.side,
    offset: guide.radius,
    kind: paintKind,
  });
  propLines.sort(
    (a, b) => a.side - b.side || a.offset - b.offset || a.start - b.start,
  );
  decorationSelection = {
    kind: "prop",
    side: guide.side,
    offset: guide.radius,
    start,
    end,
  };
  hoverPoint = null;
  syncPaintControls();
  drawControlPoints();
  void rebuildPropLines();
}

function updateSelectedEdge(edge: "start" | "end", index: number): void {
  if (!decorationSelection) {
    return;
  }
  const oldStart = decorationSelection.start;
  const oldEnd = decorationSelection.end;
  const range = findRangeExact(
    decorationSelection.kind,
    decorationSelection.side,
    decorationSelection.offset,
    oldStart,
    oldEnd,
  );
  if (!range) {
    return;
  }
  if (edge === "start") {
    decorationSelection.start = Math.max(
      0,
      Math.min(index, decorationSelection.end - MIN_SEGMENT_LENGTH),
    );
  } else {
    decorationSelection.end = Math.min(
      paintUnits(),
      Math.max(index, decorationSelection.start + MIN_SEGMENT_LENGTH),
    );
  }
  range.start = decorationSelection.start;
  range.end = decorationSelection.end;
}

function hitGuideHandle(
  x: number,
  y: number,
): { edge: "start" | "end"; guide: GuideDef } | null {
  if (!decorationSelection) {
    return null;
  }
  const guide = guideFor(
    decorationSelection.kind,
    decorationSelection.side,
    decorationSelection.offset,
  );
  const camera = currentCamera();
  if (!guide || !camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best: { edge: "start" | "end"; guide: GuideDef } | null = null;
  let bestDistance = GUIDE_HANDLE_HIT_RADIUS;
  for (const edge of ["start", "end"] as const) {
    const point = guidePoint(guide, decorationSelection[edge]);
    const projected = point
      ? projectPoint(point, camera, width, height)
      : null;
    if (!projected) {
      continue;
    }
    const distance = Math.hypot(projected[0] - x, projected[1] - y);
    if (distance <= bestDistance) {
      bestDistance = distance;
      best = { edge, guide };
    }
  }
  return best;
}

function paintLabel(kind: PropKind): string {
  switch (kind) {
    case "blocks":
      return "Blocks";
    case "tyres":
      return "Tyres";
    case "trees":
      return "Trees";
    default:
      return "Off";
  }
}

function syncPaintControls(): void {
  propToggle.textContent = `Prop: ${paintLabel(paintKind)}`;
  deleteSegmentButton.disabled = decorationSelection === null;
}

function syncTreeDetailButton(): void {
  treeDetailButton.textContent = `Trees: ${TREE_DETAIL_LABELS[treeDetailMode]}`;
}

function deleteSelectedSegment(): void {
  if (!decorationSelection || rebuilding) {
    return;
  }
  const { kind, side, offset, start, end } = decorationSelection;
  decorationSelection = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  if (kind === "kerb") {
    kerbSpans = kerbSpans.filter(
      (span) =>
        !(
          span.side === side &&
          Math.abs(span.start - start) < 1e-4 &&
          Math.abs(span.end - end) < 1e-4
        ),
    );
    syncPaintControls();
    drawControlPoints();
    void rebuildKerbSpans();
    return;
  }
  propLines = propLines.filter(
    (line) =>
      !(
        matchesLane("prop", line, side, offset) &&
        Math.abs(line.start - start) < 1e-4 &&
        Math.abs(line.end - end) < 1e-4
      ),
  );
  syncPaintControls();
  drawControlPoints();
  void rebuildPropLines();
}

function setPaintKind(kind: PropKind): void {
  paintKind = kind;
  const line = selectedPropLine();
  if (line) {
    line.kind = kind;
    syncPaintControls();
    drawControlPoints();
    void rebuildPropLines();
    return;
  }
  syncPaintControls();
}

function setOverlayVisible(visible: boolean): void {
  overlayVisible = visible;
  overlay.hidden = !visible;
  overlayToggle.textContent = visible ? "Hide overlay" : "Show overlay";
  overlayToggle.setAttribute("aria-pressed", String(visible));
  hoverInsert = null;
  hoverDeleteIndex = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  if (!visible) {
    decorationSelection = null;
    guideDrag = null;
    syncPaintControls();
    canvas.style.cursor = "";
  }
  drawControlPoints();
}

function updateHover(): void {
  hoverInsert = null;
  hoverDeleteIndex = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  if (
    !overlayVisible ||
    !lastPointer ||
    dragIndex !== null ||
    segmentPointDrag !== null ||
    guideDrag !== null ||
    carDragPointerId !== null ||
    rebuilding ||
    cameraDragging
  ) {
    canvas.style.cursor = "";
    return;
  }
  if (deleteMode) {
    const index = hitTestControlPoint(lastPointer[0], lastPointer[1]);
    hoverDeleteIndex = index >= 0 ? index : null;
    canvas.style.cursor = index >= 0 ? "crosshair" : "";
    return;
  }
  if (hitTestCarMarker(lastPointer[0], lastPointer[1])) {
    canvas.style.cursor = "grab";
    return;
  }
  if (hitTestSegmentPoint(lastPointer[0], lastPointer[1]) !== null) {
    canvas.style.cursor = "grab";
    return;
  }
  const handle = hitGuideHandle(lastPointer[0], lastPointer[1]);
  if (handle) {
    hoverHandleEdge = handle.edge;
    canvas.style.cursor = "grab";
    return;
  }
  const point = hitGuidePoint(lastPointer[0], lastPointer[1]);
  if (point) {
    hoverPoint = point;
    canvas.style.cursor = findRange(
      point.guide.kind,
      point.guide.side,
      point.guide.radius,
      point.index,
    )
      ? "pointer"
      : "copy";
    return;
  }
  if (centerline.length < 2 || controlPoints.length === 0) {
    canvas.style.cursor = "";
    return;
  }
  const camera = currentCamera();
  if (!camera) {
    canvas.style.cursor = "";
    return;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const projected = centerline.map((point) => projectPoint(point, camera, width, height));
  let bestDistance = CENTERLINE_HIT_RADIUS;
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
              ((lastPointer[0] - start[0]) * segmentX +
                (lastPointer[1] - start[1]) * segmentY) /
                lengthSquared,
            ),
          )
        : 0;
    const hitX = start[0] + segmentX * t;
    const hitY = start[1] + segmentY * t;
    const distance = Math.hypot(lastPointer[0] - hitX, lastPointer[1] - hitY);
    if (distance >= bestDistance) {
      continue;
    }
    bestDistance = distance;
    const next = (index + 1) % centerline.length;
    const pointIndex =
      Math.floor((index * routeJunctions.length) / centerline.length) %
      Math.max(routeJunctions.length, 1);
    const afterPointId = routeJunctions[pointIndex]?.id;
    if (!afterPointId) {
      continue;
    }
    hoverInsert = {
      point: lerp(centerline[index], centerline[next], t),
      screen: [hitX, hitY],
      afterPointId,
    };
  }
  canvas.style.cursor = hoverInsert ? "copy" : "";
}

function setDeleteMode(enabled: boolean): void {
  if (deleteMode === enabled) {
    return;
  }
  deleteMode = enabled;
  hoverInsert = null;
  hoverDeleteIndex = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  syncPaintControls();
  drawControlPoints();
}

function flashStatus(message: string): void {
  status.textContent = message;
  status.className = "status error";
  window.setTimeout(() => {
    if (!rebuilding && !window.__trackEditorError) {
      status.textContent = "ready";
      status.className = "status ready";
    }
  }, 1200);
}

function flashReadyStatus(message: string): void {
  status.textContent = message;
  status.className = "status ready";
  window.setTimeout(() => {
    if (!rebuilding && !window.__trackEditorError && status.textContent === message) {
      status.textContent = "ready";
      status.className = "status ready";
    }
  }, 1500);
}

function routeCenterlineColor(routeId: string): string {
  let hash = 0;
  for (let index = 0; index < routeId.length; index += 1) {
    hash = (hash * 31 + routeId.charCodeAt(index)) >>> 0;
  }
  return ROUTE_CENTERLINE_COLORS[hash % ROUTE_CENTERLINE_COLORS.length];
}

function segmentJunctionPoint(junctionId: string): Vec3 | null {
  const index = routeJunctions.findIndex((junction) => junction.id === junctionId);
  if (index >= 0) {
    return controlPoints[index] ?? null;
  }
  const junction = project?.controlPoints.find((item) => item.id === junctionId);
  if (!junction) {
    return null;
  }
  return [junction.position[0], 0, junction.position[1]];
}

function selectedSegmentEdge(): ProjectEdge | undefined {
  return selectedSegmentId
    ? project?.edges.find((edge) => edge.id === selectedSegmentId)
    : undefined;
}

function segmentInteriorY(edge: ProjectEdge): number {
  const from = segmentJunctionPoint(edge.from);
  const to = segmentJunctionPoint(edge.to);
  return from && to ? (from[1] + to[1]) * 0.5 : (from?.[1] ?? to?.[1] ?? 0);
}

function hitTestSegmentPoint(x: number, y: number): number | null {
  const edge = selectedSegmentEdge();
  const camera = currentCamera();
  if (!edge || !camera || edge.points.length === 0) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const interiorY = segmentInteriorY(edge);
  let best: number | null = null;
  let bestDistance = SEGMENT_POINT_HIT_RADIUS;
  edge.points.forEach((point, index) => {
    const projected = projectPoint(
      [point[0], interiorY, point[1]],
      camera,
      width,
      height,
    );
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

function selectedSegmentMidpoint(): Vec3 | null {
  const points = selectedSegmentId
    ? segmentCenterlines[selectedSegmentId]
    : undefined;
  if (!points || points.length === 0) {
    return null;
  }
  const point = points[Math.floor(points.length / 2)];
  return [point[0], point[1] ?? 0, point[2] ?? 0];
}

function lastPointerGroundPoint(planeY: number): Vec3 | null {
  if (!lastPointer) {
    return null;
  }
  return groundPointFromScreen(lastPointer[0], lastPointer[1], planeY);
}

function drawSegmentSelection(
  camera: CameraView,
  width: number,
  height: number,
): void {
  if (!selectedSegmentId) {
    return;
  }
  const centerlinePoints = segmentCenterlines[selectedSegmentId];
  if (centerlinePoints && centerlinePoints.length >= 2) {
    const projected = centerlinePoints
      .map((point) =>
        projectPoint([point[0], point[1] ?? 0, point[2] ?? 0], camera, width, height),
      )
      .filter((point): point is [number, number] => point !== null);
    if (projected.length >= 2) {
      overlayContext.save();
      overlayContext.globalAlpha = 0.95;
      overlayContext.lineCap = "round";
      overlayContext.lineJoin = "round";
      overlayContext.beginPath();
      overlayContext.moveTo(projected[0][0], projected[0][1]);
      for (let index = 1; index < projected.length; index += 1) {
        overlayContext.lineTo(projected[index][0], projected[index][1]);
      }
      overlayContext.lineWidth = SEGMENT_SELECTION_WIDTH;
      overlayContext.strokeStyle = SEGMENT_SELECTION_COLOR;
      overlayContext.stroke();
      overlayContext.restore();
    }
  }

  const edge = selectedSegmentEdge();
  if (!edge) {
    return;
  }
  const from = segmentJunctionPoint(edge.from);
  const to = segmentJunctionPoint(edge.to);
  const interiorY = segmentInteriorY(edge);
  const markers: Array<{
    point: Vec3;
    endpoint: boolean;
    index: number | null;
  }> = [];
  if (from) {
    markers.push({ point: from, endpoint: true, index: null });
  }
  edge.points.forEach((point, index) => {
    const dragging =
      segmentPointDrag &&
      segmentPointDrag.segmentId === edge.id &&
      segmentPointDrag.index === index;
    markers.push({
      point: dragging
        ? segmentPointDrag.position
        : [point[0], interiorY, point[1]],
      endpoint: false,
      index,
    });
  });
  if (to) {
    markers.push({ point: to, endpoint: true, index: null });
  }
  for (const marker of markers) {
    const projected = projectPoint(marker.point, camera, width, height);
    if (!projected) {
      continue;
    }
    const selected =
      marker.index !== null && marker.index === selectedSegmentPointIndex;
    overlayContext.beginPath();
    overlayContext.arc(
      projected[0],
      projected[1],
      marker.endpoint ? 5 : selected ? 6 : 4,
      0,
      Math.PI * 2,
    );
    overlayContext.fillStyle = marker.endpoint
      ? SEGMENT_SELECTION_COLOR
      : selected
        ? "#fbbf24"
        : "#fff7ed";
    overlayContext.fill();
    overlayContext.lineWidth = selected ? 3 : 2;
    overlayContext.strokeStyle = marker.endpoint ? "#ffedd5" : SEGMENT_SELECTION_COLOR;
    overlayContext.stroke();
  }
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
  updateHover();
  const drawCenterline = (points: Vec3[], color: string, active: boolean) => {
    if (points.length < 2) {
      return;
    }
    overlayContext.save();
    overlayContext.globalAlpha = active ? 0.95 : 0.32;
    overlayContext.beginPath();
    let drawing = false;
    for (let index = 0; index <= points.length; index += 1) {
      const projected = projectPoint(
        points[index % points.length],
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
    overlayContext.lineWidth = active ? 2.5 : 1.25;
    overlayContext.strokeStyle = color;
    overlayContext.stroke();
    overlayContext.restore();
  };

  for (const [routeId, points] of Object.entries(routeCenterlines)) {
    if (routeId === activeRouteId) {
      continue;
    }
    drawCenterline(points, routeCenterlineColor(routeId), false);
  }
  const activeCenterline = routeCenterlines[activeRouteId] ?? centerline;
  drawCenterline(activeCenterline, routeCenterlineColor(activeRouteId), true);
  drawSegmentSelection(camera, width, height);
  if (hoverInsert) {
    const [hitX, hitY] = hoverInsert.screen;
    overlayContext.save();
    overlayContext.setLineDash([4, 3]);
    overlayContext.beginPath();
    overlayContext.arc(hitX, hitY, CONTROL_POINT_RADIUS, 0, Math.PI * 2);
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle = "#38bdf8";
    overlayContext.stroke();
    overlayContext.fillStyle = "rgba(56, 189, 248, 0.22)";
    overlayContext.fill();
    overlayContext.restore();
    overlayContext.beginPath();
    overlayContext.moveTo(hitX - 3, hitY);
    overlayContext.lineTo(hitX + 3, hitY);
    overlayContext.moveTo(hitX, hitY - 3);
    overlayContext.lineTo(hitX, hitY + 3);
    overlayContext.lineWidth = 1.5;
    overlayContext.strokeStyle = "#e6f7ff";
    overlayContext.stroke();
  }
  const drawGuide = (
    points: Vec3[],
    color: string,
    active: boolean,
  ) => {
    const projected = points
      .map((point) => projectPoint(point, camera, width, height))
      .filter((point): point is [number, number] => point !== null);
    if (projected.length < 2) {
      return;
    }
    overlayContext.save();
    overlayContext.setLineDash([2, 5]);
    overlayContext.globalAlpha = active ? 0.85 : 0.32;
    overlayContext.beginPath();
    overlayContext.moveTo(projected[0][0], projected[0][1]);
    for (let index = 1; index < projected.length; index += 1) {
      overlayContext.lineTo(projected[index][0], projected[index][1]);
    }
    overlayContext.lineWidth = active ? 2 : 1;
    overlayContext.strokeStyle = color;
    overlayContext.stroke();
    overlayContext.restore();
  };

  const path = activePath();
  if (path && path.route.length > 0) {
    const pathPoints: Vec3[] = [];
    for (const segmentId of path.route) {
      const points = segmentCenterlines[segmentId] ?? [];
      for (const point of points) {
        pathPoints.push(toVec3(point));
      }
    }
    const projected = pathPoints
      .map((point) => projectPoint(point, camera, width, height))
      .filter((point): point is [number, number] => point !== null);
    if (projected.length >= 2) {
      overlayContext.save();
      overlayContext.beginPath();
      overlayContext.moveTo(projected[0][0], projected[0][1]);
      for (let index = 1; index < projected.length; index += 1) {
        overlayContext.lineTo(projected[index][0], projected[index][1]);
      }
      overlayContext.lineWidth = 3;
      overlayContext.strokeStyle = "#fbbf24";
      overlayContext.globalAlpha = 0.85;
      overlayContext.stroke();
      overlayContext.restore();
    }
  }

  const guides = guideDefinitions();
  const selectedGuide = decorationSelection
    ? guideFor(
        decorationSelection.kind,
        decorationSelection.side,
        decorationSelection.offset,
      )
    : null;
  const draggedGuide = guideDrag
    ? guideFor(guideDrag.kind, guideDrag.side, guideDrag.offset)
    : null;
  const activeGuide = draggedGuide ?? hoverPoint?.guide ?? selectedGuide;
  for (const guide of guides) {
    const active =
      activeGuide?.kind === guide.kind &&
      activeGuide.side === guide.side &&
      (guide.kind === "kerb" || sameOffset(activeGuide.radius, guide.radius));
    drawGuide(guidePoints(guide), guide.color, active);
  }

  if (hoverPoint) {
    const [hitX, hitY] = hoverPoint.screen;
    overlayContext.save();
    overlayContext.setLineDash([3, 3]);
    overlayContext.beginPath();
    overlayContext.arc(hitX, hitY, 8, 0, Math.PI * 2);
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle = "#e6f7ff";
    overlayContext.stroke();
    overlayContext.fillStyle = "rgba(230, 247, 255, 0.18)";
    overlayContext.fill();
    overlayContext.restore();
    overlayContext.beginPath();
    overlayContext.moveTo(hitX - 3, hitY);
    overlayContext.lineTo(hitX + 3, hitY);
    overlayContext.moveTo(hitX, hitY - 3);
    overlayContext.lineTo(hitX, hitY + 3);
    overlayContext.lineWidth = 1.5;
    overlayContext.strokeStyle = "#e6f7ff";
    overlayContext.stroke();
  }

  if (decorationSelection && selectedGuide) {
    const preview: Vec3[] = [];
    const steps = Math.max(
      2,
      Math.ceil((decorationSelection.end - decorationSelection.start) * 8),
    );
    for (let step = 0; step <= steps; step += 1) {
      const index =
        decorationSelection.start +
        ((decorationSelection.end - decorationSelection.start) * step) / steps;
      const point = guidePoint(selectedGuide, index);
      if (point) {
        preview.push(point);
      }
    }
    const projected = preview
      .map((point) => projectPoint(point, camera, width, height))
      .filter((point): point is [number, number] => point !== null);
    if (projected.length > 1) {
      overlayContext.save();
      overlayContext.globalAlpha = 0.9;
      overlayContext.beginPath();
      overlayContext.moveTo(projected[0][0], projected[0][1]);
      for (let index = 1; index < projected.length; index += 1) {
        overlayContext.lineTo(projected[index][0], projected[index][1]);
      }
      overlayContext.lineWidth = 3;
      overlayContext.strokeStyle = selectedGuide.color;
      overlayContext.stroke();
      overlayContext.restore();
    }

    for (const edge of ["start", "end"] as const) {
      const point = guidePoint(selectedGuide, decorationSelection[edge]);
      const projected = point
        ? projectPoint(point, camera, width, height)
        : null;
      if (!projected) {
        continue;
      }
      const active =
        hoverHandleEdge === edge ||
        (guideDrag?.edge === edge && guideDrag.kind === decorationSelection.kind);
      overlayContext.beginPath();
      overlayContext.arc(
        projected[0],
        projected[1],
        GUIDE_HANDLE_RADIUS + (active ? 2 : 0),
        0,
        Math.PI * 2,
      );
      overlayContext.fillStyle = active ? "#f8fafc" : selectedGuide.color;
      overlayContext.fill();
      overlayContext.lineWidth = 2;
      overlayContext.strokeStyle = "#080a0d";
      overlayContext.stroke();
    }
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

  controlPoints.forEach((point, index) => {
    const projected = projectPoint(point, camera, width, height);
    if (!projected) {
      return;
    }
    const deleteTarget = deleteMode && index === hoverDeleteIndex;
    const selected = routeJunctions[index]?.id === selectedPointId;
    const highlighted = selected || index === dragIndex;
    overlayContext.beginPath();
    overlayContext.arc(
      projected[0],
      projected[1],
      CONTROL_POINT_RADIUS + (deleteTarget || highlighted ? 3 : 0),
      0,
      Math.PI * 2,
    );
    overlayContext.fillStyle = deleteTarget
      ? "#ef4444"
      : deleteMode
        ? "#fca5a5"
        : "#fbbf24";
    overlayContext.fill();
    overlayContext.lineWidth = deleteTarget || highlighted ? 3 : 2;
    overlayContext.strokeStyle = deleteTarget
      ? "#fff1f2"
      : highlighted
        ? "#f8fafc"
        : "#080a0d";
    overlayContext.stroke();
    overlayContext.fillStyle = "#080a0d";
    overlayContext.font = "600 10px ui-monospace, monospace";
    overlayContext.textAlign = "center";
    overlayContext.textBaseline = "middle";
    overlayContext.fillText(String(index + 1), projected[0], projected[1] + 0.5);
  });
}

function moveControlPoint(index: number, x: number, y: number): void {
  const world = groundPointFromScreen(x, y, dragPlaneY);
  if (!world) {
    return;
  }
  const current = controlPoints[index];
  const pointX = Math.max(-140, Math.min(140, world[0]));
  const pointZ = Math.max(-140, Math.min(140, world[2]));
  controlPoints[index] = [pointX, current[1], pointZ];
  if (routeJunctions[index]) {
    routeJunctions[index] = {
      ...routeJunctions[index],
      position: [pointX, pointZ],
    };
  }
}

function activeRoute(): ProjectRoute | null {
  if (!project) {
    return null;
  }
  return (
    project.layouts.find((layout) => layout.id === activeRouteId) ??
    project.layouts[0] ??
    null
  );
}

function trackWidth(): number {
  const settings = project?.settings;
  if (typeof settings?.width === "number") {
    return settings.width;
  }
  const asphalt = (settings?.splines ?? []).filter(
    (spline) => spline.surface === "asphalt" && typeof spline.offset === "number",
  );
  if (asphalt.length >= 2) {
    const offsets = asphalt.map((spline) => spline.offset as number);
    return Math.abs(Math.max(...offsets) - Math.min(...offsets));
  }
  return DEFAULT_TRACK_WIDTH;
}

function routeSegmentCount(): number {
  return activeRoute()?.route.length ?? 0;
}

function selectedPointIndex(): number {
  if (!selectedPointId) {
    return -1;
  }
  return routeJunctions.findIndex((point) => point.id === selectedPointId);
}

function syncProjectControls(): void {
  const route = activeRoute();
  syncingProjectControls = true;
  try {
    layoutSelect.replaceChildren();
    if (project) {
      for (const item of project.layouts) {
        const option = document.createElement("option");
        option.value = item.id;
        option.textContent = item.name || item.id;
        option.selected = item.id === (route?.id ?? activeRouteId);
        layoutSelect.append(option);
      }
    }
    layoutNameInput.value = route?.name ?? "";
    layoutWidthInput.value = project ? String(trackWidth()) : "";
    duplicateLayoutButton.disabled = !project || !activeRouteId || rebuilding;
    deleteLayoutButton.disabled = !project || project.layouts.length <= 1;
    layoutHud.textContent = route
      ? `${route.name || route.id} · ${routeJunctions.length} junctions · ${routeSegmentCount()} segments`
      : "—";
  } finally {
    syncingProjectControls = false;
  }
}

function activePath(): ProjectPath | null {
  if (!project) {
    return null;
  }
  return project.paths.find((path) => path.id === activePathId) ?? null;
}

function syncPathControls(): void {
  const path = activePath();
  pathSelect.replaceChildren();
  if (project) {
    for (const item of project.paths) {
      const option = document.createElement("option");
      option.value = item.id;
      option.textContent = `${item.kind || item.id} (${item.route.length})`;
      option.selected = item.id === activePathId;
      pathSelect.append(option);
    }
  }
  addPathButton.disabled = rebuilding;
  deletePathButton.disabled = !path || rebuilding;
  appendPathSegmentButton.disabled = !path || !selectedSegmentId || rebuilding;
  removePathSegmentButton.disabled =
    !path || path.route.length === 0 || rebuilding;
  pathHud.textContent = path
    ? `${path.id} · ${path.kind} · ${path.route.length} segment${
        path.route.length === 1 ? "" : "s"
      }${
        selectedSegmentId
          ? ` · append ${selectedSegmentId} when it continues the chain`
          : " · select a road segment to append it"
      }`
    : "No pit lane path. Add one, then select a segment and append it.";
}

function setSelectedSegment(id: string | null): void {
  selectedSegmentId = id;
  selectedSegmentPointIndex = null;
  segmentPointDrag = null;
  decorationSelection = null;
  refreshPaintSpans();
  syncPathControls();
  updateSegmentPanel();
  drawControlPoints();
}

function updateSegmentPanel(): void {
  const edge = selectedSegmentEdge();
  if (!selectedSegmentId || !edge) {
    selectedSegmentPointIndex = null;
    segmentHud.textContent = "Select a road segment in the viewport.";
    segmentMakeUniqueButton.disabled = true;
    segmentAddPointButton.disabled = true;
    segmentDeletePointButton.disabled = true;
    return;
  }
  if (
    selectedSegmentPointIndex !== null &&
    selectedSegmentPointIndex >= edge.points.length
  ) {
    selectedSegmentPointIndex = null;
  }
  const count = edge.points.length;
  const pointLabel =
    selectedSegmentPointIndex === null
      ? "no interior point selected"
      : `interior point ${selectedSegmentPointIndex} selected`;
  segmentHud.textContent = `${edge.id} · ${edge.from} → ${edge.to} · ${count} interior point${
    count === 1 ? "" : "s"
  } · ${pointLabel}`;
  segmentMakeUniqueButton.disabled = rebuilding;
  segmentAddPointButton.disabled = rebuilding;
  segmentDeletePointButton.disabled =
    rebuilding || selectedSegmentPointIndex === null;
}

function readProject(): void {
  if (!editorViewer) {
    return;
  }
  const raw = editorViewer.editor_project();
  try {
    const parsed = JSON.parse(raw) as TrackProject;
    if (
      parsed &&
      READABLE_TRACK_PROJECT_SCHEMAS.has(parsed.schema) &&
      Array.isArray(parsed.layouts)
    ) {
      project = parsed;
      projectJson = JSON.stringify(parsed);
      if (
        activePathId &&
        !project.paths.some((path) => path.id === activePathId)
      ) {
        activePathId = "";
      }
      if (!activePathId && project.paths.length > 0) {
        activePathId = project.paths[0].id;
      }
    } else {
      project = null;
      projectJson = null;
    }
  } catch {
    project = null;
    projectJson = null;
  }
  activeRouteId = editorViewer.editor_active_layout();
  syncProjectControls();
}

function projectSnapshot(): string | null {
  if (projectJson) {
    return projectJson;
  }
  if (project) {
    return JSON.stringify(project);
  }
  return null;
}

function saveProject(): void {
  const snapshot = projectSnapshot();
  if (!snapshot) {
    return;
  }
  try {
    localStorage.setItem(TRACK_PROJECT_STORAGE_KEY, snapshot);
  } catch (error) {
    console.warn("could not save track project", error);
  }
}

function loadSavedProject(): string | null {
  try {
    return localStorage.getItem(TRACK_PROJECT_STORAGE_KEY);
  } catch {
    return null;
  }
}

function splineLateral(spline: ProjectSpline): number {
  const base = spline.base
    ? project?.settings.splines?.find((candidate) => candidate.id === spline.base)
    : undefined;
  return (base?.offset ?? 0) + (spline.offset ?? 0);
}

function propSpanToLine(
  span: ProjectPropSpan,
  units: number,
): PropLine | null {
  const spline = project?.settings.splines?.find(
    (candidate) => candidate.id === span.spline,
  );
  if (!spline) {
    return null;
  }
  const lateral = splineLateral(spline);
  return {
    start: span.start * units,
    end: span.end * units,
    side: lateral < 0 ? -1 : 1,
    offset: Math.abs(lateral),
    kind: span.kind,
  };
}

function propLineToSpan(
  line: PropLine,
  units: number,
): ProjectPropSpan | null {
  const splines = project?.settings.splines ?? [];
  const left = line.side < 0;
  const threshold = (PROP_OFFSET + PROP_FAR_OFFSET) * 0.5;
  const wantNear = line.offset <= threshold;
  let best: { distance: number; spline: ProjectSpline } | null = null;
  for (const spline of splines) {
    if (spline.role !== "prop") {
      continue;
    }
    const lateral = splineLateral(spline);
    if ((lateral < 0) !== left) {
      continue;
    }
    if (Math.abs(lateral) <= threshold !== wantNear) {
      continue;
    }
    const distance = Math.abs(Math.abs(lateral) - line.offset);
    if (!best || distance < best.distance) {
      best = { distance, spline };
    }
  }
  if (!best) {
    return null;
  }
  const scale = Math.max(units, 1);
  return {
    spline: best.spline.id,
    start: Math.max(0, Math.min(1, line.start / scale)),
    end: Math.max(0, Math.min(1, line.end / scale)),
    kind: line.kind,
  };
}

function refreshPaintSpans(): void {
  if (!editorViewer) {
    return;
  }
  if (selectedSegmentId) {
    const edge = selectedSegmentEdge();
    const units = Math.max(paintUnits(), 1);
    kerbSpans = ((edge?.kerbs ?? []) as KerbSpan[])
      .map((span) => ({
        start: span.start * units,
        end: span.end * units,
        side: span.side,
      }))
      .sort((a, b) => a.side - b.side || a.start - b.start);
    propLines = ((edge?.propSpans ?? []) as ProjectPropSpan[])
      .map((span) => propSpanToLine(span, units))
      .filter((line): line is PropLine => line !== null)
      .sort(
        (a, b) =>
          a.side - b.side || a.offset - b.offset || a.start - b.start,
      );
    return;
  }
  kerbSpans = (
    JSON.parse(editorViewer.editor_kerb_spans()) as KerbSpan[]
  ).sort((a, b) => a.side - b.side || a.start - b.start);
  propLines = JSON.parse(editorViewer.editor_prop_lines()) as PropLine[];
}

function refreshEditorState(): void {
  if (!editorViewer) {
    return;
  }
  const heights = JSON.parse(editorViewer.editor_control_points()) as Vec3[];
  routeJunctions = JSON.parse(
    editorViewer.editor_loop_points(),
  ) as RouteJunctionInfo[];
  controlPoints = routeJunctions.map((point, index) => [
    point.position[0],
    heights[index]?.[1] ?? 0,
    point.position[1],
  ]);
  if (selectedPointId && selectedPointIndex() < 0) {
    selectedPointId = null;
  }
  centerline = JSON.parse(editorViewer.editor_centerline()) as Vec3[];
  routeCenterlines = JSON.parse(
    editorViewer.editor_route_centerlines(),
  ) as Record<string, Vec3[]>;
  segmentCenterlines = JSON.parse(
    editorViewer.editor_segment_centerlines(),
  ) as Record<string, number[][]>;
  readProject();
  if (
    selectedSegmentId &&
    !project?.edges.some((edge) => edge.id === selectedSegmentId)
  ) {
    selectedSegmentId = null;
  }
  if (!selectedSegmentId) {
    selectedSegmentPointIndex = null;
  }
  refreshPaintSpans();
  syncPathControls();
  updateSegmentPanel();
  resolveSelection();
  syncPaintControls();
  const debug = JSON.parse(editorViewer.debug_info());
  renderStats(debug);
  drawsLabel.textContent = String(debug.stats.drawCalls);
}

async function testDrive(): Promise<void> {
  if (!editorViewer || rebuilding) {
    return;
  }
  testDriveButton.disabled = true;
  status.textContent = "exporting track";
  status.className = "status";
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
    if (activeRouteId) {
      target.searchParams.set("layout", activeRouteId);
    }
    window.location.href = target.toString();
  } catch (error) {
    testDriveButton.disabled = false;
    fail(error);
  }
}

testDriveButton.addEventListener("click", () => {
  void testDrive();
});

function reportActionError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

addLayoutButton.addEventListener("click", () => {
  void rebuildEditor("adding layout", () =>
    editorViewer!.add_editor_layout("").then(() => undefined),
  );
});

duplicateLayoutButton.addEventListener("click", () => {
  if (!activeRouteId || rebuilding) {
    return;
  }
  void rebuildEditor(
    "duplicating layout",
    () => editorViewer!.duplicate_editor_layout(activeRouteId).then(() => undefined),
    reportActionError,
  );
});

deleteLayoutButton.addEventListener("click", () => {
  if (!activeRouteId) {
    return;
  }
  void rebuildEditor(
    "deleting layout",
    () => editorViewer!.delete_editor_layout(activeRouteId),
    reportActionError,
  );
});

layoutSelect.addEventListener("change", () => {
  const id = layoutSelect.value;
  if (!id || id === activeRouteId) {
    return;
  }
  void rebuildEditor("switching layout", () =>
    editorViewer!.set_editor_active_layout(id),
  );
});

layoutNameInput.addEventListener("change", () => {
  if (syncingProjectControls) {
    return;
  }
  const layout = activeRoute();
  const name = layoutNameInput.value.trim();
  if (!layout || !name || name === layout.name) {
    return;
  }
  void rebuildEditor("renaming layout", () =>
    editorViewer!.rename_editor_layout(activeRouteId, name),
  );
});

layoutWidthInput.addEventListener("change", () => {
  if (syncingProjectControls) {
    return;
  }
  const width = Number(layoutWidthInput.value);
  if (!activeRouteId || !Number.isFinite(width)) {
    return;
  }
  const clamped = Math.min(MAX_TRACK_WIDTH, Math.max(MIN_TRACK_WIDTH, width));
  if (Math.abs(clamped - trackWidth()) < 1e-4) {
    return;
  }
  void rebuildEditor("resizing track", () =>
    editorViewer!.set_editor_layout_width(activeRouteId, clamped),
  );
});

saveProjectButton.addEventListener("click", () => {
  saveProject();
  status.textContent = "project saved";
  status.className = "status ready";
});

loadProjectButton.addEventListener("click", () => {
  const saved = loadSavedProject();
  if (!saved) {
    reportActionError(new Error("no saved project in this browser"));
    return;
  }
  void rebuildEditor("loading project", () => editorViewer!.set_editor_project(saved));
});

function exportProject(): void {
  const snapshot = projectSnapshot();
  if (!snapshot) {
    reportActionError(new Error("track project unavailable"));
    return;
  }
  let json = snapshot;
  try {
    json = JSON.stringify(JSON.parse(snapshot), null, 2);
  } catch {
    // Export the last known project JSON even if it cannot be pretty-printed.
  }
  const blob = new Blob([json], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "track-project.json";
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

exportProjectButton.addEventListener("click", exportProject);

importProjectButton.addEventListener("click", () => {
  projectFileInput.click();
});

projectFileInput.addEventListener("change", () => {
  const file = projectFileInput.files?.[0];
  projectFileInput.value = "";
  if (!file) {
    return;
  }
  void (async () => {
    try {
      const text = await file.text();
      JSON.parse(text);
      await rebuildEditor("importing project", () =>
        editorViewer!.set_editor_project(text),
      );
    } catch (error) {
      reportActionError(error);
    }
  })();
});

addPathButton.addEventListener("click", () => {
  if (!editorViewer || rebuilding) {
    return;
  }
  void rebuildEditor(
    "adding pit lane",
    async () => {
      activePathId = await editorViewer!.add_editor_path("pit_lane");
    },
    reportActionError,
  );
});

deletePathButton.addEventListener("click", () => {
  if (!editorViewer || !activePathId || rebuilding) {
    return;
  }
  const pathId = activePathId;
  void rebuildEditor(
    "deleting path",
    async () => {
      await editorViewer!.delete_editor_path(pathId);
      activePathId = "";
    },
    reportActionError,
  );
});

appendPathSegmentButton.addEventListener("click", () => {
  if (!editorViewer || !activePathId || !selectedSegmentId || rebuilding) {
    return;
  }
  const pathId = activePathId;
  const segmentId = selectedSegmentId;
  void rebuildEditor(
    "appending path segment",
    () => editorViewer!.append_editor_path_segment(pathId, segmentId),
    reportActionError,
  );
});

removePathSegmentButton.addEventListener("click", () => {
  const path = activePath();
  if (!editorViewer || !path || path.route.length === 0 || rebuilding) {
    return;
  }
  const pathId = path.id;
  const index = path.route.length - 1;
  void rebuildEditor(
    "removing path segment",
    () => editorViewer!.remove_editor_path_segment(pathId, index),
    reportActionError,
  );
});

pathSelect.addEventListener("change", () => {
  activePathId = pathSelect.value;
  syncPathControls();
  drawControlPoints();
});

function makeSelectedSegmentUnique(): void {
  if (!editorViewer || !activeRouteId || !selectedSegmentId || rebuilding) {
    return;
  }
  const routeId = activeRouteId;
  const segmentId = selectedSegmentId;
  let uniqueId = "";
  void (async () => {
    await rebuildEditor(
      "making segment unique",
      async () => {
        uniqueId = await editorViewer!.make_editor_segment_unique(
          routeId,
          segmentId,
        );
      },
      reportActionError,
    );
    if (uniqueId) {
      setSelectedSegment(uniqueId);
    }
  })();
}

function addSelectedSegmentPoint(): void {
  if (!editorViewer || !selectedSegmentId || rebuilding) {
    return;
  }
  const edge = selectedSegmentEdge();
  if (!edge) {
    return;
  }
  const segmentId = selectedSegmentId;
  const midpoint = selectedSegmentMidpoint();
  const ground = midpoint ?? lastPointerGroundPoint(segmentInteriorY(edge));
  if (!ground) {
    reportActionError(
      new Error("no viewport position available for a new interior point"),
    );
    return;
  }
  const afterIndex = edge.points.length === 0 ? 0 : edge.points.length - 1;
  let newIndex = -1;
  void (async () => {
    await rebuildEditor(
      "adding segment point",
      async () => {
        newIndex = await editorViewer!.insert_editor_segment_point(
          segmentId,
          afterIndex,
          ground[0],
          ground[2],
        );
      },
      reportActionError,
    );
    if (newIndex >= 0) {
      selectedSegmentPointIndex = newIndex;
      updateSegmentPanel();
      drawControlPoints();
    }
  })();
}

function deleteSelectedSegmentPoint(): void {
  if (
    !editorViewer ||
    !selectedSegmentId ||
    selectedSegmentPointIndex === null ||
    rebuilding
  ) {
    return;
  }
  const segmentId = selectedSegmentId;
  const index = selectedSegmentPointIndex;
  void (async () => {
    await rebuildEditor(
      "deleting segment point",
      () => editorViewer!.delete_editor_segment_point(segmentId, index),
      reportActionError,
    );
    selectedSegmentPointIndex = null;
    updateSegmentPanel();
    drawControlPoints();
  })();
}

segmentMakeUniqueButton.addEventListener("click", makeSelectedSegmentUnique);
segmentAddPointButton.addEventListener("click", addSelectedSegmentPoint);
segmentDeletePointButton.addEventListener("click", deleteSelectedSegmentPoint);

async function rebuildEditor(
  message: string,
  action: () => Promise<void>,
  onError: (error: unknown) => void = fail,
): Promise<void> {
  if (!editorViewer || rebuilding) {
    return;
  }
  rebuilding = true;
  suspended = true;
  duplicateLayoutButton.disabled = true;
  testDriveButton.disabled = true;
  cameraController?.setEnabled(false);
  status.textContent = message;
  status.className = "status";
  try {
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await action();
    refreshEditorState();
    saveProject();
    testDriveButton.disabled = false;
    status.textContent = "ready";
    status.className = "status ready";
  } catch (error) {
    onError(error);
  } finally {
    suspended = false;
    rebuilding = false;
    duplicateLayoutButton.disabled = !project || !activeRouteId;
    cameraController?.setEnabled(true);
    syncProjectControls();
    syncPathControls();
    updateSegmentPanel();
    drawControlPoints();
  }
}

async function rebuildKerbSpans(): Promise<void> {
  if (selectedSegmentId) {
    const segmentId = selectedSegmentId;
    const units = Math.max(paintUnits(), 1);
    const spans = kerbSpans.map((span) => ({
      start: Math.max(0, Math.min(1, span.start / units)),
      end: Math.max(0, Math.min(1, span.end / units)),
      side: span.side,
    }));
    await rebuildEditor("rebuilding kerbs", () =>
      editorViewer!.set_editor_segment_kerbs(
        segmentId,
        JSON.stringify(spans),
      ),
    );
    return;
  }
  await rebuildEditor("rebuilding kerbs", () =>
    editorViewer!.set_editor_kerb_spans(JSON.stringify(kerbSpans)),
  );
}

async function rebuildPropLines(): Promise<void> {
  if (selectedSegmentId) {
    const segmentId = selectedSegmentId;
    const units = Math.max(paintUnits(), 1);
    const spans = propLines
      .map((line) => propLineToSpan(line, units))
      .filter((span): span is ProjectPropSpan => span !== null);
    await rebuildEditor("rebuilding prop lines", () =>
      editorViewer!.set_editor_segment_props(
        segmentId,
        JSON.stringify(spans),
      ),
    );
    return;
  }
  await rebuildEditor("rebuilding prop lines", () =>
    editorViewer!.set_editor_prop_lines(JSON.stringify(propLines)),
  );
}

function selectControlPoint(id: string | null): void {
  selectedPointId = id;
  drawControlPoints();
}

async function deleteControlPoint(pointId: string): Promise<void> {
  if (!editorViewer || !activeRouteId || rebuilding) {
    return;
  }
  await rebuildEditor("deleting control point", () =>
    editorViewer!.delete_editor_control_point(activeRouteId, pointId),
  );
  if (selectedPointId === pointId) {
    selectedPointId = null;
  }
  drawControlPoints();
}

async function insertControlPoint(
  point: Vec3,
  afterPointId: string,
): Promise<void> {
  if (!editorViewer || !activeRouteId || rebuilding) {
    return;
  }
  let newId = "";
  await rebuildEditor("inserting control point", async () => {
    newId = await editorViewer!.insert_editor_control_point(
      activeRouteId,
      afterPointId,
      point[0],
      point[2],
    );
  });
  if (newId) {
    selectControlPoint(newId);
  }
}

async function moveControlPointById(
  pointId: string,
  point: Vec3,
): Promise<void> {
  if (!editorViewer || rebuilding) {
    return;
  }
  await rebuildEditor("moving control point", () =>
    editorViewer!.move_editor_control_point(pointId, point[0], point[2]),
  );
}

canvas.addEventListener(
  "pointerdown",
  (event) => {
    if (
      event.button !== 0 ||
      dragIndex !== null ||
      rebuilding
    ) {
      return;
    }
    const [x, y] = pointerPosition(event);
    if (event.shiftKey !== deleteMode) {
      setDeleteMode(event.shiftKey);
    }
    if (
      overlayVisible &&
      !deleteMode &&
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
    if (deleteMode && index >= 0) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      if (controlPoints.length <= 3) {
        flashStatus("minimum 3 control points");
        return;
      }
      const pointId = routeJunctions[index]?.id;
      hoverInsert = null;
      hoverDeleteIndex = null;
      drawControlPoints();
      if (pointId) {
        void deleteControlPoint(pointId);
      }
      return;
    }
    if (index < 0) {
      if (overlayVisible && !deleteMode) {
        const pointIndex = hitTestSegmentPoint(x, y);
        if (pointIndex !== null && selectedSegmentId) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          const edge = selectedSegmentEdge();
          const point = edge?.points[pointIndex];
          if (edge && point) {
            const interiorY = segmentInteriorY(edge);
            selectedSegmentPointIndex = pointIndex;
            segmentPointDrag = {
              segmentId: selectedSegmentId,
              index: pointIndex,
              pointerId: event.pointerId,
              planeY: interiorY,
              position: [point[0], interiorY, point[1]],
              moved: false,
            };
            updateSegmentPanel();
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
        }
        const handle = hitGuideHandle(x, y);
        if (handle && decorationSelection) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          guideDrag = {
            kind: decorationSelection.kind,
            side: decorationSelection.side,
            offset: decorationSelection.offset,
            edge: handle.edge,
            pointerId: event.pointerId,
          };
          hoverHandleEdge = handle.edge;
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
        const point = hitGuidePoint(x, y);
        if (point) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          if (
            selectSegment(
              point.guide.kind,
              point.guide.side,
              point.guide.radius,
              point.index,
            )
          ) {
            hoverPoint = point;
            drawControlPoints();
          } else {
            createSegment(point.guide, point.index);
          }
          return;
        }
      }
      const segmentId =
        overlayVisible && !deleteMode ? hitTestSegmentCenterline(x, y) : null;
      if (segmentId) {
        event.preventDefault();
        event.stopImmediatePropagation();
        cameraDragging = false;
        setSelectedSegment(segmentId);
        return;
      }
      if (selectedSegmentId || selectedSegmentPointIndex !== null) {
        setSelectedSegment(null);
      }
      if (decorationSelection) {
        decorationSelection = null;
        hoverPoint = null;
        hoverHandleEdge = null;
        syncPaintControls();
      }
      if (hoverInsert) {
        event.preventDefault();
        event.stopImmediatePropagation();
        const insert = hoverInsert;
        hoverInsert = null;
        drawControlPoints();
        void insertControlPoint(insert.point, insert.afterPointId);
      } else {
        drawControlPoints();
        if (event.button === 0) {
          cameraDragging = true;
        }
      }
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    cameraDragging = false;
    selectControlPoint(routeJunctions[index]?.id ?? null);
    dragIndex = index;
    dragPointerId = event.pointerId;
    dragPlaneY = controlPoints[index][1];
    cameraController?.setEnabled(false);
    try {
      canvas.setPointerCapture(event.pointerId);
    } catch {
      // Synthetic pointers may not support capture.
    }
    drawControlPoints();
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
    if (segmentPointDrag && event.pointerId === segmentPointDrag.pointerId) {
      event.preventDefault();
      const [x, y] = pointerPosition(event);
      const world = groundPointFromScreen(x, y, segmentPointDrag.planeY);
      if (world) {
        segmentPointDrag.position = [
          world[0],
          segmentPointDrag.planeY,
          world[2],
        ];
        segmentPointDrag.moved = true;
        drawControlPoints();
      }
      return;
    }
    if (guideDrag && event.pointerId === guideDrag.pointerId) {
      event.preventDefault();
      const guide = guideFor(
        guideDrag.kind,
        guideDrag.side,
        guideDrag.offset,
      );
      if (guide) {
        const [x, y] = pointerPosition(event);
        const point = hitGuidePoint(x, y, [guide]);
        if (point) {
          updateSelectedEdge(guideDrag.edge, point.index);
          drawControlPoints();
        }
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
  const index = dragIndex;
  const pointId = routeJunctions[index]?.id;
  dragIndex = null;
  dragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  drawControlPoints();
  if (pointId) {
    void moveControlPointById(pointId, controlPoints[index]);
  }
};

const endGuideDrag = (event: PointerEvent) => {
  if (!guideDrag || event.pointerId !== guideDrag.pointerId) {
    return;
  }
  const kind = guideDrag.kind;
  guideDrag = null;
  hoverHandleEdge = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  cameraController?.setEnabled(true);
  canvas.style.cursor = "";
  drawControlPoints();
  void (kind === "kerb" ? rebuildKerbSpans() : rebuildPropLines());
};

const endSegmentPointDrag = (event: PointerEvent) => {
  if (!segmentPointDrag || event.pointerId !== segmentPointDrag.pointerId) {
    return;
  }
  const drag = segmentPointDrag;
  segmentPointDrag = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  cameraController?.setEnabled(true);
  canvas.style.cursor = "";
  drawControlPoints();
  void (async () => {
    await rebuildEditor(
      "moving segment point",
      () =>
        editorViewer!.move_editor_segment_point(
          drag.segmentId,
          drag.index,
          drag.position[0],
          drag.position[2],
        ),
      reportActionError,
    );
    selectedSegmentPointIndex = drag.index;
    updateSegmentPanel();
    drawControlPoints();
  })();
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
  if (event.shiftKey !== deleteMode) {
    setDeleteMode(event.shiftKey);
  }
  drawControlPoints();
});

canvas.addEventListener("pointerleave", () => {
  lastPointer = null;
  hoverInsert = null;
  hoverDeleteIndex = null;
  hoverPoint = null;
  hoverHandleEdge = null;
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
  if (
    (event.key === "Delete" || event.key === "Backspace") &&
    decorationSelection &&
    !guideDrag
  ) {
    event.preventDefault();
    deleteSelectedSegment();
    return;
  }
  if (cameraViewMode === "car" && carDragPointerId === null && !guideDrag) {
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
  if (event.key === "Shift") {
    setDeleteMode(true);
  }
  if (event.key === "Escape" && decorationSelection && !guideDrag) {
    decorationSelection = null;
    hoverPoint = null;
    hoverHandleEdge = null;
    syncPaintControls();
    drawControlPoints();
  }
});

window.addEventListener("keyup", (event) => {
  if (event.key === "Shift") {
    setDeleteMode(false);
  }
});

window.addEventListener("blur", () => {
  setDeleteMode(false);
});

propToggle.addEventListener("click", () => {
  const index = PROP_KINDS.indexOf(paintKind);
  setPaintKind(PROP_KINDS[(index + 1) % PROP_KINDS.length]);
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

deleteSegmentButton.addEventListener("click", () => {
  deleteSelectedSegment();
});

overlayToggle.addEventListener("click", () => {
  setOverlayVisible(!overlayVisible);
});

const endCameraDrag = () => {
  cameraDragging = false;
};

window.addEventListener("pointerup", endDrag, { capture: true });
window.addEventListener("pointercancel", endDrag, { capture: true });
window.addEventListener("pointerup", endGuideDrag, { capture: true });
window.addEventListener("pointercancel", endGuideDrag, { capture: true });
window.addEventListener("pointerup", endSegmentPointDrag, { capture: true });
window.addEventListener("pointercancel", endSegmentPointDrag, { capture: true });
window.addEventListener("pointerup", endCarDrag, { capture: true });
window.addEventListener("pointercancel", endCarDrag, { capture: true });
window.addEventListener("pointerup", endCameraDrag, { capture: true });
window.addEventListener("pointercancel", endCameraDrag, { capture: true });

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
  status.textContent = "loading wasm";
  await init();

  resizeCanvas();
  status.textContent = "generating track";
  const viewer = await TrackViewer.create_editor(canvas);
  editorViewer = viewer;
  window.__trackEditor = viewer;
  (window as any).__setEditorProject = async (json: string) => {
    if (!editorViewer) throw new Error("editor unavailable");
    await rebuildEditor("loading project", () => editorViewer!.set_editor_project(json));
  };
  (window as any).__setEditorActiveLayout = async (id: string) => {
    if (!editorViewer) throw new Error("editor unavailable");
    await rebuildEditor("switching layout", () =>
      editorViewer!.set_editor_active_layout(id),
    );
  };
  (window as any).__editorMovePoint = (id: string, x: number, z: number) =>
    moveControlPointById(id, [x, 0, z]);
  (window as any).__editorSelectPoint = (id: string | null) =>
    selectControlPoint(id);
  (window as any).__editorSelectSegment = (id: string) => {
    setSelectedSegment(id);
  };
  if (devMode) {
    setupTrackDevMenu(viewer);
  }
  const defaultProject = viewer.editor_project();
  let bootWarning = "";
  const savedProject = loadSavedProject();
  if (savedProject) {
    try {
      await viewer.set_editor_project(savedProject);
    } catch (error) {
      console.warn("could not restore saved track project", error);
      bootWarning = "saved project could not be loaded; using default";
      try {
        await viewer.set_editor_project(defaultProject);
      } catch (fallbackError) {
        console.warn("could not restore default track project", fallbackError);
      }
    }
  }
  refreshEditorState();
  if (savedProject && !bootWarning) {
    saveProject();
  }
  syncPaintControls();
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
  status.textContent = "ready";
  status.className = "status ready";
  window.__trackEditorReady = true;
  if (bootWarning) {
    flashStatus(bootWarning);
  }
  requestAnimationFrame(frame);
  requestAnimationFrame(hideLoading);
}

function fail(error: unknown) {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__trackEditorError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
