export interface CarOption {
  id: string;
  name: string;
  url: string;
  engine: string;
  vehicle: string;
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
    id: "sc-deckard-conejo",
    name: "Deckard Conejo",
    url: "../../fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima",
    engine: "v8cross",
    vehicle: "sports",
  },
];

export const DEFAULT_CAR = CAR_OPTIONS[0];
export const CAR_STORAGE_KEY = "lapster.car.v1";

const BAY_COUNT = 10;
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
