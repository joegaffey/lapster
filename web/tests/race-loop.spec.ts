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
  expect(pageErrors).toEqual([]);
});
