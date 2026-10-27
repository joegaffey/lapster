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

  await page.mouse.move(640, 360);
  const initialBay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(initialBay.count).toBe(10);
  expect(initialBay.selected).toBe(0);

  await page.click("#bay-next");
  const nextBay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(nextBay.selected).toBe(5);
  await page.waitForTimeout(800);
  const nextCamera = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );
  expect(nextCamera.target).not.toEqual(initial.target);

  await page.keyboard.press("ArrowRight");
  const keyboardBay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(keyboardBay.selected).toBe(6);
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
  await page.mouse.move(640, 360);
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

test("garage bay navigation reverses order in the second row", async ({ page }) => {
  await page.goto(
    "/web/garage/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6&bay=5",
  );
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const bay = page.locator("#bay-label");
  await expect(bay).toContainText("Bay 5");
  await page.click("#bay-next");
  await expect(bay).toContainText("Bay 4");
  await page.click("#bay-next");
  await expect(bay).toContainText("Bay 3");
  await page.click("#bay-prev");
  await expect(bay).toContainText("Bay 4");
  await page.click("#bay-prev");
  await expect(bay).toContainText("Bay 5");
});

test("garage car arrows use the screen sides and toggle on mouse movement", async ({ page }) => {
  await page.goto(
    "/web/garage/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const nav = page.locator("#car-nav");
  await expect(nav).toHaveClass(/car-nav-hidden/);
  await expect(page.locator("#bay-prev")).toHaveAttribute("aria-label", "Previous car");
  await expect(page.locator("#bay-next")).toHaveAttribute("aria-label", "Next car");

  await page.mouse.move(640, 360);
  await expect(nav).not.toHaveClass(/car-nav-hidden/);

  const viewport = page.viewportSize()!;
  const previous = await page.locator("#bay-prev").boundingBox();
  const next = await page.locator("#bay-next").boundingBox();
  expect(previous!.x).toBeLessThan(80);
  expect(next!.x + next!.width).toBeGreaterThan(viewport.width - 80);

  await page.waitForTimeout(2500);
  await expect(nav).toHaveClass(/car-nav-hidden/);

  await page.mouse.move(650, 360);
  await expect(nav).not.toHaveClass(/car-nav-hidden/);
});

test.describe("mobile car navigation", () => {
  test.use({ hasTouch: true, isMobile: true, viewport: { width: 390, height: 844 } });

  test("garage car arrows start hidden and reveal on touch", async ({ page }) => {
    await page.goto(
      "/web/garage/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
    );
    await page.waitForFunction(
      () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
      undefined,
      { timeout: 30_000 },
    );

    const nav = page.locator("#car-nav");
    await expect(nav).toHaveClass(/car-nav-hidden/);

    await page.touchscreen.tap(195, 420);
    await expect(nav).not.toHaveClass(/car-nav-hidden/);

    await page.waitForTimeout(2500);
    await expect(nav).toHaveClass(/car-nav-hidden/);
  });
});

test("garage camera orbits but does not pan", async ({ page }) => {
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
  await page.keyboard.down("Shift");
  await page.mouse.down();
  await page.mouse.move(760, 430, { steps: 5 });
  await page.mouse.up();
  await page.keyboard.up("Shift");
  const orbited = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.camera_info()),
  );

  expect(orbited.eye).not.toEqual(initial.eye);
  expect(orbited.target).toEqual(initial.target);
});
