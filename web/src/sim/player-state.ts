import { RemoteStateBuffer } from "../net/remote-state.ts";
import type { Snapshot } from "./sab.ts";

// The local player is simulated at 500 Hz (2 ms per tick). Worker catch-up
// makes the number of ticks published between display polls vary, so driving
// the pose from arrival time makes its speed spike frame to frame. Render on
// the worker tick timeline 24 ms (12 ticks) behind the newest tick instead:
// enough to cover the remaining poll gap, while minimising input lag. This is
// the single delay for every camera view (chase, isometric, bonnet, cockpit);
// do not add per-view delays. It mirrors the remote/AI RemoteStateBuffer design.
export const PLAYER_INTERPOLATION_MS = 24;

/**
 * Tick-timeline history for the local player pose.
 *
 * Reads are pushed with `seq = tick` (the local stream has a single producer),
 * then sampled with `frame(renderNow)`. `latest()` always returns the newest
 * raw sample, for teleports and one-shot `simulate()`/`reset()` updates that
 * must not draw a delayed pose.
 */
export class PlayerStateBuffer {
  private readonly buffer: RemoteStateBuffer;
  private newest: Float32Array | null = null;

  constructor(interpolationMs = PLAYER_INTERPOLATION_MS) {
    this.buffer = new RemoteStateBuffer(interpolationMs);
  }

  get depth(): number {
    return this.buffer.depth;
  }

  get interpolationAlpha(): number {
    return this.buffer.interpolationAlpha;
  }

  get targetTimeMs(): number {
    return this.buffer.targetTimeMs;
  }

  get offsetMs(): number {
    return this.buffer.offsetMs;
  }

  push(snapshot: Pick<Snapshot, "tick" | "state">, now: number): boolean {
    const accepted = this.buffer.push(
      snapshot.tick,
      snapshot.tick,
      snapshot.state,
      now,
    );
    if (accepted) {
      this.newest = Float32Array.from(snapshot.state);
    }
    return accepted;
  }

  frame(now: number): Float32Array | null {
    return this.buffer.frame(now);
  }

  /** Newest raw sample, independent of the interpolation delay. */
  latest(): Float32Array | null {
    return this.newest;
  }

  /** Drop all history, for resets and teleports onto a new tick timeline. */
  clear(): void {
    this.buffer.clear();
    this.newest = null;
  }
}
