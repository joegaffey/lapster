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
var MAX_CAMERA_HEIGHT = 140;
var CONTROL_POINT_RADIUS = 9;
var CONTROL_POINT_HIT_RADIUS = 18;
var SECTION_HIT_RADIUS = 10;
var PROJECT_STORAGE_KEY = "lapster.track-editor.project.v6";
var PROJECT_SCHEMA = "lapster/track-project@6";
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
var treeDetailButton = document.querySelector("#tree-detail");
var overlayToggle = document.querySelector("#toggle-overlay");
var cameraViewButton = document.querySelector("#camera-view");
var testDriveButton = document.querySelector("#test-drive");
var saveProjectButton = document.querySelector("#save-project");
var loadProjectButton = document.querySelector("#load-project");
var exportProjectButton = document.querySelector("#export-project");
var importProjectButton = document.querySelector("#import-project");
var importProjectInput = document.querySelector("#import-project-file");
var resetProjectButton = document.querySelector("#reset-project");
var selectionDetails = document.querySelector("#selection-details");
var pointEditor = document.querySelector("#point-editor");
var pointEditorId = document.querySelector("#point-editor-id");
var pointEditorRows = document.querySelector("#point-editor-rows");
var pointEditorClose = document.querySelector("#point-editor-close");
var deletePointButton = document.querySelector("#delete-point");
var setStartFinishButton = document.querySelector("#set-start-finish");
var editorViewer;
var cameraController;
var project = {};
var activeLayoutId = "";
var editorPoints = [];
var sectionCenterlines = {};
var routeSectionIds = /* @__PURE__ */ new Set();
var selection = null;
var centerline = [];
var defaultProjectJson = "";
var dragIndex = null;
var dragPointerId = null;
var dragPlaneY = 0;
var dragStartPosition = null;
var insertHover = null;
var rebuilding = false;
var rebuildWaiters = [];
var suspended = false;
var cameraDragging = false;
var lastPointer = null;
var TREE_DETAIL_MODES = ["high", "low", "cards"];
var TREE_DETAIL_LABELS = {
  high: "High",
  low: "Low",
  cards: "Cards"
};
var treeDetailMode = "high";
var overlayVisible = true;
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
function hitTestControlPoint(x, y) {
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
function hitTestRouteSection(x, y) {
  const camera = currentCamera();
  if (!camera || routeSectionIds.size === 0) {
    return null;
  }
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  let best = null;
  let bestDistance = SECTION_HIT_RADIUS;
  for (const section of project.sections ?? []) {
    if (!routeSectionIds.has(section.id)) {
      continue;
    }
    const samples = sectionCenterlines[section.id];
    if (!samples || samples.length < 2) {
      continue;
    }
    const projected = samples.map(
      (point) => projectPoint(point, camera, width, height)
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
      const segmentT = lengthSquared > 1e-6 ? Math.max(
        0,
        Math.min(
          1,
          ((x - start[0]) * segmentX + (y - start[1]) * segmentY) / lengthSquared
        )
      ) : 0;
      const distance = Math.hypot(
        x - (start[0] + segmentX * segmentT),
        y - (start[1] + segmentY * segmentT)
      );
      if (distance < bestDistance) {
        bestDistance = distance;
        const next = (index + 1) % samples.length;
        best = {
          sectionId: section.id,
          position: lerp2(samples[index], samples[next], segmentT),
          t: Math.max(
            0,
            Math.min(1, (index + segmentT) / denominator)
          )
        };
      }
    }
  }
  return best;
}
function insertAfterIndex(section, t) {
  const count = section.points.length;
  const raw = section.closed === false ? Math.floor(t * (count - 1)) : Math.floor(t * count);
  return Math.max(0, Math.min(count - 1, raw));
}
function updateInsertHover() {
  insertHover = null;
  const camera = currentCamera();
  if (!camera || !overlayVisible || !lastPointer || dragIndex !== null || carDragPointerId !== null || rebuilding || cameraDragging) {
    return;
  }
  const [x, y] = lastPointer;
  if (hitTestCarMarker(x, y) || hitTestControlPoint(x, y) >= 0) {
    return;
  }
  insertHover = hitTestRouteSection(x, y);
}
function updateCursor() {
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
  if (insertHover) {
    canvas.style.cursor = "copy";
    return;
  }
  if (hitTestCarMarker(lastPointer[0], lastPointer[1]) || hitTestControlPoint(lastPointer[0], lastPointer[1]) >= 0) {
    canvas.style.cursor = "grab";
    return;
  }
  canvas.style.cursor = "";
}
function setOverlayVisible(visible) {
  overlayVisible = visible;
  overlay.hidden = !visible;
  overlayToggle.textContent = visible ? "Hide overlay" : "Show overlay";
  overlayToggle.setAttribute("aria-pressed", String(visible));
  if (!visible) {
    canvas.style.cursor = "";
  }
  drawControlPoints();
}
function setStatus(message, state = "") {
  status.textContent = message;
  status.className = `status${state ? ` ${state}` : ""}`;
}
function reportProjectError(prefix, error) {
  const message = error instanceof Error ? error.message : String(error);
  setStatus(`${prefix}: ${message}`, "error");
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
  if (selection?.kind === "section") {
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
  if (insertHover) {
    const projected = projectPoint(
      insertHover.position,
      camera,
      width,
      height
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
  const pointPositions = new Map(
    editorPoints.map((point) => [point.id, point.position])
  );
  const pointsById = new Map(editorPoints.map((point) => [point.id, point]));
  const drawnSections = selection?.kind === "section" ? (project.sections ?? []).filter(
    (section) => section.id === selection?.id
  ) : (project.sections ?? []).filter(
    (section) => routeSectionIds.has(section.id)
  );
  for (const section of drawnSections) {
    const positions = [];
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
    const seen = /* @__PURE__ */ new Set();
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
  editorPoints.forEach((point, index) => {
    const projected = projectPoint(point.position, camera, width, height);
    if (!projected) {
      return;
    }
    const selected = index === dragIndex || selection?.kind === "point" && selection.id === point.id;
    const roleColor = point.startFinish ? "#22c55e" : point.junction ? "#f97316" : point.looseEnd ? "#a78bfa" : "#fbbf24";
    const anchor = point.kind === "anchor";
    const radius = anchor ? CONTROL_POINT_RADIUS + 1 : CONTROL_POINT_RADIUS;
    overlayContext.beginPath();
    overlayContext.arc(
      projected[0],
      projected[1],
      radius,
      0,
      Math.PI * 2
    );
    overlayContext.fillStyle = selected ? "#38bdf8" : roleColor;
    overlayContext.fill();
    overlayContext.lineWidth = 2;
    overlayContext.strokeStyle = point.startFinish || point.junction || point.looseEnd || anchor ? roleColor : "#080a0d";
    overlayContext.stroke();
    if (anchor) {
      overlayContext.beginPath();
      overlayContext.arc(
        projected[0],
        projected[1],
        radius * 0.5,
        0,
        Math.PI * 2
      );
      overlayContext.fillStyle = "#080a0d";
      overlayContext.fill();
      overlayContext.lineWidth = 1.5;
      overlayContext.strokeStyle = selected ? "#38bdf8" : roleColor;
      overlayContext.stroke();
    }
    overlayContext.fillStyle = anchor ? selected ? "#38bdf8" : roleColor : "#080a0d";
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
  const current = editorPoints[index];
  if (!current) {
    return;
  }
  current.position = [
    Math.max(-140, Math.min(140, world[0])),
    current.position[1],
    Math.max(-140, Math.min(140, world[2]))
  ];
}
function pointHeights(document2, centerlines) {
  const heights = /* @__PURE__ */ new Map();
  const pointsById = new Map(
    (document2.points ?? []).map((point) => [point.id, point])
  );
  for (const section of document2.sections ?? []) {
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
function isLooseEndPoint(point) {
  if (point.junction || point.degree >= 2) {
    return false;
  }
  return (project.sections ?? []).some(
    (section) => section.closed !== true && (section.points[0] === point.id || section.points[section.points.length - 1] === point.id)
  );
}
var CONTACT_TOLERANCE = 0.5;
function nearestCenterlineContact(position, samples) {
  let best = Infinity;
  let bestPoint = null;
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
function pointContactDistance(point) {
  let best = null;
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
function contactColors(point) {
  if (point.startFinish) {
    return { dot: "#22c55e", line: "rgba(34, 197, 94, 0.45)" };
  }
  if (point.junction) {
    return { dot: "#f97316", line: "rgba(249, 115, 22, 0.45)" };
  }
  if (point.kind === "anchor") {
    return {
      dot: "rgba(148, 163, 184, 0.9)",
      line: "rgba(148, 163, 184, 0.35)"
    };
  }
  return {
    dot: "rgba(251, 191, 36, 0.85)",
    line: "rgba(251, 191, 36, 0.38)"
  };
}
function renderSelection() {
  selectionDetails.replaceChildren();
  pointEditorRows.replaceChildren();
  const addRow = (target, label, value) => {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = value;
    target.append(dt, dd);
  };
  const closePointEditor = () => {
    pointEditorId.textContent = "";
    if (pointEditor.open) {
      pointEditor.close();
    }
  };
  if (!selection) {
    closePointEditor();
    addRow(selectionDetails, "kind", "none");
    addRow(
      selectionDetails,
      "hint",
      "click a point, Alt-click a section, or click the road to insert"
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
      point.startFinish ? "start/finish" : point.junction ? "junction" : point.looseEnd ? "loose end" : "point"
    );
    const contact = pointContactDistance(point);
    addRow(
      pointEditorRows,
      "contact",
      contact === null ? "n/a" : contact < CONTACT_TOLERANCE ? "on curve" : `${contact.toFixed(1)} m`
    );
    addRow(pointEditorRows, "section degree", String(point.degree));
    addRow(pointEditorRows, "sections", point.sections.join(", ") || "none");
    if (!pointEditor.open) {
      pointEditor.show();
    }
    addRow(selectionDetails, "hint", `point ${point.id} selected`);
    return;
  }
  closePointEditor();
  const section = (project.sections ?? []).find(
    (item) => item.id === selection?.id
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
function refreshEditorState() {
  if (!editorViewer) {
    return;
  }
  project = JSON.parse(editorViewer.editor_project());
  activeLayoutId = editorViewer.editor_active_layout();
  const pointData = JSON.parse(
    editorViewer.editor_points()
  );
  const route = JSON.parse(editorViewer.editor_route());
  routeSectionIds = new Set(route.map((leg) => leg.sectionId));
  sectionCenterlines = JSON.parse(
    editorViewer.editor_section_centerlines()
  );
  const heights = pointHeights(project, sectionCenterlines);
  const activeLayout = (project.layouts ?? []).find(
    (layout) => layout.id === activeLayoutId
  );
  const startFinishId = activeLayout?.startFinish ?? "";
  editorPoints = pointData.filter(
    (point) => point.sections.some((id) => routeSectionIds.has(id))
  ).map((point) => ({
    id: point.id,
    position: [
      point.position[0],
      heights.get(point.id) ?? 0,
      point.position[1]
    ],
    kind: point.kind ?? "handle",
    junction: point.junction,
    end: point.end,
    degree: point.degree,
    sections: point.sections,
    startFinish: startFinishId !== "" ? point.id === startFinishId : point.startFinish,
    looseEnd: isLooseEndPoint(point)
  }));
  centerline = JSON.parse(editorViewer.editor_route_centerline());
  if (selection?.kind === "point" && !editorPoints.some((point) => point.id === selection?.id)) {
    selection = null;
  }
  if (selection?.kind === "section" && !(project.sections ?? []).some((section) => section.id === selection?.id)) {
    selection = null;
  }
  renderSelection();
  const debug = JSON.parse(editorViewer.debug_info());
  renderStats(debug);
  drawsLabel.textContent = String(debug.stats.drawCalls);
}
async function testDrive() {
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
pointEditorClose.addEventListener("click", () => {
  selection = null;
  renderSelection();
  drawControlPoints();
});
async function rebuildEditor(message, action, errorPrefix = "edit failed") {
  if (!editorViewer || rebuilding) {
    return;
  }
  rebuilding = true;
  suspended = true;
  testDriveButton.disabled = true;
  cameraController?.setEnabled(false);
  setStatus(message);
  try {
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
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
function waitForRebuild() {
  if (!rebuilding) {
    return Promise.resolve();
  }
  return new Promise((resolve) => rebuildWaiters.push(resolve));
}
async function insertPointAt(hit) {
  const section = (project.sections ?? []).find(
    (item) => item.id === hit.sectionId
  );
  if (!section || !editorViewer) {
    return;
  }
  const afterIndex = insertAfterIndex(section, hit.t);
  await rebuildEditor("inserting point", async () => {
    const pointId = await editorViewer.insert_editor_point(
      hit.sectionId,
      afterIndex,
      hit.position[0],
      hit.position[2]
    );
    selection = { kind: "point", id: pointId };
  });
}
async function deleteSelectedPoint() {
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
      await editorViewer.delete_editor_point(pointId);
      selection = null;
    },
    null
  );
}
async function setStartFinishPoint() {
  if (!editorViewer || selection?.kind !== "point" || rebuilding) {
    return;
  }
  const pointId = selection.id;
  await rebuildEditor("setting start/finish", async () => {
    await editorViewer.set_editor_start_finish(pointId);
  });
}
async function moveEditorPoint(point) {
  await rebuildEditor(
    "rebuilding track",
    () => editorViewer.move_editor_point(
      point.id,
      point.position[0],
      point.position[2]
    )
  );
}
function projectJson() {
  return editorViewer?.editor_project() ?? "{}";
}
function projectFilename() {
  const name = (project.name ?? "track-project").trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return `${name || "track-project"}.json`;
}
function downloadProject() {
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
function readStoredProject() {
  try {
    return localStorage.getItem(PROJECT_STORAGE_KEY);
  } catch {
    return null;
  }
}
function saveProject() {
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
async function loadProject(source, label) {
  if (!editorViewer || rebuilding) {
    return false;
  }
  rebuilding = true;
  suspended = true;
  testDriveButton.disabled = true;
  cameraController?.setEnabled(false);
  setStatus(label);
  try {
    await new Promise((resolve) => requestAnimationFrame(() => resolve()));
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
async function importProjectFile(file) {
  const source = await file.text();
  if (!source.trim()) {
    reportProjectError("import failed", "the selected file is empty");
    return;
  }
  await loadProject(source, "importing project");
}
function loadStoredProject() {
  const stored = readStoredProject();
  if (!stored) {
    reportProjectError("load failed", "no saved project in this browser");
    return;
  }
  void loadProject(stored, "loading project");
}
function resetProject() {
  if (!editorViewer) {
    return;
  }
  void loadProject(defaultProjectJson, "resetting project").then((loaded) => {
    if (loaded) {
      try {
        localStorage.removeItem(PROJECT_STORAGE_KEY);
      } catch {
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
    if (overlayVisible && carDragPointerId === null && hitTestCarMarker(x, y)) {
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
      }
      drawControlPoints();
      return;
    }
    const sectionHit = overlayVisible ? hitTestRouteSection(x, y) : null;
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
  const point = editorPoints[dragIndex];
  const startPosition = dragStartPosition;
  dragIndex = null;
  dragPointerId = null;
  dragStartPosition = null;
  try {
    canvas.releasePointerCapture(event.pointerId);
  } catch {
  }
  drawControlPoints();
  if (point && startPosition && (Math.abs(point.position[0] - startPosition[0]) > 1e-6 || Math.abs(point.position[1] - startPosition[1]) > 1e-6 || Math.abs(point.position[2] - startPosition[2]) > 1e-6)) {
    void moveEditorPoint(point);
  }
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
  drawControlPoints();
});
canvas.addEventListener("pointerleave", () => {
  lastPointer = null;
  canvas.style.cursor = "";
  drawControlPoints();
});
window.addEventListener("keydown", (event) => {
  const target = event.target;
  if (target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
    return;
  }
  if ((event.key === "Delete" || event.key === "Backspace") && selection?.kind === "point" && !event.repeat) {
    event.preventDefault();
    void deleteSelectedPoint();
    return;
  }
  if (event.key === "Escape") {
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
var endCameraDrag = () => {
  cameraDragging = false;
};
window.addEventListener("pointerup", endDrag, { capture: true });
window.addEventListener("pointercancel", endDrag, { capture: true });
window.addEventListener("pointerup", endCarDrag, { capture: true });
window.addEventListener("pointercancel", endCarDrag, { capture: true });
window.addEventListener("pointerup", endCameraDrag, { capture: true });
window.addEventListener("pointercancel", endCameraDrag, { capture: true });
function syncTreeDetailButton() {
  treeDetailButton.textContent = `Trees: ${TREE_DETAIL_LABELS[treeDetailMode]}`;
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
  setStatus("ready", "ready");
  window.__trackEditorReady = true;
  requestAnimationFrame(frame);
  requestAnimationFrame(hideLoading);
}
function fail(error) {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__trackEditorError = message;
  setStatus(message, "error");
  console.error(error);
}
boot().catch(fail);
export {
  PROJECT_SCHEMA,
  PROJECT_STORAGE_KEY
};
