import { expect, test } from "@playwright/test";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function waitForCountdown(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => {
      const label = document.querySelector("#race-countdown");
      return label !== null && !label.hasAttribute("hidden");
    },
    undefined,
    { timeout: 10_000 },
  );
}

async function waitForCountdownFinished(
  page: import("@playwright/test").Page,
) {
  await page.waitForFunction(
    () => {
      const label = document.querySelector("#race-countdown");
      return label !== null && label.hasAttribute("hidden");
    },
    undefined,
    { timeout: 15_000 },
  );
}

interface LayoutView {
  closed: boolean;
  points: number[][];
  half_width: number;
}

function distanceToLayout(position: number[], layout: LayoutView): number {
  const count = layout.points.length;
  if (count < 2) {
    return Number.POSITIVE_INFINITY;
  }
  const segments = layout.closed ? count : count - 1;
  let closest = Number.POSITIVE_INFINITY;
  for (let index = 0; index < segments; index += 1) {
    const start = layout.points[index];
    const end = layout.points[(index + 1) % count];
    const dx = end[0] - start[0];
    const dy = end[1] - start[1];
    const dz = end[2] - start[2];
    const lengthSq = dx * dx + dy * dy + dz * dz;
    const offsetX = position[0] - start[0];
    const offsetY = position[1] - start[1];
    const offsetZ = position[2] - start[2];
    const t =
      lengthSq > 0
        ? Math.min(
            1,
            Math.max(
              0,
              (offsetX * dx + offsetY * dy + offsetZ * dz) / lengthSq,
            ),
          )
        : 0;
    const nearestX = start[0] + dx * t - position[0];
    const nearestY = start[1] + dy * t - position[1];
    const nearestZ = start[2] + dz * t - position[2];
    closest = Math.min(
      closest,
      Math.hypot(nearestX, nearestY, nearestZ),
    );
  }
  return closest;
}

interface AiCarView {
  slot: number;
  position: number[];
}

interface FieldView {
  slots: number;
  aiCount: number;
  cars: AiCarView[];
}

async function fieldState(
  page: import("@playwright/test").Page,
): Promise<FieldView> {
  return page.evaluate(() => {
    const state = JSON.parse((window as any).__driveViewer.telemetry());
    return {
      slots: state.remoteSlots,
      aiCount: state.aiCount,
      cars: (state.remoteCars ?? []).map((car: any) => ({
        slot: car.slot,
        position: car.position,
      })),
    };
  });
}

async function waitForField(
  page: import("@playwright/test").Page,
  count: number,
) {
  await page.waitForFunction(
    (expected) => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      return Array.isArray(state.remoteCars) && state.remoteCars.length === expected;
    },
    count,
    { timeout: 15_000 },
  );
}

async function waitForFieldMovement(
  page: import("@playwright/test").Page,
  starts: Record<number, number[]>,
) {
  await page.waitForFunction(
    (positions) => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      const cars = state.remoteCars ?? [];
      const expected = Object.keys(positions).length;
      return (
        cars.length === expected &&
        cars.every((car: any) => {
          const start = positions[car.slot];
          return (
            Array.isArray(start) &&
            car.position.every((value: number) => Number.isFinite(value)) &&
            car.position.some(
              (value: number, index: number) =>
                Math.abs(value - start[index]) > 0.1,
            )
          );
        })
      );
    },
    starts,
    { timeout: 20_000 },
  );
}

async function assertFieldOnRoad(
  page: import("@playwright/test").Page,
  layout: LayoutView,
  count: number,
  durationMs: number,
) {
  const roadLimit = layout.half_width + 1.0;
  let samples = 0;
  const deadline = Date.now() + durationMs;
  while (Date.now() < deadline) {
    const field = await fieldState(page);
    expect(field.cars).toHaveLength(count);
    for (const car of field.cars) {
      expect(
        distanceToLayout(car.position, layout),
        `AI ${car.slot} left the road at [${car.position.join(", ")}]`,
      ).toBeLessThanOrEqual(roadLimit);
    }
    samples += 1;
    await page.waitForTimeout(250);
  }
  expect(samples).toBeGreaterThan(5);
}

test("launcher opens race-loop with the race layout available", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop");
  expect(new URL(page.url()).searchParams.get("track")).toBe("race-loop");

  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();

  await waitForCountdown(page);
  await expect(page.locator("#race-countdown")).toHaveText(/^(3|2|1|GO)$/);

  const drive = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      error: (window as any).__driveViewerError,
      telemetry: JSON.parse(viewer.telemetry()),
      race: JSON.parse(viewer.race()),
    };
  });
  expect(drive.error).toBeUndefined();
  expect(drive.telemetry.track).toBe("race-loop");
  expect(drive.telemetry.raceAvailable).toBe(true);
  expect(drive.telemetry.layoutId).toBe("main");
  expect(drive.race.available).toBe(true);
  expect(drive.race.layout.layout_id).toBe("main");
  expect(drive.race.layout.closed).toBe(true);
  expect(drive.race.grid).toHaveLength(8);
  expect(drive.race.sectors).toHaveLength(4);

  const grid = drive.race.grid[0].position as number[];
  const position = drive.telemetry.position as number[];
  expect(
    Math.hypot(position[0] - grid[0], position[2] - grid[2]),
    `player at [${position.join(", ")}] instead of grid [${grid.join(", ")}]`,
  ).toBeLessThan(1.0);

  await expect(page.locator("#race-panel")).toBeVisible();
  await expect(page.locator("#race-results")).toBeHidden();
  const initialRace = drive.telemetry.raceState;
  expect(initialRace.available).toBe(true);
  expect(initialRace.lap).toBe(1);
  expect(initialRace.sector).toBeGreaterThanOrEqual(1);
  expect(initialRace.sector).toBeLessThanOrEqual(4);
  expect(initialRace.progress).toBeGreaterThanOrEqual(0);
  expect(initialRace.progress).toBeLessThanOrEqual(1);
  expect(initialRace.wrong_way).toBe(false);
  // The car is stationary on the grid; the race starts at GO.
  expect(initialRace.started).toBe(false);
  await expect(page.locator("#race-lap")).toHaveText("1/3");
  await expect(page.locator("#race-position")).toHaveText("P1/4");
  await expect(page.locator("#race-sector")).toHaveText(
    `S${initialRace.sector}/4`,
  );
  await expect(page.locator("#race-current")).toHaveText(/^\d+:\d{2}\.\d{2}$/);
  await expect(page.locator("#race-last")).toHaveText("—");
  await expect(page.locator("#race-best")).toHaveText("—");
  await expect(page.locator("#race-wrong-way")).toBeHidden();

  await expect(page.getByTestId("race-ai")).toBeVisible();
  await expect(page.getByTestId("race-ai")).toContainText("AI");
  await expect(page.locator("#race-ai")).toHaveText("3");

  await waitForField(page, 3);
  const aiStart = await fieldState(page);
  expect(aiStart.slots).toBe(3);
  expect(aiStart.aiCount).toBe(3);
  expect(aiStart.cars).toHaveLength(3);
  expect(aiStart.cars.map((car) => car.slot).sort()).toEqual([1, 2, 3]);
  for (const car of aiStart.cars) {
    expect(
      car.position.every((value: number) => Number.isFinite(value)),
    ).toBe(true);
  }
  const starts: Record<number, number[]> = {};
  for (const car of aiStart.cars) {
    starts[car.slot] = car.position;
  }

  await waitForCountdownFinished(page);
  await waitForFieldMovement(page, starts);

  const layout = drive.race.layout as LayoutView;
  await assertFieldOnRoad(page, layout, 3, 5_000);

  const before = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      started: JSON.parse(viewer.telemetry()).raceState.started as boolean,
      current: document.querySelector("#race-current")?.textContent ?? "",
    };
  });
  expect(before.started).toBe(false);

  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    () => {
      const state = JSON.parse((window as any).__driveViewer.telemetry()).raceState;
      return (
        state.available &&
        state.started === true &&
        state.lap === 1 &&
        state.wrong_way === false
      );
    },
    undefined,
    { timeout: 15_000 },
  );
  await page.keyboard.up("w");
  await page.waitForFunction(
    (initial) => {
      const current = document.querySelector("#race-current")?.textContent ?? "";
      return current !== initial;
    },
    before.current,
    { timeout: 5_000 },
  );

  const agreement = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      telemetry: JSON.parse(viewer.telemetry()).raceState,
      direct: JSON.parse(viewer.race_state()),
      sector: document.querySelector("#race-sector")?.textContent ?? "",
      lap: document.querySelector("#race-lap")?.textContent ?? "",
      position: document.querySelector("#race-position")?.textContent ?? "",
      current: document.querySelector("#race-current")?.textContent ?? "",
    };
  });
  expect(agreement.telemetry.available).toBe(agreement.direct.available);
  expect(agreement.telemetry.available).toBe(true);
  expect(agreement.telemetry.lap).toBe(agreement.direct.lap);
  expect(agreement.telemetry.sector).toBe(agreement.direct.sector);
  expect(agreement.sector).toBe(`S${agreement.telemetry.sector}/4`);
  expect(agreement.telemetry.started).toBe(true);
  expect(agreement.telemetry.wrong_way).toBe(false);
  expect(agreement.current).not.toBe(before.current);
  expect(agreement.lap).toBe("1/3");
  expect(agreement.position).toMatch(/^P\d+\/4$/);
  await expect(page.locator("#race-results")).toBeHidden();

  expect(pageErrors).toEqual([]);
});

test("reset during a local race re-grids and restarts the countdown", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&ai=1");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await waitForCountdown(page);
  await waitForCountdownFinished(page);

  await page.getByTestId("reset").click();
  await waitForCountdown(page);
  await expect(page.locator("#race-countdown")).toHaveText(/^(3|2|1|GO)$/);

  const drive = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      telemetry: JSON.parse(viewer.telemetry()),
      race: JSON.parse(viewer.race()),
    };
  });
  const grid = drive.race.grid[0].position as number[];
  const position = drive.telemetry.position as number[];
  expect(
    Math.hypot(position[0] - grid[0], position[2] - grid[2]),
  ).toBeLessThan(1.0);
  expect(drive.telemetry.raceState.lap).toBe(1);
  expect(drive.telemetry.raceState.started).toBe(false);
  await expect(page.locator("#race-lap")).toHaveText("1/3");
  await expect(page.locator("#race-position")).toHaveText("P1/2");

  await waitForCountdownFinished(page);
  expect(pageErrors).toEqual([]);
});

test("results restart action hides the overlay and restarts the countdown", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&ai=1");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await waitForCountdownFinished(page);
  await expect(page.locator("#race-results")).toBeHidden();

  await page.evaluate(() => {
    const results = document.querySelector<HTMLElement>("#race-results");
    if (!results) {
      throw new Error("results overlay missing");
    }
    results.hidden = false;
  });
  await expect(page.locator("#race-results")).toBeVisible();

  await page.getByTestId("race-results-restart").click();
  await expect(page.locator("#race-results")).toBeHidden();
  await waitForCountdown(page);
  await expect(page.locator("#race-lap")).toHaveText("1/3");
  await expect(page.locator("#race-position")).toHaveText(/^P\d+\/2$/);
  expect(pageErrors).toEqual([]);
});

test("?ai=1 requests a single AI opponent", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&ai=1");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await expect(page.locator("#race-panel")).toBeVisible();
  await waitForCountdownFinished(page);

  await expect(page.getByTestId("race-ai")).toBeVisible();
  await expect(page.locator("#race-ai")).toHaveText("1");
  await waitForField(page, 1);

  const field = await fieldState(page);
  expect(field.slots).toBe(1);
  expect(field.aiCount).toBe(1);
  expect(field.cars).toHaveLength(1);
  expect(field.cars[0].slot).toBe(1);
  expect(
    field.cars[0].position.every((value: number) => Number.isFinite(value)),
  ).toBe(true);
  await expect(page.locator("#race-position")).toHaveText(/^P\d+\/2$/);
  expect(pageErrors).toEqual([]);
});

test("?ai=0 keeps the race field empty", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&ai=0");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await expect(page.locator("#race-panel")).toBeVisible();
  await waitForCountdownFinished(page);
  await expect(page.getByTestId("race-ai")).toBeHidden();
  await page.waitForTimeout(750);

  const field = await fieldState(page);
  expect(field.slots).toBe(0);
  expect(field.aiCount).toBe(0);
  expect(field.cars).toEqual([]);
  await expect(page.getByTestId("race-ai")).toBeHidden();
  await expect(page.locator("#race-position")).toHaveText("P1/1");
  expect(pageErrors).toEqual([]);
});

test("?laps=2 sets the race lap total", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&ai=0&laps=2");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await expect(page.locator("#race-panel")).toBeVisible();
  await expect(page.locator("#race-lap")).toHaveText("1/2");
  expect(pageErrors).toEqual([]);
});

test("?physics=main starts the race countdown without AI", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&physics=main");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await waitForCountdown(page);
  await expect(page.locator("#race-countdown")).toHaveText(/^(3|2|1|GO)$/);
  await waitForCountdownFinished(page);

  const drive = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      error: (window as any).__driveViewerError,
      telemetry: JSON.parse(viewer.telemetry()),
      race: JSON.parse(viewer.race()),
    };
  });
  expect(drive.error).toBeUndefined();
  expect(drive.telemetry.raceAvailable).toBe(true);
  expect(drive.race.available).toBe(true);

  const grid = drive.race.grid[0].position as number[];
  const position = drive.telemetry.position as number[];
  expect(
    Math.hypot(position[0] - grid[0], position[2] - grid[2]),
  ).toBeLessThan(1.0);
  expect(drive.telemetry.raceState.lap).toBe(1);
  expect(drive.telemetry.raceState.started).toBe(false);
  expect(drive.telemetry.remoteCars).toEqual([]);

  await expect(page.locator("#race-position")).toHaveText("P1/1");
  await expect(page.locator("#race-lap")).toHaveText("1/3");
  await expect(page.locator("#race-ai")).toBeHidden();
  expect(pageErrors).toEqual([]);
});

test("?aiPace=fast keeps the three AI field on the road", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&aiPace=fast");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await expect(page.locator("#race-panel")).toBeVisible();
  await expect(page.getByTestId("race-ai")).toBeVisible();
  await expect(page.locator("#race-ai")).toHaveText("3");
  await waitForCountdownFinished(page);

  await waitForField(page, 3);
  const field = await fieldState(page);
  expect(field.slots).toBe(3);
  expect(field.aiCount).toBe(3);
  expect(field.cars).toHaveLength(3);
  const starts: Record<number, number[]> = {};
  for (const car of field.cars) {
    starts[car.slot] = car.position;
  }
  await waitForFieldMovement(page, starts);

  const race = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.race()),
  );
  await assertFieldOnRoad(page, race.layout as LayoutView, 3, 4_000);
  expect(pageErrors).toEqual([]);
});
