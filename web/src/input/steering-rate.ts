/**
 * Moves a normalized steering command toward a target at a bounded rate.
 *
 * `maxRate` is in steering units per second and `dt` is the elapsed time in
 * seconds. A non-positive (or non-finite) rate applies the target directly,
 * which is how `0` disables the limit. The result is clamped to `-1..1`.
 */
export function rateLimitSteering(
  current: number,
  target: number,
  maxRate: number,
  dt: number,
): number {
  const from = Number.isFinite(current) ? current : 0;
  const to = Number.isFinite(target) ? Math.max(-1, Math.min(1, target)) : 0;
  const elapsed = Number.isFinite(dt) && dt > 0 ? dt : 0;
  if (!Number.isFinite(maxRate) || maxRate <= 0) {
    return to;
  }
  const maxDelta = maxRate * elapsed;
  const delta = Math.max(-maxDelta, Math.min(maxDelta, to - from));
  return Math.max(-1, Math.min(1, from + delta));
}
