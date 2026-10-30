import { expect, test } from "@playwright/test";

test("skidpad grows stylized grass blades", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto(
    "/web/track-viewer/?track=../../fixtures/tracks/skidpad05/pack.sima&v=grass",
  );
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );
  await page.waitForTimeout(300);

  const info = await page.evaluate(() => ({
    error: (window as any).__trackViewerError,
    debug: JSON.parse((window as any).__trackViewer.debug_info()),
  }));
  expect(info.error).toBeUndefined();
  expect(info.debug.stats.grassBlades).toBeGreaterThan(200_000);
  expect(pageErrors).toEqual([]);
});

test("grass window follows the car", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });
  await page.addInitScript(() => {
    localStorage.setItem("lapster.settings.v3", JSON.stringify({ grassShader: true }));
  });

  await page.goto("/web/drive/?v=grass-follow");
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const result = await page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    const start = JSON.parse(viewer.telemetry()).position;
    await viewer.simulate(4.0, 1.0, 0.0, 0.0, 1);
    const end = JSON.parse(viewer.telemetry()).position;
    return Math.hypot(end[0] - start[0], end[2] - start[2]);
  });

  expect(result).toBeGreaterThan(20);
  expect(pageErrors).toEqual([]);
});

test("grass texture and shader toggles are independent", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto(
    "/web/track-viewer/?track=../../fixtures/tracks/skidpad05/pack.sima&v=grass-toggles",
  );
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );

  await page.click("#assists-open");
  await page.click("#tab-graphics");
  await expect(page.locator("#grass-texture-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#grass-shader-toggle")).toHaveAttribute("aria-pressed", "false");

  await page.locator("#grass-texture-toggle").dispatchEvent("click");
  await page.locator("#grass-shader-toggle").dispatchEvent("click");
  await expect(page.locator("#grass-texture-toggle")).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator("#grass-shader-toggle")).toHaveAttribute("aria-pressed", "true");
  await page.waitForTimeout(300);
  expect(pageErrors).toEqual([]);
});
