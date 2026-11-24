/**
 * Car-to-car contact event parsing and validation.
 *
 * The physics worker publishes `{ type: "contacts", contacts: [...] }` where
 * each entry has the shape below. Player is slot 0; AI slots are 1..n. Events
 * are local feedback only and never change race state.
 */
export interface ContactEvent {
  a: number;
  b: number;
  point: [number, number, number];
  normal: [number, number, number];
  speed: number;
  tick: number;
}

/** World-space radius within which an AI-to-AI contact still gets audio. */
export const CONTACT_SOUND_RADIUS_M = 12;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isSlot(value: unknown): value is number {
  return Number.isInteger(value) && (value as number) >= 0;
}

function parseVec3(value: unknown): [number, number, number] | null {
  if (!Array.isArray(value) || value.length !== 3) {
    return null;
  }
  const [x, y, z] = value;
  if (!isFiniteNumber(x) || !isFiniteNumber(y) || !isFiniteNumber(z)) {
    return null;
  }
  return [x, y, z];
}

/** Parse one contact entry, returning null when the shape is malformed. */
export function parseContactEvent(value: unknown): ContactEvent | null {
  if (!isRecord(value)) {
    return null;
  }
  const { a, b, point, normal, speed, tick } = value;
  if (!isSlot(a) || !isSlot(b) || a === b) {
    return null;
  }
  if (!isFiniteNumber(speed) || speed < 0) {
    return null;
  }
  if (!isSlot(tick)) {
    return null;
  }
  const parsedPoint = parseVec3(point);
  const parsedNormal = parseVec3(normal);
  if (!parsedPoint || !parsedNormal) {
    return null;
  }
  return {
    a,
    b,
    point: parsedPoint,
    normal: parsedNormal,
    speed,
    tick,
  };
}

/**
 * Parse a `contacts` payload, dropping malformed entries.
 *
 * Non-array payloads produce an empty list. Each invalid entry is ignored so a
 * single bad event cannot suppress the valid ones.
 */
export function parseContacts(value: unknown): ContactEvent[] {
  if (!Array.isArray(value)) {
    return [];
  }
  const contacts: ContactEvent[] = [];
  for (const entry of value) {
    const contact = parseContactEvent(entry);
    if (contact) {
      contacts.push(contact);
    }
  }
  return contacts;
}

export function isPlayerContact(contact: ContactEvent): boolean {
  return contact.a === 0 || contact.b === 0;
}

/** True when the contact point lies within `radius` of `point` on the XZ plane. */
export function contactNearPoint(
  contact: ContactEvent,
  point: readonly [number, number, number] | null,
  radius = CONTACT_SOUND_RADIUS_M,
): boolean {
  if (!point || !Number.isFinite(radius) || radius < 0) {
    return false;
  }
  const dx = contact.point[0] - point[0];
  const dz = contact.point[2] - point[2];
  return dx * dx + dz * dz <= radius * radius;
}

/**
 * Sound gating: always for a player contact, otherwise only when near the
 * player, so a busy AI field cannot produce a wall of impact noise.
 */
export function shouldPlayContactSound(
  contact: ContactEvent,
  playerPosition: readonly [number, number, number] | null,
  radius = CONTACT_SOUND_RADIUS_M,
): boolean {
  return isPlayerContact(contact) || contactNearPoint(contact, playerPosition, radius);
}
