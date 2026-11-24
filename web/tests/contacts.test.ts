import test from "node:test";
import assert from "node:assert/strict";

import {
  CONTACT_SOUND_RADIUS_M,
  contactNearPoint,
  isPlayerContact,
  parseContactEvent,
  parseContacts,
  shouldPlayContactSound,
  type ContactEvent,
} from "../src/sim/contacts.ts";

function validContact(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    a: 0,
    b: 1,
    point: [1, 0.5, -2],
    normal: [0, 0, 1],
    speed: 3.2,
    tick: 1234,
    ...overrides,
  };
}

test("parses a valid contact event", () => {
  const contact = parseContactEvent(validContact());
  assert.deepEqual(contact, {
    a: 0,
    b: 1,
    point: [1, 0.5, -2],
    normal: [0, 0, 1],
    speed: 3.2,
    tick: 1234,
  });
});

test("rejects malformed contact entries", () => {
  assert.equal(parseContactEvent(null), null);
  assert.equal(parseContactEvent([]), null);
  assert.equal(parseContactEvent({}), null);
  assert.equal(parseContactEvent(validContact({ a: -1 })), null);
  assert.equal(parseContactEvent(validContact({ b: 1.5 })), null);
  assert.equal(parseContactEvent(validContact({ a: 2, b: 2 })), null);
  assert.equal(parseContactEvent(validContact({ speed: Number.NaN })), null);
  assert.equal(parseContactEvent(validContact({ speed: -0.1 })), null);
  assert.equal(parseContactEvent(validContact({ tick: -1 })), null);
  assert.equal(parseContactEvent(validContact({ tick: 1.5 })), null);
  assert.equal(parseContactEvent(validContact({ point: [0, 1] })), null);
  assert.equal(parseContactEvent(validContact({ point: [0, 1, Number.NaN] })), null);
  assert.equal(parseContactEvent(validContact({ normal: [0, 0, Number.POSITIVE_INFINITY] })), null);
  assert.equal(parseContactEvent(validContact({ normal: "up" })), null);
});

test("drops malformed entries and keeps valid ones", () => {
  const contacts = parseContacts([
    validContact(),
    null,
    validContact({ a: 1, b: 2, speed: 1 }),
    validContact({ a: 0, b: 0 }),
    validContact({ speed: Number.POSITIVE_INFINITY }),
    validContact({ a: 0, b: 3, speed: 5 }),
  ]);
  assert.equal(contacts.length, 3);
  assert.deepEqual(
    contacts.map((contact) => [contact.a, contact.b, contact.speed]),
    [
      [0, 1, 3.2],
      [1, 2, 1],
      [0, 3, 5],
    ],
  );
});

test("empty and non-array payloads produce no contacts", () => {
  assert.deepEqual(parseContacts([]), []);
  assert.deepEqual(parseContacts(null), []);
  assert.deepEqual(parseContacts({ contacts: [] }), []);
  assert.deepEqual(parseContacts("[]"), []);
});

test("gates audio to player contacts or contacts near the player", () => {
  const player: ContactEvent = {
    a: 0,
    b: 1,
    point: [100, 0, 100],
    normal: [0, 0, 1],
    speed: 2,
    tick: 1,
  };
  const aiPairFar: ContactEvent = {
    a: 1,
    b: 2,
    point: [100, 0, 100],
    normal: [0, 0, 1],
    speed: 2,
    tick: 2,
  };
  const aiPairNear: ContactEvent = {
    ...aiPairFar,
    point: [3, 50, 4],
  };

  assert.equal(isPlayerContact(player), true);
  assert.equal(isPlayerContact(aiPairFar), false);
  assert.equal(shouldPlayContactSound(player, null), true);
  assert.equal(shouldPlayContactSound(aiPairFar, null), false);
  assert.equal(shouldPlayContactSound(aiPairFar, [0, 0, 0]), false);
  assert.equal(shouldPlayContactSound(aiPairNear, [0, 0, 0]), true);
  assert.equal(contactNearPoint(aiPairNear, [0, 0, 0]), true);
  assert.equal(contactNearPoint(aiPairNear, [0, 0, 0], 4), false);
});

test("nearby check ignores height and distance beyond the radius", () => {
  const contact: ContactEvent = {
    a: 1,
    b: 2,
    point: [CONTACT_SOUND_RADIUS_M, 25, 0],
    normal: [0, 0, 1],
    speed: 4,
    tick: 3,
  };
  assert.equal(contactNearPoint(contact, [0, -25, 0]), true);
  assert.equal(contactNearPoint(contact, [CONTACT_SOUND_RADIUS_M, -25, 0]), true);
  assert.equal(
    contactNearPoint(contact, [CONTACT_SOUND_RADIUS_M * 2 + 1, 0, 0]),
    false,
  );
});
