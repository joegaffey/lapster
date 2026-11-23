import { expect, test } from "@playwright/test";

const GUTHRIE_EAGLE = "../../fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima";

test("Guthrie Eagle uses mapped Speed Dreams physics and drives", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(`/web/drive/?car=${GUTHRIE_EAGLE}`);
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
  expect(initial.telemetry.car).toBe("67gp-guthrie-eagle");
  expect(initial.debug.car.mass).toBe(620);
  expect(initial.debug.car.wheelbase).toBeCloseTo(2.46, 2);

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

  expect(pageErrors).toEqual([]);
});
