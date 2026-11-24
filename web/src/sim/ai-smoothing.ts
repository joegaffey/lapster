import { SNAPSHOT_STATE } from "../types/asset-layout.ts";

const DEFAULT_TIME_CONSTANT_SECONDS = 0.02;
// Clamp the render delta so a paused tab or a hitch does not blend a whole
// frame's worth of motion at once. The low bound keeps the filter advancing on
// zero/invalid deltas instead of freezing the AI pose.
const MIN_DT_SECONDS = 0.001;
const MAX_DT_SECONDS = 0.25;

/** Stable neutral suspension travel used by the `?aiNoWheels=1` diagnostic. */
export const AI_FROZEN_SUSPENSION_TRAVEL = 0.05;

/**
 * Freeze the wheel/suspension fields of an AI render frame in place.
 *
 * Temporary `?aiNoWheels=1` diagnostic: isolates whether per-frame suspension
 * and wheel motion is the cause of AI-only visual stutter. Position, rotation,
 * velocity and every other field are left untouched, so the body pose still
 * follows the filtered frame. The suspension constant and zero steer/spin are
 * stable across frames so they cannot jitter.
 */
export function freezeAiWheels(
  frame: Float32Array,
  suspensionTravel = AI_FROZEN_SUSPENSION_TRAVEL,
): Float32Array {
  for (let wheel = 0; wheel < 4; wheel += 1) {
    frame[SNAPSHOT_STATE.suspensionTravel + wheel] = suspensionTravel;
    frame[SNAPSHOT_STATE.wheelSteerAngle + wheel] = 0;
    frame[SNAPSHOT_STATE.wheelSpinAngle + wheel] = 0;
  }
  return frame;
}

/**
 * Time-based low-pass filter for the AI render pose.
 *
 * The 500 Hz physics/AI rate is unchanged; this only smooths the pose the
 * viewer draws, on top of the interpolation delay. Position is exponentially
 * blended toward the incoming frame and the rotation quaternion is nlerped
 * (with sign correction) so consecutive frames never take the long way round.
 * Every non-pose field is taken verbatim from the incoming frame.
 */
export function smoothAiPose(
  previous: Float32Array | null,
  frame: Float32Array,
  dtSeconds: number,
  timeConstantSeconds = DEFAULT_TIME_CONSTANT_SECONDS,
): Float32Array {
  const output = Float32Array.from(frame);
  if (!previous || previous.length !== frame.length) {
    return output;
  }

  const tau =
    Number.isFinite(timeConstantSeconds) && timeConstantSeconds > 0
      ? timeConstantSeconds
      : DEFAULT_TIME_CONSTANT_SECONDS;
  const sanitizedDt = Number.isFinite(dtSeconds)
    ? Math.min(MAX_DT_SECONDS, Math.max(MIN_DT_SECONDS, dtSeconds))
    : MIN_DT_SECONDS;
  const factor = 1 - Math.exp(-sanitizedDt / tau);

  const positionStart = SNAPSHOT_STATE.position;
  for (let index = positionStart; index < positionStart + 3; index += 1) {
    output[index] = previous[index] + (frame[index] - previous[index]) * factor;
  }

  nlerpQuaternion(previous, frame, factor, output);
  return output;
}

function nlerpQuaternion(
  previous: Float32Array,
  frame: Float32Array,
  factor: number,
  output: Float32Array,
): void {
  const offset = SNAPSHOT_STATE.rotation;
  const ax = previous[offset];
  const ay = previous[offset + 1];
  const az = previous[offset + 2];
  const aw = previous[offset + 3];
  let bx = frame[offset];
  let by = frame[offset + 1];
  let bz = frame[offset + 2];
  let bw = frame[offset + 3];

  // Quaternions q and -q encode the same rotation. Flip the incoming frame when
  // it points away from the previous one so the blend stays on the short arc.
  const dot = ax * bx + ay * by + az * bz + aw * bw;
  if (dot < 0) {
    bx = -bx;
    by = -by;
    bz = -bz;
    bw = -bw;
  }

  let x = ax + (bx - ax) * factor;
  let y = ay + (by - ay) * factor;
  let z = az + (bz - az) * factor;
  let w = aw + (bw - aw) * factor;
  const length = Math.hypot(x, y, z, w);
  if (length > 1e-6) {
    x /= length;
    y /= length;
    z /= length;
    w /= length;
  } else {
    x = bx;
    y = by;
    z = bz;
    w = bw;
  }
  output[offset] = x;
  output[offset + 1] = y;
  output[offset + 2] = z;
  output[offset + 3] = w;
}
