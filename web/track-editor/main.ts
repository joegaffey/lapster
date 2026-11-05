import init, { TrackViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { EDITOR_TRACK_ID, EDITOR_TRACK_CACHE_KEY } from "../src/assets/editor-track.ts";
import { setupTrackCamera, type OrbitCameraController } from "../src/ui/track-camera.ts";
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

type PropKind = "blocks" | "tyres" | "off";

interface PropLine {
  start: number;
  end: number;
  side: number;
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
  start: number;
  end: number;
}

interface GuideDrag {
  kind: "kerb" | "prop";
  side: number;
  edge: "start" | "end";
  pointerId: number;
}

const MAX_CAMERA_HEIGHT = 140;
const CONTROL_POINT_RADIUS = 9;
const CONTROL_POINT_HIT_RADIUS = 18;
const CENTERLINE_HIT_RADIUS = 12;
const KERB_RADIUS = 6.4;
const PROP_OFFSET = 14;
const GUIDE_HIT_RADIUS = 12;
const GUIDE_HANDLE_RADIUS = 7;
const GUIDE_HANDLE_HIT_RADIUS = 16;
const DEFAULT_SEGMENT_LENGTH = 6;
const MIN_SEGMENT_LENGTH = 0.5;
const SURFACE_COLORS: Record<number, string> = {
  0: "#33a6ff",
  1: "#33ff59",
  2: "#b35933",
};

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
const propSectionLabel = document.querySelector<HTMLElement>("#prop-section-label")!;
const propKindButtons = Array.from(
  document.querySelectorAll<HTMLButtonElement>("[data-prop-kind]"),
);
const overlayToggle = document.querySelector<HTMLButtonElement>("#toggle-overlay")!;
const testDriveButton = document.querySelector<HTMLButtonElement>("#test-drive")!;

let editorViewer: TrackViewer | undefined;
let cameraController: OrbitCameraController | undefined;
let controlPoints: Vec3[] = [];
let centerline: Vec3[] = [];
let dragIndex: number | null = null;
let dragPointerId: number | null = null;
let dragPlaneY = 0;
let rebuilding = false;
let suspended = false;
let cameraDragging = false;
let deleteMode = false;
let hoverDeleteIndex: number | null = null;
let lastPointer: [number, number] | null = null;
let hoverInsert: { point: Vec3; screen: [number, number]; insertIndex: number } | null = null;
let kerbSpans: KerbSpan[] = [];
let propLines: PropLine[] = [];
let hoverPoint: GuidePoint | null = null;
let hoverHandleEdge: "start" | "end" | null = null;
let selectedSegment: SegmentSelection | null = null;
let guideDrag: GuideDrag | null = null;
let paintKind: PropKind = "blocks";
let overlayVisible = true;

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

function guideFor(kind: SegmentKind, side: number): GuideDef | null {
  return (
    guideDefinitions().find(
      (guide) => guide.kind === kind && guide.side === side,
    ) ?? null
  );
}

function findRange(
  kind: SegmentKind,
  side: number,
  index: number,
): { start: number; end: number; side: number } | null {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return (
    ranges.find(
      (range) =>
        range.side === side &&
        index >= range.start - 1e-4 &&
        index <= range.end + 1e-4,
    ) ?? null
  );
}

function findRangeExact(
  kind: SegmentKind,
  side: number,
  start: number,
  end: number,
): { start: number; end: number; side: number } | null {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return (
    ranges.find(
      (range) =>
        range.side === side &&
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
      line.side === selectedSegment!.side &&
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
      selectedSegment.start,
      selectedSegment.end,
    ) ??
    findRange(
      selectedSegment.kind,
      selectedSegment.side,
      (selectedSegment.start + selectedSegment.end) * 0.5,
    );
  if (range) {
    selectedSegment.start = range.start;
    selectedSegment.end = range.end;
  } else {
    selectedSegment = null;
  }
}

function subtractRanges<T extends { start: number; end: number; side: number }>(
  items: T[],
  side: number,
  start: number,
  end: number,
): T[] {
  const result: T[] = [];
  for (const item of items) {
    if (item.side !== side || item.end <= start || item.start >= end) {
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

function selectSegment(kind: SegmentKind, side: number, index: number): boolean {
  const range = findRange(kind, side, index);
  if (!range) {
    return false;
  }
  selectedSegment = {
    kind,
    side,
    start: range.start,
    end: range.end,
  };
  if (kind === "prop") {
    const line = propLines.find(
      (item) =>
        item.side === range.side &&
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

function createSegment(guide: GuideDef, index: number): void {
  const count = centerline.length;
  if (guide.kind === "prop" && paintKind === "off") {
    flashStatus("Choose Blocks or Tyres to add props");
    return;
  }
  const length = Math.min(DEFAULT_SEGMENT_LENGTH, count);
  const start = Math.max(0, Math.min(count - length, index - length * 0.5));
  const end = Math.min(count, start + length);
  if (end - start < MIN_SEGMENT_LENGTH) {
    return;
  }

  if (guide.kind === "kerb") {
    kerbSpans = subtractRanges(kerbSpans, guide.side, start, end);
    kerbSpans.push({ start, end, side: guide.side });
    kerbSpans.sort((a, b) => a.side - b.side || a.start - b.start);
    selectedSegment = {
      kind: "kerb",
      side: guide.side,
      start,
      end,
    };
    hoverPoint = null;
    syncPaintControls();
    drawControlPoints();
    void rebuildKerbSpans();
    return;
  }

  propLines = subtractRanges(propLines, guide.side, start, end);
  propLines.push({
    start,
    end,
    side: guide.side,
    kind: paintKind,
  });
  propLines.sort((a, b) => a.side - b.side || a.start - b.start);
  selectedSegment = {
    kind: "prop",
    side: guide.side,
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
  const guide = guideFor(selectedSegment.kind, selectedSegment.side);
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
  if (kind === "blocks") {
    return "Blocks";
  }
  return kind === "tyres" ? "Tyres" : "Off";
}

function syncPaintControls(): void {
  propSectionLabel.textContent = `Painting: ${paintLabel(paintKind)}`;
  for (const button of propKindButtons) {
    button.disabled = false;
    button.classList.toggle("active", button.dataset.propKind === paintKind);
  }
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
    const insertIndex =
      (Math.floor((index * controlPoints.length) / centerline.length) %
        controlPoints.length) +
      1;
    hoverInsert = {
      point: lerp(centerline[index], centerline[next], t),
      screen: [hitX, hitY],
      insertIndex,
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
    ? guideFor(selectedSegment.kind, selectedSegment.side)
    : null;
  const draggedGuide = guideDrag
    ? guideFor(guideDrag.kind, guideDrag.side)
    : null;
  const activeGuide = draggedGuide ?? hoverPoint?.guide ?? selectedGuide;
  for (const guide of guides) {
    const active =
      activeGuide?.kind === guide.kind && activeGuide.side === guide.side;
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

  controlPoints.forEach((point, index) => {
    const projected = projectPoint(point, camera, width, height);
    if (!projected) {
      return;
    }
    const deleteTarget = deleteMode && index === hoverDeleteIndex;
    overlayContext.beginPath();
    overlayContext.arc(
      projected[0],
      projected[1],
      CONTROL_POINT_RADIUS + (deleteTarget ? 3 : 0),
      0,
      Math.PI * 2,
    );
    overlayContext.fillStyle = deleteTarget
      ? "#ef4444"
      : deleteMode
        ? "#fca5a5"
        : index === dragIndex
          ? "#38bdf8"
          : "#fbbf24";
    overlayContext.fill();
    overlayContext.lineWidth = deleteTarget ? 3 : 2;
    overlayContext.strokeStyle = deleteTarget ? "#fff1f2" : "#080a0d";
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
  const current = controlPoints[index];
  controlPoints[index] = [
    Math.max(-140, Math.min(140, world[0])),
    current[1],
    Math.max(-140, Math.min(140, world[2])),
  ];
}

function refreshEditorState(): void {
  if (!editorViewer) {
    return;
  }
  controlPoints = JSON.parse(editorViewer.editor_control_points()) as Vec3[];
  centerline = JSON.parse(editorViewer.editor_centerline()) as Vec3[];
  kerbSpans = (
    JSON.parse(editorViewer.editor_kerb_spans()) as KerbSpan[]
  ).sort((a, b) => a.side - b.side || a.start - b.start);
  propLines = JSON.parse(editorViewer.editor_prop_lines()) as PropLine[];
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
  status.textContent = message;
  status.className = "status";
  try {
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await action();
    refreshEditorState();
    testDriveButton.disabled = false;
    status.textContent = "ready";
    status.className = "status ready";
  } catch (error) {
    fail(error);
  } finally {
    suspended = false;
    rebuilding = false;
    cameraController?.setEnabled(true);
    drawControlPoints();
  }
}

async function rebuildControlPoints(): Promise<void> {
  await rebuildEditor("rebuilding track", () =>
    editorViewer!.set_editor_control_points(
      JSON.stringify(
        controlPoints.map((point) => [point[0], point[1], point[2]]),
      ),
    ),
  );
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
    if (event.shiftKey !== deleteMode) {
      setDeleteMode(event.shiftKey);
    }
    const [x, y] = pointerPosition(event);
    const index = overlayVisible ? hitTestControlPoint(x, y) : -1;
    if (deleteMode && index >= 0) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      if (controlPoints.length <= 3) {
        flashStatus("minimum 3 control points");
        return;
      }
      controlPoints.splice(index, 1);
      hoverInsert = null;
      hoverDeleteIndex = null;
      drawControlPoints();
      void rebuildControlPoints();
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
            selectSegment(point.guide.kind, point.guide.side, point.index)
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
        controlPoints.splice(hoverInsert.insertIndex, 0, [
          hoverInsert.point[0],
          hoverInsert.point[1],
          hoverInsert.point[2],
        ]);
        hoverInsert = null;
        drawControlPoints();
        void rebuildControlPoints();
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
    if (guideDrag && event.pointerId === guideDrag.pointerId) {
      event.preventDefault();
      const guide = guideFor(guideDrag.kind, guideDrag.side);
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
  dragIndex = null;
  dragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
    // Pointer capture may already be released.
  }
  drawControlPoints();
  void rebuildControlPoints();
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
  if (event.key === "Shift") {
    setDeleteMode(true);
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

for (const button of propKindButtons) {
  button.addEventListener("click", () => {
    const kind = button.dataset.propKind as PropKind | undefined;
    if (kind) {
      setPaintKind(kind);
    }
  });
}

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
  controlPoints = JSON.parse(viewer.editor_control_points()) as Vec3[];
  centerline = JSON.parse(viewer.editor_centerline()) as Vec3[];
  kerbSpans = (
    JSON.parse(viewer.editor_kerb_spans()) as KerbSpan[]
  ).sort((a, b) => a.start - b.start);
  propLines = JSON.parse(viewer.editor_prop_lines()) as PropLine[];
  syncPaintControls();
  cameraController = setupTrackCamera(canvas, viewer, { maxHeight: MAX_CAMERA_HEIGHT });

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
