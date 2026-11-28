import test from "node:test";
import assert from "node:assert/strict";

import {
  CAR_CATEGORIES,
  CATEGORY_STORAGE_KEY,
  DEFAULT_CATEGORY,
  bayFromQuery,
  carsInCategory,
  categoryFromQuery,
  findCarInCategory,
  isCarCategory,
  loadSelectedBay,
  loadSelectedCar,
  loadSelectedCategory,
  normalizeBay,
  saveSelectedBay,
  saveSelectedCar,
  saveSelectedCategory,
  selectedBay,
} from "../src/ui/car-selection.ts";

class MemoryStorage {
  private readonly values = new Map<string, string>();

  get length(): number {
    return this.values.size;
  }

  clear(): void {
    this.values.clear();
  }

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  key(index: number): string | null {
    return Array.from(this.values.keys())[index] ?? null;
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }

  setItem(key: string, value: string): void {
    this.values.set(key, String(value));
  }
}

const storage = new MemoryStorage();
Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: storage,
});

const reset = () => storage.clear();

test("category values, names, and query parsing", () => {
  assert.equal(DEFAULT_CATEGORY, "road");
  assert.deepEqual(CAR_CATEGORIES, [
    { id: "road", name: "Road" },
    { id: "gp67", name: "GP 67" },
  ]);
  assert.equal(categoryFromQuery("?category=road"), "road");
  assert.equal(categoryFromQuery("?category=gp67"), "gp67");
  assert.equal(categoryFromQuery("?category=ROAD"), null);
  assert.equal(categoryFromQuery("?category="), null);
  assert.equal(categoryFromQuery(""), null);
  assert.equal(isCarCategory("road"), true);
  assert.equal(isCarCategory("gp67"), true);
  assert.equal(isCarCategory("truck"), false);
  assert.equal(isCarCategory(null), false);
});

test("category persists and defaults to road", () => {
  reset();
  assert.equal(loadSelectedCategory(), "road");
  assert.equal(saveSelectedCategory("gp67"), "gp67");
  assert.equal(storage.getItem(CATEGORY_STORAGE_KEY), "gp67");
  assert.equal(loadSelectedCategory(), "gp67");
  assert.equal(saveSelectedCategory("invalid" as never), "road");
  assert.equal(loadSelectedCategory(), "road");
});

test("catalog filters by category in CAR_OPTIONS order", () => {
  assert.deepEqual(
    carsInCategory("road").map((car) => car.id),
    ["mazda-rx7", "sc-deckard-conejo"],
  );
  assert.deepEqual(
    carsInCategory("gp67").map((car) => car.id),
    [
      "67gp-cavallo-123",
      "67gp-guthrie-eagle",
      "67gp-kuiper-t81",
      "67gp-mango-ms7",
      "67gp-murasama-37a",
      "67gp-upping-ut19",
    ],
  );
  assert.equal(findCarInCategory("mazda-rx7", "road")?.name, "Mazda RX-7");
  assert.equal(findCarInCategory("mazda-rx7", "gp67"), null);
  assert.equal(
    findCarInCategory("67gp-cavallo-123", "gp67")?.name,
    "Cavallo 123",
  );
});

test("stored car from another category falls back to the category first car", () => {
  reset();
  storage.setItem("lapster.car.v1", "67gp-cavallo-123");
  saveSelectedCategory("road");
  assert.equal(loadSelectedCar().id, "mazda-rx7");
  assert.equal(loadSelectedCar("gp67").id, "67gp-cavallo-123");

  reset();
  storage.setItem("lapster.car.v1", "sc-deckard-conejo");
  assert.equal(loadSelectedCar("gp67").id, "67gp-cavallo-123");
  assert.equal(loadSelectedCar("road").id, "sc-deckard-conejo");
});

test("selecting a car saves only ids in the selected category", () => {
  reset();
  saveSelectedCategory("road");
  assert.equal(saveSelectedCar("sc-deckard-conejo").id, "sc-deckard-conejo");
  assert.equal(storage.getItem("lapster.car.v1"), "sc-deckard-conejo");

  assert.equal(saveSelectedCar("67gp-guthrie-eagle", "road").id, "mazda-rx7");
  assert.equal(storage.getItem("lapster.car.v1"), "mazda-rx7");

  assert.equal(saveSelectedCar("mazda-rx7", "gp67").id, "67gp-cavallo-123");
  assert.equal(storage.getItem("lapster.car.v1"), "67gp-cavallo-123");
});

test("bay normalisation uses the filtered category count", () => {
  reset();
  assert.equal(normalizeBay(4, 2), 0);
  assert.equal(normalizeBay(-1, 2), 1);
  assert.equal(normalizeBay(5, 6), 5);
  assert.equal(normalizeBay(7, 0), 7 % 8);
  assert.equal(normalizeBay(9), 1);

  assert.equal(bayFromQuery("?bay=2", 2), 1);
  assert.equal(bayFromQuery("?bay=3", 2), 0);
  assert.equal(bayFromQuery("?bay=2"), 1);
  assert.equal(bayFromQuery("?bay=9"), 0);

  storage.setItem("lapster.garage.car.v1", "5");
  assert.equal(loadSelectedBay(2), 1);
  assert.equal(loadSelectedBay(), 5);
  assert.equal(saveSelectedBay(7, 6), 1);
  assert.equal(storage.getItem("lapster.garage.car.v1"), "1");
  assert.equal(saveSelectedBay(7), 7);

  assert.equal(selectedBay("?bay=2", 2), 1);
  assert.equal(selectedBay(""), 7);
});
