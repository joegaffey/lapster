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

type PointKind = "anchor" | "handle";

interface EditorPointData {
  id: string;
  position: [number, number];
  kind: PointKind;
  junction: string | null;
  end: string | null;
  degree: number;
  sections: string[];
  startFinish: boolean;
}

interface EditorPoint {
  id: string;
  position: Vec3;
  kind: PointKind;
  junction: string | null;
  end: string | null;
  degree: number;
  sections: string[];
  startFinish: boolean;
  looseEnd: boolean;
}

interface RouteLeg {
  sectionId: string;
  entry: string;
  exit: string;
  wrap?: boolean;
}

interface ProjectPoint {
  id: string;
  position: [number, number];
  kind?: PointKind;
  junction?: { kind?: string } | null;
  end?: string | null;
}

interface ContactInfo {
  point: Vec3;
  distance: number;
}

interface SectionHit {
  sectionId: string;
  position: Vec3;
  t: number;
}

interface ProjectSection {
  id: string;
  closed?: boolean;
  points: string[];
}

interface ProjectLayout {
  id: string;
  name?: string;
  route?: RouteLeg[];
  startFinish?: string;
}

type DecorationKind = "kerb" | "prop";
type PropKind = "blocks" | "tyres" | "trees" | "off";

interface DecorationLeg {
  sectionId: string;
  entryFraction: number;
  exitFraction: number;
  wrap?: boolean;
}

interface DecorationPath {
  id: string;
  kind: DecorationKind;
  side: number;
  lane?: string;
  chain: DecorationLeg[];
  elements?: number[];
  phase?: number;
  propKind?: PropKind;
  spacing?: number;
}

interface GuideRail {
  kind: DecorationKind;
  side: number;
  offset: number;
  lane?: string | null;
}

interface KerbSpan {
  start: number;
  end: number;
  side: number;
}

interface BakedPropLine {
  start: number;
  end: number;
  side: number;
  offset: number;
  kind: PropKind;
  spacing?: number | null;
  phase?: number;
}

interface TrackProjectDocument {
  schema?: string;
  name?: string;
  points?: ProjectPoint[];
  sections?: ProjectSection[];
  layouts?: ProjectLayout[];
  decorations?: DecorationPath[];
}

interface PaintStroke {
  pointerId: number;
  sectionId: string;
  rail: GuideRail;
  entryFraction: number;
  currentFraction: number;
}

type EditorMode = "points" | "kerbs" | "props";

function modeDecorationKind(mode: EditorMode): DecorationKind | null {
  if (mode === "kerbs") {
    return "kerb";
  }
  if (mode === "props") {
    return "prop";
  }
  return null;
}

interface ResizeDrag {
  pointerId: number;
  decorationId: string;
  endpoint: "entry" | "exit";
  sectionId: string;
  rail: GuideRail;
  currentFraction: number;
}

type Selection =
  | { kind: "point"; id: string }
  | { kind: "section"; id: string }
  | null;

const MAX_CAMERA_HEIGHT = 140;
const CONTROL_POINT_RADIUS = 9;
const CONTROL_POINT_HIT_RADIUS = 18;
const SECTION_HIT_RADIUS = 10;
const RAIL_HIT_RADIUS = 14;
const KERB_RAIL_OFFSET = 6.4;
const NEAR_PROP_OFFSET = 14;
const FAR_PROP_OFFSET = 19;
const DECORATION_HIT_RADIUS = 12;
const HANDLE_HIT_RADIUS = 14;
const MIN_DECORATION_FRACTION = 0.01;
const PROJECT_STORAGE_KEY = "lapster.track-editor.project.v6";
const PROJECT_SCHEMA = "lapster/track-project@6";
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
const selectionDetails =
  document.querySelector<HTMLElement>("#selection-details")!;
const pointEditor = document.querySelector<HTMLDialogElement>("#point-editor")!;
const pointEditorId =
  document.querySelector<HTMLElement>("#point-editor-id")!;
const pointEditorRows =
  document.querySelector<HTMLElement>("#point-editor-rows")!;
const pointEditorClose =
  document.querySelector<HTMLButtonElement>("#point-editor-close")!;
const deletePointButton =
  document.querySelector<HTMLButtonElement>("#delete-point")!;
const setStartFinishButton =
  document.querySelector<HTMLButtonElement>("#set-start-finish")!;
const makeJunctionButton =
  document.querySelector<HTMLButtonElement>("#make-junction")!;
const modeButtons = Array.from(
  document.querySelectorAll<HTMLButtonElement>("#editor-mode button"),
);
const decorationInspector =
  document.querySelector<HTMLDialogElement>("#decoration-inspector")!;
const decorationTitle =
  document.querySelector<HTMLElement>("#decoration-inspector-title")!;
const decorationHint =
  document.querySelector<HTMLElement>("#decoration-inspector-hint")!;
const decorationSide =
  document.querySelector<HTMLSelectElement>("#decoration-side")!;
const decorationSideField =
  document.querySelector<HTMLElement>("#decoration-side-field")!;
const decorationLane =
  document.querySelector<HTMLSelectElement>("#decoration-lane")!;
const decorationLaneField =
  document.querySelector<HTMLElement>("#decoration-lane-field")!;
const decorationPropKind =
  document.querySelector<HTMLSelectElement>("#decoration-prop-kind")!;
const decorationPropKindField =
  document.querySelector<HTMLElement>("#decoration-prop-kind-field")!;
const decorationSpacing =
  document.querySelector<HTMLInputElement>("#decoration-spacing")!;
const decorationSpacingField =
  document.querySelector<HTMLElement>("#decoration-spacing-field")!;
const decorationPhase =
  document.querySelector<HTMLInputElement>("#decoration-phase")!;
const decorationElements =
  document.querySelector<HTMLInputElement>("#decoration-elements")!;
const decorationElementsField =
  document.querySelector<HTMLElement>("#decoration-elements-field")!;
const decorationApply =
  document.querySelector<HTMLButtonElement>("#decoration-apply")!;
const decorationDelete =
  document.querySelector<HTMLButtonElement>("#decoration-delete")!;
const decorationStatus =
  document.querySelector<HTMLElement>("#decoration-status")!;

let editorViewer: TrackViewer | undefined;
let cameraController: OrbitCameraController | undefined;
let project: TrackProjectDocument = {};
let activeLayoutId = "";
let editorPoints: EditorPoint[] = [];
let sectionCenterlines: Record<string, Vec3[]> = {};
let routeSectionIds = new Set<string>();
let selection: Selection = null;
let centerline: Vec3[] = [];
let defaultProjectJson = "";
let dragIndex: number | null = null;
let dragPointerId: number | null = null;
let dragPlaneY = 0;
let dragStartPosition: Vec3 | null = null;
let insertHover: SectionHit | null = null;
let rebuilding = false;
let rebuildWaiters: Array<() => void> = [];
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
let routeLegs: RouteLeg[] = [];
let guideRails: GuideRail[] = [];
let editorMode: EditorMode = "points";
let decorationSeedAttempted = false;
let selectedDecorationId: string | null = null;
let hoverDecorationId: string | null = null;
let paintStroke: PaintStroke | null = null;
let resizeDrag: ResizeDrag | null = null;
let decorationIdSequence = 0;
let newKerbPhase = 0;
let newKerbElements = "";
let newPropPhase = 0;
let newPropSpacing = "";
let newPropKind: PropKind = "trees";
let newPropKindExplicit = false;

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

const clampFraction = (value: number): number =>
  Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));

function sectionIsClosed(sectionId: string): boolean {
  const section = (project.sections ?? []).find((item) => item.id === sectionId);
  return section?.closed !== false;
}

function railOffsetPoint(
  samples: Vec3[],
  index: number,
  closed: boolean,
  rail: GuideRail,
): Vec3 {
  const count = samples.length;
  const wrapped = ((index % count) + count) % count;
  const previousIndex = closed
    ? (wrapped + count - 1) % count
    : Math.max(0, wrapped - 1);
  const nextIndex = closed
    ? (wrapped + 1) % count
    : Math.min(count - 1, wrapped + 1);
  const previous = samples[previousIndex];
  const next = samples[nextIndex];
  let tangent = normalize([
    next[0] - previous[0],
    0,
    next[2] - previous[2],
  ]);
  if (Math.abs(tangent[0]) < 1e-8 && Math.abs(tangent[2]) < 1e-8) {
    tangent = [0, 0, -1];
  }
  const binormal: Vec3 = [-tangent[2], 0, tangent[0]];
  const sample = samples[wrapped];
  const lateral = rail.offset * rail.side;
  return [
    sample[0] + binormal[0] * lateral,
    sample[1] + 0.05,
    sample[2] + binormal[2] * lateral,
  ];
}

function railOffsetPoints(sectionId: string, rail: GuideRail): Vec3[] {
  const samples = sectionCenterlines[sectionId];
  if (!samples || samples.length < 2) {
    return [];
  }
  const closed = sectionIsClosed(sectionId);
  return samples.map((_, index) =>
    railOffsetPoint(samples, index, closed, rail),
  );
}

function railPointAtFraction(
  sectionId: string,
  rail: GuideRail,
  fraction: number,
): Vec3 | null {
  const samples = sectionCenterlines[sectionId];
  if (!samples || samples.length < 2) {
    return null;
  }
  const closed = sectionIsClosed(sectionId);
  const position = clampFraction(fraction) * (samples.length - 1);
  const first = Math.floor(position);
  const second = Math.min(samples.length - 1, first + 1);
  const t = position - first;
  return lerp(
    railOffsetPoint(samples, first, closed, rail),
    railOffsetPoint(samples, second, closed, rail),
    t,
  );
}

function railsForMode(mode: EditorMode): GuideRail[] {
  const kind = modeDecorationKind(mode);
  if (!kind) {
    return [];
  }
  return guideRails.filter((rail) => rail.kind === kind);
}

function decorationsForMode(mode: EditorMode): DecorationPath[] {
  const kind = modeDecorationKind(mode);
  if (!kind) {
    return [];
  }
  return (project.decorations ?? []).filter(
    (decoration) => decoration.kind === kind,
  );
}

function findDecoration(id: string | null): DecorationPath | null {
  if (!id) {
    return null;
  }
  return (project.decorations ?? []).find((decoration) => decoration.id === id) ?? null;
}

function selectedDecoration(): DecorationPath | null {
  const kind = modeDecorationKind(editorMode);
  if (!kind) {
    return null;
  }
  const decoration = findDecoration(selectedDecorationId);
  return decoration && decoration.kind === kind ? decoration : null;
}

function railForDecoration(decoration: DecorationPath): GuideRail | null {
  const side = decoration.side < 0 ? -1 : 1;
  if (decoration.kind === "kerb") {
    return (
      guideRails.find(
        (rail) => rail.kind === "kerb" && rail.side === side,
      ) ?? {
        kind: "kerb",
        side,
        offset: KERB_RAIL_OFFSET,
        lane: null,
      }
    );
  }
  const lane = decoration.lane ?? "";
  const byLane = guideRails.find(
    (rail) =>
      rail.kind === "prop" &&
      rail.side === side &&
      (rail.lane ?? "") === lane,
  );
  if (byLane) {
    return byLane;
  }
  const numeric = Number(lane);
  if (Number.isFinite(numeric) && lane.trim() !== "") {
    return (
      guideRails.find(
        (rail) =>
          rail.kind === "prop" &&
          rail.side === side &&
          Math.abs(rail.offset - numeric) < 1e-4,
      ) ?? { kind: "prop", side, offset: numeric, lane }
    );
  }
  const offset = lane === "far" ? FAR_PROP_OFFSET : NEAR_PROP_OFFSET;
  return (
    guideRails.find(
      (rail) =>
        rail.kind === "prop" &&
        rail.side === side &&
        Math.abs(rail.offset - offset) < 1e-4,
    ) ?? { kind: "prop", side, offset, lane: lane === "far" ? "far" : "near" }
  );
}

function railColor(rail: GuideRail, selected: boolean): string {
  const lane = (rail.lane ?? "").toLowerCase();
  if (rail.kind === "kerb") {
    return selected ? "#f9a8d4" : "rgba(244, 114, 182, 0.82)";
  }
  if (lane === "far" || Math.abs(rail.offset - FAR_PROP_OFFSET) < 1e-4) {
    return selected ? "#86efac" : "rgba(74, 222, 128, 0.82)";
  }
  if (rail.side < 0) {
    return selected ? "#fdba74" : "rgba(251, 146, 60, 0.82)";
  }
  return selected ? "#5eead4" : "rgba(45, 212, 191, 0.82)";
}

function railHitForMode(
  x: number,
  y: number,
  mode: EditorMode,
): { sectionId: string; fraction: number; rail: GuideRail } | null {
  for (const rail of railsForMode(mode)) {
    const hit = railScreenHit(x, y, rail);
    if (hit) {
      return { ...hit, rail };
    }
  }
  return null;
}

function railScreenHit(
  x: number,
  y: number,
  rail: GuideRail,
): { sectionId: string; fraction: number } | null {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best: { sectionId: string; fraction: number } | null = null;
  let bestDistance = RAIL_HIT_RADIUS;
  for (const sectionId of routeSectionIds) {
    const points = railOffsetPoints(sectionId, rail);
    if (points.length < 2) {
      continue;
    }
    const projected = points.map((point) =>
      projectPoint(point, camera, width, height),
    );
    const segmentCount = sectionIsClosed(sectionId)
      ? projected.length
      : projected.length - 1;
    for (let index = 0; index < segmentCount; index += 1) {
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
      const distance = Math.hypot(
        x - (start[0] + segmentX * t),
        y - (start[1] + segmentY * t),
      );
      if (distance < bestDistance) {
        bestDistance = distance;
        best = {
          sectionId,
          fraction: clampFraction(
            (index + t) / Math.max(1, points.length - 1),
          ),
        };
      }
    }
  }
  return best;
}

function railFractionOnSection(
  sectionId: string,
  rail: GuideRail,
  x: number,
  y: number,
): number {
  const camera = currentCamera();
  const points = railOffsetPoints(sectionId, rail);
  if (!camera || points.length < 2) {
    return 0;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const projected = points.map((point) =>
    projectPoint(point, camera, width, height),
  );
  const segmentCount = sectionIsClosed(sectionId)
    ? projected.length
    : projected.length - 1;
  let bestFraction = 0;
  let bestDistance = Infinity;
  for (let index = 0; index < segmentCount; index += 1) {
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
    const distance = Math.hypot(
      x - (start[0] + segmentX * t),
      y - (start[1] + segmentY * t),
    );
    if (distance < bestDistance) {
      bestDistance = distance;
      bestFraction = clampFraction(
        (index + t) / Math.max(1, points.length - 1),
      );
    }
  }
  return bestFraction;
}

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
  editorPoints.forEach((point, index) => {
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

function hitTestRouteSection(x: number, y: number): SectionHit | null {
  const camera = currentCamera();
  if (!camera || (project.sections ?? []).length === 0) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best: SectionHit | null = null;
  let bestDistance = SECTION_HIT_RADIUS;
  for (const section of project.sections ?? []) {
    const samples = sectionCenterlines[section.id];
    if (!samples || samples.length < 2) {
      continue;
    }
    const projected = samples.map((point) =>
      projectPoint(point, camera, width, height),
    );
    const segmentCount = section.closed === false ? projected.length - 1 : projected.length;
    const denominator = Math.max(1, samples.length - 1);
    for (let index = 0; index < segmentCount; index += 1) {
      const start = projected[index];
      const end = projected[(index + 1) % projected.length];
      if (!start || !end) {
        continue;
      }
      const segmentX = end[0] - start[0];
      const segmentY = end[1] - start[1];
      const lengthSquared = segmentX * segmentX + segmentY * segmentY;
      const segmentT =
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
      const distance = Math.hypot(
        x - (start[0] + segmentX * segmentT),
        y - (start[1] + segmentY * segmentT),
      );
      if (distance < bestDistance) {
        bestDistance = distance;
        const next = (index + 1) % samples.length;
        best = {
          sectionId: section.id,
          position: lerp(samples[index], samples[next], segmentT),
          t: Math.max(
            0,
            Math.min(1, (index + segmentT) / denominator),
          ),
        };
      }
    }
  }
  return best;
}

function insertAfterIndex(
  section: ProjectSection,
  t: number,
): number {
  const count = section.points.length;
  const raw =
    section.closed === false
      ? Math.floor(t * (count - 1))
      : Math.floor(t * count);
  return Math.max(0, Math.min(count - 1, raw));
}

function updateInsertHover(): void {
  insertHover = null;
  const camera = currentCamera();
  if (
    editorMode !== "points" ||
    !camera ||
    !overlayVisible ||
    !lastPointer ||
    dragIndex !== null ||
    carDragPointerId !== null ||
    rebuilding ||
    cameraDragging
  ) {
    return;
  }
  const [x, y] = lastPointer;
  if (hitTestCarMarker(x, y) || hitTestControlPoint(x, y) >= 0) {
    return;
  }
  insertHover = hitTestRouteSection(x, y);
}

function updateCursor(): void {
  if (rebuilding || cameraDragging) {
    return;
  }
  if (dragIndex !== null || carDragPointerId !== null) {
    return;
  }
  if (!overlayVisible || !lastPointer) {
    canvas.style.cursor = "";
    return;
  }
  const [x, y] = lastPointer;
  if (editorMode !== "points") {
    if (resizeDrag) {
      canvas.style.cursor = "ew-resize";
      return;
    }
    if (hitTestDecorationHandle(x, y)) {
      canvas.style.cursor = "ew-resize";
      return;
    }
    if (hoverDecorationId) {
      canvas.style.cursor = "pointer";
      return;
    }
    if (railHitForMode(x, y, editorMode)) {
      canvas.style.cursor = "crosshair";
      return;
    }
    canvas.style.cursor = "";
    return;
  }
  if (insertHover) {
    canvas.style.cursor = "copy";
    return;
  }
  if (
    hitTestCarMarker(x, y) ||
    hitTestControlPoint(x, y) >= 0
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

function setDecorationStatus(
  message: string,
  state: "" | "ready" | "error" = "",
): void {
  decorationStatus.textContent = message;
  decorationStatus.className = `status${state ? ` ${state}` : ""}`;
}

function sideLabel(side: number): string {
  return side < 0 ? "Left" : "Right";
}

function laneForOffset(offset: number): string {
  if (Math.abs(offset - NEAR_PROP_OFFSET) < 1e-4) {
    return "near";
  }
  if (Math.abs(offset - FAR_PROP_OFFSET) < 1e-4) {
    return "far";
  }
  return String(offset);
}

function parseElementLengths(value: string): number[] | null {
  const parts = value
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part !== "");
  const lengths: number[] = [];
  for (const part of parts) {
    const length = Number(part);
    if (!Number.isFinite(length) || length < 0) {
      return null;
    }
    lengths.push(length);
  }
  return lengths;
}

function parseOptionalSpacing(value: string): number | undefined | null {
  if (value.trim() === "") {
    return undefined;
  }
  const spacing = Number(value);
  if (!Number.isFinite(spacing) || spacing <= 0) {
    return null;
  }
  return spacing;
}

function propKindValue(value: string): PropKind {
  return value === "tyres" ? "tyres" : value === "trees" ? "trees" : "blocks";
}

function cloneDecoration(decoration: DecorationPath): DecorationPath {
  const clone: DecorationPath = {
    ...decoration,
    chain: decoration.chain.map((leg) => ({ ...leg })),
  };
  if (decoration.elements) {
    clone.elements = [...decoration.elements];
  }
  return clone;
}

function railLane(rail: GuideRail): string {
  if (rail.lane) {
    return rail.lane;
  }
  return laneForOffset(rail.offset);
}

function decorationScreenDistance(
  x: number,
  y: number,
  decoration: DecorationPath,
): number {
  const rail = railForDecoration(decoration);
  const camera = currentCamera();
  if (!rail || !camera) {
    return Infinity;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best = Infinity;
  for (const leg of decoration.chain) {
    const start = Math.min(leg.entryFraction, leg.exitFraction);
    const end = Math.max(leg.entryFraction, leg.exitFraction);
    const steps = Math.max(6, Math.ceil((end - start) * 120));
    let previous: [number, number] | null = null;
    for (let index = 0; index <= steps; index += 1) {
      const fraction = start + ((end - start) * index) / steps;
      const world = railPointAtFraction(leg.sectionId, rail, fraction);
      const projected = world
        ? projectPoint(world, camera, width, height)
        : null;
      if (!projected) {
        previous = null;
        continue;
      }
      if (previous) {
        const segmentX = projected[0] - previous[0];
        const segmentY = projected[1] - previous[1];
        const lengthSquared = segmentX * segmentX + segmentY * segmentY;
        const t =
          lengthSquared > 1e-6
            ? Math.max(
                0,
                Math.min(
                  1,
                  ((x - previous[0]) * segmentX +
                    (y - previous[1]) * segmentY) /
                    lengthSquared,
                ),
              )
            : 0;
        const distance = Math.hypot(
          x - (previous[0] + segmentX * t),
          y - (previous[1] + segmentY * t),
        );
        if (distance < best) {
          best = distance;
        }
      }
      previous = projected;
    }
  }
  return best;
}

function hitTestDecoration(x: number, y: number): string | null {
  if (editorMode === "points") {
    return null;
  }
  let bestId: string | null = null;
  let bestDistance = DECORATION_HIT_RADIUS;
  for (const decoration of decorationsForMode(editorMode)) {
    const distance = decorationScreenDistance(x, y, decoration);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestId = decoration.id;
    }
  }
  return bestId;
}

function decorationEndpoint(
  decoration: DecorationPath,
  endpoint: "entry" | "exit",
): { sectionId: string; fraction: number; rail: GuideRail } | null {
  const rail = railForDecoration(decoration);
  if (!rail || decoration.chain.length === 0) {
    return null;
  }
  const leg =
    endpoint === "entry"
      ? decoration.chain[0]
      : decoration.chain[decoration.chain.length - 1];
  return {
    sectionId: leg.sectionId,
    fraction: endpoint === "entry" ? leg.entryFraction : leg.exitFraction,
    rail,
  };
}

function hitTestDecorationHandle(
  x: number,
  y: number,
): {
  endpoint: "entry" | "exit";
  sectionId: string;
  fraction: number;
  rail: GuideRail;
} | null {
  const decoration = selectedDecoration();
  const camera = currentCamera();
  if (!decoration || !camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best: {
    endpoint: "entry" | "exit";
    sectionId: string;
    fraction: number;
    rail: GuideRail;
  } | null = null;
  let bestDistance = HANDLE_HIT_RADIUS;
  for (const endpoint of ["entry", "exit"] as const) {
    const target = decorationEndpoint(decoration, endpoint);
    if (!target) {
      continue;
    }
    const world = railPointAtFraction(
      target.sectionId,
      target.rail,
      target.fraction,
    );
    const projected = world
      ? projectPoint(world, camera, width, height)
      : null;
    if (!projected) {
      continue;
    }
    const distance = Math.hypot(projected[0] - x, projected[1] - y);
    if (distance < bestDistance) {
      bestDistance = distance;
      best = { endpoint, ...target };
    }
  }
  return best;
}

function resizedFractions(
  entry: number,
  exit: number,
  endpoint: "entry" | "exit",
  fraction: number,
): { entry: number; exit: number } {
  let nextEntry = entry;
  let nextExit = exit;
  const clamped = clampFraction(fraction);
  if (endpoint === "entry") {
    nextEntry = Math.min(clamped, exit - MIN_DECORATION_FRACTION);
  } else {
    nextExit = Math.max(clamped, entry + MIN_DECORATION_FRACTION);
  }
  if (nextEntry < 0) {
    nextEntry = 0;
    nextExit = Math.max(nextExit, MIN_DECORATION_FRACTION);
  }
  if (nextExit > 1) {
    nextExit = 1;
    nextEntry = Math.min(nextEntry, 1 - MIN_DECORATION_FRACTION);
  }
  return {
    entry: clampFraction(nextEntry),
    exit: clampFraction(nextExit),
  };
}

function decorationSpanColor(
  decoration: DecorationPath,
  selected: boolean,
  hovered: boolean,
): string {
  const rail = railForDecoration(decoration);
  const lane = rail ? railLane(rail).toLowerCase() : (decoration.lane ?? "near");
  if (decoration.kind === "kerb") {
    return selected
      ? "rgba(252, 231, 243, 1)"
      : hovered
        ? "rgba(249, 168, 212, 0.98)"
        : "rgba(244, 114, 182, 0.72)";
  }
  if (lane === "far") {
    return selected
      ? "rgba(220, 252, 231, 1)"
      : hovered
        ? "rgba(134, 239, 172, 0.98)"
        : "rgba(74, 222, 128, 0.72)";
  }
  if (decoration.side < 0) {
    return selected
      ? "rgba(255, 237, 213, 1)"
      : hovered
        ? "rgba(253, 186, 116, 0.98)"
        : "rgba(251, 146, 60, 0.72)";
  }
  return selected
    ? "rgba(204, 251, 241, 1)"
    : hovered
      ? "rgba(94, 234, 212, 0.98)"
      : "rgba(45, 212, 191, 0.72)";
}

function drawRailLine(
  camera: CameraView,
  width: number,
  height: number,
  rail: GuideRail,
  color: string,
  lineWidth: number,
  dash: number[],
): void {
  for (const sectionId of routeSectionIds) {
    const points = railOffsetPoints(sectionId, rail);
    if (points.length < 2) {
      continue;
    }
    overlayContext.save();
    overlayContext.beginPath();
    let drawing = false;
    const closed = sectionIsClosed(sectionId);
    const last = closed ? points.length : points.length - 1;
    for (let index = 0; index <= last; index += 1) {
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
    overlayContext.lineWidth = lineWidth;
    overlayContext.strokeStyle = color;
    overlayContext.setLineDash(dash);
    overlayContext.stroke();
    overlayContext.restore();
  }
}

function drawRailSpan(
  camera: CameraView,
  width: number,
  height: number,
  sectionId: string,
  rail: GuideRail,
  entry: number,
  exit: number,
  color: string,
  lineWidth: number,
): void {
  const start = Math.min(entry, exit);
  const end = Math.max(entry, exit);
  if (end - start <= 1e-5) {
    return;
  }
  const steps = Math.max(8, Math.ceil((end - start) * 120));
  overlayContext.save();
  overlayContext.beginPath();
  let drawing = false;
  for (let index = 0; index <= steps; index += 1) {
    const fraction = start + ((end - start) * index) / steps;
    const world = railPointAtFraction(sectionId, rail, fraction);
    const projected = world
      ? projectPoint(world, camera, width, height)
      : null;
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
  overlayContext.lineWidth = lineWidth;
  overlayContext.strokeStyle = color;
  overlayContext.setLineDash([]);
  overlayContext.stroke();
  overlayContext.restore();
}

function drawRailTick(
  camera: CameraView,
  width: number,
  height: number,
  sectionId: string,
  rail: GuideRail,
  fraction: number,
  color: string,
): void {
  const delta = 0.004;
  const before = railPointAtFraction(sectionId, rail, fraction - delta);
  const after = railPointAtFraction(sectionId, rail, fraction + delta);
  const center = railPointAtFraction(sectionId, rail, fraction);
  if (!before || !after || !center) {
    return;
  }
  const a = projectPoint(before, camera, width, height);
  const b = projectPoint(after, camera, width, height);
  const c = projectPoint(center, camera, width, height);
  if (!a || !b || !c) {
    return;
  }
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const length = Math.hypot(dx, dy) || 1;
  const nx = -dy / length;
  const ny = dx / length;
  overlayContext.save();
  overlayContext.beginPath();
  overlayContext.moveTo(c[0] - nx * 4, c[1] - ny * 4);
  overlayContext.lineTo(c[0] + nx * 4, c[1] + ny * 4);
  overlayContext.lineWidth = 2;
  overlayContext.strokeStyle = color;
  overlayContext.setLineDash([]);
  overlayContext.stroke();
  overlayContext.restore();
}

function drawDecorationOverlay(
  camera: CameraView,
  width: number,
  height: number,
): void {
  if (editorMode === "points") {
    return;
  }
  for (const rail of railsForMode(editorMode)) {
    drawRailLine(
      camera,
      width,
      height,
      rail,
      railColor(rail, false),
      1.1,
      [4, 5],
    );
  }

  for (const decoration of decorationsForMode(editorMode)) {
    const rail = railForDecoration(decoration);
    if (!rail) {
      continue;
    }
    const selected = decoration.id === selectedDecorationId;
    const hovered = decoration.id === hoverDecorationId && !selected;
    const color = decorationSpanColor(decoration, selected, hovered);
    const lineWidth = selected ? 6 : hovered ? 5 : 3.5;
    for (const leg of decoration.chain) {
      drawRailSpan(
        camera,
        width,
        height,
        leg.sectionId,
        rail,
        leg.entryFraction,
        leg.exitFraction,
        color,
        lineWidth,
      );
      drawRailTick(
        camera,
        width,
        height,
        leg.sectionId,
        rail,
        leg.entryFraction,
        color,
      );
      drawRailTick(
        camera,
        width,
        height,
        leg.sectionId,
        rail,
        leg.exitFraction,
        color,
      );
    }
  }

  if (resizeDrag) {
    const decoration = findDecoration(resizeDrag.decorationId);
    const rail = decoration ? railForDecoration(decoration) : null;
    if (decoration && rail) {
      const legIndex =
        resizeDrag.endpoint === "entry" ? 0 : decoration.chain.length - 1;
      const leg = decoration.chain[legIndex];
      if (leg) {
        const fractions = resizedFractions(
          leg.entryFraction,
          leg.exitFraction,
          resizeDrag.endpoint,
          resizeDrag.currentFraction,
        );
        drawRailSpan(
          camera,
          width,
          height,
          leg.sectionId,
          rail,
          fractions.entry,
          fractions.exit,
          "rgba(250, 204, 21, 0.95)",
          4,
        );
      }
    }
  }

  if (paintStroke) {
    const start = Math.min(
      paintStroke.entryFraction,
      paintStroke.currentFraction,
    );
    const end = Math.max(
      paintStroke.entryFraction,
      paintStroke.currentFraction,
    );
    drawRailSpan(
      camera,
      width,
      height,
      paintStroke.sectionId,
      paintStroke.rail,
      start,
      end,
      "rgba(250, 204, 21, 0.95)",
      4,
    );
  }

  const selected = selectedDecoration();
  if (selected) {
    const cameraTarget = currentCamera();
    if (!cameraTarget) {
      return;
    }
    for (const endpoint of ["entry", "exit"] as const) {
      const target = decorationEndpoint(selected, endpoint);
      if (!target) {
        continue;
      }
      let fraction = target.fraction;
      if (resizeDrag && resizeDrag.decorationId === selected.id) {
        const legIndex =
          resizeDrag.endpoint === "entry" ? 0 : selected.chain.length - 1;
        const leg = selected.chain[legIndex];
        if (leg) {
          const fractions = resizedFractions(
            leg.entryFraction,
            leg.exitFraction,
            resizeDrag.endpoint,
            resizeDrag.currentFraction,
          );
          fraction =
            resizeDrag.endpoint === "entry" ? fractions.entry : fractions.exit;
        }
      }
      const world = railPointAtFraction(
        target.sectionId,
        target.rail,
        fraction,
      );
      const projected = world
        ? projectPoint(world, cameraTarget, width, height)
        : null;
      if (!projected) {
        continue;
      }
      overlayContext.save();
      overlayContext.beginPath();
      overlayContext.arc(projected[0], projected[1], 6, 0, Math.PI * 2);
      overlayContext.fillStyle = "#f8fafc";
      overlayContext.fill();
      overlayContext.lineWidth = 2;
      overlayContext.strokeStyle = "#0f172a";
      overlayContext.stroke();
      overlayContext.restore();
    }
  }
}

function renderDecorationInspector(): void {
  if (editorMode === "points") {
    if (decorationInspector.open) {
      decorationInspector.close();
    }
    return;
  }
  if (!decorationInspector.open) {
    decorationInspector.show();
  }
  const isKerb = editorMode === "kerbs";
  const decoration = selectedDecoration();
  decorationTitle.textContent = isKerb ? "Kerb" : "Prop";
  decorationHint.textContent = decoration
    ? `${sideLabel(decoration.side)} ${isKerb ? "kerb" : "prop"} selected · drag an endpoint handle to resize`
    : isKerb
      ? "Drag a kerb rail to paint a new span."
      : "Drag a prop rail to paint. Trees default on far, blocks on near.";
  decorationSideField.hidden = !decoration;
  decorationLaneField.hidden = !decoration || isKerb;
  decorationPropKindField.hidden = isKerb;
  decorationSpacingField.hidden = isKerb;
  decorationElementsField.hidden = !isKerb;
  decorationApply.disabled = !decoration;
  decorationDelete.hidden = !decoration;

  if (decoration) {
    decorationSide.value = decoration.side < 0 ? "-1" : "1";
    decorationPhase.value = String(decoration.phase ?? 0);
    if (isKerb) {
      decorationElements.value = (decoration.elements ?? []).join(", ");
    } else {
      decorationLane.value =
        (decoration.lane ?? "near") === "far" ? "far" : "near";
      decorationPropKind.value = propKindValue(decoration.propKind ?? "blocks");
      decorationSpacing.value =
        typeof decoration.spacing === "number"
          ? String(decoration.spacing)
          : "";
    }
    return;
  }

  if (isKerb) {
    decorationPhase.value = String(newKerbPhase);
    decorationElements.value = newKerbElements;
  } else {
    decorationPhase.value = String(newPropPhase);
    decorationSpacing.value = newPropSpacing;
    decorationPropKind.value = newPropKind;
  }
}

function syncModeButtons(): void {
  for (const button of modeButtons) {
    const active = button.dataset.mode === editorMode;
    button.setAttribute("aria-pressed", String(active));
  }
}

function setMode(mode: EditorMode): void {
  const changed = mode !== editorMode;
  editorMode = mode;
  selectedDecorationId = null;
  hoverDecorationId = null;
  paintStroke = null;
  resizeDrag = null;
  selection = null;
  cameraDragging = false;
  renderSelection();
  syncModeButtons();
  renderDecorationInspector();
  if (changed) {
    if (mode === "kerbs") {
      setDecorationStatus("Drag a kerb rail to paint.");
    } else if (mode === "props") {
      setDecorationStatus("Drag a prop rail to paint.");
    } else {
      setDecorationStatus("");
    }
  }
  if ((mode === "kerbs" || mode === "props") && !decorationSeedAttempted) {
    decorationSeedAttempted = true;
    void seedDefaultDecorations();
  }
  if (overlayVisible) {
    canvas.style.cursor = "";
  }
  drawControlPoints();
}

async function applyDecorationProject(
  next: TrackProjectDocument,
  message = "updating decorations",
): Promise<boolean> {
  if (!editorViewer || rebuilding) {
    return false;
  }
  rebuilding = true;
  suspended = true;
  testDriveButton.disabled = true;
  cameraController?.setEnabled(false);
  setStatus(message);
  try {
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve()),
    );
    await editorViewer.set_editor_project(JSON.stringify(next));
    refreshEditorState();
    testDriveButton.disabled = false;
    setStatus("ready", "ready");
    return true;
  } catch (error) {
    const text = error instanceof Error ? error.message : String(error);
    try {
      await editorViewer.set_editor_project(editorViewer.editor_project());
    } catch (reloadError) {
      console.warn("could not reload the previous track project", reloadError);
    }
    refreshEditorState();
    setDecorationStatus(text, "error");
    setStatus(`decoration failed: ${text}`, "error");
    return false;
  } finally {
    suspended = false;
    rebuilding = false;
    testDriveButton.disabled = false;
    cameraController?.setEnabled(true);
    renderDecorationInspector();
    drawControlPoints();
  }
}

async function applyDecorationProperties(): Promise<void> {
  const decoration = selectedDecoration();
  if (!decoration || !editorViewer) {
    return;
  }
  const updated = cloneDecoration(decoration);
  updated.side = Number(decorationSide.value) < 0 ? -1 : 1;
  updated.phase = numberInputValue(decorationPhase, updated.phase ?? 0);
  if (updated.kind === "kerb") {
    const elements = parseElementLengths(decorationElements.value);
    if (elements === null) {
      setDecorationStatus(
        "element lengths must be comma-separated numbers",
        "error",
      );
      return;
    }
    updated.elements = elements;
  } else {
    updated.lane = decorationLane.value === "far" ? "far" : "near";
    updated.propKind = propKindValue(decorationPropKind.value);
    const spacing = parseOptionalSpacing(decorationSpacing.value);
    if (spacing === null) {
      setDecorationStatus(
        "spacing must be a positive number or empty",
        "error",
      );
      return;
    }
    if (spacing === undefined) {
      delete updated.spacing;
    } else {
      updated.spacing = spacing;
    }
  }
  const applied = await applyDecorationProject(
    {
      ...project,
      decorations: (project.decorations ?? []).map((item) =>
        item.id === updated.id ? updated : item,
      ),
    },
    "updating decoration",
  );
  if (applied) {
    setDecorationStatus("decoration updated", "ready");
  }
}

async function commitDecorationResize(): Promise<void> {
  const drag = resizeDrag;
  resizeDrag = null;
  if (!drag || !editorViewer) {
    return;
  }
  const decoration = findDecoration(drag.decorationId);
  if (!decoration) {
    return;
  }
  const updated = cloneDecoration(decoration);
  const legIndex = drag.endpoint === "entry" ? 0 : updated.chain.length - 1;
  const leg = updated.chain[legIndex];
  if (!leg) {
    return;
  }
  const fractions = resizedFractions(
    leg.entryFraction,
    leg.exitFraction,
    drag.endpoint,
    drag.currentFraction,
  );
  if (
    Math.abs(fractions.entry - leg.entryFraction) < 1e-5 &&
    Math.abs(fractions.exit - leg.exitFraction) < 1e-5
  ) {
    drawControlPoints();
    return;
  }
  leg.entryFraction = fractions.entry;
  leg.exitFraction = fractions.exit;
  const applied = await applyDecorationProject(
    {
      ...project,
      decorations: (project.decorations ?? []).map((item) =>
        item.id === updated.id ? updated : item,
      ),
    },
    "resizing decoration",
  );
  if (applied) {
    setDecorationStatus("decoration resized", "ready");
  }
}

async function seedDefaultDecorations(): Promise<void> {
  if (!editorViewer) {
    return;
  }
  if ((project.decorations ?? []).length > 0) {
    return;
  }
  if (routeLegs.length !== 1 || routeSectionIds.size !== 1) {
    setDecorationStatus("defaults not imported for multi-section routes yet");
    return;
  }
  const sectionId = routeLegs[0]?.sectionId ?? "";
  const routeLength = centerline.length;
  if (!sectionId || routeLength <= 0) {
    setDecorationStatus("no active route to import defaults onto");
    return;
  }
  const decorations: DecorationPath[] = [];
  const kerbSpans = JSON.parse(
    editorViewer.editor_kerb_spans(),
  ) as KerbSpan[];
  kerbSpans.forEach((span, index) => {
    const entryFraction = clampFraction(span.start / routeLength);
    const exitFraction = clampFraction(span.end / routeLength);
    if (exitFraction - entryFraction <= 1e-4) {
      return;
    }
    decorations.push({
      id: `default-kerb-${index}`,
      kind: "kerb",
      side: span.side < 0 ? -1 : 1,
      chain: [
        {
          sectionId,
          entryFraction,
          exitFraction,
          wrap: false,
        },
      ],
      elements: [],
      phase: 0,
    });
  });
  const propLines = JSON.parse(
    editorViewer.editor_prop_lines(),
  ) as BakedPropLine[];
  propLines.forEach((line, index) => {
    if (line.kind === "off") {
      return;
    }
    const entryFraction = clampFraction(line.start / routeLength);
    const exitFraction = clampFraction(line.end / routeLength);
    if (exitFraction - entryFraction <= 1e-4) {
      return;
    }
    const decoration: DecorationPath = {
      id: `default-prop-${index}`,
      kind: "prop",
      side: line.side < 0 ? -1 : 1,
      lane: laneForOffset(line.offset),
      chain: [
        {
          sectionId,
          entryFraction,
          exitFraction,
          wrap: false,
        },
      ],
      phase: line.phase ?? 0,
      propKind: line.kind,
    };
    if (
      typeof line.spacing === "number" &&
      Number.isFinite(line.spacing)
    ) {
      decoration.spacing = line.spacing;
    }
    decorations.push(decoration);
  });
  if (decorations.length === 0) {
    setDecorationStatus("no baked decorations to import");
    return;
  }
  setDecorationStatus("importing default decorations…");
  const applied = await applyDecorationProject(
    { ...project, decorations },
    "importing default decorations",
  );
  if (applied) {
    setDecorationStatus(
      `imported ${decorations.length} default decorations`,
      "ready",
    );
  }
}

function nextDecorationId(kind: DecorationKind): string {
  decorationIdSequence += 1;
  return `paint-${kind}-${Date.now()}-${decorationIdSequence}`;
}

function numberInputValue(input: HTMLInputElement, fallback: number): number {
  const value = Number(input.value);
  return Number.isFinite(value) ? value : fallback;
}

function paintPropKindForRail(rail: GuideRail): PropKind {
  if (newPropKindExplicit) {
    return newPropKind;
  }
  return (rail.lane ?? "").toLowerCase() === "far" ||
    Math.abs(rail.offset - FAR_PROP_OFFSET) < 1e-4
    ? "trees"
    : "blocks";
}

function buildPaintDecoration(
  stroke: PaintStroke,
  entryFraction: number,
  exitFraction: number,
): DecorationPath {
  const kind = modeDecorationKind(editorMode) ?? "kerb";
  const decoration: DecorationPath = {
    id: nextDecorationId(kind),
    kind,
    side: stroke.rail.side < 0 ? -1 : 1,
    chain: [
      {
        sectionId: stroke.sectionId,
        entryFraction,
        exitFraction,
        wrap: false,
      },
    ],
    phase: kind === "kerb" ? newKerbPhase : newPropPhase,
  };
  if (kind === "kerb") {
    decoration.elements = parseElementLengths(newKerbElements) ?? [];
    return decoration;
  }
  const lane = stroke.rail.lane ?? laneForOffset(stroke.rail.offset);
  decoration.lane = lane;
  decoration.propKind = paintPropKindForRail(stroke.rail);
  const spacing = parseOptionalSpacing(newPropSpacing);
  if (typeof spacing === "number") {
    decoration.spacing = spacing;
  }
  return decoration;
}

async function commitPaint(
  stroke: PaintStroke,
  entryFraction: number,
  exitFraction: number,
): Promise<void> {
  if (!editorViewer) {
    return;
  }
  if (
    editorMode === "kerbs" &&
    parseElementLengths(newKerbElements) === null
  ) {
    setDecorationStatus(
      "element lengths must be comma-separated numbers",
      "error",
    );
    return;
  }
  if (editorMode === "props" && parseOptionalSpacing(newPropSpacing) === null) {
    setDecorationStatus(
      "spacing must be a positive number or empty",
      "error",
    );
    return;
  }
  const decoration = buildPaintDecoration(stroke, entryFraction, exitFraction);
  const applied = await applyDecorationProject(
    {
      ...project,
      decorations: [...(project.decorations ?? []), decoration],
    },
    "painting decoration",
  );
  if (applied) {
    selectedDecorationId = decoration.id;
    renderDecorationInspector();
    drawControlPoints();
    setDecorationStatus(
      `painted ${decoration.kind} ${sideLabel(decoration.side)}`,
      "ready",
    );
  }
}

async function deleteDecoration(id: string): Promise<void> {
  if (!editorViewer) {
    return;
  }
  const applied = await applyDecorationProject(
    {
      ...project,
      decorations: (project.decorations ?? []).filter(
        (decoration) => decoration.id !== id,
      ),
    },
    "deleting decoration",
  );
  if (applied) {
    if (selectedDecorationId === id) {
      selectedDecorationId = null;
      renderDecorationInspector();
      drawControlPoints();
    }
    setDecorationStatus("decoration deleted", "ready");
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
  updateInsertHover();
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
    overlayContext.lineWidth = editorMode === "points" ? 2 : 1;
    overlayContext.strokeStyle =
      editorMode === "points"
        ? "rgba(56, 189, 248, 0.9)"
        : "rgba(56, 189, 248, 0.32)";
    overlayContext.stroke();
  }

  if (editorMode === "points") {
    for (const section of project.sections ?? []) {
      if (routeSectionIds.has(section.id)) {
        continue;
      }
      const samples = sectionCenterlines[section.id];
      if (!samples || samples.length < 2) {
        continue;
      }
      overlayContext.save();
      overlayContext.beginPath();
      let drawing = false;
      for (const sample of samples) {
        const projected = projectPoint(sample, camera, width, height);
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
      overlayContext.strokeStyle = "rgba(167, 139, 250, 0.8)";
      overlayContext.setLineDash([6, 4]);
      overlayContext.stroke();
      overlayContext.restore();
    }
  }

  if (editorMode === "points" && selection?.kind === "section") {
    const samples = sectionCenterlines[selection.id];
    if (samples && samples.length > 1) {
      overlayContext.beginPath();
      let drawing = false;
      for (const sample of samples) {
        const projected = projectPoint(sample, camera, width, height);
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
      overlayContext.lineWidth = 5;
      overlayContext.strokeStyle = "rgba(245, 158, 11, 0.9)";
      overlayContext.stroke();
    }
  }

  if (editorMode !== "points") {
    drawDecorationOverlay(camera, width, height);
  }

  if (editorMode === "points" && insertHover) {
    const projected = projectPoint(
      insertHover.position,
      camera,
      width,
      height,
    );
    if (projected) {
      const [insertX, insertY] = projected;
      overlayContext.save();
      overlayContext.beginPath();
      overlayContext.arc(insertX, insertY, 7, 0, Math.PI * 2);
      overlayContext.lineWidth = 2;
      overlayContext.strokeStyle = "rgba(56, 189, 248, 0.95)";
      overlayContext.setLineDash([3, 3]);
      overlayContext.stroke();
      overlayContext.setLineDash([]);
      overlayContext.beginPath();
      overlayContext.moveTo(insertX - 4, insertY);
      overlayContext.lineTo(insertX + 4, insertY);
      overlayContext.moveTo(insertX, insertY - 4);
      overlayContext.lineTo(insertX, insertY + 4);
      overlayContext.lineWidth = 2;
      overlayContext.strokeStyle = "rgba(56, 189, 248, 0.95)";
      overlayContext.stroke();
      overlayContext.restore();
    }
  }

  if (editorMode === "points") {
  const pointPositions = new Map(
    editorPoints.map((point) => [point.id, point.position]),
  );
  const pointsById = new Map(editorPoints.map((point) => [point.id, point]));
  const drawnSections =
    selection?.kind === "section"
      ? (project.sections ?? []).filter(
          (section) => section.id === selection?.id,
        )
      : (project.sections ?? []);

  for (const section of drawnSections) {
    const positions: Vec3[] = [];
    for (const id of section.points) {
      const position = pointPositions.get(id);
      if (position) {
        positions.push(position);
      }
    }
    if (positions.length > 1) {
      const polygon = section.closed ? [...positions, positions[0]] : positions;
      overlayContext.save();
      overlayContext.beginPath();
      let drawing = false;
      for (const position of polygon) {
        const projected = projectPoint(position, camera, width, height);
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
      overlayContext.lineWidth = 1;
      overlayContext.strokeStyle = "rgba(148, 163, 184, 0.65)";
      overlayContext.setLineDash([4, 4]);
      overlayContext.stroke();
      overlayContext.restore();
    }

    const samples = sectionCenterlines[section.id];
    if (!samples || samples.length === 0) {
      continue;
    }
    const seen = new Set<string>();
    for (const id of section.points) {
      if (seen.has(id)) {
        continue;
      }
      seen.add(id);
      const point = pointsById.get(id);
      if (!point) {
        continue;
      }
      const contact = nearestCenterlineContact(point.position, samples);
      if (!contact || contact.distance <= CONTACT_TOLERANCE) {
        continue;
      }
      const start = projectPoint(point.position, camera, width, height);
      const end = projectPoint(contact.point, camera, width, height);
      if (!start || !end) {
        continue;
      }
      const colors = contactColors(point);
      overlayContext.save();
      overlayContext.beginPath();
      overlayContext.moveTo(start[0], start[1]);
      overlayContext.lineTo(end[0], end[1]);
      overlayContext.lineWidth = 1;
      overlayContext.strokeStyle = colors.line;
      overlayContext.setLineDash([2, 4]);
      overlayContext.stroke();
      overlayContext.setLineDash([]);
      overlayContext.beginPath();
      overlayContext.arc(end[0], end[1], 2.5, 0, Math.PI * 2);
      overlayContext.fillStyle = colors.dot;
      overlayContext.fill();
      overlayContext.restore();
    }
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

  if (editorMode === "points") {
  editorPoints.forEach((point, index) => {
    const projected = projectPoint(point.position, camera, width, height);
    if (!projected) {
      return;
    }
    const selected =
      index === dragIndex ||
      (selection?.kind === "point" && selection.id === point.id);
    const roleColor = point.startFinish
      ? "#22c55e"
      : point.junction
        ? "#f97316"
        : point.looseEnd
          ? "#a78bfa"
          : "#fbbf24";
    const anchor = point.kind === "anchor";
    const radius = anchor ? CONTROL_POINT_RADIUS + 1 : CONTROL_POINT_RADIUS;
    overlayContext.beginPath();
    overlayContext.arc(
      projected[0],
      projected[1],
      radius,
      0,
      Math.PI * 2,
    );
    overlayContext.fillStyle = selected ? "#38bdf8" : roleColor;
    overlayContext.fill();
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle =
      point.startFinish || point.junction || point.looseEnd || anchor
        ? roleColor
        : "#080a0d";
    overlayContext.stroke();
    if (anchor) {
      overlayContext.beginPath();
      overlayContext.arc(
        projected[0],
        projected[1],
        radius * 0.5,
        0,
        Math.PI * 2,
      );
      overlayContext.fillStyle = "#080a0d";
      overlayContext.fill();
      overlayContext.lineWidth = 1.5;
      overlayContext.strokeStyle = selected ? "#38bdf8" : roleColor;
      overlayContext.stroke();
    }
    overlayContext.fillStyle = anchor
      ? selected
        ? "#38bdf8"
        : roleColor
      : "#080a0d";
    overlayContext.font = "600 10px ui-monospace, monospace";
    overlayContext.textAlign = "center";
    overlayContext.textBaseline = "middle";
    overlayContext.fillText(String(index + 1), projected[0], projected[1] + 0.5);
  });
  }
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
  const current = editorPoints[index];
  if (!current) {
    return;
  }
  current.position = [
    Math.max(-140, Math.min(140, world[0])),
    current.position[1],
    Math.max(-140, Math.min(140, world[2])),
  ];
}

function pointHeights(
  document: TrackProjectDocument,
  centerlines: Record<string, Vec3[]>,
): Map<string, number> {
  const heights = new Map<string, number>();
  const pointsById = new Map(
    (document.points ?? []).map((point) => [point.id, point]),
  );
  for (const section of document.sections ?? []) {
    const samples = centerlines[section.id];
    if (!samples || samples.length === 0) {
      continue;
    }
    for (const id of section.points) {
      if (heights.has(id)) {
        continue;
      }
      const point = pointsById.get(id);
      if (!point) {
        continue;
      }
      let best = Infinity;
      let bestY = 0;
      for (const sample of samples) {
        const dx = sample[0] - point.position[0];
        const dz = sample[2] - point.position[1];
        const distance = dx * dx + dz * dz;
        if (distance < best) {
          best = distance;
          bestY = sample[1];
        }
      }
      if (best < Infinity) {
        heights.set(id, bestY);
      }
    }
  }
  return heights;
}

function isLooseEndPoint(point: EditorPointData): boolean {
  if (point.junction || point.degree >= 2) {
    return false;
  }
  return (project.sections ?? []).some(
    (section) =>
      section.closed !== true &&
      (section.points[0] === point.id ||
        section.points[section.points.length - 1] === point.id),
  );
}

/**
 * Mirrors the model's `make_junction` eligibility: the point must not already
 * be a junction or a layout start/finish, and it must be an interior point of
 * at least one section with enough points to stay valid.
 */
function pointCanMakeJunction(point: EditorPoint): boolean {
  if (point.startFinish || point.junction) {
    return false;
  }
  return (project.sections ?? []).some((section) => {
    if (section.points.length < 3) {
      return false;
    }
    const index = section.points.indexOf(point.id);
    return index > 0 && index + 1 < section.points.length;
  });
}

const CONTACT_TOLERANCE = 0.5;

function nearestCenterlineContact(
  position: Vec3,
  samples: Vec3[],
): ContactInfo | null {
  let best = Infinity;
  let bestPoint: Vec3 | null = null;
  for (const sample of samples) {
    const dx = sample[0] - position[0];
    const dz = sample[2] - position[2];
    const distanceSquared = dx * dx + dz * dz;
    if (distanceSquared < best) {
      best = distanceSquared;
      bestPoint = [sample[0], sample[1], sample[2]];
    }
  }
  if (!bestPoint) {
    return null;
  }
  return { point: bestPoint, distance: Math.sqrt(best) };
}

function pointContactDistance(point: EditorPoint): number | null {
  let best: number | null = null;
  for (const sectionId of point.sections) {
    const samples = sectionCenterlines[sectionId];
    if (!samples || samples.length === 0) {
      continue;
    }
    const contact = nearestCenterlineContact(point.position, samples);
    if (contact && (best === null || contact.distance < best)) {
      best = contact.distance;
    }
  }
  return best;
}

function contactColors(point: EditorPoint): { dot: string; line: string } {
  if (point.startFinish) {
    return { dot: "#22c55e", line: "rgba(34, 197, 94, 0.45)" };
  }
  if (point.junction) {
    return { dot: "#f97316", line: "rgba(249, 115, 22, 0.45)" };
  }
  if (point.kind === "anchor") {
    return {
      dot: "rgba(148, 163, 184, 0.9)",
      line: "rgba(148, 163, 184, 0.35)",
    };
  }
  return {
    dot: "rgba(251, 191, 36, 0.85)",
    line: "rgba(251, 191, 36, 0.38)",
  };
}

function renderSelection(): void {
  selectionDetails.replaceChildren();
  pointEditorRows.replaceChildren();
  makeJunctionButton.hidden = true;
  makeJunctionButton.disabled = true;
  const addRow = (
    target: HTMLElement,
    label: string,
    value: string,
  ): void => {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = value;
    target.append(dt, dd);
  };
  const closePointEditor = (): void => {
    pointEditorId.textContent = "";
    if (pointEditor.open) {
      pointEditor.close();
    }
  };
  if (editorMode !== "points") {
    closePointEditor();
    return;
  }
  if (!selection) {
    closePointEditor();
    addRow(selectionDetails, "kind", "none");
    addRow(
      selectionDetails,
      "hint",
      "click a point, Alt-click a section, or click the road to insert",
    );
    return;
  }
  if (selection.kind === "point") {
    const point = editorPoints.find((item) => item.id === selection?.id);
    if (!point) {
      selection = null;
      renderSelection();
      return;
    }
    pointEditorId.textContent = point.id;
    addRow(pointEditorRows, "kind", "point");
    addRow(pointEditorRows, "id", point.id);
    addRow(pointEditorRows, "point kind", point.kind);
    addRow(
      pointEditorRows,
      "role",
      point.startFinish
        ? "start/finish"
        : point.junction
          ? "junction"
          : point.looseEnd
            ? "loose end"
            : "point",
    );
    const contact = pointContactDistance(point);
    addRow(
      pointEditorRows,
      "contact",
      contact === null
        ? "n/a"
        : contact < CONTACT_TOLERANCE
          ? "on curve"
          : `${contact.toFixed(1)} m`,
    );
    addRow(pointEditorRows, "section degree", String(point.degree));
    addRow(pointEditorRows, "sections", point.sections.join(", ") || "none");
    const canMakeJunction = pointCanMakeJunction(point);
    makeJunctionButton.hidden = !canMakeJunction;
    makeJunctionButton.disabled = !canMakeJunction;
    if (editorMode === "points" && !pointEditor.open) {
      pointEditor.show();
    }
    addRow(selectionDetails, "hint", `point ${point.id} selected`);
    return;
  }
  closePointEditor();
  const section = (project.sections ?? []).find(
    (item) => item.id === selection?.id,
  );
  if (!section) {
    selection = null;
    renderSelection();
    return;
  }
  addRow(selectionDetails, "kind", "section");
  addRow(selectionDetails, "id", section.id);
  addRow(selectionDetails, "state", section.closed ? "closed" : "open");
  addRow(selectionDetails, "points", String(section.points.length));
}

function refreshEditorState(): void {
  if (!editorViewer) {
    return;
  }
  project = JSON.parse(editorViewer.editor_project()) as TrackProjectDocument;
  activeLayoutId = editorViewer.editor_active_layout();
  const pointData = JSON.parse(
    editorViewer.editor_points(),
  ) as EditorPointData[];
  const route = JSON.parse(editorViewer.editor_route()) as RouteLeg[];
  routeLegs = route;
  routeSectionIds = new Set(route.map((leg) => leg.sectionId));
  sectionCenterlines = JSON.parse(
    editorViewer.editor_section_centerlines(),
  ) as Record<string, Vec3[]>;
  try {
    guideRails = JSON.parse(editorViewer.editor_guide_rails()) as GuideRail[];
  } catch {
    guideRails = [];
  }
  const heights = pointHeights(project, sectionCenterlines);
  const activeLayout = (project.layouts ?? []).find(
    (layout) => layout.id === activeLayoutId,
  );
  const startFinishId = activeLayout?.startFinish ?? "";
  editorPoints = pointData
    .map((point) => ({
      id: point.id,
      position: [
        point.position[0],
        heights.get(point.id) ?? 0,
        point.position[1],
      ],
      kind: point.kind ?? "handle",
      junction: point.junction,
      end: point.end,
      degree: point.degree,
      sections: point.sections,
      startFinish:
        startFinishId !== "" ? point.id === startFinishId : point.startFinish,
      looseEnd: isLooseEndPoint(point),
    }));
  centerline = JSON.parse(editorViewer.editor_route_centerline()) as Vec3[];
  if (
    selection?.kind === "point" &&
    !editorPoints.some((point) => point.id === selection?.id)
  ) {
    selection = null;
  }
  if (
    selection?.kind === "section" &&
    !(project.sections ?? []).some((section) => section.id === selection?.id)
  ) {
    selection = null;
  }
  renderSelection();
  if (selectedDecorationId) {
    const selected = findDecoration(selectedDecorationId);
    if (!selected || selected.kind !== modeDecorationKind(editorMode)) {
      selectedDecorationId = null;
    }
  }
  if (hoverDecorationId && !findDecoration(hoverDecorationId)) {
    hoverDecorationId = null;
  }
  renderDecorationInspector();
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

deletePointButton.addEventListener("click", () => {
  void deleteSelectedPoint();
});

setStartFinishButton.addEventListener("click", () => {
  void setStartFinishPoint();
});

makeJunctionButton.addEventListener("click", () => {
  void makeJunctionFromSelection();
});

pointEditorClose.addEventListener("click", () => {
  selection = null;
  renderSelection();
  drawControlPoints();
});

async function rebuildEditor(
  message: string,
  action: () => Promise<void>,
  errorPrefix: string | null = "edit failed",
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
    if (errorPrefix === null) {
      const text = error instanceof Error ? error.message : String(error);
      setStatus(text, "error");
    } else {
      reportProjectError(errorPrefix, error);
    }
    testDriveButton.disabled = false;
  } finally {
    suspended = false;
    rebuilding = false;
    const waiters = rebuildWaiters;
    rebuildWaiters = [];
    for (const resolve of waiters) {
      resolve();
    }
    cameraController?.setEnabled(true);
    drawControlPoints();
  }
}

function waitForRebuild(): Promise<void> {
  if (!rebuilding) {
    return Promise.resolve();
  }
  return new Promise((resolve) => rebuildWaiters.push(resolve));
}

async function insertPointAt(hit: SectionHit): Promise<void> {
  const section = (project.sections ?? []).find(
    (item) => item.id === hit.sectionId,
  );
  if (!section || !editorViewer) {
    return;
  }
  const afterIndex = insertAfterIndex(section, hit.t);
  await rebuildEditor("inserting point", async () => {
    const pointId = await editorViewer!.insert_editor_point(
      hit.sectionId,
      afterIndex,
      hit.position[0],
      hit.position[2],
    );
    selection = { kind: "point", id: pointId };
  });
}

async function deleteSelectedPoint(): Promise<void> {
  if (!editorViewer || selection?.kind !== "point") {
    return;
  }
  const pointId = selection.id;
  while (rebuilding) {
    await waitForRebuild();
  }
  if (selection?.kind !== "point" || selection.id !== pointId) {
    return;
  }
  await rebuildEditor(
    "deleting point",
    async () => {
      await editorViewer!.delete_editor_point(pointId);
      selection = null;
    },
    null,
  );
}

async function setStartFinishPoint(): Promise<void> {
  if (!editorViewer || selection?.kind !== "point" || rebuilding) {
    return;
  }
  const pointId = selection.id;
  await rebuildEditor("setting start/finish", async () => {
    await editorViewer!.set_editor_start_finish(pointId);
  });
}

async function makeJunctionFromSelection(): Promise<void> {
  if (!editorViewer || selection?.kind !== "point" || rebuilding) {
    return;
  }
  const pointId = selection.id;
  await rebuildEditor(
    "making junction",
    async () => {
      const branchSectionId =
        await editorViewer!.make_editor_junction(pointId);
      selection = { kind: "section", id: branchSectionId };
    },
    null,
  );
}

async function moveEditorPoint(point: EditorPoint): Promise<void> {
  await rebuildEditor("rebuilding track", () =>
    editorViewer!.move_editor_point(
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

for (const button of modeButtons) {
  button.addEventListener("click", () => {
    const mode = button.dataset.mode;
    if (mode === "points" || mode === "kerbs" || mode === "props") {
      setMode(mode);
    }
  });
}

decorationApply.addEventListener("click", () => {
  void applyDecorationProperties();
});

decorationDelete.addEventListener("click", () => {
  const decoration = selectedDecoration();
  if (decoration) {
    void deleteDecoration(decoration.id);
  }
});

decorationPhase.addEventListener("input", () => {
  if (selectedDecoration()) {
    return;
  }
  if (editorMode === "kerbs") {
    newKerbPhase = numberInputValue(decorationPhase, newKerbPhase);
  } else if (editorMode === "props") {
    newPropPhase = numberInputValue(decorationPhase, newPropPhase);
  }
});

decorationElements.addEventListener("input", () => {
  if (editorMode === "kerbs" && !selectedDecoration()) {
    newKerbElements = decorationElements.value;
  }
});

decorationSpacing.addEventListener("input", () => {
  if (editorMode === "props" && !selectedDecoration()) {
    newPropSpacing = decorationSpacing.value;
  }
});

decorationPropKind.addEventListener("change", () => {
  if (editorMode === "props" && !selectedDecoration()) {
    newPropKind = propKindValue(decorationPropKind.value);
    newPropKindExplicit = true;
  }
});

for (const control of [decorationSide, decorationLane]) {
  control.addEventListener("change", () => {
    if (selectedDecoration()) {
      setDecorationStatus("press Apply to update the decoration");
    }
  });
}

for (const input of [
  decorationSide,
  decorationLane,
  decorationPropKind,
  decorationSpacing,
  decorationPhase,
  decorationElements,
]) {
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && selectedDecoration()) {
      event.preventDefault();
      void applyDecorationProperties();
    }
  });
}

syncModeButtons();
renderDecorationInspector();

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
    if (editorMode !== "points") {
      if (overlayVisible) {
        const handle = hitTestDecorationHandle(x, y);
        const decoration = selectedDecoration();
        if (handle && decoration) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          resizeDrag = {
            pointerId: event.pointerId,
            decorationId: decoration.id,
            endpoint: handle.endpoint,
            sectionId: handle.sectionId,
            rail: handle.rail,
            currentFraction: handle.fraction,
          };
          cameraController?.setEnabled(false);
          try {
            canvas.setPointerCapture(event.pointerId);
          } catch {
            // Synthetic pointers may not support capture.
          }
          setDecorationStatus("drag the handle to resize");
          drawControlPoints();
          return;
        }
        const hitId = hitTestDecoration(x, y);
        if (hitId) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          selectedDecorationId = hitId;
          renderDecorationInspector();
          drawControlPoints();
          return;
        }
        const railHit = railHitForMode(x, y, editorMode);
        if (railHit) {
          event.preventDefault();
          event.stopImmediatePropagation();
          cameraDragging = false;
          selectedDecorationId = null;
          renderDecorationInspector();
          paintStroke = {
            pointerId: event.pointerId,
            sectionId: railHit.sectionId,
            rail: railHit.rail,
            entryFraction: railHit.fraction,
            currentFraction: railHit.fraction,
          };
          cameraController?.setEnabled(false);
          try {
            canvas.setPointerCapture(event.pointerId);
          } catch {
            // Synthetic pointers may not support capture.
          }
          setDecorationStatus("drag along the rail to paint");
          drawControlPoints();
          return;
        }
      }
      selectedDecorationId = null;
      renderDecorationInspector();
      drawControlPoints();
      if (event.button === 0) {
        cameraDragging = true;
      }
      return;
    }
    const index = overlayVisible ? hitTestControlPoint(x, y) : -1;
    if (index >= 0) {
      selection = { kind: "point", id: editorPoints[index].id };
      renderSelection();
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      dragIndex = index;
      dragPointerId = event.pointerId;
      dragPlaneY = editorPoints[index].position[1];
      dragStartPosition = [...editorPoints[index].position];
      cameraController?.setEnabled(false);
      try {
        canvas.setPointerCapture(event.pointerId);
      } catch {
        // Synthetic pointers may not support capture.
      }
      drawControlPoints();
      return;
    }
    const sectionHit = overlayVisible
      ? hitTestRouteSection(x, y)
      : null;
    if (sectionHit) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      if (event.altKey) {
        selection = { kind: "section", id: sectionHit.sectionId };
        renderSelection();
        drawControlPoints();
      } else {
        void insertPointAt(sectionHit);
      }
      return;
    }
    selection = null;
    renderSelection();
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
    if (resizeDrag && event.pointerId === resizeDrag.pointerId) {
      event.preventDefault();
      const [x, y] = pointerPosition(event);
      resizeDrag.currentFraction = railFractionOnSection(
        resizeDrag.sectionId,
        resizeDrag.rail,
        x,
        y,
      );
      drawControlPoints();
      return;
    }
    if (paintStroke && event.pointerId === paintStroke.pointerId) {
      event.preventDefault();
      const [x, y] = pointerPosition(event);
      const otherSection = railScreenHit(x, y, paintStroke.rail);
      if (
        otherSection &&
        otherSection.sectionId !== paintStroke.sectionId
      ) {
        setDecorationStatus(
          "single-section paint: clamped to the starting section",
          "error",
        );
      }
      paintStroke.currentFraction = railFractionOnSection(
        paintStroke.sectionId,
        paintStroke.rail,
        x,
        y,
      );
      drawControlPoints();
      return;
    }
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
  const point = editorPoints[dragIndex];
  const startPosition = dragStartPosition;
  dragIndex = null;
  dragPointerId = null;
  dragStartPosition = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  drawControlPoints();
  if (
    point &&
    startPosition &&
    (Math.abs(point.position[0] - startPosition[0]) > 1e-6 ||
      Math.abs(point.position[1] - startPosition[1]) > 1e-6 ||
      Math.abs(point.position[2] - startPosition[2]) > 1e-6)
  ) {
    void moveEditorPoint(point);
  }
};

const endPaint = (event: PointerEvent) => {
  if (!paintStroke || event.pointerId !== paintStroke.pointerId) {
    return;
  }
  const stroke = paintStroke;
  paintStroke = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  drawControlPoints();
  if (event.type === "pointercancel") {
    setDecorationStatus("paint cancelled");
    return;
  }
  const entryFraction = Math.min(
    stroke.entryFraction,
    stroke.currentFraction,
  );
  const exitFraction = Math.max(
    stroke.entryFraction,
    stroke.currentFraction,
  );
  if (exitFraction - entryFraction <= 1e-4) {
    setDecorationStatus("drag along the rail to paint");
    return;
  }
  void commitPaint(stroke, entryFraction, exitFraction);
};

const endResize = (event: PointerEvent) => {
  if (!resizeDrag || event.pointerId !== resizeDrag.pointerId) {
    return;
  }
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  if (event.type === "pointercancel") {
    resizeDrag = null;
    cameraController?.setEnabled(true);
    setDecorationStatus("resize cancelled");
    drawControlPoints();
    return;
  }
  void commitDecorationResize();
  cameraController?.setEnabled(true);
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
  if (
    editorMode !== "points" &&
    overlayVisible &&
    !paintStroke &&
    !resizeDrag &&
    carDragPointerId === null
  ) {
    hoverDecorationId = hitTestDecoration(lastPointer[0], lastPointer[1]);
  } else if (editorMode === "points") {
    hoverDecorationId = null;
  }
  drawControlPoints();
});

canvas.addEventListener("pointerleave", () => {
  lastPointer = null;
  hoverDecorationId = null;
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
    selection?.kind === "point" &&
    !event.repeat
  ) {
    event.preventDefault();
    void deleteSelectedPoint();
    return;
  }
  if (event.key === "Escape" && (paintStroke || resizeDrag)) {
    const hadPaint = paintStroke !== null;
    paintStroke = null;
    resizeDrag = null;
    cameraController?.setEnabled(true);
    setDecorationStatus(hadPaint ? "paint cancelled" : "resize cancelled");
    drawControlPoints();
    return;
  }
  if (event.key === "Escape") {
    if (editorMode !== "points") {
      selectedDecorationId = null;
      hoverDecorationId = null;
      renderDecorationInspector();
      drawControlPoints();
      return;
    }
    selection = null;
    renderSelection();
    drawControlPoints();
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
window.addEventListener("pointerup", endPaint, { capture: true });
window.addEventListener("pointercancel", endPaint, { capture: true });
window.addEventListener("pointerup", endResize, { capture: true });
window.addEventListener("pointercancel", endResize, { capture: true });
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
