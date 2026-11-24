import { smoothAiPose } from "./ai-smoothing.ts";

/**
 * Temporary AI stutter diagnostics.
 *
 * `?aiNoSmooth=1` skips the pose filter and draws the interpolated buffer
 * frame. `?aiRaw=1` skips both the interpolation buffer and the pose filter and
 * draws the latest raw worker frame. `aiRaw` wins when both flags are set.
 */
export type AiRenderSource = "smooth" | "interpolated" | "raw";

export function aiRenderSource(
  noSmooth: boolean,
  raw: boolean,
): AiRenderSource {
  if (raw) {
    return "raw";
  }
  if (noSmooth) {
    return "interpolated";
  }
  return "smooth";
}

export interface AiRenderFrame {
  /** Buffer or raw frame the pose was derived from, before filtering. */
  source: Float32Array;
  /** Frame handed to the viewer; safe for the caller to mutate in place. */
  drawn: Float32Array;
}

/**
 * Select and prepare the frame the viewer should draw for one AI slot.
 *
 * The drawn frame is always a fresh copy, so freezing wheels or viewer writes
 * never mutate the interpolation history or the stored raw frame. Returns null
 * when the selected source has no frame yet.
 */
export function resolveAiRenderFrame(
  source: AiRenderSource,
  bufferFrame: Float32Array | null,
  rawFrame: Float32Array | null,
  previousSmoothed: Float32Array | null,
  dtSeconds: number,
): AiRenderFrame | null {
  const base = source === "raw" ? rawFrame : bufferFrame;
  if (!base) {
    return null;
  }
  if (source === "smooth") {
    return {
      source: base,
      drawn: smoothAiPose(previousSmoothed, base, dtSeconds),
    };
  }
  return { source: base, drawn: Float32Array.from(base) };
}
