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

interface SegmentSelection {
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

interface BranchDraftPoint {
  id: string;
  position: Vec3;
}

interface BranchDraft {
  sourceLoopId: string;
  junctionId: string;
  points: BranchDraftPoint[];
}

interface VenueSettings {
  width: number;
  kerb: { width: number; depth: number; elementLength: number };
  pit?: unknown;
}

interface ProjectLoop {
  id: string;
  name: string;
  controlPoints: string[];
  sectors: unknown[];
  cameras: unknown[];
  spawns: unknown[];
}

interface ProjectControlPoint {
  id: string;
  position: [number, number];
  kind?: string;
  tangent?: [number, number] | null;
  [key: string]: unknown;
}

type EditorSegmentRole = "main" | "branch";

interface EditorSegment {
  from: string;
  to: string;
  role: EditorSegmentRole;
  kerbs: unknown[];
  propSpans: unknown[];
  [key: string]: unknown;
}

interface TrackProject {
  schema: string;
  name: string;
  settings: VenueSettings;
  loops: ProjectLoop[];
  controlPoints: ProjectControlPoint[];
  segments: EditorSegment[];
  pitLane?: unknown;
  [key: string]: unknown;
}

interface LoopPointInfo {
  id: string;
  position: [number, number];
  kind: string;
  shared: boolean;
  sharedCount: number;
}

const TRACK_PROJECT_SCHEMA = "lapster/track-project@3";
const TRACK_PROJECT_STORAGE_KEY = "lapster.track-project.v1";
const DEFAULT_TRACK_WIDTH = 12;
const MIN_TRACK_WIDTH = 6;
const MAX_TRACK_WIDTH = 30;

const MAX_CAMERA_HEIGHT = 140;
const CONTROL_POINT_RADIUS = 9;
const CONTROL_POINT_HIT_RADIUS = 18;
const CENTERLINE_HIT_RADIUS = 12;
const KERB_RADIUS = 6.4;
const PROP_OFFSET = 14;
const PROP_FAR_OFFSET = 19;
const GUIDE_HIT_RADIUS = 12;
const GUIDE_HANDLE_RADIUS = 7;
const GUIDE_HANDLE_HIT_RADIUS = 16;
const BRANCH_DRAFT_DISTANCE = 25;
const BRANCH_APPROACH_DISTANCE = 20;
const BRANCH_SNAP_RADIUS = 12;
const LOOP_CENTERLINE_COLORS = [
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
const deleteLayoutButton =
  document.querySelector<HTMLButtonElement>("#delete-layout")!;
const layoutNameInput = document.querySelector<HTMLInputElement>("#layout-name")!;
const layoutWidthInput =
  document.querySelector<HTMLInputElement>("#layout-width")!;
const layoutHud = document.querySelector<HTMLElement>("#layout-hud")!;
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
const makeUniqueButton =
  document.querySelector<HTMLButtonElement>("#make-unique")!;

let editorViewer: TrackViewer | undefined;
let project: TrackProject | null = null;
let projectJson: string | null = null;
let activeLayoutId = "";
let syncingProjectControls = false;
let cameraController: OrbitCameraController | undefined;
let controlPoints: Vec3[] = [];
let loopPoints: LoopPointInfo[] = [];
let selectedPointId: string | null = null;
let centerline: Vec3[] = [];
let loopCenterlines: Record<string, Vec3[]> = {};
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
let selectedSegment: SegmentSelection | null = null;
let guideDrag: GuideDrag | null = null;
let branchDraft: BranchDraft | null = null;
let branchDraftPointSeq = 0;
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

function loopOutsideSide(): number {
  let area = 0;
  for (let index = 0; index < centerline.length; index += 1) {
    const point = centerline[index];
    const next = centerline[(index + 1) % centerline.length];
    area += point[0] * next[2] - next[0] * point[2];
  }
  return area > 0 ? -1 : 1;
}

function guideDefinitions(): GuideDef[] {
  const outside = loopOutsideSide();
  return [
    { kind: "kerb", side: -1, radius: KERB_RADIUS, color: "#fb7185" },
    { kind: "kerb", side: 1, radius: KERB_RADIUS, color: "#fb7185" },
    { kind: "prop", side: outside, radius: PROP_OFFSET, color: "#fb923c" },
    { kind: "prop", side: -outside, radius: PROP_OFFSET, color: "#2dd4bf" },
    { kind: "prop", side: outside, radius: PROP_FAR_OFFSET, color: "#f472b6" },
    { kind: "prop", side: -outside, radius: PROP_FAR_OFFSET, color: "#c084fc" },
  ];
}

function binormalAt(sample: number): Vec3 {
  const count = centerline.length;
  const index = ((sample % count) + count) % count;
  const incoming = normalize(
    sub(centerline[index], centerline[(index + count - 1) % count]),
  );
  const outgoing = normalize(
    sub(centerline[(index + 1) % count], centerline[index]),
  );
  const tangent = normalize(add(incoming, outgoing));
  return [-tangent[2], 0, tangent[0]];
}

function offsetPoint(
  index: number,
  side: number,
  radius: number,
): Vec3 | null {
  if (centerline.length < 2) {
    return null;
  }
  const count = centerline.length;
  const position = Math.max(0, Math.min(count, index));
  const base = Math.floor(position) % count;
  const next = (base + 1) % count;
  const t = position - Math.floor(position);
  const point = lerp(centerline[base], centerline[next], t);
  const binormal = normalize(lerp(binormalAt(base), binormalAt(next), t));
  return add(point, scale(binormal, radius * side));
}

function guidePoint(guide: GuideDef, index: number): Vec3 | null {
  return offsetPoint(index, guide.side, guide.radius);
}

function guidePoints(guide: GuideDef): Vec3[] {
  const points: Vec3[] = [];
  for (let index = 0; index <= centerline.length; index += 1) {
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
        index: Math.max(0, Math.min(centerline.length, index + t)),
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
  if (!selectedSegment || selectedSegment.kind !== "prop") {
    return undefined;
  }
  return propLines.find(
    (line) =>
      matchesLane(
        "prop",
        line,
        selectedSegment!.side,
        selectedSegment!.offset,
      ) &&
      Math.abs(line.start - selectedSegment!.start) < 1e-4 &&
      Math.abs(line.end - selectedSegment!.end) < 1e-4,
  );
}

function resolveSelection(): void {
  if (!selectedSegment) {
    return;
  }
  const range =
    findRangeExact(
      selectedSegment.kind,
      selectedSegment.side,
      selectedSegment.offset,
      selectedSegment.start,
      selectedSegment.end,
    ) ??
    findRange(
      selectedSegment.kind,
      selectedSegment.side,
      selectedSegment.offset,
      (selectedSegment.start + selectedSegment.end) * 0.5,
    );
  if (range) {
    selectedSegment.start = range.start;
    selectedSegment.end = range.end;
    if (range.offset !== undefined) {
      selectedSegment.offset = range.offset;
    }
  } else {
    selectedSegment = null;
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
  selectedSegment = {
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
  const count = centerline.length;
  if (count < 2) {
    return MIN_SEGMENT_LENGTH;
  }
  let perimeter = 0;
  for (let index = 0; index < count; index += 1) {
    const point = centerline[index];
    const next = centerline[(index + 1) % count];
    perimeter += Math.hypot(
      next[0] - point[0],
      next[1] - point[1],
      next[2] - point[2],
    );
  }
  const sampleSpacing = perimeter / count;
  return Math.max(
    MIN_SEGMENT_LENGTH,
    PROP_SEGMENT_LENGTH_METERS / Math.max(sampleSpacing, 1e-4),
  );
}

function createSegment(guide: GuideDef, index: number): void {
  const count = centerline.length;
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
    selectedSegment = {
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
  selectedSegment = {
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
  if (!selectedSegment) {
    return;
  }
  const oldStart = selectedSegment.start;
  const oldEnd = selectedSegment.end;
  const range = findRangeExact(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.offset,
    oldStart,
    oldEnd,
  );
  if (!range) {
    return;
  }
  if (edge === "start") {
    selectedSegment.start = Math.max(
      0,
      Math.min(index, selectedSegment.end - MIN_SEGMENT_LENGTH),
    );
  } else {
    selectedSegment.end = Math.min(
      centerline.length,
      Math.max(index, selectedSegment.start + MIN_SEGMENT_LENGTH),
    );
  }
  range.start = selectedSegment.start;
  range.end = selectedSegment.end;
}

function hitGuideHandle(
  x: number,
  y: number,
): { edge: "start" | "end"; guide: GuideDef } | null {
  if (!selectedSegment) {
    return null;
  }
  const guide = guideFor(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.offset,
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
    const point = guidePoint(guide, selectedSegment[edge]);
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
  deleteSegmentButton.disabled = selectedSegment === null;
}

function syncTreeDetailButton(): void {
  treeDetailButton.textContent = `Trees: ${TREE_DETAIL_LABELS[treeDetailMode]}`;
}

function deleteSelectedSegment(): void {
  if (!selectedSegment || rebuilding) {
    return;
  }
  const { kind, side, offset, start, end } = selectedSegment;
  selectedSegment = null;
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
    selectedSegment = null;
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
      Math.floor((index * loopPoints.length) / centerline.length) %
      Math.max(loopPoints.length, 1);
    const afterPointId = loopPoints[pointIndex]?.id;
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

function loopCenterlineColor(loopId: string): string {
  let hash = 0;
  for (let index = 0; index < loopId.length; index += 1) {
    hash = (hash * 31 + loopId.charCodeAt(index)) >>> 0;
  }
  return LOOP_CENTERLINE_COLORS[hash % LOOP_CENTERLINE_COLORS.length];
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

  for (const [loopId, points] of Object.entries(loopCenterlines)) {
    if (loopId === activeLayoutId) {
      continue;
    }
    drawCenterline(points, loopCenterlineColor(loopId), false);
  }
  const activeCenterline = loopCenterlines[activeLayoutId] ?? centerline;
  drawCenterline(activeCenterline, loopCenterlineColor(activeLayoutId), true);
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

  const guides = guideDefinitions();
  const selectedGuide = selectedSegment
    ? guideFor(
        selectedSegment.kind,
        selectedSegment.side,
        selectedSegment.offset,
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

  if (selectedSegment && selectedGuide) {
    const preview: Vec3[] = [];
    const steps = Math.max(
      2,
      Math.ceil((selectedSegment.end - selectedSegment.start) * 8),
    );
    for (let step = 0; step <= steps; step += 1) {
      const index =
        selectedSegment.start +
        ((selectedSegment.end - selectedSegment.start) * step) / steps;
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
      const point = guidePoint(selectedGuide, selectedSegment[edge]);
      const projected = point
        ? projectPoint(point, camera, width, height)
        : null;
      if (!projected) {
        continue;
      }
      const active =
        hoverHandleEdge === edge ||
        (guideDrag?.edge === edge && guideDrag.kind === selectedSegment.kind);
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
    const selected = loopPoints[index]?.id === selectedPointId;
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

  if (branchDraft) {
    const junctionIndex = loopPoints.findIndex(
      (point) => point.id === branchDraft!.junctionId,
    );
    const draftWorld: Vec3[] = [];
    if (junctionIndex >= 0 && controlPoints[junctionIndex]) {
      draftWorld.push(controlPoints[junctionIndex]);
    }
    for (const point of branchDraft.points) {
      draftWorld.push(point.position);
    }
    overlayContext.save();
    overlayContext.setLineDash([6, 4]);
    overlayContext.globalAlpha = 0.95;
    overlayContext.beginPath();
    let drawing = false;
    for (const point of draftWorld) {
      const projected = projectPoint(point, camera, width, height);
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
    overlayContext.strokeStyle = "#f472b6";
    overlayContext.stroke();
    overlayContext.restore();
    branchDraft.points.forEach((point, index) => {
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
      overlayContext.fillStyle = "#22d3ee";
      overlayContext.fill();
      overlayContext.lineWidth = 2;
      overlayContext.strokeStyle = "#f472b6";
      overlayContext.stroke();
      overlayContext.fillStyle = "#06121a";
      overlayContext.font = "600 10px ui-monospace, monospace";
      overlayContext.textAlign = "center";
      overlayContext.textBaseline = "middle";
      overlayContext.fillText(String(index + 1), projected[0], projected[1] + 0.5);
    });
  }
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
  if (loopPoints[index]) {
    loopPoints[index] = {
      ...loopPoints[index],
      position: [pointX, pointZ],
    };
  }
}

function activeLayout(): ProjectLoop | null {
  if (!project) {
    return null;
  }
  return (
    project.loops.find((layout) => layout.id === activeLayoutId) ??
    project.loops[0] ??
    null
  );
}

function trackWidth(): number {
  return project?.settings?.width ?? DEFAULT_TRACK_WIDTH;
}

function sharedPointCount(): number {
  return loopPoints.filter((point) => point.shared).length;
}

function selectedPointIndex(): number {
  if (!selectedPointId) {
    return -1;
  }
  return loopPoints.findIndex((point) => point.id === selectedPointId);
}

function syncPointControls(): void {
  const index = selectedPointIndex();
  const point = index >= 0 ? loopPoints[index] : null;
  makeUniqueButton.disabled = !point || rebuilding;
}

function segmentKey(from: string, to: string): string {
  return `${from}->${to}`;
}

function uniqueBranchBase(pointCount: number): {
  loopId: string;
  name: string;
  pointIds: string[];
} {
  const loopIds = new Set(project?.loops.map((loop) => loop.id) ?? []);
  const pointIds = new Set(project?.controlPoints.map((point) => point.id) ?? []);
  let number = 1;
  for (;;) {
    const loopId = `branch-${number}`;
    const ids = Array.from(
      { length: pointCount },
      (_, index) => `${loopId}-cp-${index + 1}`,
    );
    if (!loopIds.has(loopId) && ids.every((id) => !pointIds.has(id))) {
      return { loopId, name: `Branch ${number}`, pointIds: ids };
    }
    number += 1;
  }
}

function normalize2d(value: [number, number]): [number, number] {
  const length = Math.hypot(value[0], value[1]);
  return length > 1e-6 ? [value[0] / length, value[1] / length] : [1, 0];
}

function loopTangentAt(
  route: string[],
  positions: Map<string, [number, number]>,
  index: number,
): [number, number] | null {
  const count = route.length;
  const previous = positions.get(route[(index - 1 + count) % count]);
  const next = positions.get(route[(index + 1) % count]);
  if (!previous || !next) {
    return null;
  }
  return normalize2d([next[0] - previous[0], next[1] - previous[1]]);
}

function loopNormalAt(
  route: string[],
  positions: Map<string, [number, number]>,
  index: number,
): [number, number] | null {
  const tangent = loopTangentAt(route, positions, index);
  if (!tangent) {
    return null;
  }
  return normalize2d([-tangent[1], tangent[0]]);
}

function loopCentroid(
  route: string[],
  positions: Map<string, [number, number]>,
): [number, number] {
  const points = route
    .map((id) => positions.get(id))
    .filter((position): position is [number, number] => Boolean(position));
  if (points.length === 0) {
    return [0, 0];
  }
  return [
    points.reduce((sum, position) => sum + position[0], 0) / points.length,
    points.reduce((sum, position) => sum + position[1], 0) / points.length,
  ];
}

function outwardNormal(
  position: [number, number],
  normal: [number, number],
  centroid: [number, number],
): [number, number] {
  const distanceToCentroid = (candidate: [number, number]): number =>
    Math.hypot(candidate[0] - centroid[0], candidate[1] - centroid[1]);
  const plus: [number, number] = [
    position[0] + normal[0] * BRANCH_DRAFT_DISTANCE,
    position[1] + normal[1] * BRANCH_DRAFT_DISTANCE,
  ];
  const minus: [number, number] = [
    position[0] - normal[0] * BRANCH_DRAFT_DISTANCE,
    position[1] - normal[1] * BRANCH_DRAFT_DISTANCE,
  ];
  return distanceToCentroid(plus) >= distanceToCentroid(minus)
    ? normal
    : [-normal[0], -normal[1]];
}

function startBranchDraft(index: number): void {
  if (!project || !editorViewer || rebuilding) {
    return;
  }
  const layout = activeLayout();
  const junctionId = loopPoints[index]?.id;
  const junctionPosition = controlPoints[index];
  if (!layout || !junctionId || !junctionPosition) {
    return;
  }
  const route = layout.controlPoints;
  const startIndex = route.indexOf(junctionId);
  if (startIndex < 0) {
    flashStatus("Ctrl+click a control point on the active loop");
    return;
  }
  const positions = new Map(
    project.controlPoints.map((point) => [point.id, point.position]),
  );
  const position = positions.get(junctionId);
  if (!position) {
    flashStatus("branch point unavailable");
    return;
  }
  const normal = loopNormalAt(route, positions, startIndex);
  const centroid = loopCentroid(route, positions);
  const outward = normal
    ? outwardNormal(position, normal, centroid)
    : ([1, 0] as [number, number]);
  const firstPoint: Vec3 = [
    position[0] + outward[0] * BRANCH_DRAFT_DISTANCE,
    junctionPosition[1],
    position[1] + outward[1] * BRANCH_DRAFT_DISTANCE,
  ];
  branchDraft = {
    sourceLoopId: layout.id,
    junctionId,
    points: [{ id: `draft-${branchDraftPointSeq++}`, position: firstPoint }],
  };
  flashStatus(
    "branch started · Ctrl+click to add a point, Ctrl+click a loop point to close, Esc cancels",
  );
  drawControlPoints();
}

function sourceLoopControlPoints(
  loopId: string,
): { id: string; position: [number, number] }[] {
  const loop = project?.loops.find((item) => item.id === loopId);
  if (!project || !loop) {
    return [];
  }
  const positions = new Map(
    project.controlPoints.map((point) => [point.id, point.position]),
  );
  return loop.controlPoints.flatMap((id) => {
    const position = positions.get(id);
    return position ? [{ id, position }] : [];
  });
}

function appendBranchDraftPoint(x: number, y: number): void {
  if (!branchDraft) {
    return;
  }
  const planeY = branchDraft.points[0]?.position[1] ?? 0;
  const world = groundPointFromScreen(x, y, planeY);
  if (!world) {
    flashStatus("cannot place branch point here");
    return;
  }
  let nearestId: string | null = null;
  let nearestDistance = BRANCH_SNAP_RADIUS;
  for (const point of sourceLoopControlPoints(branchDraft.sourceLoopId)) {
    const distance = Math.hypot(
      world[0] - point.position[0],
      world[2] - point.position[1],
    );
    if (distance <= nearestDistance) {
      nearestDistance = distance;
      nearestId = point.id;
    }
  }
  if (nearestId) {
    void closeBranchDraft(nearestId);
    return;
  }
  branchDraft.points.push({
    id: `draft-${branchDraftPointSeq++}`,
    position: world,
  });
  drawControlPoints();
}

function cancelBranchDraft(): void {
  if (!branchDraft) {
    return;
  }
  branchDraft = null;
  flashStatus("branch cancelled");
  drawControlPoints();
}

async function closeBranchDraft(endId: string): Promise<void> {
  if (!editorViewer || !project || !branchDraft || rebuilding) {
    return;
  }
  const draft = branchDraft;
  const layout =
    project.loops.find((loop) => loop.id === draft.sourceLoopId) ??
    activeLayout();
  if (!layout) {
    cancelBranchDraft();
    return;
  }
  const route = layout.controlPoints;
  const startIndex = route.indexOf(draft.junctionId);
  const closesToJunction = endId === draft.junctionId;
  const endIndex = closesToJunction ? startIndex : route.indexOf(endId);
  if (startIndex < 0 || endIndex < 0) {
    flashStatus("Ctrl+click a different control point to close the branch");
    return;
  }
  const positions = new Map(
    project.controlPoints.map((point) => [point.id, point.position]),
  );
  const endPosition = positions.get(endId);
  const endTangent =
    endPosition && endIndex >= 0
      ? loopTangentAt(route, positions, endIndex)
      : null;
  const approachPoints: [number, number][] = [];
  if (endPosition && endTangent) {
    // Default branches loop past an outer point beside the closing junction
    // before turning in along the main loop tangent.
    if (!closesToJunction && draft.points.length <= 1) {
      const endNormal = loopNormalAt(route, positions, endIndex);
      if (endNormal) {
        const centroid = loopCentroid(route, positions);
        const outward = outwardNormal(endPosition, endNormal, centroid);
        approachPoints.push([
          endPosition[0] + outward[0] * BRANCH_DRAFT_DISTANCE,
          endPosition[1] + outward[1] * BRANCH_DRAFT_DISTANCE,
        ]);
      }
    }
    const nextIndex = (endIndex + 1) % route.length;
    const nextPosition = positions.get(route[nextIndex]);
    const approachBase = nextPosition ?? endPosition;
    approachPoints.push([
      approachBase[0] - endTangent[0] * BRANCH_APPROACH_DISTANCE,
      approachBase[1] - endTangent[1] * BRANCH_APPROACH_DISTANCE,
    ]);
  }
  const base = uniqueBranchBase(draft.points.length + approachPoints.length);
  const draftControlPoints: ProjectControlPoint[] = [
    ...draft.points.map(
      (point, index): ProjectControlPoint => ({
        id: base.pointIds[index],
        position: [point.position[0], point.position[2]],
        kind: "normal",
      }),
    ),
    ...approachPoints.map(
      (position, index): ProjectControlPoint => ({
        id: base.pointIds[draft.points.length + index],
        position,
        kind: "normal",
      }),
    ),
  ];
  const draftIds = draftControlPoints.map((point) => point.id);
  const newRoute = closesToJunction
    ? [draft.junctionId, ...draftIds]
    : [draft.junctionId, ...draftIds, endId];
  if (!closesToJunction) {
    for (
      let index = (endIndex + 1) % route.length;
      index !== startIndex;
      index = (index + 1) % route.length
    ) {
      newRoute.push(route[index]);
    }
  }
  const segments = project.segments.map((segment) => ({
    ...segment,
    role: segment.role === "branch" ? "branch" : "main",
    kerbs: Array.isArray(segment.kerbs) ? segment.kerbs : [],
    propSpans: Array.isArray(segment.propSpans) ? segment.propSpans : [],
  }));
  const pairs = new Set(
    segments.map((segment) => segmentKey(segment.from, segment.to)),
  );
  const addSegment = (
    from: string,
    to: string,
    role: EditorSegmentRole,
  ): void => {
    const key = segmentKey(from, to);
    if (pairs.has(key)) {
      return;
    }
    pairs.add(key);
    segments.push({ from, to, role, kerbs: [], propSpans: [] });
  };
  const branchRoute = closesToJunction
    ? []
    : [draft.junctionId, ...draftIds, endId];
  for (let index = 0; index < branchRoute.length - 1; index += 1) {
    addSegment(branchRoute[index], branchRoute[index + 1], "branch");
  }
  for (let index = 0; index < newRoute.length; index += 1) {
    addSegment(newRoute[index], newRoute[(index + 1) % newRoute.length], "main");
  }
  project.controlPoints.push(...draftControlPoints);
  project.segments = segments;
  project.loops.push({
    id: base.loopId,
    name: base.name,
    controlPoints: newRoute,
    sectors: [],
    cameras: [],
    spawns: [],
  });
  branchDraft = null;
  drawControlPoints();
  await rebuildEditor("creating branch", async () => {
    await editorViewer!.set_editor_project(JSON.stringify(project));
    await editorViewer!.set_editor_active_layout(base.loopId);
  });
}

function syncProjectControls(): void {
  const layout = activeLayout();
  syncingProjectControls = true;
  try {
    layoutSelect.replaceChildren();
    if (project) {
      for (const item of project.loops) {
        const option = document.createElement("option");
        option.value = item.id;
        option.textContent = item.name || item.id;
        option.selected = item.id === activeLayoutId;
        layoutSelect.append(option);
      }
    }
    layoutNameInput.value = layout?.name ?? "";
    layoutWidthInput.value = project ? String(trackWidth()) : "";
    deleteLayoutButton.disabled = !project || project.loops.length <= 1;
    layoutHud.textContent = layout
      ? `${layout.name || layout.id} · ${loopPoints.length} control points · ${sharedPointCount()} shared`
      : "—";
  } finally {
    syncingProjectControls = false;
  }
  syncPointControls();
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
      parsed.schema === TRACK_PROJECT_SCHEMA &&
      Array.isArray(parsed.loops)
    ) {
      parsed.segments = Array.isArray(parsed.segments)
        ? parsed.segments.map((segment) => ({
            ...segment,
            role: segment.role === "branch" ? "branch" : "main",
            kerbs: Array.isArray(segment.kerbs) ? segment.kerbs : [],
            propSpans: Array.isArray(segment.propSpans) ? segment.propSpans : [],
          }))
        : [];
      project = parsed;
      projectJson = JSON.stringify(parsed);
    } else {
      project = null;
      projectJson = null;
    }
  } catch {
    project = null;
    projectJson = null;
  }
  activeLayoutId = editorViewer.editor_active_layout();
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

function refreshEditorState(): void {
  if (!editorViewer) {
    return;
  }
  const heights = JSON.parse(editorViewer.editor_control_points()) as Vec3[];
  loopPoints = JSON.parse(editorViewer.editor_loop_points()) as LoopPointInfo[];
  controlPoints = loopPoints.map((point, index) => [
    point.position[0],
    heights[index]?.[1] ?? 0,
    point.position[1],
  ]);
  if (selectedPointId && selectedPointIndex() < 0) {
    selectedPointId = null;
  }
  centerline = JSON.parse(editorViewer.editor_centerline()) as Vec3[];
  loopCenterlines = JSON.parse(
    editorViewer.editor_loop_centerlines(),
  ) as Record<string, Vec3[]>;
  kerbSpans = (
    JSON.parse(editorViewer.editor_kerb_spans()) as KerbSpan[]
  ).sort((a, b) => a.side - b.side || a.start - b.start);
  propLines = JSON.parse(editorViewer.editor_prop_lines()) as PropLine[];
  readProject();
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
    if (activeLayoutId) {
      target.searchParams.set("layout", activeLayoutId);
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

deleteLayoutButton.addEventListener("click", () => {
  if (!activeLayoutId) {
    return;
  }
  void rebuildEditor(
    "deleting layout",
    () => editorViewer!.delete_editor_layout(activeLayoutId),
    reportActionError,
  );
});

makeUniqueButton.addEventListener("click", () => {
  void makeSelectedPointUnique();
});

layoutSelect.addEventListener("change", () => {
  const id = layoutSelect.value;
  if (!id || id === activeLayoutId) {
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
  const layout = activeLayout();
  const name = layoutNameInput.value.trim();
  if (!layout || !name || name === layout.name) {
    return;
  }
  void rebuildEditor("renaming layout", () =>
    editorViewer!.rename_editor_layout(activeLayoutId, name),
  );
});

layoutWidthInput.addEventListener("change", () => {
  if (syncingProjectControls) {
    return;
  }
  const width = Number(layoutWidthInput.value);
  if (!activeLayoutId || !Number.isFinite(width)) {
    return;
  }
  const clamped = Math.min(MAX_TRACK_WIDTH, Math.max(MIN_TRACK_WIDTH, width));
  if (Math.abs(clamped - trackWidth()) < 1e-4) {
    return;
  }
  void rebuildEditor("resizing track", () =>
    editorViewer!.set_editor_layout_width(activeLayoutId, clamped),
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
    cameraController?.setEnabled(true);
    drawControlPoints();
  }
}

async function rebuildKerbSpans(): Promise<void> {
  await rebuildEditor("rebuilding kerbs", () =>
    editorViewer!.set_editor_kerb_spans(JSON.stringify(kerbSpans)),
  );
}

async function rebuildPropLines(): Promise<void> {
  await rebuildEditor("rebuilding prop lines", () =>
    editorViewer!.set_editor_prop_lines(JSON.stringify(propLines)),
  );
}

function selectControlPoint(id: string | null): void {
  selectedPointId = id;
  syncPointControls();
  drawControlPoints();
}

async function deleteControlPoint(pointId: string): Promise<void> {
  if (!editorViewer || !activeLayoutId || rebuilding) {
    return;
  }
  await rebuildEditor("deleting control point", () =>
    editorViewer!.delete_editor_control_point(activeLayoutId, pointId),
  );
  if (selectedPointId === pointId) {
    selectedPointId = null;
  }
  syncPointControls();
  drawControlPoints();
}

async function insertControlPoint(
  point: Vec3,
  afterPointId: string,
): Promise<void> {
  if (!editorViewer || !activeLayoutId || rebuilding) {
    return;
  }
  let newId = "";
  await rebuildEditor("inserting control point", async () => {
    newId = await editorViewer!.insert_editor_control_point(
      activeLayoutId,
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

async function makeSelectedPointUnique(): Promise<void> {
  const index = selectedPointIndex();
  const point = index >= 0 ? loopPoints[index] : null;
  if (!point || !editorViewer || !activeLayoutId || rebuilding) {
    return;
  }
  let newId = "";
  await rebuildEditor("making control point unique", async () => {
    newId = await editorViewer!.make_editor_control_point_unique(
      activeLayoutId,
      point.id,
    );
  });
  if (newId) {
    selectControlPoint(newId);
  }
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
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      const index = overlayVisible ? hitTestControlPoint(x, y) : -1;
      if (branchDraft) {
        if (index >= 0) {
          const pointId = loopPoints[index]?.id;
          if (!pointId || pointId === branchDraft.junctionId) {
            flashStatus("Ctrl+click a different control point to close the branch");
            return;
          }
          void closeBranchDraft(pointId);
          return;
        }
        appendBranchDraftPoint(x, y);
        return;
      }
      if (index < 0) {
        flashStatus("Ctrl+click a control point to start a branch");
        return;
      }
      startBranchDraft(index);
      return;
    }
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
      const pointId = loopPoints[index]?.id;
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
        const handle = hitGuideHandle(x, y);
        if (handle && selectedSegment) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          guideDrag = {
            kind: selectedSegment.kind,
            side: selectedSegment.side,
            offset: selectedSegment.offset,
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
      if (selectedSegment) {
        selectedSegment = null;
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
    selectControlPoint(loopPoints[index]?.id ?? null);
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
  const pointId = loopPoints[index]?.id;
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
    selectedSegment &&
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
  if (event.key === "Escape" && branchDraft) {
    event.preventDefault();
    cancelBranchDraft();
    return;
  }
  if (event.key === "Escape" && selectedSegment && !guideDrag) {
    selectedSegment = null;
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
  (window as any).__editorMakeUnique = (loopId: string, pointId: string) =>
    rebuildEditor("making control point unique", async () => {
      const newId = await editorViewer!.make_editor_control_point_unique(
        loopId,
        pointId,
      );
      selectControlPoint(newId);
      return newId as any;
    });
  (window as any).__editorSelectPoint = (id: string | null) =>
    selectControlPoint(id);
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
