import { SNAPSHOT_STATE } from "../types/asset-layout.ts";

/**
 * Temporary `?aiDebug=1` diagnostics. These helpers only sample data for the
 * `window.__aiDebug` history; they never affect simulation or rendering.
 */

export const AI_DEBUG_HISTORY_LIMIT = 300;

export type Vec3 = [number, number, number];
export type Vec4 = [number, number, number, number];

/** Pose and wheel fields shared by the raw and rendered sample streams. */
export interface AiPoseSample {
  /** Rotation quaternion (x, y, z, w). */
  rotation: Vec4;
  /** Angular velocity (x, y, z), rad/s. */
  angularVelocity: Vec3;
  /** Suspension travel per wheel, metres. */
  suspensionTravel: Vec4;
  /** Wheel steer angle per wheel, radians. */
  wheelSteerAngle: Vec4;
  /** Wheel spin angle per wheel, radians. */
  wheelSpinAngle: Vec4;
  /** Heading around the world Y axis, derived from the rotation quaternion. */
  yaw: number;
}

export interface AiRawSample extends AiPoseSample {
  /** performance.now() at which the worker frame arrived. */
  t: number;
  slot: number;
  tick: number;
  position: Vec3;
  /** Horizontal speed from the frame velocity x/z, m/s. */
  speed: number;
}

export interface AiRenderMetrics {
  alpha: number;
  depth: number;
  jitter: number;
  offsetMs: number;
  age: number;
}

export interface AiRenderSample extends AiRenderMetrics, AiPoseSample {
  /** performance.now() of the render frame. */
  t: number;
  slot: number;
  tick: number;
  /** Interpolated pose position before the pose filter. */
  position: Vec3;
  /** Pose-filtered position actually handed to the viewer. */
  smoothed: Vec3;
}

export interface AiDebugHistory {
  readonly limit: number;
  readonly raw: AiRawSample[];
  readonly rendered: AiRenderSample[];
}

export function createAiDebugHistory(
  limit = AI_DEBUG_HISTORY_LIMIT,
): AiDebugHistory {
  return { limit, raw: [], rendered: [] };
}

function vec3At(values: ArrayLike<number>, start: number): Vec3 {
  return [values[start], values[start + 1], values[start + 2]];
}

function vec4At(values: ArrayLike<number>, start: number): Vec4 {
  return [
    values[start],
    values[start + 1],
    values[start + 2],
    values[start + 3],
  ];
}

/**
 * Yaw (rotation about the world Y axis) in radians from a quaternion. The
 * result is unchanged when the quaternion is negated (q and -q encode the same
 * rotation).
 */
export function quaternionYaw(
  x: number,
  y: number,
  z: number,
  w: number,
): number {
  return Math.atan2(2 * (w * y + x * z), 1 - 2 * (y * y + x * x));
}

function samplePose(values: ArrayLike<number>): AiPoseSample {
  const rotation = vec4At(values, SNAPSHOT_STATE.rotation);
  return {
    rotation,
    angularVelocity: vec3At(values, SNAPSHOT_STATE.angularVelocity),
    suspensionTravel: vec4At(values, SNAPSHOT_STATE.suspensionTravel),
    wheelSteerAngle: vec4At(values, SNAPSHOT_STATE.wheelSteerAngle),
    wheelSpinAngle: vec4At(values, SNAPSHOT_STATE.wheelSpinAngle),
    yaw: quaternionYaw(rotation[0], rotation[1], rotation[2], rotation[3]),
  };
}

export function sampleRawAiFrame(
  slot: number,
  frame: ArrayLike<number>,
  t: number,
  tick: number,
): AiRawSample {
  const position = SNAPSHOT_STATE.position;
  const velocity = SNAPSHOT_STATE.velocity;
  return {
    t,
    slot,
    tick,
    position: vec3At(frame, position),
    speed: Math.hypot(frame[velocity], frame[velocity + 2]),
    ...samplePose(frame),
  };
}

export function sampleRenderedAiPose(
  slot: number,
  frame: ArrayLike<number>,
  smoothed: ArrayLike<number> | null,
  t: number,
  tick: number,
  metrics: AiRenderMetrics,
): AiRenderSample {
  const position = SNAPSHOT_STATE.position;
  const rendered = smoothed ?? frame;
  return {
    t,
    slot,
    tick,
    position: vec3At(frame, position),
    smoothed: vec3At(rendered, position),
    alpha: metrics.alpha,
    depth: metrics.depth,
    jitter: metrics.jitter,
    offsetMs: metrics.offsetMs,
    age: metrics.age,
    ...samplePose(rendered),
  };
}

/** Append a sample and drop the oldest entries beyond `limit`. */
export function pushBounded<T>(history: T[], sample: T, limit: number): void {
  history.push(sample);
  if (history.length > limit) {
    history.splice(0, history.length - limit);
  }
}
