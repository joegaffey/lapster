/** Pure distance and lite-voice gain helpers for remote engine audio. */

export const REMOTE_REFERENCE_DISTANCE = 25;
export const REMOTE_MAX_DISTANCE = 250;
// Keep AI engines audible under the player engine without dominating the mix.
const REMOTE_MASTER_GAIN = 0.35;

/** Distance falloff for a remote car; zero at or beyond the audible radius. */
export function distanceGain(distance: number): number {
  if (!Number.isFinite(distance) || distance >= REMOTE_MAX_DISTANCE) {
    return 0;
  }
  const falloff = Math.max(0, 1 - distance / REMOTE_MAX_DISTANCE);
  const inverse = 1 / (1 + distance / REMOTE_REFERENCE_DISTANCE);
  return REMOTE_MASTER_GAIN * inverse * falloff;
}

/**
 * Gain for a lite oscillator voice. `base` is the distance gain; throttle load
 * and rpm lift it, then it is clamped so close cars never overpower the player.
 */
export function liteGain(base: number, rpm: number, throttle: number): number {
  const load = Math.max(0, Math.min(1, throttle));
  const rpmFactor = Math.min(1, Math.max(0, rpm) / 9_000);
  return Math.min(1, base * (0.55 + load * 0.45 + rpmFactor * 0.25));
}
