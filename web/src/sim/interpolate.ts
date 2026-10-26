import { SNAPSHOT_STATE } from "../types/asset-layout.ts";
import type { Snapshot, SnapshotReader } from "./sab.ts";

interface TimedSnapshot {
  tick: number;
  state: Float32Array;
  receivedAt: number;
}

export class SnapshotSmoother {
  private previous: TimedSnapshot | null = null;
  private current: TimedSnapshot | null = null;

  constructor(private readonly reader: SnapshotReader) {}

  update(now: number): boolean {
    const snapshot = this.reader.readLatest();
    if (!snapshot) {
      return false;
    }
    if (this.current?.tick === snapshot.tick) {
      return false;
    }
    this.previous = this.current;
    this.current = {
      tick: snapshot.tick,
      state: new Float32Array(snapshot.state),
      receivedAt: now,
    };
    return true;
  }

  state(now: number): Float32Array | null {
    if (!this.current) {
      return null;
    }
    if (!this.previous) {
      return this.current.state;
    }
    const renderTime = now - 2;
    if (renderTime <= this.previous.receivedAt) {
      return this.previous.state;
    }
    if (renderTime >= this.current.receivedAt) {
      return this.current.state;
    }
    const span = this.current.receivedAt - this.previous.receivedAt;
    if (span <= 0) {
      return this.current.state;
    }
    const alpha = (renderTime - this.previous.receivedAt) / span;
    return interpolateState(this.previous.state, this.current.state, alpha);
  }

  latest(): Float32Array | null {
    return this.current?.state ?? null;
  }
}

function lerp(a: number, b: number, alpha: number): number {
  return a + (b - a) * alpha;
}

function interpolateState(previous: Float32Array, current: Float32Array, alpha: number): Float32Array {
  const output = new Float32Array(current.length);
  output.set(current);

  for (let index = SNAPSHOT_STATE.position; index < SNAPSHOT_STATE.rotation; index += 1) {
    output[index] = lerp(previous[index], current[index], alpha);
  }
  slerpQuaternion(previous, current, alpha, output);

  const groups: Array<[number, number]> = [
    [SNAPSHOT_STATE.velocity, 3],
    [SNAPSHOT_STATE.angularVelocity, 3],
    [SNAPSHOT_STATE.wheelAngularSpeeds, 4],
    [SNAPSHOT_STATE.suspensionTravel, 4],
    [SNAPSHOT_STATE.slipRatio, 4],
    [SNAPSHOT_STATE.slipAngle, 4],
    [SNAPSHOT_STATE.tireLoad, 4],
    [SNAPSHOT_STATE.surfaceFriction, 4],
    [SNAPSHOT_STATE.wheelSteerAngle, 4],
    [SNAPSHOT_STATE.wheelSpinAngle, 4],
    [SNAPSHOT_STATE.contactCenter, 3],
    [SNAPSHOT_STATE.contactNormal, 3],
  ];
  for (const [start, length] of groups) {
    for (let index = 0; index < length; index += 1) {
      output[start + index] = lerp(previous[start + index], current[start + index], alpha);
    }
  }

  output[SNAPSHOT_STATE.engineRpm] = lerp(
    previous[SNAPSHOT_STATE.engineRpm],
    current[SNAPSHOT_STATE.engineRpm],
    alpha,
  );
  output[SNAPSHOT_STATE.rackTorque] = lerp(
    previous[SNAPSHOT_STATE.rackTorque],
    current[SNAPSHOT_STATE.rackTorque],
    alpha,
  );
  output[SNAPSHOT_STATE.throttle] = lerp(
    previous[SNAPSHOT_STATE.throttle],
    current[SNAPSHOT_STATE.throttle],
    alpha,
  );
  output[SNAPSHOT_STATE.brake] = lerp(
    previous[SNAPSHOT_STATE.brake],
    current[SNAPSHOT_STATE.brake],
    alpha,
  );
  output[SNAPSHOT_STATE.absActive] = lerp(
    previous[SNAPSHOT_STATE.absActive],
    current[SNAPSHOT_STATE.absActive],
    alpha,
  );
  output[SNAPSHOT_STATE.escActivity] = lerp(
    previous[SNAPSHOT_STATE.escActivity],
    current[SNAPSHOT_STATE.escActivity],
    alpha,
  );
  return output;
}

function slerpQuaternion(
  previous: Float32Array,
  current: Float32Array,
  alpha: number,
  output: Float32Array,
): void {
  const offset = SNAPSHOT_STATE.rotation;
  let ax = previous[offset];
  let ay = previous[offset + 1];
  let az = previous[offset + 2];
  let aw = previous[offset + 3];
  const bx = current[offset];
  const by = current[offset + 1];
  const bz = current[offset + 2];
  const bw = current[offset + 3];
  let cosine = ax * bx + ay * by + az * bz + aw * bw;
  if (cosine < 0) {
    ax = -ax;
    ay = -ay;
    az = -az;
    aw = -aw;
    cosine = -cosine;
  }
  if (cosine > 0.9995) {
    output[offset] = lerp(ax, bx, alpha);
    output[offset + 1] = lerp(ay, by, alpha);
    output[offset + 2] = lerp(az, bz, alpha);
    output[offset + 3] = lerp(aw, bw, alpha);
    normalizeQuaternion(output, offset);
    return;
  }
  const theta = Math.acos(Math.min(1, Math.max(-1, cosine)));
  const sinTheta = Math.sin(theta);
  const scaleA = Math.sin((1 - alpha) * theta) / sinTheta;
  const scaleB = Math.sin(alpha * theta) / sinTheta;
  output[offset] = ax * scaleA + bx * scaleB;
  output[offset + 1] = ay * scaleA + by * scaleB;
  output[offset + 2] = az * scaleA + bz * scaleB;
  output[offset + 3] = aw * scaleA + bw * scaleB;
  normalizeQuaternion(output, offset);
}

function normalizeQuaternion(state: Float32Array, offset: number): void {
  const length = Math.hypot(
    state[offset],
    state[offset + 1],
    state[offset + 2],
    state[offset + 3],
  );
  if (length > 1e-6) {
    state[offset] /= length;
    state[offset + 1] /= length;
    state[offset + 2] /= length;
    state[offset + 3] /= length;
  }
}
