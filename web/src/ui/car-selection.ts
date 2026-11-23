export interface CarOption {
  id: string;
  name: string;
  url: string;
  engine: string;
  vehicle: string;
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
  },
  {
    id: "67gp-cavallo-123",
    name: "Cavallo 123",
    url: "../../fixtures/cars/67gp-cavallo-123/67gp-cavallo-123.sima",
    engine: "v12",
    vehicle: "sports",
  },
  {
    id: "67gp-guthrie-eagle",
    name: "Guthrie Eagle",
    url: "../../fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima",
    engine: "v12",
    vehicle: "sports",
  },
  {
    id: "67gp-kuiper-t81",
    name: "Kuiper T81",
    url: "../../fixtures/cars/67gp-kuiper-t81/67gp-kuiper-t81.sima",
    engine: "v12",
    vehicle: "sports",
  },
  {
    id: "67gp-mango-ms7",
    name: "Mango MS7",
    url: "../../fixtures/cars/67gp-mango-ms7/67gp-mango-ms7.sima",
    engine: "v8cross",
    vehicle: "sports",
  },
  {
    id: "67gp-murasama-37a",
    name: "Murasama 37A",
    url: "../../fixtures/cars/67gp-murasama-37a/67gp-murasama-37a.sima",
    engine: "v12",
    vehicle: "sports",
  },
  {
    id: "67gp-upping-ut19",
    name: "Upping UT19",
    url: "../../fixtures/cars/67gp-upping-ut19/67gp-upping-ut19.sima",
    engine: "v8cross",
    vehicle: "sports",
  },
  {
    id: "sc-deckard-conejo",
    name: "Deckard Conejo",
    url: "../../fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima",
    engine: "v8cross",
    vehicle: "sports",
    paintable: true,
    defaultPaint: 0,
  },
];

export const DEFAULT_CAR = CAR_OPTIONS[0];
export const CAR_STORAGE_KEY = "lapster.car.v1";
export const PAINT_STORAGE_KEY = "lapster.garage.paint.v1";

const BAY_COUNT = CAR_OPTIONS.length;
const STORAGE_KEY = "lapster.garage.car.v1";

export function findCar(id: string | null | undefined): CarOption | null {
  return CAR_OPTIONS.find((car) => car.id === id) ?? null;
}

export function loadSelectedCar(): CarOption {
  try {
    return findCar(localStorage.getItem(CAR_STORAGE_KEY)) ?? DEFAULT_CAR;
  } catch {
    return DEFAULT_CAR;
  }
}

export function saveSelectedCar(id: string): CarOption {
  const car = findCar(id) ?? DEFAULT_CAR;
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

export function normalizeBay(index: number): number {
  if (!Number.isFinite(index)) {
    return 0;
  }
  return ((Math.trunc(index) % BAY_COUNT) + BAY_COUNT) % BAY_COUNT;
}

export function bayFromQuery(search: string): number | null {
  const value = new URLSearchParams(search).get("bay");
  if (value === null || value.trim() === "") {
    return null;
  }
  const bay = Number(value);
  return Number.isFinite(bay) ? normalizeBay(Math.trunc(bay) - 1) : null;
}

export function loadSelectedBay(): number {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === null ? 0 : normalizeBay(Number(stored));
  } catch {
    return 0;
  }
}

export function saveSelectedBay(index: number): number {
  const bay = normalizeBay(index);
  try {
    localStorage.setItem(STORAGE_KEY, String(bay));
  } catch {
    return bay;
  }
  return bay;
}

export function selectedBay(search = window.location.search): number {
  return bayFromQuery(search) ?? loadSelectedBay();
}
