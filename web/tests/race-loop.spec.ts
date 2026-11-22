import { expect, test } from "@playwright/test";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
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
