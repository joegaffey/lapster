const NAME_STORAGE_KEY = "lapster.net.name.v1";
const PASSWORD_STORAGE_KEY = "lapster.net.password.v1";
const ROOM_ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

export function makeRoomCode(length = 6): string {
  const bytes = new Uint8Array(length);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  return Array.from(bytes, (value) => ROOM_ALPHABET[value % ROOM_ALPHABET.length]).join("");
}

export function normalizeRoomCode(value: string | null | undefined): string | null {
  const code = value?.trim().toUpperCase() ?? "";
  if (!code || code.length > 32 || !/^[A-Z0-9_-]+$/.test(code)) {
    return null;
  }
  return code;
}

export function loadPlayerName(): string {
  try {
    return localStorage.getItem(NAME_STORAGE_KEY)?.trim() || "";
  } catch {
    return "";
  }
}

export function savePlayerName(name: string): void {
  try {
    localStorage.setItem(NAME_STORAGE_KEY, name.trim());
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

export function loadRoomPassword(): string {
  try {
    return sessionStorage.getItem(PASSWORD_STORAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

export function saveRoomPassword(password: string): void {
  try {
    if (password) {
      sessionStorage.setItem(PASSWORD_STORAGE_KEY, password);
    } else {
      sessionStorage.removeItem(PASSWORD_STORAGE_KEY);
    }
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

export function lobbyInviteUrl(
  baseHref: string,
  room: string,
  car: string,
  track: string,
  relayUrl?: string | null,
): string {
  const url = new URL(baseHref);
  url.search = "";
  url.hash = "";
  url.searchParams.set("room", room);
  url.searchParams.set("car", car);
  url.searchParams.set("track", track);
  if (relayUrl) {
    url.searchParams.set("ws", relayUrl);
  }
  return url.toString();
}

export function driveRoomUrl(
  baseHref: string,
  room: string,
  car: string,
  track: string,
  relayUrl?: string | null,
): string {
  const url = new URL(baseHref);
  url.search = "";
  url.hash = "";
  url.searchParams.set("mp", "1");
  url.searchParams.set("room", room);
  url.searchParams.set("car", car);
  url.searchParams.set("track", track);
  if (relayUrl) {
    url.searchParams.set("ws", relayUrl);
  }
  return url.toString();
}
