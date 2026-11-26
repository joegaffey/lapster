import type { CarCategory } from "../assets/types.ts";

export type { CarCategory };

export interface CarOption {
  id: string;
  name: string;
  url: string;
  engine: string;
  vehicle: string;
  category: CarCategory;
  paintable?: boolean;
  defaultPaint?: number;
}

export const CAR_OPTIONS: CarOption[] = [
  {
    id: "mazda-rx7",
    name: "Mazda RX-7",
    url: "../../fixtures/cars/mazda-rx7/mazda-rx7.sima",
    engine: "rotary2",
    vehicle: "sports",
    category: "road",
  },
  {
    id: "sc-deckard-conejo",
    name: "Deckard Conejo",
    url: "../../fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima",
    engine: "v8cross",
    vehicle: "sports",
    category: "road",
    paintable: true,
    defaultPaint: 0,
  },
  {
    id: "67gp-cavallo-123",
    name: "Cavallo 123",
    url: "../../fixtures/cars/67gp-cavallo-123/67gp-cavallo-123.sima",
    engine: "v12",
    vehicle: "sports",
    category: "gp67",
  },
  {
    id: "67gp-guthrie-eagle",
    name: "Guthrie Eagle",
    url: "../../fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima",
    engine: "v12",
    vehicle: "sports",
    category: "gp67",
  },
  {
    id: "67gp-kuiper-t81",
    name: "Kuiper T81",
    url: "../../fixtures/cars/67gp-kuiper-t81/67gp-kuiper-t81.sima",
    engine: "v12",
    vehicle: "sports",
    category: "gp67",
  },
  {
    id: "67gp-mango-ms7",
    name: "Mango MS7",
    url: "../../fixtures/cars/67gp-mango-ms7/67gp-mango-ms7.sima",
    engine: "v8cross",
    vehicle: "sports",
    category: "gp67",
  },
  {
    id: "67gp-murasama-37a",
    name: "Murasama 37A",
    url: "../../fixtures/cars/67gp-murasama-37a/67gp-murasama-37a.sima",
    engine: "v12",
    vehicle: "sports",
    category: "gp67",
  },
  {
    id: "67gp-upping-ut19",
    name: "Upping UT19",
    url: "../../fixtures/cars/67gp-upping-ut19/67gp-upping-ut19.sima",
    engine: "v8cross",
    vehicle: "sports",
    category: "gp67",
  },
];

export const DEFAULT_CAR = CAR_OPTIONS[0];
export const CAR_STORAGE_KEY = "lapster.car.v1";
export const PAINT_STORAGE_KEY = "lapster.garage.paint.v1";

export const CATEGORY_STORAGE_KEY = "lapster.category.v1";
export const DEFAULT_CATEGORY: CarCategory = "road";
export const CAR_CATEGORIES: ReadonlyArray<{ id: CarCategory; name: string }> = [
  { id: "road", name: "Road" },
  { id: "gp67", name: "GP 67" },
];

const BAY_COUNT = CAR_OPTIONS.length;
const STORAGE_KEY = "lapster.garage.car.v1";

export function isCarCategory(value: unknown): value is CarCategory {
  return value === "road" || value === "gp67";
}

export function categoryFromQuery(search: string): CarCategory | null {
  const value = new URLSearchParams(search).get("category");
  return isCarCategory(value) ? value : null;
}

export function loadSelectedCategory(): CarCategory {
  try {
    const stored = localStorage.getItem(CATEGORY_STORAGE_KEY);
    return isCarCategory(stored) ? stored : DEFAULT_CATEGORY;
  } catch {
    return DEFAULT_CATEGORY;
  }
}

export function saveSelectedCategory(category: CarCategory): CarCategory {
  const resolved = isCarCategory(category) ? category : DEFAULT_CATEGORY;
  try {
    localStorage.setItem(CATEGORY_STORAGE_KEY, resolved);
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
  return resolved;
}

export function carsInCategory(category: CarCategory): CarOption[] {
  return CAR_OPTIONS.filter((car) => car.category === category);
}

export function findCar(id: string | null | undefined): CarOption | null {
  return CAR_OPTIONS.find((car) => car.id === id) ?? null;
}

export function findCarInCategory(
  id: string | null | undefined,
  category: CarCategory,
): CarOption | null {
  return carsInCategory(category).find((car) => car.id === id) ?? null;
}

export function loadSelectedCar(
  category: CarCategory = loadSelectedCategory(),
): CarOption {
  const options = carsInCategory(category);
  const fallback = options[0] ?? DEFAULT_CAR;
  try {
    return (
      findCarInCategory(localStorage.getItem(CAR_STORAGE_KEY), category) ??
      fallback
    );
  } catch {
    return fallback;
  }
}

export function saveSelectedCar(
  id: string,
  category: CarCategory = loadSelectedCategory(),
): CarOption {
  const car =
    findCarInCategory(id, category) ??
    carsInCategory(category)[0] ??
    DEFAULT_CAR;
  try {
    localStorage.setItem(CAR_STORAGE_KEY, car.id);
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
  return car;
}

function loadPaintMap(): Record<string, number> {
  try {
    const raw = localStorage.getItem(PAINT_STORAGE_KEY);
    if (!raw) {
      return {};
    }
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return {};
    }
    const map: Record<string, number> = {};
    for (const [carId, value] of Object.entries(parsed)) {
      if (Number.isInteger(value) && (value as number) >= 0) {
        map[carId] = value as number;
      }
    }
    return map;
  } catch {
    return {};
  }
}

export function loadCarPaint(carId: string): number | null {
  const map = loadPaintMap();
  return Object.prototype.hasOwnProperty.call(map, carId) ? map[carId] : null;
}

export function saveCarPaint(carId: string, paint: number): number {
  if (!Number.isInteger(paint) || paint < 0) {
    return loadCarPaint(carId) ?? 0;
  }
  const map = loadPaintMap();
  map[carId] = paint;
  try {
    localStorage.setItem(PAINT_STORAGE_KEY, JSON.stringify(map));
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
  return paint;
}

function bayCount(count: number): number {
  return Number.isFinite(count) && count > 0 ? Math.trunc(count) : BAY_COUNT;
}

export function normalizeBay(
  index: number,
  count: number = BAY_COUNT,
): number {
  const bays = bayCount(count);
  if (!Number.isFinite(index)) {
    return 0;
  }
  return ((Math.trunc(index) % bays) + bays) % bays;
}

export function bayFromQuery(
  search: string,
  count: number = BAY_COUNT,
): number | null {
  const value = new URLSearchParams(search).get("bay");
  if (value === null || value.trim() === "") {
    return null;
  }
  const bay = Number(value);
  return Number.isFinite(bay)
    ? normalizeBay(Math.trunc(bay) - 1, count)
    : null;
}

export function loadSelectedBay(count: number = BAY_COUNT): number {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === null ? 0 : normalizeBay(Number(stored), count);
  } catch {
    return 0;
  }
}

export function saveSelectedBay(
  index: number,
  count: number = BAY_COUNT,
): number {
  const bay = normalizeBay(index, count);
  try {
    localStorage.setItem(STORAGE_KEY, String(bay));
  } catch {
    return bay;
  }
  return bay;
}

export function selectedBay(
  search = window.location.search,
  count: number = BAY_COUNT,
): number {
  return bayFromQuery(search, count) ?? loadSelectedBay(count);
}
