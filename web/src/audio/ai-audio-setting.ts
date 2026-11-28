/** Pure helpers for the persisted AI/opponent audio voice setting. */

export const AI_VOICE_STORAGE_KEY = "lapster.audio.aiVoices.v1";
export const AI_VOICE_MIN = 0;
export const AI_VOICE_MAX = 5;
export const AI_VOICE_DEFAULT = 3;
/** Number of plain oscillator voices used by the `?aiAudio=lite` diagnostic. */
export const AI_LITE_VOICES = 3;

export type AiAudioMode = "off" | "nearest" | "full" | "lite";

export interface AiAudioResolution {
  mode: AiAudioMode;
  voices: number;
}

/** Minimal storage surface so tests can pass a plain in-memory map. */
export interface AiVoiceStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function defaultStorage(): AiVoiceStorage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

/** Clamp any input to a whole number of opponent voices in 0..5. */
export function clampAiVoices(value: unknown): number {
  if (value === null || value === undefined) {
    return AI_VOICE_DEFAULT;
  }
  const numeric =
    typeof value === "number"
      ? value
      : typeof value === "string" && value.trim() !== ""
        ? Number(value)
        : Number.NaN;
  if (!Number.isFinite(numeric)) {
    return AI_VOICE_DEFAULT;
  }
  return Math.min(AI_VOICE_MAX, Math.max(AI_VOICE_MIN, Math.round(numeric)));
}

/** Map a stored voice count to the effective audio mode and voice cap. */
export function aiAudioFromVoices(value: unknown): AiAudioResolution {
  const voices = clampAiVoices(value);
  if (voices === 0) {
    return { mode: "off", voices: 0 };
  }
  if (voices === 1) {
    return { mode: "nearest", voices: 1 };
  }
  return { mode: "full", voices };
}

/** Load the persisted voice count, defaulting to three opposing cars. */
export function loadAiVoices(storage?: AiVoiceStorage | null): number {
  try {
    const target = storage ?? defaultStorage();
    const raw = target?.getItem(AI_VOICE_STORAGE_KEY);
    return raw === null || raw === undefined
      ? AI_VOICE_DEFAULT
      : clampAiVoices(raw);
  } catch {
    return AI_VOICE_DEFAULT;
  }
}

/** Persist the clamped voice count and return the value that was stored. */
export function saveAiVoices(
  value: unknown,
  storage?: AiVoiceStorage | null,
): number {
  const voices = clampAiVoices(value);
  try {
    const target = storage ?? defaultStorage();
    target?.setItem(AI_VOICE_STORAGE_KEY, String(voices));
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
  return voices;
}

function readOverride(
  input: string | URLSearchParams | null | undefined,
): string | null {
  if (input === null || input === undefined) {
    return null;
  }
  if (typeof input === "string") {
    const query = input.startsWith("?") ? input.slice(1) : input;
    return new URLSearchParams(query).get("aiAudio");
  }
  return input.get("aiAudio");
}

/**
 * Parse the `?aiAudio` diagnostic override. Returns null when the query does
 * not request a supported mode so the persisted setting stays in effect.
 */
export function parseAiAudioOverride(
  input: string | URLSearchParams | null | undefined,
): AiAudioResolution | null {
  switch (readOverride(input)) {
    case "off":
      return { mode: "off", voices: 0 };
    case "nearest":
      return { mode: "nearest", voices: 1 };
    case "full":
      return { mode: "full", voices: AI_VOICE_MAX };
    case "lite":
      return { mode: "lite", voices: AI_LITE_VOICES };
    default:
      return null;
  }
}
