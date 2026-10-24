import { expect, test } from "@playwright/test";

test("garage camera orbits, zooms, and resets", async ({ page }) => {
  await page.goto(
    "/web/garage/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const initial = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );

  await page.mouse.move(700, 400);
  await page.mouse.down();
  await page.mouse.move(760, 430, { steps: 5 });
  await page.mouse.up();

  const orbited = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  expect(orbited.eye).not.toEqual(initial.eye);
  expect(orbited.target).toEqual(initial.target);

  await page.mouse.move(760, 430);
  await page.mouse.wheel(0, -500);
  const zoomed = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  expect(zoomed.eye).not.toEqual(orbited.eye);

  await page.mouse.move(700, 400);
  await page.mouse.down();
  await page.mouse.move(700, 800, { steps: 10 });
  await page.mouse.up();
  const low = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  expect(low.eye[1]).toBeGreaterThan(initial.target[1]);

  await page.click("#reset-camera");
  const reset = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  expect(reset.eye).toEqual(initial.eye);
  expect(reset.target).toEqual(initial.target);
});
