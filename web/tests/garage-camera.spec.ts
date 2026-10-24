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

  const initialBay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(initialBay.count).toBe(10);
  expect(initialBay.selected).toBe(0);

  await page.click("#bay-next");
  const nextBay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(nextBay.selected).toBe(1);
  await page.waitForTimeout(800);
  const nextCamera = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  expect(nextCamera.target).not.toEqual(initial.target);

  await page.keyboard.press("ArrowRight");
  const keyboardBay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(keyboardBay.selected).toBe(2);
});

test("garage camera flies between bays", async ({ page }) => {
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
  await page.click("#bay-next");
  await page.waitForTimeout(150);
  const midFlight = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  await page.waitForTimeout(800);
  const final = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );

  expect(midFlight.target).not.toEqual(initial.target);
  expect(midFlight.target).not.toEqual(final.target);
  expect(final.target).not.toEqual(initial.target);
});
