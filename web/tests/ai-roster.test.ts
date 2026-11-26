import test from "node:test";
import assert from "node:assert/strict";

import { selectAiRoster } from "../src/race/ai-roster.ts";
import type { CarCategory } from "../src/ui/car-selection.ts";

const ids = (category: CarCategory, player: string | null, count: number) =>
  selectAiRoster(category, player, count).map((car) => car.id);

test("gp67 roster uses every other car before repeating the player", () => {
  assert.deepEqual(ids("gp67", "67gp-cavallo-123", 6), [
    "67gp-guthrie-eagle",
    "67gp-kuiper-t81",
    "67gp-mango-ms7",
    "67gp-murasama-37a",
    "67gp-upping-ut19",
    "67gp-cavallo-123",
  ]);
  assert.deepEqual(ids("gp67", "67gp-cavallo-123", 8), [
    "67gp-guthrie-eagle",
    "67gp-kuiper-t81",
    "67gp-mango-ms7",
    "67gp-murasama-37a",
    "67gp-upping-ut19",
    "67gp-cavallo-123",
    "67gp-guthrie-eagle",
    "67gp-kuiper-t81",
  ]);
  assert.deepEqual(ids("gp67", "67gp-guthrie-eagle", 6), [
    "67gp-cavallo-123",
    "67gp-kuiper-t81",
    "67gp-mango-ms7",
    "67gp-murasama-37a",
    "67gp-upping-ut19",
    "67gp-guthrie-eagle",
  ]);
});

test("road roster alternates between the other car and the player", () => {
  assert.deepEqual(ids("road", "mazda-rx7", 4), [
    "sc-deckard-conejo",
    "mazda-rx7",
    "sc-deckard-conejo",
    "mazda-rx7",
  ]);
  assert.deepEqual(ids("road", "sc-deckard-conejo", 4), [
    "mazda-rx7",
    "sc-deckard-conejo",
    "mazda-rx7",
    "sc-deckard-conejo",
  ]);
});

test("player car is the last resort and count 0/1 is handled", () => {
  assert.deepEqual(ids("gp67", "67gp-cavallo-123", 0), []);
  assert.deepEqual(ids("gp67", "67gp-cavallo-123", 1), [
    "67gp-guthrie-eagle",
  ]);
  assert.deepEqual(ids("road", "mazda-rx7", 1), ["sc-deckard-conejo"]);
  assert.deepEqual(ids("road", "mazda-rx7", 0), []);
  assert.deepEqual(ids("gp67", "67gp-cavallo-123", -1), []);
});

test("unknown or empty category produces no AI cars", () => {
  assert.deepEqual(ids("truck" as CarCategory, "mazda-rx7", 3), []);
});
