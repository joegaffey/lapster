export interface TrackOption {
  id: string;
  name: string;
  url: string;
}

export const TRACK_OPTIONS: TrackOption[] = [
  {
    id: "skidpad05",
    name: "Skidpad 0.5",
    url: "../../fixtures/tracks/skidpad05/pack.sima",
  },
  {
    id: "drift-sketchfab",
    name: "Drift Tracks",
    url: "../../fixtures/tracks/drift-sketchfab/pack.sima",
  },
];

export const DEFAULT_TRACK = TRACK_OPTIONS[0];
export const TRACK_STORAGE_KEY = "lapster.track.v1";

export function findTrack(id: string | null | undefined): TrackOption | null {
  return TRACK_OPTIONS.find((track) => track.id === id) ?? null;
}

export function loadSelectedTrack(): TrackOption {
  try {
    return findTrack(localStorage.getItem(TRACK_STORAGE_KEY)) ?? DEFAULT_TRACK;
  } catch {
    return DEFAULT_TRACK;
  }
}

export function saveSelectedTrack(id: string): TrackOption {
  const track = findTrack(id) ?? DEFAULT_TRACK;
  try {
    localStorage.setItem(TRACK_STORAGE_KEY, track.id);
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
  return track;
}
