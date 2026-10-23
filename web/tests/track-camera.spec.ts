import { expect, test } from "@playwright/test";

test("track viewer camera orbit, zoom, and reset", async ({ page }) => {
  await page.goto("/web/track-viewer/?track=../../fixtures/tracks/minimal/pack.sima&v=cp6");
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const initial = await page.evaluate(() =>
    JSON.parse((window as any).__trackViewer.camera_info()),
  );

  await page.mouse.move(700, 400);
  await page.mouse.down();
  await page.mouse.move(760, 430, { steps: 5 });
  await page.mouse.up();

  const orbited = await page.evaluate(() =>
    JSON.parse((window as any).__trackViewer.camera_info()),
  );
  expect(orbited.eye).not.toEqual(initial.eye);

  await page.mouse.move(760, 430);
  await page.mouse.wheel(0, -500);
  const zoomed = await page.evaluate(() =>
    JSON.parse((window as any).__trackViewer.camera_info()),
  );
  expect(zoomed.eye).not.toEqual(orbited.eye);

  await page.click("#reset-camera");
  const reset = await page.evaluate(() =>
    JSON.parse((window as any).__trackViewer.camera_info()),
  );
  expect(reset.eye).toEqual(initial.eye);
  expect(reset.target).toEqual(initial.target);
});
