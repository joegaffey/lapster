import { expect, test } from "@playwright/test";

test("skidpad pack includes solid wall collision", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto(
    "/web/track-viewer/?track=../../fixtures/tracks/skidpad05/pack.sima&v=wall-collision",
  );
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const info = await page.evaluate(() => ({
    error: (window as any).__trackViewerError,
    debug: JSON.parse((window as any).__trackViewer.debug_info()),
  }));

  expect(info.error).toBeUndefined();
  expect(info.debug.stats.collisionTriangles).toBeGreaterThan(70_000);
  expect(
    info.debug.surfaces.some((surface: { name: string }) => surface.name === "wall"),
  ).toBe(true);
  expect(pageErrors).toEqual([]);
});
