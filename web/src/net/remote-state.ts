import { interpolateState } from "../sim/interpolate.ts";
import { NET_STATE_FLOATS } from "./protocol.ts";

const TICKS_PER_SECOND = 500;
const MS_PER_TICK = 1000 / TICKS_PER_SECOND;
// AI frames arrive every 4 sim ticks (8 ms at 500 Hz) and are drawn with a
// 60 ms interpolation delay. Six frames only gave 48 ms of history, which left
// almost no margin past the target and made the remote car hold/jump at frame
// boundaries. Keep 24 frames (192 ms) so the 60 ms target sits comfortably
// inside the window, while staying bounded relative to the ~250 ms clock-jump
// reset.
const MAX_HISTORY = 24;
const CLOCK_SMOOTHING = 0.08;
const JITTER_SMOOTHING = 0.1;
// A pause (for example the race countdown) leaves the remote tick clock stale
// while local time keeps moving. Smoothing across that gap freezes the remote
// car and then makes it lurch to catch up, so snap instead once the sample
// offset error is this large.
const MAX_CLOCK_ERROR_MS = 250;

interface TimedState {
  seq: number;
  tick: number;
  remoteTimeMs: number;
  state: Float32Array;
  receivedAt: number;
}

export class RemoteStateBuffer {
  private history: TimedState[] = [];
  private clockOffsetMs = 0;
  private hasClock = false;
  private jitterMs = 0;
  private alpha = 0;
  private targetMs = 0;

  private readonly interpolationMs: number;

  constructor(interpolationMs = 100) {
    this.interpolationMs = interpolationMs;
  }

  get depth(): number {
    return this.history.length;
  }

  get jitter(): number {
    return this.jitterMs;
  }

  get offsetMs(): number {
    return this.clockOffsetMs;
  }

  get interpolationAlpha(): number {
    return this.alpha;
  }

  get targetTimeMs(): number {
    return this.targetMs;
  }

  push(seq: number, tick: number, state: ArrayLike<number>, now: number): boolean {
    if (state.length !== NET_STATE_FLOATS || !Number.isFinite(tick) || tick < 0) {
      return false;
    }
    const latest = this.history[this.history.length - 1];
    if (latest) {
      if (tick < latest.tick) {
        this.history = [];
        this.hasClock = false;
        this.jitterMs = 0;
      } else if (tick === latest.tick) {
        if (seq <= latest.seq) {
          return false;
        }
        latest.seq = seq;
        latest.state = Float32Array.from(state);
        latest.receivedAt = now;
        return true;
      }
    }

    const remoteTimeMs = tick * MS_PER_TICK;
    const sampleOffset = now - remoteTimeMs;
    if (!this.hasClock) {
      this.clockOffsetMs = sampleOffset;
      this.hasClock = true;
    } else {
      const error = sampleOffset - this.clockOffsetMs;
      if (Math.abs(error) > MAX_CLOCK_ERROR_MS) {
        // Timeline discontinuity: adopt the new sample offset immediately and
        // drop the stale jitter estimate. Keep the frame history so the next
        // frame() can still interpolate across the boundary.
        this.clockOffsetMs = sampleOffset;
        this.jitterMs = 0;
      } else {
        this.jitterMs += (Math.abs(error) - this.jitterMs) * JITTER_SMOOTHING;
        this.clockOffsetMs += error * CLOCK_SMOOTHING;
      }
    }

    this.history.push({
      seq,
      tick,
      remoteTimeMs,
      state: Float32Array.from(state),
      receivedAt: now,
    });
    if (this.history.length > MAX_HISTORY) {
      this.history.splice(0, this.history.length - MAX_HISTORY);
    }
    return true;
  }

  frame(now: number): Float32Array | null {
    if (this.history.length === 0) {
      return null;
    }
    const target = now - this.clockOffsetMs - this.interpolationMs;
    this.targetMs = target;
    const oldest = this.history[0];
    const newest = this.history[this.history.length - 1];
    if (target <= oldest.remoteTimeMs) {
      this.alpha = 0;
      return oldest.state;
    }
    if (target >= newest.remoteTimeMs) {
      this.alpha = 1;
      return newest.state;
    }
    for (let index = this.history.length - 1; index > 0; index -= 1) {
      const current = this.history[index];
      const previous = this.history[index - 1];
      if (target >= previous.remoteTimeMs && target <= current.remoteTimeMs) {
        const span = current.remoteTimeMs - previous.remoteTimeMs;
        const alpha = span > 0 ? (target - previous.remoteTimeMs) / span : 1;
        this.alpha = alpha;
        return interpolateState(previous.state, current.state, alpha);
      }
    }
    this.alpha = 1;
    return newest.state;
  }

  age(now: number): number {
    const newest = this.history[this.history.length - 1];
    return newest ? now - newest.receivedAt : Number.POSITIVE_INFINITY;
  }
}
