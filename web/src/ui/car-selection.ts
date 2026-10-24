const BAY_COUNT = 10;
const STORAGE_KEY = "lapster.garage.car.v1";

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
