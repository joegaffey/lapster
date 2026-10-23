type Vec3 = [number, number, number];

interface CameraInfo {
  eye: number[];
  target: number[];
  fovY: number;
}

interface CameraViewer {
  camera_info(): string;
  set_camera(
    eyeX: number,
    eyeY: number,
    eyeZ: number,
    targetX: number,
    targetY: number,
    targetZ: number,
  ): void;
  reset_camera(): void;
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const add = (a: Vec3, b: Vec3): Vec3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (v: Vec3, amount: number): Vec3 => [v[0] * amount, v[1] * amount, v[2] * amount];
const length = (v: Vec3) => Math.hypot(v[0], v[1], v[2]);
const normalize = (v: Vec3): Vec3 => {
  const len = length(v);
  return len > 1e-8 ? [v[0] / len, v[1] / len, v[2] / len] : [0, 0, 0];
};
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const toVec3 = (value: number[]): Vec3 => [value[0] ?? 0, value[1] ?? 0, value[2] ?? 0];

export function setupTrackCamera(canvas: HTMLCanvasElement, viewer: CameraViewer) {
  let info: CameraInfo;
  try {
    info = JSON.parse(viewer.camera_info()) as CameraInfo;
  } catch {
    return;
  }

  const initialEye = toVec3(info.eye);
  const initialTarget = toVec3(info.target);
  let eye = [...initialEye] as Vec3;
  let target = [...initialTarget] as Vec3;
  let mode: "orbit" | "pan" = "orbit";
  const pointers = new Map<number, { x: number; y: number }>();
  let pinch: { distance: number; centroid: [number, number] } | null = null;

  const apply = () => {
    viewer.set_camera(eye[0], eye[1], eye[2], target[0], target[1], target[2]);
  };

  const orbit = (dx: number, dy: number) => {
    const offset = sub(eye, target);
    const radius = length(offset);
    if (radius < 1e-4) {
      return;
    }
    const yaw = Math.atan2(offset[0], offset[2]) - dx * 0.006;
    const pitch = clamp(Math.asin(clamp(offset[1] / radius, -1, 1)) + dy * 0.006, -1.45, 1.45);
    eye = [
      target[0] + Math.sin(yaw) * Math.cos(pitch) * radius,
      target[1] + Math.sin(pitch) * radius,
      target[2] + Math.cos(yaw) * Math.cos(pitch) * radius,
    ];
  };

  const pan = (dx: number, dy: number) => {
    const forward = normalize(sub(target, eye));
    let right = normalize(cross(forward, [0, 1, 0]));
    if (length(right) < 1e-4) {
      right = [1, 0, 0];
    }
    const up = normalize(cross(right, forward));
    const radius = Math.max(length(sub(eye, target)), 0.01);
    const worldPerPixel =
      (2 * radius * Math.tan(info.fovY * 0.5)) / Math.max(canvas.clientHeight, 1);
    const offset = add(scale(right, -dx * worldPerPixel), scale(up, dy * worldPerPixel));
    eye = add(eye, offset);
    target = add(target, offset);
  };

  const zoomBy = (factor: number) => {
    const offset = sub(eye, target);
    const radius = clamp(length(offset) * factor, 0.5, 5000);
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
      centroid: [(first.x + second.x) * 0.5, (first.y + second.y) * 0.5] as [number, number],
    };
  };

  canvas.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    try {
      canvas.setPointerCapture(event.pointerId);
    } catch {
      // Synthetic or already-released pointers cannot be captured.
    }
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    mode = event.button === 2 || event.shiftKey ? "pan" : "orbit";
    if (pointers.size === 2) {
      pinch = pinchState();
    }
    canvas.classList.add("dragging");
  });

  canvas.addEventListener("pointermove", (event) => {
    const previous = pointers.get(event.pointerId);
    if (!previous) {
      return;
    }
    const next = { x: event.clientX, y: event.clientY };
    pointers.set(event.pointerId, next);

    if (pointers.size >= 2) {
      const state = pinchState();
      if (state && pinch) {
        pan(state.centroid[0] - pinch.centroid[0], state.centroid[1] - pinch.centroid[1]);
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

  const endPointer = (event: PointerEvent) => {
    pointers.delete(event.pointerId);
    pinch = null;
    if (pointers.size === 0) {
      canvas.classList.remove("dragging");
    }
  };
  canvas.addEventListener("pointerup", endPointer);
  canvas.addEventListener("pointercancel", endPointer);
  canvas.addEventListener("lostpointercapture", endPointer);
  canvas.addEventListener("contextmenu", (event) => event.preventDefault());

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      zoomBy(Math.exp(event.deltaY * 0.001));
      apply();
    },
    { passive: false },
  );

  document.querySelector<HTMLButtonElement>("#reset-camera")?.addEventListener("click", () => {
    eye = [...initialEye] as Vec3;
    target = [...initialTarget] as Vec3;
    viewer.reset_camera();
  });
}
