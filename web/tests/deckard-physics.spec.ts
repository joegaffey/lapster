import { expect, test } from "@playwright/test";

const DECKARD = "../../fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima";

test("Deckard Conejo uses mapped Speed Dreams physics and drives", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(`/web/drive/?car=${DECKARD}`);
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const initial = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    debug: JSON.parse((window as any).__driveViewer.debug_info()),
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
  }));
  expect(initial.error).toBeUndefined();
  expect(initial.telemetry.car).toBe("sc-deckard-conejo");
  expect(initial.debug.car.mass).toBe(1700);
  expect(initial.debug.car.wheelbase).toBeCloseTo(2.73, 2);

  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).gear > 1,
    undefined,
    { timeout: 15_000 },
  );
  await page.keyboard.up("w");

  const moving = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(moving.gear).toBeGreaterThan(1);
  expect(moving.speedKph).toBeGreaterThan(20);
  expect(moving.rpm).toBeGreaterThan(1_000);

  await page.keyboard.down("s");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph < 3,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("s");
  expect(pageErrors).toEqual([]);
});
