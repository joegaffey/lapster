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
  await expect(page.locator(".hud-exit")).toBeVisible();
  await expect(page.locator(".hud-exit")).toHaveText("Exit");
  await expect(page.locator(".hud-exit")).toHaveAttribute("href", "../");
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
  expect(telemetry.shadowMapSize).toBe(2048);
  expect(telemetry.environmentShadowMode).toBe("baked");
  expect(telemetry.carShadowMode).toBe("dynamic");
  expect(telemetry.carShadowMapSize).toBe(2048);
  expect(telemetry.bakedShadowResolution).toBe(2048);
  expect(pageErrors).toEqual([]);
});

test.describe("pre-boot info panel", () => {
  test.use({ javaScriptEnabled: false });

  test("info starts hidden on every demo before scripts run", async ({ page }) => {
    for (const path of ["/web/garage/", "/web/drive/", "/web/track-viewer/"]) {
      await page.goto(path);
      await expect(page.locator("#hud")).toBeHidden();
    }
  });
});


test("garage and track expose graphics-only shadow settings", async ({ page }) => {
  const waitFor = (expression: string) =>
    page.waitForFunction(expression, undefined, { timeout: 30_000 });

  await page.goto(
    "/web/garage/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await waitFor(() => (window as any).__garageViewerReady === true || (window as any).__garageViewerError);
  await page.locator("#assists-open").click();
  await expect(page.locator("#tab-gameplay")).toHaveCount(0);
  await expect(page.locator("#tab-graphics")).toHaveAttribute("aria-selected", "true");
  const environment = page.locator("#environment-shadows");
  const carShadow = page.locator("#car-shadow");
  await environment.selectOption("baked");
  await carShadow.selectOption("silhouette");
  let state = await page.evaluate(() => ({
    environment: (window as any).__garageViewer.environment_shadow_mode(),
    car: (window as any).__garageViewer.car_shadow_mode(),
    resolution: (window as any).__garageViewer.baked_resolution(),
  }));
  expect(state.environment).toBe("baked");
  expect(state.car).toBe("silhouette");
  expect(state.resolution).toBe(2048);
  await page.locator("#time-of-day").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "18";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await page.locator("#time-of-day-apply").click();
  expect(await page.evaluate(() => (window as any).__garageViewer.time_of_day())).toBeCloseTo(18);
  await page.locator("#assists-close").click();

  await page.goto("/web/track-viewer/?track=../../fixtures/tracks/minimal/pack.sima&v=cp6");
  await waitFor(() => (window as any).__trackViewerReady === true || (window as any).__trackViewerError);
  await page.locator("#assists-open").click();
  await expect(page.locator("#tab-gameplay")).toHaveCount(0);
  await expect(page.locator("#settings-graphics")).toBeVisible();
  await page.locator("#environment-shadows").selectOption("baked");
  state = await page.evaluate(() => ({
    environment: (window as any).__trackViewer.environment_shadow_mode(),
    resolution: (window as any).__trackViewer.baked_resolution(),
  }));
  expect(state.environment).toBe("baked");
  expect(state.resolution).toBe(2048);
});
