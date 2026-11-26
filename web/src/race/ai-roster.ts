import {
  carsInCategory,
  type CarCategory,
  type CarOption,
} from "../ui/car-selection.ts";

/**
 * Pick AI cars from one category, preferring cars other than the player's own.
 * The player car is only used once the rest of the category has been cycled,
 * so a mixed grid appears as early as possible. Counts beyond the pool repeat
 * it deterministically in the same order.
 */
export function selectAiRoster(
  category: CarCategory,
  playerCarId: string | null | undefined,
  count: number,
): CarOption[] {
  const cars = carsInCategory(category);
  if (cars.length === 0) {
    return [];
  }
  const requested =
    Number.isFinite(count) && count > 0 ? Math.trunc(count) : 0;
  if (requested === 0) {
    return [];
  }
  const player = cars.find((car) => car.id === playerCarId) ?? null;
  const pool = player
    ? [...cars.filter((car) => car.id !== player.id), player]
    : cars;
  return Array.from(
    { length: requested },
    (_, index) => pool[index % pool.length],
  );
}
