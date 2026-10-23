import { expect, test } from "@playwright/test";

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

test("mobile starts collapsed and exposes touch controls", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const hud = page.locator("#hud");
  const toggle = page.locator("#hud-toggle");
  await expect(hud).toHaveClass(/hud-collapsed/);
  await expect(toggle).toHaveText("Info");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#speed")).toBeHidden();

  await toggle.click();
  await expect(hud).not.toHaveClass(/hud-collapsed/);
  await expect(toggle).toHaveText("Hide info");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#speed")).toBeVisible();

  await toggle.click();
  await expect(hud).toHaveClass(/hud-collapsed/);

  const throttle = page.locator('[data-key="w"]');
  await expect(page.locator("#touch-controls")).toBeVisible();
  await expect(page.locator("#touch-reset")).toHaveCount(0);
  await expect(throttle).toBeVisible();
  const box = await throttle.boundingBox();
  expect(box).not.toBeNull();
  if (!box) {
    return;
  }
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 1,
    undefined,
    { timeout: 10_000 },
  );
  await page.mouse.up();

  const telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.speedKph).toBeGreaterThan(1);
  expect(telemetry.contacts).toBe(4);
  expect(pageErrors).toEqual([]);
});
