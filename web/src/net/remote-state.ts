import { interpolateState } from "../sim/interpolate.ts";
import { NET_STATE_FLOATS } from "./protocol.ts";

interface TimedState {
  seq: number;
  tick: number;
  state: Float32Array;
  receivedAt: number;
}

export class RemoteStateBuffer {
  private previous: TimedState | null = null;
  private current: TimedState | null = null;

  private readonly interpolationMs: number;

  constructor(interpolationMs = 80) {
    this.interpolationMs = interpolationMs;
  }

  push(seq: number, tick: number, state: number[], now: number): boolean {
    if (state.length !== NET_STATE_FLOATS) {
      return false;
    }
    if (this.current && tick <= this.current.tick && seq <= this.current.seq) {
      return false;
    }
    this.previous = this.current;
    this.current = {
      seq,
      tick,
      state: Float32Array.from(state),
      receivedAt: now,
    };
    return true;
  }

  frame(now: number): Float32Array | null {
    if (!this.current) {
      return null;
    }
    if (!this.previous) {
      return this.current.state;
    }
    const renderTime = now - this.interpolationMs;
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

  age(now: number): number {
    return this.current ? now - this.current.receivedAt : Number.POSITIVE_INFINITY;
  }
}
