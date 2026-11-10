// web/track-editor/main.ts
import init, { TrackViewer } from "../pkg/sim_render.js";

// web/src/assets/cache.ts
var DB_NAME = "sim-assets";
var DB_VERSION = 1;
var STORE_NAME = "packs";
var IndexedDbPackCache = class {
  async get(key) {
    const db = await openDatabase();
    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, "readonly");
      const request = transaction.objectStore(STORE_NAME).get(key);
      request.onsuccess = () => {
        const value = request.result;
        resolve(value instanceof Uint8Array ? value : void 0);
      };
      request.onerror = () => reject(request.error);
    });
  }
  async put(key, bytes) {
    const db = await openDatabase();
    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, "readwrite");
      transaction.objectStore(STORE_NAME).put(bytes, key);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }
  async delete(key) {
    const db = await openDatabase();
    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, "readwrite");
      transaction.objectStore(STORE_NAME).delete(key);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }
};
function openDatabase() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE_NAME)) {
        request.result.createObjectStore(STORE_NAME);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

// web/src/assets/editor-track.ts
var EDITOR_TRACK_ID = "editor";
var EDITOR_TRACK_CACHE_KEY = "lapster.editor-track.v1";

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
    setView(nextEye, nextTarget) {
      const nextTargetVec = toVec3(nextTarget);
      minTargetY = Math.min(minTargetY, nextTargetVec[1]);
      minEyeY = Math.min(minEyeY, Math.max(nextTargetVec[1] + 0.2, 0.05));
      cancelFlight();
      eye = toVec3(nextEye);
      target = nextTargetVec;
      apply();
    },
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

// web/src/ui/car-setup.ts
var number = (path, label, min, max, step, unit) => ({ path, label, min, max, step, unit });
var json = (path, label) => ({ path, label, kind: "json" });
var axleSections = (axle, title) => {
  const prefix = `${axle}`;
  return [
    {
      title: `${title} suspension`,
      fields: [
        number(`${prefix}.suspension.travel`, "Travel", 0.02, 0.5, 0.01, "m"),
        number(`${prefix}.suspension.springRate`, "Spring rate", 5e3, 3e5, 1e3, "N/m"),
        number(`${prefix}.suspension.damperBump`, "Bump damping", 500, 5e4, 100, "N\xB7s/m"),
        number(`${prefix}.suspension.damperRebound`, "Rebound damping", 500, 5e4, 100, "N\xB7s/m"),
        number(`${prefix}.suspension.antiRoll`, "Anti-roll", 0, 1e5, 500, "N/m")
      ]
    },
    {
      title: `${title} tire`,
      fields: [
        number(`${prefix}.tire.lateral.b`, "Lateral B", 0.5, 30, 0.1),
        number(`${prefix}.tire.lateral.c`, "Lateral C", 0.5, 3, 0.01),
        number(`${prefix}.tire.lateral.d`, "Lateral D", 0.1, 2.5, 0.01),
        number(`${prefix}.tire.lateral.e`, "Lateral E", 0, 2, 0.01),
        number(`${prefix}.tire.longitudinal.b`, "Longitudinal B", 0.5, 30, 0.1),
        number(`${prefix}.tire.longitudinal.c`, "Longitudinal C", 0.5, 3, 0.01),
        number(`${prefix}.tire.longitudinal.d`, "Longitudinal D", 0.1, 2.5, 0.01),
        number(`${prefix}.tire.longitudinal.e`, "Longitudinal E", 0, 2, 0.01),
        number(`${prefix}.tire.relaxationLength`, "Relaxation length", 0.01, 3, 0.01, "m"),
        number(`${prefix}.tire.peakSlipRatio`, "Peak slip ratio", 0.01, 1, 0.01),
        number(`${prefix}.tire.frictionScale`, "Friction scale", 0.1, 2, 0.01)
      ]
    }
  ];
};
var CAR_SETUP_SECTIONS = [
  {
    title: "Chassis",
    fields: [
      number("chassis.inertia.0", "Inertia X", 50, 1e4, 10, "kg\xB7m\xB2"),
      number("chassis.inertia.1", "Inertia Y", 50, 1e4, 10, "kg\xB7m\xB2"),
      number("chassis.inertia.2", "Inertia Z", 50, 1e4, 10, "kg\xB7m\xB2"),
      number("chassis.centerOfGravity.0", "CG X", -2, 2, 0.01, "m"),
      number("chassis.centerOfGravity.1", "CG height", -2, 2, 0.01, "m"),
      number("chassis.centerOfGravity.2", "CG Z", -2, 2, 0.01, "m")
    ]
  },
  ...axleSections("front", "Front"),
  ...axleSections("rear", "Rear"),
  {
    title: "Engine",
    fields: [
      number("engine.idleRpm", "Idle", 400, 2500, 50, "rpm"),
      number("engine.redlineRpm", "Redline", 1500, 15e3, 100, "rpm"),
      json("engine.torqueCurve", "Torque curve [[rpm,Nm],\u2026]")
    ]
  },
  {
    title: "Drivetrain",
    fields: [
      number("drivetrain.finalDrive", "Final drive", 1, 8, 0.01, ":1"),
      json("drivetrain.gearRatios", "Gear ratios [..]")
    ]
  },
  {
    title: "Brakes",
    fields: [
      number("brakes.front", "Front torque", 100, 1e4, 50, "Nm"),
      number("brakes.rear", "Rear torque", 100, 1e4, 50, "Nm")
    ]
  },
  {
    title: "Steering",
    fields: [
      {
        ...number("steering.maxAngle", "Maximum lock", 5, 70, 0.5, "\xB0"),
        factor: 180 / Math.PI
      }
    ]
  },
  {
    title: "Aero",
    fields: [
      number("aero.dragCoefficient", "Drag coefficient", 0.05, 2, 0.01, "Cd"),
      number("aero.frontalArea", "Frontal area", 0.5, 5, 0.05, "m\xB2")
    ]
  }
];

// web/src/ui/dev-menu.ts
var STYLE_ID = "lapster-dev-menu-style";
var STYLE = `
#dev-drawer {
  position: fixed;
  inset: 0 auto 0 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  width: min(360px, 94vw);
  padding: 0;
  border-right: 1px solid #2a303a;
  background: rgba(8, 10, 13, 0.98);
  color: #e6e8eb;
  font: 11px/1.4 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  overflow: hidden;
  visibility: hidden;
  transform: translateX(calc(-100% - 8px));
  transition: transform 180ms ease, visibility 0s linear 180ms;
  box-shadow: 18px 0 48px rgba(0, 0, 0, 0.45);
}
#dev-drawer.dev-open {
  visibility: visible;
  transform: translateX(0);
  transition: transform 180ms ease, visibility 0s;
}
#dev-drawer .dev-head {
  position: relative;
  z-index: 2;
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 0;
  padding: 12px;
  border-bottom: 1px solid #2a303a;
  background: rgba(8, 10, 13, 0.98);
}
#dev-drawer .dev-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 12px 12px;
}
#dev-drawer .dev-head h2 {
  margin: 0;
  color: #e6e8eb;
  font-size: 13px;
  letter-spacing: 0;
  text-transform: none;
}
#dev-drawer .dev-badge {
  margin-left: 6px;
  padding: 1px 5px;
  border: 1px solid #38bdf8;
  border-radius: 3px;
  color: #38bdf8;
  font-size: 9px;
  letter-spacing: 0.08em;
}
#dev-open {
  position: fixed;
  left: 0;
  top: 50%;
  z-index: 31;
  min-width: 44px;
  padding: 6px 9px;
  border: 1px solid #3a4350;
  border-left: 0;
  border-radius: 0 4px 4px 0;
  background: rgba(8, 10, 13, 0.92);
  color: #38bdf8;
  font: 10px/1 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  letter-spacing: 0.12em;
  opacity: 0.82;
  transform: translateY(-50%);
  transition: transform 180ms ease, opacity 180ms ease, visibility 0s linear 0s;
  cursor: pointer;
}
#dev-open:hover { background: #151a21; opacity: 1; }
body.dev-menu-open #dev-open {
  visibility: hidden;
  opacity: 0;
  pointer-events: none;
  transform: translate(-100%, -50%);
  transition: transform 180ms ease, opacity 180ms ease, visibility 0s linear 180ms;
}
#dev-drawer button,
#dev-drawer select,
#dev-drawer input[type="text"] {
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
  color: #e6e8eb;
  padding: 4px 6px;
  font: inherit;
  font-size: 10px;
}
#dev-drawer button { cursor: pointer; }
#dev-drawer button:hover { background: #1c2430; }
#dev-drawer button:disabled { cursor: default; opacity: 0.55; }
#dev-drawer details {
  border-bottom: 1px solid #202630;
  padding: 0 0 8px;
}
#dev-drawer details > summary {
  padding: 9px 0 6px;
  color: #8f9bab;
  cursor: pointer;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  user-select: none;
}
#dev-drawer details details {
  border-bottom: 1px solid #202630;
  padding: 0 0 6px 8px;
}
#dev-drawer details details:last-child { border-bottom: 0; }
#dev-drawer details details > summary {
  padding: 7px 0 5px;
  color: #6f7b8b;
  letter-spacing: 0.04em;
  text-transform: none;
}
#dev-drawer .dev-grid {
  display: grid;
  gap: 6px;
}
#dev-drawer .dev-field {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 4px 8px;
}
#dev-drawer .dev-field > span { min-width: 0; }
#dev-drawer .dev-field output {
  color: #8f9bab;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
#dev-drawer .dev-field input[type="range"] {
  grid-column: 1 / -1;
  width: 100%;
  height: 16px;
  margin: 0;
  accent-color: #38bdf8;
}
#dev-drawer .dev-field select,
#dev-drawer .dev-field input[type="text"] {
  grid-column: 1 / -1;
  width: 100%;
  min-width: 0;
}
#dev-drawer .dev-field input[type="color"] {
  grid-column: 1 / -1;
  width: 100%;
  height: 22px;
  padding: 1px;
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
}
#dev-drawer .dev-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
#dev-drawer .dev-toggle input {
  width: 14px;
  height: 14px;
  margin: 0;
  accent-color: #38bdf8;
}
#dev-drawer .dev-footer {
  position: relative;
  z-index: 3;
  display: grid;
  flex: none;
  gap: 6px;
  margin: 0;
  padding: 10px 12px calc(10px + env(safe-area-inset-bottom));
  border-top: 1px solid #2a303a;
  background: rgba(8, 10, 13, 0.98);
}
#dev-drawer .dev-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
#dev-drawer .dev-status {
  margin: 0;
  color: #8f9bab;
  font-size: 10px;
  line-height: 1.4;
}
#dev-drawer .dev-status.error { color: #f87171; }
#dev-drawer .dev-note {
  margin: 0;
  padding: 0 0 4px;
  color: #6f7b8b;
  font-size: 10px;
  line-height: 1.35;
}
#dev-drawer .dev-audio {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
#dev-drawer .dev-audio span {
  flex: 1;
  color: #8f9bab;
}
@media (min-width: 900px) {
  body.dev-menu-open #hud {
    left: 376px;
    width: min(400px, calc(100vw - 408px));
  }
  body.dev-menu-open #fps-counter { left: 376px; }
}
#dev-drawer .dev-subhead {
  margin: 8px 0 2px;
  color: #cbd5e1;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
`;
function fieldRow(labelText, control, output) {
  const label = document.createElement("label");
  label.className = "dev-field";
  const name = document.createElement("span");
  name.textContent = labelText;
  label.append(name);
  if (output) {
    label.append(output);
  } else {
    const spacer = document.createElement("span");
    label.append(spacer);
  }
  label.append(control);
  return label;
}
function toggleRow(labelText, input) {
  const label = document.createElement("label");
  label.className = "dev-toggle";
  const name = document.createElement("span");
  name.textContent = labelText;
  label.append(name, input);
  return label;
}
function addSection(host, title) {
  const details = document.createElement("details");
  const summary = document.createElement("summary");
  summary.textContent = title;
  const grid = document.createElement("div");
  grid.className = "dev-grid";
  details.append(summary, grid);
  host.append(details);
  return details;
}
function createDevMenuShell(titleText, onOpen) {
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }
  const openButton = document.createElement("button");
  openButton.id = "dev-open";
  openButton.type = "button";
  openButton.textContent = "DEV";
  openButton.title = "Open developer settings";
  openButton.setAttribute("aria-label", "Open developer settings");
  openButton.setAttribute("aria-controls", "dev-drawer");
  openButton.setAttribute("aria-expanded", "false");
  document.body.append(openButton);
  const drawer = document.createElement("aside");
  drawer.id = "dev-drawer";
  drawer.setAttribute("aria-label", "Developer settings");
  drawer.setAttribute("aria-hidden", "true");
  const head = document.createElement("header");
  head.className = "dev-head";
  const title = document.createElement("h2");
  title.innerHTML = `${titleText} <span class="dev-badge">DEV</span>`;
  const closeButton = document.createElement("button");
  closeButton.id = "dev-close";
  closeButton.type = "button";
  closeButton.textContent = "Close";
  head.append(title, closeButton);
  drawer.append(head);
  const content = document.createElement("div");
  content.className = "dev-body";
  drawer.append(content);
  const status2 = document.createElement("p");
  status2.id = "dev-status";
  status2.className = "dev-status";
  status2.setAttribute("aria-live", "polite");
  status2.textContent = "Changes apply live";
  document.body.append(drawer);
  const isOpen = () => drawer.classList.contains("dev-open");
  const open = () => {
    onOpen();
    document.body.classList.add("dev-menu-open");
    drawer.classList.add("dev-open");
    drawer.setAttribute("aria-hidden", "false");
    openButton.setAttribute("aria-expanded", "true");
  };
  const close = () => {
    document.body.classList.remove("dev-menu-open");
    drawer.classList.remove("dev-open");
    drawer.setAttribute("aria-hidden", "true");
    openButton.setAttribute("aria-expanded", "false");
  };
  const toggle = () => {
    if (isOpen()) close();
    else open();
  };
  openButton.addEventListener("click", toggle);
  closeButton.addEventListener("click", close);
  window.addEventListener("keydown", (event) => {
    const target = event.target;
    if (target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "SELECT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
      return;
    }
    if (event.key === "\\") {
      event.preventDefault();
      toggle();
    } else if (event.key === "Escape" && isOpen()) {
      event.preventDefault();
      close();
    }
  });
  return { drawer, content, status: status2, openButton, closeButton, open, close, toggle };
}
function setupTrackDevMenu(viewer) {
  const shell = createDevMenuShell("Track rendering", () => {
    refreshBlend();
    refreshRacingLine();
    shell.status.className = "dev-status";
    shell.status.textContent = "Changes apply live";
  });
  const { drawer, content, status: status2 } = shell;
  const refreshBlend = () => {
    blend.value = viewer.track_blend_mode();
  };
  const racingLine = document.createElement("input");
  racingLine.id = "dev-racing-line";
  racingLine.type = "checkbox";
  racingLine.addEventListener("change", () => {
    viewer.set_racing_line_visible?.(racingLine.checked);
    status2.className = "dev-status";
    status2.textContent = racingLine.checked ? "Racing line shown" : "Racing line hidden";
  });
  const refreshRacingLine = () => {
    racingLine.checked = viewer.racing_line_visible?.() ?? true;
  };
  const blend = document.createElement("select");
  blend.id = "dev-track-blend";
  for (const [value, label] of [
    ["dither", "Dither"],
    ["soft", "Soft"],
    ["hard", "Hard cut"]
  ]) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    blend.append(option);
  }
  blend.addEventListener("change", () => {
    viewer.set_track_blend_mode(blend.value);
    status2.className = "dev-status";
    status2.textContent = "Edge blend updated";
  });
  const visuals = addSection(content, "Track rendering").querySelector(".dev-grid");
  visuals.append(fieldRow("Edge blend", blend));
  visuals.append(toggleRow("Racing line", racingLine));
  const note = document.createElement("p");
  note.className = "dev-note";
  note.textContent = "Dither is the shipped look. Soft blends the edge with a smooth gradient. Hard cut keeps a crisp reference edge.";
  visuals.append(note);
  const footer = document.createElement("div");
  footer.className = "dev-footer";
  footer.append(status2);
  drawer.append(footer);
  refreshBlend();
  refreshRacingLine();
  return shell;
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
var STYLE_ID2 = "lapster-fps-style";
var COUNTER_ID = "fps-counter";
var counter = null;
var enabled = false;
var frames = 0;
var elapsedMs = 0;
var STYLE2 = `
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
  if (!document.querySelector(`#${STYLE_ID2}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID2;
    style.textContent = STYLE2;
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
var TRACK_PROJECT_SCHEMA = "lapster/track-project@3";
var TRACK_PROJECT_STORAGE_KEY = "lapster.track-project.v1";
var DEFAULT_TRACK_WIDTH = 12;
var MIN_TRACK_WIDTH = 6;
var MAX_TRACK_WIDTH = 30;
var MAX_CAMERA_HEIGHT = 140;
var CONTROL_POINT_RADIUS = 9;
var CONTROL_POINT_HIT_RADIUS = 18;
var CENTERLINE_HIT_RADIUS = 12;
var KERB_RADIUS = 6.4;
var PROP_OFFSET = 14;
var PROP_FAR_OFFSET = 19;
var GUIDE_HIT_RADIUS = 12;
var GUIDE_HANDLE_RADIUS = 7;
var GUIDE_HANDLE_HIT_RADIUS = 16;
var BRANCH_DRAFT_DISTANCE = 25;
var BRANCH_APPROACH_DISTANCE = 20;
var BRANCH_SNAP_RADIUS = 12;
var LOOP_CENTERLINE_COLORS = [
  "#38bdf8",
  "#a78bfa",
  "#34d399",
  "#fbbf24",
  "#f472b6",
  "#fb923c",
  "#22d3ee",
  "#c084fc"
];
var DEFAULT_KERB_SEGMENT_LENGTH = 6;
var PROP_SEGMENT_LENGTH_METERS = 3;
var MIN_SEGMENT_LENGTH = 0.5;
var SURFACE_COLORS = {
  0: "#33a6ff",
  1: "#33ff59",
  2: "#b35933"
};
var devMode = new URLSearchParams(window.location.search).has("dev") && new URLSearchParams(window.location.search).get("dev") !== "0";
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
var propToggle = document.querySelector("#prop-toggle");
var treeDetailButton = document.querySelector("#tree-detail");
var deleteSegmentButton = document.querySelector("#delete-segment");
var overlayToggle = document.querySelector("#toggle-overlay");
var cameraViewButton = document.querySelector("#camera-view");
var testDriveButton = document.querySelector("#test-drive");
var layoutSelect = document.querySelector("#layout-select");
var addLayoutButton = document.querySelector("#add-layout");
var deleteLayoutButton = document.querySelector("#delete-layout");
var layoutNameInput = document.querySelector("#layout-name");
var layoutWidthInput = document.querySelector("#layout-width");
var layoutHud = document.querySelector("#layout-hud");
var saveProjectButton = document.querySelector("#save-project");
var loadProjectButton = document.querySelector("#load-project");
var exportProjectButton = document.querySelector("#export-project");
var importProjectButton = document.querySelector("#import-project");
var projectFileInput = document.querySelector("#project-file");
var makeUniqueButton = document.querySelector("#make-unique");
var editorViewer;
var project = null;
var projectJson = null;
var activeLayoutId = "";
var syncingProjectControls = false;
var cameraController;
var controlPoints = [];
var loopPoints = [];
var selectedPointId = null;
var centerline = [];
var loopCenterlines = {};
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
var branchDraft = null;
var branchDraftPointSeq = 0;
var paintKind = "blocks";
var TREE_DETAIL_MODES = ["high", "low", "cards"];
var TREE_DETAIL_LABELS = {
  high: "High",
  low: "Low",
  cards: "Cards"
};
var treeDetailMode = "high";
var overlayVisible = true;
var PROP_KINDS = ["blocks", "tyres", "trees", "off"];
var CAMERA_VIEWS = ["front", "back", "left", "right", "car"];
var CAMERA_VIEW_LABELS = {
  front: "Front",
  back: "Back",
  left: "Left",
  right: "Right",
  car: "Car"
};
var CAR_MARKER_HEIGHT = 3.2;
var CAR_MARKER_HIT_RADIUS = 16;
var CAR_CAMERA_HEIGHT = 1.4;
var CAR_LOOK_AHEAD = 7;
var CAR_ARROW_STEP = 1;
var cameraViewMode = "front";
var carIndex = 0;
var carDragPointerId = null;
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
function centerlinePoint(index) {
  const count = centerline.length;
  if (count === 0) {
    return [0, 0, 0];
  }
  const wrapped = (index % count + count) % count;
  const base = Math.floor(wrapped) % count;
  const next = (base + 1) % count;
  return lerp2(centerline[base], centerline[next], wrapped - Math.floor(wrapped));
}
function centerlineTangent(index) {
  const count = centerline.length;
  if (count < 2) {
    return [0, 0, -1];
  }
  const wrapped = (index % count + count) % count;
  const base = Math.floor(wrapped) % count;
  const t = wrapped - Math.floor(wrapped);
  const previous = centerline[(base + count - 1) % count];
  const current = centerline[base];
  const next = centerline[(base + 1) % count];
  const incoming = normalize2(sub2(current, previous));
  const outgoing = normalize2(sub2(next, current));
  return normalize2(lerp2(incoming, outgoing, t));
}
function carMarkerPoint() {
  const point = centerlinePoint(carIndex);
  return [point[0], point[1] + CAR_MARKER_HEIGHT, point[2]];
}
function carMarkerScreen() {
  const camera = currentCamera();
  if (!camera) {
    return null;
  }
  return projectPoint(
    carMarkerPoint(),
    camera,
    canvas.clientWidth,
    canvas.clientHeight
  );
}
function hitTestCarMarker(x, y) {
  const projected = carMarkerScreen();
  return !!projected && Math.hypot(projected[0] - x, projected[1] - y) <= CAR_MARKER_HIT_RADIUS;
}
function nearestCenterlineIndex(x, y) {
  const camera = currentCamera();
  if (!camera || centerline.length < 2) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const projected = centerline.map(
    (point) => projectPoint(point, camera, width, height)
  );
  let bestDistance = Infinity;
  let bestIndex = null;
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
        ((x - start[0]) * segmentX + (y - start[1]) * segmentY) / lengthSquared
      )
    ) : 0;
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
function applyCameraView() {
  if (!cameraController || centerline.length === 0) {
    return;
  }
  if (cameraViewMode === "car") {
    const point = centerlinePoint(carIndex);
    const tangent = centerlineTangent(carIndex);
    const eye2 = add2(point, [0, CAR_CAMERA_HEIGHT, 0]);
    const target = add2(
      add2(point, scale2(tangent, CAR_LOOK_AHEAD)),
      [0, CAR_CAMERA_HEIGHT, 0]
    );
    cameraController.setView(eye2, target);
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
  const center = [
    (minX + maxX) * 0.5,
    (minY + maxY) * 0.5,
    (minZ + maxZ) * 0.5
  ];
  const span = Math.max(maxX - minX, maxZ - minZ);
  const distance = Math.max(span * 1.25, 55);
  const height = Math.max(span, 40);
  let eye = [center[0], center[1] + height, center[2] + distance];
  if (cameraViewMode === "back") {
    eye = [center[0], center[1] + height, center[2] - distance];
  } else if (cameraViewMode === "left") {
    eye = [center[0] - distance, center[1] + height, center[2]];
  } else if (cameraViewMode === "right") {
    eye = [center[0] + distance, center[1] + height, center[2]];
  }
  cameraController.setView(eye, center);
}
function syncCameraViewButton() {
  cameraViewButton.textContent = `View: ${CAMERA_VIEW_LABELS[cameraViewMode]}`;
}
function cycleCameraView() {
  const index = CAMERA_VIEWS.indexOf(cameraViewMode);
  cameraViewMode = CAMERA_VIEWS[(index + 1) % CAMERA_VIEWS.length];
  syncCameraViewButton();
  applyCameraView();
  drawControlPoints();
}
function moveCar(delta) {
  const count = centerline.length;
  if (count === 0) {
    return;
  }
  carIndex = ((carIndex + delta) % count + count) % count;
  if (cameraViewMode === "car") {
    applyCameraView();
  }
  drawControlPoints();
}
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
function groundPointFromScreen(x, y, planeY) {
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
  return add2(ray.origin, scale2(ray.direction, distance));
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
    { kind: "prop", side: -outside, radius: PROP_OFFSET, color: "#2dd4bf" },
    { kind: "prop", side: outside, radius: PROP_FAR_OFFSET, color: "#f472b6" },
    { kind: "prop", side: -outside, radius: PROP_FAR_OFFSET, color: "#c084fc" }
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
function sameOffset(a, b) {
  return Math.abs(a - b) < 1e-4;
}
function guideFor(kind, side, offset) {
  return guideDefinitions().find(
    (guide) => guide.kind === kind && guide.side === side && (kind === "kerb" || sameOffset(guide.radius, offset))
  ) ?? null;
}
function matchesLane(kind, range, side, offset) {
  return range.side === side && (kind === "kerb" || sameOffset(range.offset ?? PROP_OFFSET, offset));
}
function findRange(kind, side, offset, index) {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return ranges.find(
    (range) => matchesLane(kind, range, side, offset) && index >= range.start - 1e-4 && index <= range.end + 1e-4
  ) ?? null;
}
function findRangeExact(kind, side, offset, start, end) {
  const ranges = kind === "kerb" ? kerbSpans : propLines;
  return ranges.find(
    (range) => matchesLane(kind, range, side, offset) && Math.abs(range.start - start) < 1e-4 && Math.abs(range.end - end) < 1e-4
  ) ?? null;
}
function selectedPropLine() {
  if (!selectedSegment || selectedSegment.kind !== "prop") {
    return void 0;
  }
  return propLines.find(
    (line) => matchesLane(
      "prop",
      line,
      selectedSegment.side,
      selectedSegment.offset
    ) && Math.abs(line.start - selectedSegment.start) < 1e-4 && Math.abs(line.end - selectedSegment.end) < 1e-4
  );
}
function resolveSelection() {
  if (!selectedSegment) {
    return;
  }
  const range = findRangeExact(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.offset,
    selectedSegment.start,
    selectedSegment.end
  ) ?? findRange(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.offset,
    (selectedSegment.start + selectedSegment.end) * 0.5
  );
  if (range) {
    selectedSegment.start = range.start;
    selectedSegment.end = range.end;
    if (range.offset !== void 0) {
      selectedSegment.offset = range.offset;
    }
  } else {
    selectedSegment = null;
  }
}
function subtractRanges(items, kind, side, offset, start, end) {
  const result = [];
  for (const item of items) {
    if (!matchesLane(kind, item, side, offset) || item.end <= start || item.start >= end) {
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
function selectSegment(kind, side, offset, index) {
  const range = findRange(kind, side, offset, index);
  if (!range) {
    return false;
  }
  selectedSegment = {
    kind,
    side,
    offset: range.offset ?? offset,
    start: range.start,
    end: range.end
  };
  if (kind === "prop") {
    const line = propLines.find(
      (item) => matchesLane("prop", item, range.side, offset) && Math.abs(item.start - range.start) < 1e-4 && Math.abs(item.end - range.end) < 1e-4
    );
    if (line) {
      paintKind = line.kind;
    }
  }
  syncPaintControls();
  return true;
}
function propSegmentLength() {
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
      next[2] - point[2]
    );
  }
  const sampleSpacing = perimeter / count;
  return Math.max(
    MIN_SEGMENT_LENGTH,
    PROP_SEGMENT_LENGTH_METERS / Math.max(sampleSpacing, 1e-4)
  );
}
function createSegment(guide, index) {
  const count = centerline.length;
  if (guide.kind === "prop" && paintKind === "off") {
    flashStatus("Choose a prop type to add props");
    return;
  }
  const defaultLength = guide.kind === "prop" ? propSegmentLength() : DEFAULT_KERB_SEGMENT_LENGTH;
  const length2 = Math.min(defaultLength, count);
  const start = Math.max(0, Math.min(count - length2, index - length2 * 0.5));
  const end = Math.min(count, start + length2);
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
      end
    );
    kerbSpans.push({ start, end, side: guide.side });
    kerbSpans.sort((a, b) => a.side - b.side || a.start - b.start);
    selectedSegment = {
      kind: "kerb",
      side: guide.side,
      offset: guide.radius,
      start,
      end
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
    end
  );
  propLines.push({
    start,
    end,
    side: guide.side,
    offset: guide.radius,
    kind: paintKind
  });
  propLines.sort(
    (a, b) => a.side - b.side || a.offset - b.offset || a.start - b.start
  );
  selectedSegment = {
    kind: "prop",
    side: guide.side,
    offset: guide.radius,
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
    selectedSegment.offset,
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
  const guide = guideFor(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.offset
  );
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
function syncPaintControls() {
  propToggle.textContent = `Prop: ${paintLabel(paintKind)}`;
  deleteSegmentButton.disabled = selectedSegment === null;
}
function syncTreeDetailButton() {
  treeDetailButton.textContent = `Trees: ${TREE_DETAIL_LABELS[treeDetailMode]}`;
}
function deleteSelectedSegment() {
  if (!selectedSegment || rebuilding) {
    return;
  }
  const { kind, side, offset, start, end } = selectedSegment;
  selectedSegment = null;
  hoverPoint = null;
  hoverHandleEdge = null;
  if (kind === "kerb") {
    kerbSpans = kerbSpans.filter(
      (span) => !(span.side === side && Math.abs(span.start - start) < 1e-4 && Math.abs(span.end - end) < 1e-4)
    );
    syncPaintControls();
    drawControlPoints();
    void rebuildKerbSpans();
    return;
  }
  propLines = propLines.filter(
    (line) => !(matchesLane("prop", line, side, offset) && Math.abs(line.start - start) < 1e-4 && Math.abs(line.end - end) < 1e-4)
  );
  syncPaintControls();
  drawControlPoints();
  void rebuildPropLines();
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
  if (!overlayVisible || !lastPointer || dragIndex !== null || guideDrag !== null || carDragPointerId !== null || rebuilding || cameraDragging) {
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
    const pointIndex = Math.floor(index * loopPoints.length / centerline.length) % Math.max(loopPoints.length, 1);
    const afterPointId = loopPoints[pointIndex]?.id;
    if (!afterPointId) {
      continue;
    }
    hoverInsert = {
      point: lerp2(centerline[index], centerline[next], t),
      screen: [hitX, hitY],
      afterPointId
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
function loopCenterlineColor(loopId) {
  let hash = 0;
  for (let index = 0; index < loopId.length; index += 1) {
    hash = hash * 31 + loopId.charCodeAt(index) >>> 0;
  }
  return LOOP_CENTERLINE_COLORS[hash % LOOP_CENTERLINE_COLORS.length];
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
  const drawCenterline = (points, color, active) => {
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
  const selectedGuide = selectedSegment ? guideFor(
    selectedSegment.kind,
    selectedSegment.side,
    selectedSegment.offset
  ) : null;
  const draggedGuide = guideDrag ? guideFor(guideDrag.kind, guideDrag.side, guideDrag.offset) : null;
  const activeGuide = draggedGuide ?? hoverPoint?.guide ?? selectedGuide;
  for (const guide of guides) {
    const active = activeGuide?.kind === guide.kind && activeGuide.side === guide.side && (guide.kind === "kerb" || sameOffset(activeGuide.radius, guide.radius));
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
  const carMarker = carMarkerPoint();
  const carProjected = projectPoint(carMarker, camera, width, height);
  const carAheadProjected = projectPoint(
    add2(carMarker, scale2(centerlineTangent(carIndex), 4)),
    camera,
    width,
    height
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
        carY - uy * 3 + py * 4
      );
      overlayContext.lineTo(
        carX - ux * 3 - px * 4,
        carY - uy * 3 - py * 4
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
      Math.PI * 2
    );
    overlayContext.fillStyle = deleteTarget ? "#ef4444" : deleteMode ? "#fca5a5" : "#fbbf24";
    overlayContext.fill();
    overlayContext.lineWidth = deleteTarget || highlighted ? 3 : 2;
    overlayContext.strokeStyle = deleteTarget ? "#fff1f2" : highlighted ? "#f8fafc" : "#080a0d";
    overlayContext.stroke();
    overlayContext.fillStyle = "#080a0d";
    overlayContext.font = "600 10px ui-monospace, monospace";
    overlayContext.textAlign = "center";
    overlayContext.textBaseline = "middle";
    overlayContext.fillText(String(index + 1), projected[0], projected[1] + 0.5);
  });
  if (branchDraft) {
    const junctionIndex = loopPoints.findIndex(
      (point) => point.id === branchDraft.junctionId
    );
    const draftWorld = [];
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
        Math.PI * 2
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
function moveControlPoint(index, x, y) {
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
      position: [pointX, pointZ]
    };
  }
}
function activeLayout() {
  if (!project) {
    return null;
  }
  return project.loops.find((layout) => layout.id === activeLayoutId) ?? project.loops[0] ?? null;
}
function trackWidth() {
  return project?.settings?.width ?? DEFAULT_TRACK_WIDTH;
}
function sharedPointCount() {
  return loopPoints.filter((point) => point.shared).length;
}
function selectedPointIndex() {
  if (!selectedPointId) {
    return -1;
  }
  return loopPoints.findIndex((point) => point.id === selectedPointId);
}
function syncPointControls() {
  const index = selectedPointIndex();
  const point = index >= 0 ? loopPoints[index] : null;
  makeUniqueButton.disabled = !point || rebuilding;
}
function segmentKey(from, to) {
  return `${from}->${to}`;
}
function uniqueBranchBase(pointCount) {
  const loopIds = new Set(project?.loops.map((loop) => loop.id) ?? []);
  const pointIds = new Set(project?.controlPoints.map((point) => point.id) ?? []);
  let number2 = 1;
  for (; ; ) {
    const loopId = `branch-${number2}`;
    const ids = Array.from(
      { length: pointCount },
      (_, index) => `${loopId}-cp-${index + 1}`
    );
    if (!loopIds.has(loopId) && ids.every((id) => !pointIds.has(id))) {
      return { loopId, name: `Branch ${number2}`, pointIds: ids };
    }
    number2 += 1;
  }
}
function normalize2d(value) {
  const length2 = Math.hypot(value[0], value[1]);
  return length2 > 1e-6 ? [value[0] / length2, value[1] / length2] : [1, 0];
}
function loopTangentAt(route, positions, index) {
  const count = route.length;
  const previous = positions.get(route[(index - 1 + count) % count]);
  const next = positions.get(route[(index + 1) % count]);
  if (!previous || !next) {
    return null;
  }
  return normalize2d([next[0] - previous[0], next[1] - previous[1]]);
}
function loopNormalAt(route, positions, index) {
  const tangent = loopTangentAt(route, positions, index);
  if (!tangent) {
    return null;
  }
  return normalize2d([-tangent[1], tangent[0]]);
}
function loopCentroid(route, positions) {
  const points = route.map((id) => positions.get(id)).filter((position) => Boolean(position));
  if (points.length === 0) {
    return [0, 0];
  }
  return [
    points.reduce((sum, position) => sum + position[0], 0) / points.length,
    points.reduce((sum, position) => sum + position[1], 0) / points.length
  ];
}
function outwardNormal(position, normal, centroid) {
  const distanceToCentroid = (candidate) => Math.hypot(candidate[0] - centroid[0], candidate[1] - centroid[1]);
  const plus = [
    position[0] + normal[0] * BRANCH_DRAFT_DISTANCE,
    position[1] + normal[1] * BRANCH_DRAFT_DISTANCE
  ];
  const minus = [
    position[0] - normal[0] * BRANCH_DRAFT_DISTANCE,
    position[1] - normal[1] * BRANCH_DRAFT_DISTANCE
  ];
  return distanceToCentroid(plus) >= distanceToCentroid(minus) ? normal : [-normal[0], -normal[1]];
}
function startBranchDraft(index) {
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
    project.controlPoints.map((point) => [point.id, point.position])
  );
  const position = positions.get(junctionId);
  if (!position) {
    flashStatus("branch point unavailable");
    return;
  }
  const normal = loopNormalAt(route, positions, startIndex);
  const centroid = loopCentroid(route, positions);
  const outward = normal ? outwardNormal(position, normal, centroid) : [1, 0];
  const firstPoint = [
    position[0] + outward[0] * BRANCH_DRAFT_DISTANCE,
    junctionPosition[1],
    position[1] + outward[1] * BRANCH_DRAFT_DISTANCE
  ];
  branchDraft = {
    sourceLoopId: layout.id,
    junctionId,
    points: [{ id: `draft-${branchDraftPointSeq++}`, position: firstPoint }]
  };
  flashStatus(
    "branch started \xB7 Ctrl+click to add a point, Ctrl+click a loop point to close, Esc cancels"
  );
  drawControlPoints();
}
function sourceLoopControlPoints(loopId) {
  const loop = project?.loops.find((item) => item.id === loopId);
  if (!project || !loop) {
    return [];
  }
  const positions = new Map(
    project.controlPoints.map((point) => [point.id, point.position])
  );
  return loop.controlPoints.flatMap((id) => {
    const position = positions.get(id);
    return position ? [{ id, position }] : [];
  });
}
function appendBranchDraftPoint(x, y) {
  if (!branchDraft) {
    return;
  }
  const planeY = branchDraft.points[0]?.position[1] ?? 0;
  const world = groundPointFromScreen(x, y, planeY);
  if (!world) {
    flashStatus("cannot place branch point here");
    return;
  }
  let nearestId = null;
  let nearestDistance = BRANCH_SNAP_RADIUS;
  for (const point of sourceLoopControlPoints(branchDraft.sourceLoopId)) {
    const distance = Math.hypot(
      world[0] - point.position[0],
      world[2] - point.position[1]
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
    position: world
  });
  drawControlPoints();
}
function cancelBranchDraft() {
  if (!branchDraft) {
    return;
  }
  branchDraft = null;
  flashStatus("branch cancelled");
  drawControlPoints();
}
async function closeBranchDraft(endId) {
  if (!editorViewer || !project || !branchDraft || rebuilding) {
    return;
  }
  const draft = branchDraft;
  const layout = project.loops.find((loop) => loop.id === draft.sourceLoopId) ?? activeLayout();
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
    project.controlPoints.map((point) => [point.id, point.position])
  );
  const endPosition = positions.get(endId);
  const endTangent = endPosition && endIndex >= 0 ? loopTangentAt(route, positions, endIndex) : null;
  const approachPoints = [];
  if (endPosition && endTangent) {
    if (!closesToJunction && draft.points.length <= 1) {
      const endNormal = loopNormalAt(route, positions, endIndex);
      if (endNormal) {
        const centroid = loopCentroid(route, positions);
        const outward = outwardNormal(endPosition, endNormal, centroid);
        approachPoints.push([
          endPosition[0] + outward[0] * BRANCH_DRAFT_DISTANCE,
          endPosition[1] + outward[1] * BRANCH_DRAFT_DISTANCE
        ]);
      }
    }
    const nextIndex = (endIndex + 1) % route.length;
    const nextPosition = positions.get(route[nextIndex]);
    const approachBase = nextPosition ?? endPosition;
    approachPoints.push([
      approachBase[0] - endTangent[0] * BRANCH_APPROACH_DISTANCE,
      approachBase[1] - endTangent[1] * BRANCH_APPROACH_DISTANCE
    ]);
  }
  const base = uniqueBranchBase(draft.points.length + approachPoints.length);
  const draftControlPoints = [
    ...draft.points.map(
      (point, index) => ({
        id: base.pointIds[index],
        position: [point.position[0], point.position[2]],
        kind: "normal"
      })
    ),
    ...approachPoints.map(
      (position, index) => ({
        id: base.pointIds[draft.points.length + index],
        position,
        kind: "normal"
      })
    )
  ];
  const draftIds = draftControlPoints.map((point) => point.id);
  const newRoute = closesToJunction ? [draft.junctionId, ...draftIds] : [draft.junctionId, ...draftIds, endId];
  if (!closesToJunction) {
    for (let index = (endIndex + 1) % route.length; index !== startIndex; index = (index + 1) % route.length) {
      newRoute.push(route[index]);
    }
  }
  const segments = project.segments.map((segment) => ({
    ...segment,
    role: segment.role === "branch" ? "branch" : "main",
    kerbs: Array.isArray(segment.kerbs) ? segment.kerbs : [],
    propSpans: Array.isArray(segment.propSpans) ? segment.propSpans : []
  }));
  const pairs = new Set(
    segments.map((segment) => segmentKey(segment.from, segment.to))
  );
  const addSegment = (from, to, role) => {
    const key = segmentKey(from, to);
    if (pairs.has(key)) {
      return;
    }
    pairs.add(key);
    segments.push({ from, to, role, kerbs: [], propSpans: [] });
  };
  const branchRoute = closesToJunction ? [] : [draft.junctionId, ...draftIds, endId];
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
    spawns: []
  });
  branchDraft = null;
  drawControlPoints();
  await rebuildEditor("creating branch", async () => {
    await editorViewer.set_editor_project(JSON.stringify(project));
    await editorViewer.set_editor_active_layout(base.loopId);
  });
}
function syncProjectControls() {
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
    layoutHud.textContent = layout ? `${layout.name || layout.id} \xB7 ${loopPoints.length} control points \xB7 ${sharedPointCount()} shared` : "\u2014";
  } finally {
    syncingProjectControls = false;
  }
  syncPointControls();
}
function readProject() {
  if (!editorViewer) {
    return;
  }
  const raw = editorViewer.editor_project();
  try {
    const parsed = JSON.parse(raw);
    if (parsed && parsed.schema === TRACK_PROJECT_SCHEMA && Array.isArray(parsed.loops)) {
      parsed.segments = Array.isArray(parsed.segments) ? parsed.segments.map((segment) => ({
        ...segment,
        role: segment.role === "branch" ? "branch" : "main",
        kerbs: Array.isArray(segment.kerbs) ? segment.kerbs : [],
        propSpans: Array.isArray(segment.propSpans) ? segment.propSpans : []
      })) : [];
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
function projectSnapshot() {
  if (projectJson) {
    return projectJson;
  }
  if (project) {
    return JSON.stringify(project);
  }
  return null;
}
function saveProject() {
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
function loadSavedProject() {
  try {
    return localStorage.getItem(TRACK_PROJECT_STORAGE_KEY);
  } catch {
    return null;
  }
}
function refreshEditorState() {
  if (!editorViewer) {
    return;
  }
  const heights = JSON.parse(editorViewer.editor_control_points());
  loopPoints = JSON.parse(editorViewer.editor_loop_points());
  controlPoints = loopPoints.map((point, index) => [
    point.position[0],
    heights[index]?.[1] ?? 0,
    point.position[1]
  ]);
  if (selectedPointId && selectedPointIndex() < 0) {
    selectedPointId = null;
  }
  centerline = JSON.parse(editorViewer.editor_centerline());
  loopCenterlines = JSON.parse(
    editorViewer.editor_loop_centerlines()
  );
  kerbSpans = JSON.parse(editorViewer.editor_kerb_spans()).sort((a, b) => a.side - b.side || a.start - b.start);
  propLines = JSON.parse(editorViewer.editor_prop_lines());
  readProject();
  resolveSelection();
  syncPaintControls();
  const debug = JSON.parse(editorViewer.debug_info());
  renderStats(debug);
  drawsLabel.textContent = String(debug.stats.drawCalls);
}
async function testDrive() {
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
function reportActionError(error) {
  const message = error instanceof Error ? error.message : String(error);
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}
addLayoutButton.addEventListener("click", () => {
  void rebuildEditor(
    "adding layout",
    () => editorViewer.add_editor_layout("").then(() => void 0)
  );
});
deleteLayoutButton.addEventListener("click", () => {
  if (!activeLayoutId) {
    return;
  }
  void rebuildEditor(
    "deleting layout",
    () => editorViewer.delete_editor_layout(activeLayoutId),
    reportActionError
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
  void rebuildEditor(
    "switching layout",
    () => editorViewer.set_editor_active_layout(id)
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
  void rebuildEditor(
    "renaming layout",
    () => editorViewer.rename_editor_layout(activeLayoutId, name)
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
  void rebuildEditor(
    "resizing track",
    () => editorViewer.set_editor_layout_width(activeLayoutId, clamped)
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
  void rebuildEditor("loading project", () => editorViewer.set_editor_project(saved));
});
function exportProject() {
  const snapshot = projectSnapshot();
  if (!snapshot) {
    reportActionError(new Error("track project unavailable"));
    return;
  }
  let json2 = snapshot;
  try {
    json2 = JSON.stringify(JSON.parse(snapshot), null, 2);
  } catch {
  }
  const blob = new Blob([json2], {
    type: "application/json"
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
      await rebuildEditor(
        "importing project",
        () => editorViewer.set_editor_project(text)
      );
    } catch (error) {
      reportActionError(error);
    }
  })();
});
async function rebuildEditor(message, action, onError = fail) {
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
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
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
function selectControlPoint(id) {
  selectedPointId = id;
  syncPointControls();
  drawControlPoints();
}
async function deleteControlPoint(pointId) {
  if (!editorViewer || !activeLayoutId || rebuilding) {
    return;
  }
  await rebuildEditor(
    "deleting control point",
    () => editorViewer.delete_editor_control_point(activeLayoutId, pointId)
  );
  if (selectedPointId === pointId) {
    selectedPointId = null;
  }
  syncPointControls();
  drawControlPoints();
}
async function insertControlPoint(point, afterPointId) {
  if (!editorViewer || !activeLayoutId || rebuilding) {
    return;
  }
  let newId = "";
  await rebuildEditor("inserting control point", async () => {
    newId = await editorViewer.insert_editor_control_point(
      activeLayoutId,
      afterPointId,
      point[0],
      point[2]
    );
  });
  if (newId) {
    selectControlPoint(newId);
  }
}
async function moveControlPointById(pointId, point) {
  if (!editorViewer || rebuilding) {
    return;
  }
  await rebuildEditor(
    "moving control point",
    () => editorViewer.move_editor_control_point(pointId, point[0], point[2])
  );
}
async function makeSelectedPointUnique() {
  const index = selectedPointIndex();
  const point = index >= 0 ? loopPoints[index] : null;
  if (!point || !editorViewer || !activeLayoutId || rebuilding) {
    return;
  }
  let newId = "";
  await rebuildEditor("making control point unique", async () => {
    newId = await editorViewer.make_editor_control_point_unique(
      activeLayoutId,
      point.id
    );
  });
  if (newId) {
    selectControlPoint(newId);
  }
}
canvas.addEventListener(
  "pointerdown",
  (event) => {
    if (event.button !== 0 || dragIndex !== null || rebuilding) {
      return;
    }
    const [x, y] = pointerPosition(event);
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      const index2 = overlayVisible ? hitTestControlPoint(x, y) : -1;
      if (branchDraft) {
        if (index2 >= 0) {
          const pointId = loopPoints[index2]?.id;
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
      if (index2 < 0) {
        flashStatus("Ctrl+click a control point to start a branch");
        return;
      }
      startBranchDraft(index2);
      return;
    }
    if (event.shiftKey !== deleteMode) {
      setDeleteMode(event.shiftKey);
    }
    if (overlayVisible && !deleteMode && carDragPointerId === null && hitTestCarMarker(x, y)) {
      event.preventDefault();
      event.stopImmediatePropagation();
      cameraDragging = false;
      carDragPointerId = event.pointerId;
      cameraController?.setEnabled(false);
      try {
        canvas.setPointerCapture(event.pointerId);
      } catch {
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
          if (selectSegment(
            point.guide.kind,
            point.guide.side,
            point.guide.radius,
            point.index
          )) {
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
    }
    drawControlPoints();
  },
  { capture: true }
);
window.addEventListener(
  "pointermove",
  (event) => {
    if (carDragPointerId !== null && event.pointerId === carDragPointerId) {
      event.preventDefault();
      const [x2, y2] = pointerPosition(event);
      const index = nearestCenterlineIndex(x2, y2);
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
        guideDrag.offset
      );
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
  const index = dragIndex;
  const pointId = loopPoints[index]?.id;
  dragIndex = null;
  dragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
  }
  drawControlPoints();
  if (pointId) {
    void moveControlPointById(pointId, controlPoints[index]);
  }
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
var endCarDrag = (event) => {
  if (carDragPointerId === null || event.pointerId !== carDragPointerId) {
    return;
  }
  carDragPointerId = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
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
  if (target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
    return;
  }
  if ((event.key === "Delete" || event.key === "Backspace") && selectedSegment && !guideDrag) {
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
var endCameraDrag = () => {
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
  window.__setEditorProject = async (json2) => {
    if (!editorViewer) throw new Error("editor unavailable");
    await rebuildEditor("loading project", () => editorViewer.set_editor_project(json2));
  };
  window.__setEditorActiveLayout = async (id) => {
    if (!editorViewer) throw new Error("editor unavailable");
    await rebuildEditor(
      "switching layout",
      () => editorViewer.set_editor_active_layout(id)
    );
  };
  window.__editorMovePoint = (id, x, z) => moveControlPointById(id, [x, 0, z]);
  window.__editorMakeUnique = (loopId, pointId) => rebuildEditor("making control point unique", async () => {
    const newId = await editorViewer.make_editor_control_point_unique(
      loopId,
      pointId
    );
    selectControlPoint(newId);
    return newId;
  });
  window.__editorSelectPoint = (id) => selectControlPoint(id);
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
    attachResetButton: false
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
function fail(error) {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__trackEditorError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}
boot().catch(fail);
