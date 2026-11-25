/**
 * Resolve the `?spectate=1..9` flag to a valid AI camera slot.
 *
 * Returns `null` when spectating is not requested or unavailable: the flag is
 * absent, there are no AI cars, or the session is multiplayer or solo. A
 * present flag without a usable number defaults to slot 1, and every value is
 * clamped to `1..=aiCount` so the render API never receives a local or empty
 * slot.
 */
export function resolveSpectatorSlot(
  raw: string | null,
  aiCount: number,
  multiplayer: boolean,
  solo: boolean,
): number | null {
  if (raw === null || multiplayer || solo) {
    return null;
  }
  if (!Number.isFinite(aiCount) || aiCount < 1) {
    return null;
  }
  const parsed = Number(raw);
  const requested = Number.isFinite(parsed) ? Math.trunc(parsed) : 1;
  return Math.min(Math.max(1, requested), Math.trunc(aiCount));
}
