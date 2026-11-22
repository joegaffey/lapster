import { expect, test } from "@playwright/test";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
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

test("launcher opens race-loop with the race layout available", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/");
  const raceLoop = page.locator('[data-track-id="race-loop"]');
  await expect(raceLoop).toBeVisible();
  await raceLoop.click();
  await page.waitForURL(/\/web\/drive\//);
  expect(new URL(page.url()).searchParams.get("track")).toBe("race-loop");

  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();

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

  await expect(page.locator("#race-panel")).toBeVisible();
  const initialRace = drive.telemetry.raceState;
  expect(initialRace.available).toBe(true);
  expect(initialRace.lap).toBe(1);
  expect(initialRace.sector).toBe(1);
  expect(initialRace.progress).toBeLessThanOrEqual(0.02);
  expect(initialRace.wrong_way).toBe(false);
  // The car is stationary at the start/finish; the timer starts on movement.
  expect(initialRace.started).toBe(false);
  await expect(page.locator("#race-lap")).toHaveText("1");
  await expect(page.locator("#race-sector")).toHaveText("S1/4");
  await expect(page.locator("#race-current")).toHaveText(/^\d+:\d{2}\.\d{2}$/);
  await expect(page.locator("#race-last")).toHaveText("—");
  await expect(page.locator("#race-best")).toHaveText("—");
  await expect(page.locator("#race-wrong-way")).toBeHidden();

  await page.waitForFunction(
    () => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      return Array.isArray(state.remoteCars) && state.remoteCars.length === 1;
    },
    undefined,
    { timeout: 10_000 },
  );
  const aiStart = await page.evaluate(() => {
    const state = JSON.parse((window as any).__driveViewer.telemetry());
    return { slots: state.remoteSlots, cars: state.remoteCars };
  });
  expect(aiStart.slots).toBe(1);
  expect(aiStart.cars).toHaveLength(1);
  expect(aiStart.cars[0].slot).toBe(1);
  expect(
    aiStart.cars[0].position.every((value: number) => Number.isFinite(value)),
  ).toBe(true);
  const aiStartPosition = aiStart.cars[0].position as number[];

  await page.waitForFunction(
    (start) => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      const car = state.remoteCars?.[0];
      return (
        car &&
        car.position.every((value: number) => Number.isFinite(value)) &&
        car.position.some(
          (value: number, index: number) => Math.abs(value - start[index]) > 0.1,
        )
      );
    },
    aiStartPosition,
    { timeout: 15_000 },
  );

  const layout = drive.race.layout as LayoutView;
  const roadLimit = layout.half_width + 1.0;
  const samples: number[][] = [];
  const sampleDeadline = Date.now() + 5_000;
  while (Date.now() < sampleDeadline) {
    const position = await page.evaluate(() => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      return state.remoteCars?.[0]?.position as number[] | undefined;
    });
    if (position) {
      samples.push(position);
      expect(
        distanceToLayout(position, layout),
        `AI left the road at [${position.join(", ")}]`,
      ).toBeLessThanOrEqual(roadLimit);
    }
    await page.waitForTimeout(250);
  }
  expect(samples.length).toBeGreaterThan(5);

  const before = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      progress: JSON.parse(viewer.telemetry()).raceState.progress as number,
      current: document.querySelector("#race-current")?.textContent ?? "",
    };
  });

  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    (initial) => {
      const state = JSON.parse((window as any).__driveViewer.telemetry()).raceState;
      const current = document.querySelector("#race-current")?.textContent ?? "";
      return (
        state.available &&
        state.started === true &&
        state.wrong_way === false &&
        state.progress > initial.progress + 0.001 &&
        current !== initial.current
      );
    },
    before,
    { timeout: 15_000 },
  );
  await page.keyboard.up("w");

  const agreement = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      telemetry: JSON.parse(viewer.telemetry()).raceState,
      direct: JSON.parse(viewer.race_state()),
      sector: document.querySelector("#race-sector")?.textContent ?? "",
      current: document.querySelector("#race-current")?.textContent ?? "",
    };
  });
  expect(agreement.telemetry.available).toBe(agreement.direct.available);
  expect(agreement.telemetry.available).toBe(true);
  expect(agreement.telemetry.lap).toBe(agreement.direct.lap);
  expect(agreement.telemetry.sector).toBe(agreement.direct.sector);
  expect(agreement.sector).toBe(`S${agreement.telemetry.sector}/4`);
  expect(agreement.telemetry.progress).toBeGreaterThan(before.progress);
  expect(agreement.telemetry.wrong_way).toBe(false);
  expect(agreement.telemetry.started).toBe(true);
  expect(agreement.current).not.toBe(before.current);

  expect(pageErrors).toEqual([]);
});
