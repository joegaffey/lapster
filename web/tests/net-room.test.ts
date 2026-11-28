import test from "node:test";
import assert from "node:assert/strict";

import {
  driveRoomUrl,
  lobbyInviteUrl,
  makeRoomCode,
  normalizeRelayUrl,
  normalizeRoomCode,
} from "../src/net/room.ts";
import { parseServerMessage, validState } from "../src/net/protocol.ts";

test("room codes are shareable and normalised", () => {
  for (let index = 0; index < 20; index += 1) {
    const code = makeRoomCode();
    assert.match(code, /^[A-HJ-NP-Z2-9]{6}$/);
    assert.equal(normalizeRoomCode(` ${code.toLowerCase()} `), code);
  }
  assert.equal(normalizeRoomCode("bad code"), null);
  assert.equal(normalizeRoomCode(""), null);
});

test("relay URLs normalise to websocket protocols", () => {
  assert.equal(
    normalizeRelayUrl("https://relay.example.test/ws", "https://page.example/"),
    "wss://relay.example.test/ws",
  );
  assert.equal(
    normalizeRelayUrl("ws://127.0.0.1:3000/ws", "http://127.0.0.1/"),
    "ws://127.0.0.1:3000/ws",
  );
  assert.equal(normalizeRelayUrl("", "https://page.example/"), null);
  assert.equal(normalizeRelayUrl("ftp://relay", "https://page.example/"), null);
});

test("invite links keep the room, car, and track", () => {
  const url = new URL(
    lobbyInviteUrl(
      "https://example.test/web/multiplayer/?password=secret#x",
      "ABC123",
      "mazda-rx7",
      "skidpad05",
      "wss://relay.example.test/ws",
    ),
  );
  assert.equal(url.searchParams.get("room"), "ABC123");
  assert.equal(url.searchParams.get("car"), "mazda-rx7");
  assert.equal(url.searchParams.get("track"), "skidpad05");
  assert.equal(url.searchParams.get("password"), null);
  assert.equal(url.searchParams.get("ws"), "wss://relay.example.test/ws");
  assert.equal(url.hash, "");
});

test("room URLs carry the selected category", () => {
  const invite = new URL(
    lobbyInviteUrl(
      "https://example.test/web/multiplayer/",
      "ABC123",
      "67gp-cavallo-123",
      "skidpad05",
      "wss://relay.example.test/ws",
      "gp67",
    ),
  );
  assert.equal(invite.searchParams.get("category"), "gp67");
  assert.equal(invite.searchParams.get("car"), "67gp-cavallo-123");

  const drive = new URL(
    driveRoomUrl(
      "https://example.test/web/drive/",
      "ABC123",
      "67gp-cavallo-123",
      "skidpad05",
      null,
      "gp67",
    ),
  );
  assert.equal(drive.searchParams.get("category"), "gp67");
  assert.equal(drive.searchParams.get("mp"), "1");
  assert.equal(drive.searchParams.get("car"), "67gp-cavallo-123");
});

test("protocol rejects malformed messages and state arrays", () => {
  assert.equal(parseServerMessage("not json"), null);
  assert.equal(parseServerMessage('{"type":"unknown"}'), null);
  assert.equal(
    parseServerMessage('{"type":"error","message":"room is full"}')?.type,
    "error",
  );
  assert.equal(validState(Array.from({ length: 64 }, () => 0)), true);
  assert.equal(validState([1, 2, 3]), false);
  assert.equal(validState(Array.from({ length: 64 }, () => Number.NaN)), false);
});
