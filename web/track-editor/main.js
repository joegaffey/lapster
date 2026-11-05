// web/track-editor/main.ts
import init, { TrackViewer } from "../pkg/sim_render.js";

// web/src/ui/track-camera.ts
var clamp = (value, min, max) => Math.min(max, Math.max(min, value));
var add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
var sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
var scale = (v, amount) => [v[0] * amount, v[1] * amount, v[2] * amount];
var length = (v) => Math.hypot(v[0], v[1], v[2]);
var normalize = (v) => {
  const len = length(v);
  return len > 1e-8 ? [v[0] / len, v[1] / len, v[2] / len] : [0, 0, 0];
};
var cross = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0]
];
var toVec3 = (value) => [value[0] ?? 0, value[1] ?? 0, value[2] ?? 0];
var lerp = (a, b, t) => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t
];
var easeInOutCubic = (t) => t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
var shortestAngle = (delta) => Math.atan2(Math.sin(delta), Math.cos(delta));
var prefersReducedMotion = () => window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches ?? false;
function setupTrackCamera(canvas2, viewer, options = {}) {
  let info;
  try {
    info = JSON.parse(viewer.camera_info());
  } catch {
    return;
  }
  let initialEye = toVec3(info.eye);
  let initialTarget = toVec3(info.target);
  let eye = [...initialEye];
  let target = [...initialTarget];
  let mode = "orbit";
  const pointers = /* @__PURE__ */ new Map();
  let pinch = null;
  const allowPan = options.allowPan ?? true;
  const configuredMaxHeight = options.maxHeight;
  let minTargetY = initialTarget[1];
  let minEyeY = Math.max(initialTarget[1] + 0.2, 0.05);
  let maxTargetY = configuredMaxHeight === void 0 ? Infinity : Math.max(configuredMaxHeight, initialTarget[1]);
  let maxEyeY = configuredMaxHeight === void 0 ? Infinity : Math.max(configuredMaxHeight, initialEye[1]);
  let flightFrame = 0;
  let enabled2 = true;
  const cancelFlight = () => {
    if (flightFrame !== 0) {
      cancelAnimationFrame(flightFrame);
      flightFrame = 0;
    }
  };
  const apply = () => {
    target[1] = clamp(target[1], minTargetY, maxTargetY);
    eye[1] = clamp(eye[1], minEyeY, maxEyeY);
    viewer.set_camera(eye[0], eye[1], eye[2], target[0], target[1], target[2]);
    viewer.set_camera_fov?.(info.fovY);
  };
  const orbit = (dx, dy) => {
    const offset = sub(eye, target);
    const radius = length(offset);
    if (radius < 1e-4) {
      return;
    }
    const yaw = Math.atan2(offset[0], offset[2]) - dx * 6e-3;
    const pitch = clamp(Math.asin(clamp(offset[1] / radius, -1, 1)) + dy * 6e-3, -1.45, 1.45);
    eye = [
      target[0] + Math.sin(yaw) * Math.cos(pitch) * radius,
      target[1] + Math.sin(pitch) * radius,
      target[2] + Math.cos(yaw) * Math.cos(pitch) * radius
    ];
  };
  const pan = (dx, dy) => {
    const forward = normalize(sub(target, eye));
    let right = normalize(cross(forward, [0, 1, 0]));
    if (length(right) < 1e-4) {
      right = [1, 0, 0];
    }
    const up = normalize(cross(right, forward));
    const radius = Math.max(length(sub(eye, target)), 0.01);
    const worldPerPixel = 2 * radius * Math.tan(info.fovY * 0.5) / Math.max(canvas2.clientHeight, 1);
    const offset = add(scale(right, -dx * worldPerPixel), scale(up, dy * worldPerPixel));
    eye = add(eye, offset);
    target = add(target, offset);
  };
  const zoomBy = (factor) => {
    const offset = sub(eye, target);
    const radius = clamp(length(offset) * factor, 0.5, 5e3);
    eye = add(target, scale(normalize(offset), radius));
  };
  const pinchState = () => {
    const points = [...pointers.values()];
    if (points.length < 2) {
      return null;
    }
    const first = points[0];
    const second = points[1];
    return {
      distance: Math.hypot(second.x - first.x, second.y - first.y),
      centroid: [(first.x + second.x) * 0.5, (first.y + second.y) * 0.5]
    };
  };
  canvas2.addEventListener("pointerdown", (event) => {
    if (!enabled2) {
      return;
    }
    event.preventDefault();
    cancelFlight();
    try {
      canvas2.setPointerCapture(event.pointerId);
    } catch {
    }
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    mode = allowPan && (event.button === 2 || event.shiftKey) ? "pan" : "orbit";
    if (pointers.size === 2) {
      pinch = pinchState();
    }
    canvas2.classList.add("dragging");
  });
  canvas2.addEventListener("pointermove", (event) => {
    if (!enabled2) {
      return;
    }
    const previous = pointers.get(event.pointerId);
    if (!previous) {
      return;
    }
    const next = { x: event.clientX, y: event.clientY };
    pointers.set(event.pointerId, next);
    if (pointers.size >= 2) {
      const state = pinchState();
      if (state && pinch) {
        if (allowPan) {
          pan(state.centroid[0] - pinch.centroid[0], state.centroid[1] - pinch.centroid[1]);
        }
        if (state.distance > 0 && pinch.distance > 0) {
          zoomBy(pinch.distance / state.distance);
        }
      }
      pinch = state;
      apply();
      return;
    }
    const dx = next.x - previous.x;
    const dy = next.y - previous.y;
    if (mode === "pan") {
      pan(dx, dy);
    } else {
      orbit(dx, dy);
    }
    apply();
  });
  const endPointer = (event) => {
    pointers.delete(event.pointerId);
    pinch = null;
    if (pointers.size === 0) {
      canvas2.classList.remove("dragging");
    }
  };
  canvas2.addEventListener("pointerup", endPointer);
  canvas2.addEventListener("pointercancel", endPointer);
  canvas2.addEventListener("lostpointercapture", endPointer);
  canvas2.addEventListener("contextmenu", (event) => event.preventDefault());
  canvas2.addEventListener(
    "wheel",
    (event) => {
      if (!enabled2) {
        return;
      }
      event.preventDefault();
      cancelFlight();
      zoomBy(Math.exp(event.deltaY * 1e-3));
      apply();
    },
    { passive: false }
  );
  if (options.attachResetButton !== false) {
    document.querySelector("#reset-camera")?.addEventListener("click", () => {
      if (!enabled2) {
        return;
      }
      cancelFlight();
      eye = [...initialEye];
      target = [...initialTarget];
      viewer.reset_camera();
    });
  }
  return {
    nudge(deltaSeconds, orbitX, orbitY, panX, panY, zoom) {
      if (!enabled2) {
        return;
      }
      cancelFlight();
      if (orbitX !== 0 || orbitY !== 0) {
        orbit(orbitX * 140 * deltaSeconds, orbitY * 140 * deltaSeconds);
      }
      if (panX !== 0 || panY !== 0) {
        pan(panX * 320 * deltaSeconds, panY * 320 * deltaSeconds);
      }
      if (zoom !== 0) {
        zoomBy(Math.exp(zoom * deltaSeconds * 1.4));
      }
      apply();
    },
    setEnabled(value) {
      enabled2 = value;
      if (!enabled2) {
        cancelFlight();
        pointers.clear();
        pinch = null;
        canvas2.classList.remove("dragging");
      }
    },
    setHome(next, animate = false) {
      info = next;
      const nextEye = toVec3(next.eye);
      const nextTarget = toVec3(next.target);
      initialEye = nextEye;
      initialTarget = nextTarget;
      minTargetY = initialTarget[1];
      minEyeY = Math.max(initialTarget[1] + 0.2, 0.05);
      maxTargetY = configuredMaxHeight === void 0 ? Infinity : Math.max(configuredMaxHeight, initialTarget[1]);
      maxEyeY = configuredMaxHeight === void 0 ? Infinity : Math.max(configuredMaxHeight, initialEye[1]);
      cancelFlight();
      if (!animate || prefersReducedMotion()) {
        eye = [...nextEye];
        target = [...nextTarget];
        apply();
        return;
      }
      const fromEye = [...eye];
      const fromTarget = [...target];
      const fromOffset = sub(fromEye, fromTarget);
      const toOffset = sub(nextEye, nextTarget);
      const fromRadius = Math.max(length(fromOffset), 1e-4);
      const toRadius = Math.max(length(toOffset), 1e-4);
      const fromYaw = Math.atan2(fromOffset[0], fromOffset[2]);
      const toYaw = Math.atan2(toOffset[0], toOffset[2]);
      const yawDelta = shortestAngle(toYaw - fromYaw);
      const fromPitch = Math.asin(clamp(fromOffset[1] / fromRadius, -1, 1));
      const toPitch = Math.asin(clamp(toOffset[1] / toRadius, -1, 1));
      const distance = Math.max(
        length(sub(nextEye, fromEye)),
        length(sub(nextTarget, fromTarget))
      );
      const duration = clamp(420 + distance * 10, 480, 1200);
      const start = performance.now();
      const step = (now) => {
        const progress = clamp((now - start) / duration, 0, 1);
        const eased = easeInOutCubic(progress);
        target = lerp(fromTarget, nextTarget, eased);
        const yaw = fromYaw + yawDelta * eased;
        const pitch = fromPitch + (toPitch - fromPitch) * eased;
        const radius = fromRadius + (toRadius - fromRadius) * eased;
        const horizontal = Math.cos(pitch) * radius;
        eye = [
          target[0] + Math.sin(yaw) * horizontal,
          target[1] + Math.sin(pitch) * radius,
          target[2] + Math.cos(yaw) * horizontal
        ];
        apply();
        if (progress < 1) {
          flightFrame = requestAnimationFrame(step);
        } else {
          flightFrame = 0;
          eye = [...nextEye];
          target = [...nextTarget];
          apply();
        }
      };
      flightFrame = requestAnimationFrame(step);
    }
  };
}

// web/src/ui/hud.ts
function setupHud() {
  const hud = document.querySelector("#hud");
  if (!hud || hud.querySelector("#hud-toggle")) {
    return;
  }
  const toggle = document.createElement("button");
  toggle.id = "hud-toggle";
  toggle.className = "hud-toggle";
  toggle.type = "button";
  toggle.setAttribute("aria-controls", "hud");
  const host = document.querySelector("#viewer-controls") ?? hud;
  host.prepend(toggle);
  const setCollapsed = (collapsed) => {
    hud.classList.toggle("hud-collapsed", collapsed);
    toggle.textContent = collapsed ? "Info" : "Hide info";
    toggle.setAttribute("aria-expanded", String(!collapsed));
  };
  setCollapsed(true);
  toggle.addEventListener("click", () => {
    setCollapsed(!hud.classList.contains("hud-collapsed"));
  });
}

// web/src/ui/fps.ts
var STYLE_ID = "lapster-fps-style";
var COUNTER_ID = "fps-counter";
var counter = null;
var enabled = false;
var frames = 0;
var elapsedMs = 0;
var STYLE = `
#fps-counter {
  position: fixed;
  top: 16px;
  left: 16px;
  z-index: 9;
  padding: 4px 7px;
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: rgba(10, 13, 17, 0.82);
  color: #cbd5e1;
  font: 11px/1.2 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
  pointer-events: none;
}
#fps-counter[hidden] { display: none; }
#hud { z-index: 10; }
@media (max-width: 720px) {
  #fps-counter { top: 84px; left: 12px; }
}
`;
function ensureCounter() {
  if (counter) {
    return counter;
  }
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }
  counter = document.createElement("div");
  counter.id = COUNTER_ID;
  counter.dataset.testid = COUNTER_ID;
  counter.setAttribute("role", "status");
  counter.setAttribute("aria-label", "Frames per second");
  counter.textContent = "\u2014 fps";
  counter.hidden = true;
  document.body.append(counter);
  return counter;
}
function recordFrame(deltaMs) {
  if (!enabled) {
    return;
  }
  const element = ensureCounter();
  frames += 1;
  elapsedMs += Math.max(deltaMs, 0);
  if (elapsedMs >= 500) {
    element.textContent = `${Math.round(frames * 1e3 / elapsedMs)} fps`;
    frames = 0;
    elapsedMs = 0;
  }
}

// web/src/ui/loading.ts
var LOADING_ID = "viewer-loading";
function loadingElement() {
  return document.querySelector(`#${LOADING_ID}`);
}
function hideLoading() {
  const element = loadingElement();
  if (element) {
    element.hidden = true;
  }
}
function showLoadingFailure(error) {
  const element = loadingElement();
  if (!element) {
    return;
  }
  const message = error instanceof Error ? error.message : String(error);
  element.hidden = false;
  element.dataset.state = "error";
  const label = element.querySelector("[data-loading-label]");
  if (label) {
    label.textContent = message ? `Failed to load: ${message}` : "Failed to load";
  }
}

// web/track-editor/main.ts
var MAX_CAMERA_HEIGHT = 140;
var CONTROL_POINT_RADIUS = 9;
var CONTROL_POINT_HIT_RADIUS = 18;
var CENTERLINE_HIT_RADIUS = 12;
var KERB_RADIUS = 6.4;
var PROP_OFFSET = 14;
var GUIDE_HIT_RADIUS = 12;
var GUIDE_HANDLE_RADIUS = 7;
var GUIDE_HANDLE_HIT_RADIUS = 16;
var DEFAULT_SEGMENT_LENGTH = 6;
var MIN_SEGMENT_LENGTH = 0.5;
var SURFACE_COLORS = {
  0: "#33a6ff",
  1: "#33ff59",
  2: "#b35933"
};
var canvas = document.querySelector("#view");
var overlay = document.querySelector("#control-points");
var overlayContext = overlay.getContext("2d");
var status = document.querySelector("#status");
var stats = document.querySelector("#stats");
var bounds = document.querySelector("#bounds");
var surfaces = document.querySelector("#surfaces");
var meshes = document.querySelector("#meshes");
var fpsLabel = document.querySelector("#fps");
var frameLabel = document.querySelector("#frame-ms");
var drawsLabel = document.querySelector("#draws");
var propSectionLabel = document.querySelector("#prop-section-label");
var propKindButtons = Array.from(
  document.querySelectorAll("[data-prop-kind]")
);
var overlayToggle = document.querySelector("#toggle-overlay");
var editorViewer;
var cameraController;
var controlPoints = [];
var centerline = [];
var dragIndex = null;
var dragPointerId = null;
var dragPlaneY = 0;
var rebuilding = false;
var suspended = false;
var cameraDragging = false;
var deleteMode = false;
var hoverDeleteIndex = null;
var lastPointer = null;
var hoverInsert = null;
var kerbSpans = [];
var propLines = [];
var hoverPoint = null;
var hoverHandleEdge = null;
var selectedSegment = null;
var guideDrag = null;
var paintKind = "blocks";
var overlayVisible = true;
setupHud();
var add2 = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
var sub2 = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
var scale2 = (value, amount) => [
  value[0] * amount,
  value[1] * amount,
  value[2] * amount
];
var dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
var cross2 = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0]
];
var toVec32 = (value) => [value[0] ?? 0, value[1] ?? 0, value[2] ?? 0];
var normalize2 = (value) => {
  const length2 = Math.hypot(value[0], value[1], value[2]);
  return length2 > 1e-8 ? scale2(value, 1 / length2) : [0, 0, 0];
};
var lerp2 = (a, b, t) => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t
];
function currentCamera() {
  if (!editorViewer) {
    return null;
  }
  try {
    return JSON.parse(editorViewer.camera_info());
  } catch {
    return null;
  }
}
function cameraBasis(camera) {
  const eye = toVec32(camera.eye);
  const target = toVec32(camera.target);
  const forward = normalize2(sub2(target, eye));
  const right = normalize2(cross2(forward, [0, 1, 0]));
  const up = normalize2(cross2(right, forward));
  return { eye, forward, right, up };
}
function projectPoint(point, camera, width, height) {
  const { eye, forward, right, up } = cameraBasis(camera);
  const offset = sub2(point, eye);
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
function screenRay(x, y, camera, width, height) {
  const { eye, forward, right, up } = cameraBasis(camera);
  const tanHalf = Math.tan(camera.fovY * 0.5);
  const aspect = Math.max(width, 1) / Math.max(height, 1);
  const ndcX = x / Math.max(width, 1) * 2 - 1;
  const ndcY = 1 - y / Math.max(height, 1) * 2;
  const direction = normalize2(
    add2(
      forward,
      add2(scale2(right, ndcX * tanHalf * aspect), scale2(up, ndcY * tanHalf))
    )
  );
  return { origin: eye, direction };
}
function pointerPosition(event) {
  const rect = canvas.getBoundingClientRect();
  return [event.clientX - rect.left, event.clientY - rect.top];
}
function hitTestControlPoint(x, y) {
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
function loopOutsideSide() {
  let area = 0;
  for (let index = 0; index < centerline.length; index += 1) {
    const point = centerline[index];
    const next = centerline[(index + 1) % centerline.length];
    area += point[0] * next[2] - next[0] * point[2];
  }
  return area > 0 ? -1 : 1;
}
function guideDefinitions() {
  const outside = loopOutsideSide();
  return [
    { kind: "kerb", side: -1, radius: KERB_RADIUS, color: "#fb7185" },
    { kind: "kerb", side: 1, radius: KERB_RADIUS, color: "#fb7185" },
    { kind: "prop", side: outside, radius: PROP_OFFSET, color: "#fb923c" },
    { kind: "prop", side: -outside, radius: PROP_OFFSET, color: "#2dd4bf" }
  ];
}
function binormalAt(sample) {
  const count = centerline.length;
  const index = (sample % count + count) % count;
  const incoming = normalize2(
    sub2(centerline[index], centerline[(index + count - 1) % count])
  );
  const outgoing = normalize2(
    sub2(centerline[(index + 1) % count], centerline[index])
  );
  const tangent = normalize2(add2(incoming, outgoing));
  return [-tangent[2], 0, tangent[0]];
}
function offsetPoint(index, side, radius) {
  if (centerline.length < 2) {
    return null;
  }
  const count = centerline.length;
  const position = Math.max(0, Math.min(count, index));
  const base = Math.floor(position) % count;
  const next = (base + 1) % count;
  const t = position - Math.floor(position);
  const point = lerp2(centerline[base], centerline[next], t);
  const binormal = normalize2(lerp2(binormalAt(base), binormalAt(next), t));
  return add2(point, scale2(binormal, radius * side));
}
function guidePoint(guide, index) {
  return offsetPoint(index, guide.side, guide.radius);
}
function guidePoints(guide) {
  const points = [];
  for (let index = 0; index <= centerline.length; index += 1) {
    const point = guidePoint(guide, index);
    if (point) {
      points.push(point);
    }
  }
  return points;
}
function hitGuidePoint(x, y, guides = guideDefinitions()) {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best = null;
  let bestDistance = GUIDE_HIT_RADIUS;
  for (const guide of guides) {
    const projected = guidePoints(guide).map(
      (point) => projectPoint(point, camera, width, height)
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
      const t = lengthSquared > 1e-6 ? Math.max(
        0,
        Math.min(
          1,
          ((x - start[0]) * segmentX + (y - start[1]) * segmentY) / lengthSquared
        )
      ) : 0;
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
        screen: [hitX, hitY]
      };
    }
  }
  return best;
}
function guideFor(kind, side) {
  return guideDefinitions().find(
    (guide) => guide.kind === kind && guide.side === side
  ) ?? null;
}
function findRange(kind, side, index) {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return ranges.find(
    (range) => range.side === side && index >= range.start - 1e-4 && index <= range.end + 1e-4
  ) ?? null;
}
function findRangeExact(kind, side, start, end) {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return ranges.find(
    (range) => range.side === side && Math.abs(range.start - start) < 1e-4 && Math.abs(range.end - end) < 1e-4
  ) ?? null;
}
function selectedPropLine() {
  if (!selectedSegment || selectedSegment.kind !== "prop") {
    return void 0;
  }
  return propLines.find(
    (line) => line.side === selectedSegment.side && Math.abs(line.start - selectedSegment.start) < 1e-4 && Math.abs(line.end - selectedSegment.end) < 1e-4
  );
}
function resolveSelection() {
  if (!selectedSegment) {
    return;
  }
  const range = findRangeExact(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.start,
    selectedSegment.end
  ) ?? findRange(
    selectedSegment.kind,
    selectedSegment.side,
    (selectedSegment.start + selectedSegment.end) * 0.5
  );
  if (range) {
    selectedSegment.start = range.start;
    selectedSegment.end = range.end;
  } else {
    selectedSegment = null;
  }
}
function subtractRanges(items, side, start, end) {
  const result = [];
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
function selectSegment(kind, side, index) {
  const range = findRange(kind, side, index);
  if (!range) {
    return false;
  }
  selectedSegment = {
    kind,
    side,
    start: range.start,
    end: range.end
  };
  if (kind === "prop") {
    const line = propLines.find(
      (item) => item.side === range.side && Math.abs(item.start - range.start) < 1e-4 && Math.abs(item.end - range.end) < 1e-4
    );
    if (line) {
      paintKind = line.kind;
    }
  }
  syncPaintControls();
  return true;
}
function createSegment(guide, index) {
  const count = centerline.length;
  if (guide.kind === "prop" && paintKind === "off") {
    flashStatus("Choose Blocks or Tyres to add props");
    return;
  }
  const length2 = Math.min(DEFAULT_SEGMENT_LENGTH, count);
  const start = Math.max(0, Math.min(count - length2, index - length2 * 0.5));
  const end = Math.min(count, start + length2);
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
      end
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
    kind: paintKind
  });
  propLines.sort((a, b) => a.side - b.side || a.start - b.start);
  selectedSegment = {
    kind: "prop",
    side: guide.side,
    start,
    end
  };
  hoverPoint = null;
  syncPaintControls();
  drawControlPoints();
  void rebuildPropLines();
}
function updateSelectedEdge(edge, index) {
  if (!selectedSegment) {
    return;
  }
  const oldStart = selectedSegment.start;
  const oldEnd = selectedSegment.end;
  const range = findRangeExact(
    selectedSegment.kind,
    selectedSegment.side,
    oldStart,
    oldEnd
  );
  if (!range) {
    return;
  }
  if (edge === "start") {
    selectedSegment.start = Math.max(
      0,
      Math.min(index, selectedSegment.end - MIN_SEGMENT_LENGTH)
    );
  } else {
    selectedSegment.end = Math.min(
      centerline.length,
      Math.max(index, selectedSegment.start + MIN_SEGMENT_LENGTH)
    );
  }
  range.start = selectedSegment.start;
  range.end = selectedSegment.end;
}
function hitGuideHandle(x, y) {
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
  let best = null;
  let bestDistance = GUIDE_HANDLE_HIT_RADIUS;
  for (const edge of ["start", "end"]) {
    const point = guidePoint(guide, selectedSegment[edge]);
    const projected = point ? projectPoint(point, camera, width, height) : null;
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
function paintLabel(kind) {
  if (kind === "blocks") {
    return "Blocks";
  }
  return kind === "tyres" ? "Tyres" : "Off";
}
function syncPaintControls() {
  propSectionLabel.textContent = `Painting: ${paintLabel(paintKind)}`;
  for (const button of propKindButtons) {
    button.disabled = false;
    button.classList.toggle("active", button.dataset.propKind === paintKind);
  }
}
function setPaintKind(kind) {
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
function setOverlayVisible(visible) {
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
function updateHover() {
  hoverInsert = null;
  hoverDeleteIndex = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  if (!overlayVisible || !lastPointer || dragIndex !== null || guideDrag !== null || rebuilding || cameraDragging) {
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
      point.index
    ) ? "pointer" : "copy";
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
  const projected = centerline.map((point2) => projectPoint(point2, camera, width, height));
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
    const t = lengthSquared > 1e-6 ? Math.max(
      0,
      Math.min(
        1,
        ((lastPointer[0] - start[0]) * segmentX + (lastPointer[1] - start[1]) * segmentY) / lengthSquared
      )
    ) : 0;
    const hitX = start[0] + segmentX * t;
    const hitY = start[1] + segmentY * t;
    const distance = Math.hypot(lastPointer[0] - hitX, lastPointer[1] - hitY);
    if (distance >= bestDistance) {
      continue;
    }
    bestDistance = distance;
    const next = (index + 1) % centerline.length;
    const insertIndex = Math.floor(index * controlPoints.length / centerline.length) % controlPoints.length + 1;
    hoverInsert = {
      point: lerp2(centerline[index], centerline[next], t),
      screen: [hitX, hitY],
      insertIndex
    };
  }
  canvas.style.cursor = hoverInsert ? "copy" : "";
}
function setDeleteMode(enabled2) {
  if (deleteMode === enabled2) {
    return;
  }
  deleteMode = enabled2;
  hoverInsert = null;
  hoverDeleteIndex = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  syncPaintControls();
  drawControlPoints();
}
function flashStatus(message) {
  status.textContent = message;
  status.className = "status error";
  window.setTimeout(() => {
    if (!rebuilding && !window.__trackEditorError) {
      status.textContent = "ready";
      status.className = "status ready";
    }
  }, 1200);
}
function drawControlPoints() {
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
        height
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
  const drawGuide = (points, color, active) => {
    const projected = points.map((point) => projectPoint(point, camera, width, height)).filter((point) => point !== null);
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
  const selectedGuide = selectedSegment ? guideFor(selectedSegment.kind, selectedSegment.side) : null;
  const draggedGuide = guideDrag ? guideFor(guideDrag.kind, guideDrag.side) : null;
  const activeGuide = draggedGuide ?? hoverPoint?.guide ?? selectedGuide;
  for (const guide of guides) {
    const active = activeGuide?.kind === guide.kind && activeGuide.side === guide.side;
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
    const preview = [];
    const steps = Math.max(
      2,
      Math.ceil((selectedSegment.end - selectedSegment.start) * 8)
    );
    for (let step = 0; step <= steps; step += 1) {
      const index = selectedSegment.start + (selectedSegment.end - selectedSegment.start) * step / steps;
      const point = guidePoint(selectedGuide, index);
      if (point) {
        preview.push(point);
      }
    }
    const projected = preview.map((point) => projectPoint(point, camera, width, height)).filter((point) => point !== null);
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
    for (const edge of ["start", "end"]) {
      const point = guidePoint(selectedGuide, selectedSegment[edge]);
      const projected2 = point ? projectPoint(point, camera, width, height) : null;
      if (!projected2) {
        continue;
      }
      const active = hoverHandleEdge === edge || guideDrag?.edge === edge && guideDrag.kind === selectedSegment.kind;
      overlayContext.beginPath();
      overlayContext.arc(
        projected2[0],
        projected2[1],
        GUIDE_HANDLE_RADIUS + (active ? 2 : 0),
        0,
        Math.PI * 2
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
      Math.PI * 2
    );
    overlayContext.fillStyle = deleteTarget ? "#ef4444" : deleteMode ? "#fca5a5" : index === dragIndex ? "#38bdf8" : "#fbbf24";
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
function moveControlPoint(index, x, y) {
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
  const world = add2(ray.origin, scale2(ray.direction, distance));
  const current = controlPoints[index];
  controlPoints[index] = [
    Math.max(-140, Math.min(140, world[0])),
    current[1],
    Math.max(-140, Math.min(140, world[2]))
  ];
}
function refreshEditorState() {
  if (!editorViewer) {
    return;
  }
  controlPoints = JSON.parse(editorViewer.editor_control_points());
  centerline = JSON.parse(editorViewer.editor_centerline());
  kerbSpans = JSON.parse(editorViewer.editor_kerb_spans()).sort((a, b) => a.side - b.side || a.start - b.start);
  propLines = JSON.parse(editorViewer.editor_prop_lines());
  resolveSelection();
  syncPaintControls();
  const debug = JSON.parse(editorViewer.debug_info());
  renderStats(debug);
  drawsLabel.textContent = String(debug.stats.drawCalls);
}
async function rebuildEditor(message, action) {
  if (!editorViewer || rebuilding) {
    return;
  }
  rebuilding = true;
  suspended = true;
  cameraController?.setEnabled(false);
  status.textContent = message;
  status.className = "status";
  try {
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
    await action();
    refreshEditorState();
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
async function rebuildControlPoints() {
  await rebuildEditor(
    "rebuilding track",
    () => editorViewer.set_editor_control_points(
      JSON.stringify(
        controlPoints.map((point) => [point[0], point[1], point[2]])
      )
    )
  );
}
async function rebuildKerbSpans() {
  await rebuildEditor(
    "rebuilding kerbs",
    () => editorViewer.set_editor_kerb_spans(JSON.stringify(kerbSpans))
  );
}
async function rebuildPropLines() {
  await rebuildEditor(
    "rebuilding prop lines",
    () => editorViewer.set_editor_prop_lines(JSON.stringify(propLines))
  );
}
canvas.addEventListener(
  "pointerdown",
  (event) => {
    if (event.button !== 0 || dragIndex !== null || rebuilding) {
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
            pointerId: event.pointerId
          };
          hoverHandleEdge = handle.edge;
          cameraController?.setEnabled(false);
          try {
            canvas.setPointerCapture(event.pointerId);
          } catch {
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
          if (selectSegment(point.guide.kind, point.guide.side, point.index)) {
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
          hoverInsert.point[2]
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
    }
    drawControlPoints();
  },
  { capture: true }
);
window.addEventListener(
  "pointermove",
  (event) => {
    if (guideDrag && event.pointerId === guideDrag.pointerId) {
      event.preventDefault();
      const guide = guideFor(guideDrag.kind, guideDrag.side);
      if (guide) {
        const [x2, y2] = pointerPosition(event);
        const point = hitGuidePoint(x2, y2, [guide]);
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
  { capture: true, passive: false }
);
var endDrag = (event) => {
  if (dragIndex === null || event.pointerId !== dragPointerId) {
    return;
  }
  dragIndex = null;
  dragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
  }
  drawControlPoints();
  void rebuildControlPoints();
};
var endGuideDrag = (event) => {
  if (!guideDrag || event.pointerId !== guideDrag.pointerId) {
    return;
  }
  const kind = guideDrag.kind;
  guideDrag = null;
  hoverHandleEdge = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
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
    const kind = button.dataset.propKind;
    if (kind) {
      setPaintKind(kind);
    }
  });
}
overlayToggle.addEventListener("click", () => {
  setOverlayVisible(!overlayVisible);
});
var endCameraDrag = () => {
  cameraDragging = false;
};
window.addEventListener("pointerup", endDrag, { capture: true });
window.addEventListener("pointercancel", endDrag, { capture: true });
window.addEventListener("pointerup", endGuideDrag, { capture: true });
window.addEventListener("pointercancel", endGuideDrag, { capture: true });
window.addEventListener("pointerup", endCameraDrag, { capture: true });
window.addEventListener("pointercancel", endCameraDrag, { capture: true });
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
function renderStats(debug) {
  stats.replaceChildren();
  const entries = [
    ["vertices", String(debug.stats.vertices)],
    ["triangles", String(debug.stats.triangles)],
    ["draw calls", String(debug.stats.drawCalls)],
    ["collision tris", String(debug.stats.collisionTriangles)],
    ["grass blades", String(debug.stats.grassBlades ?? 0)],
    ["spawns", String(debug.spawns.length)]
  ];
  for (const [label, value] of entries) {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = value;
    stats.append(dt, dd);
  }
  bounds.textContent = `${debug.bounds.min.map((value) => value.toFixed(1)).join(", ")} \u2192 ${debug.bounds.max.map((value) => value.toFixed(1)).join(", ")}`;
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
    friction.textContent = `\u03BC ${surface.friction.toFixed(2)}`;
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
  controlPoints = JSON.parse(viewer.editor_control_points());
  centerline = JSON.parse(viewer.editor_centerline());
  kerbSpans = JSON.parse(viewer.editor_kerb_spans()).sort((a, b) => a.start - b.start);
  propLines = JSON.parse(viewer.editor_prop_lines());
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
  let frames2 = 0;
  let accumulated = 0;
  let frameTime = 0;
  const frame = (now) => {
    const delta = now - last;
    last = now;
    frames2 += 1;
    accumulated += delta;
    frameTime += delta;
    recordFrame(delta);
    if (accumulated >= 500) {
      fpsLabel.textContent = (frames2 * 1e3 / accumulated).toFixed(1);
      frameLabel.textContent = `${(frameTime / frames2).toFixed(2)} ms`;
      frames2 = 0;
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
  status.textContent = "ready";
  status.className = "status ready";
  window.__trackEditorReady = true;
  requestAnimationFrame(frame);
  requestAnimationFrame(hideLoading);
}
function fail(error) {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__trackEditorError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}
boot().catch(fail);
