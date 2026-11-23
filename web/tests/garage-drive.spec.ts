import { expect, test } from "@playwright/test";

test("selected garage car becomes the driving car", async ({ page }) => {
  await page.goto("/web/garage/");
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );

  await page.mouse.move(640, 360);
  await page.click("#bay-next");
  await page.click("#bay-next");

  const bay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(bay.selected).toBe(2);
  expect(bay.colorName).toBe("Yellow");

  const driveHref = await page.locator("#drive-car").getAttribute("href");
  const driveUrl = new URL(driveHref!);
  expect(driveUrl.searchParams.get("bay")).toBe("3");
  expect(driveUrl.searchParams.get("car")).toContain("67gp-cavallo-123");
  expect(driveUrl.searchParams.has("paint")).toBe(false);
  expect(await page.evaluate(() => localStorage.getItem("lapster.garage.car.v1"))).toBe("2");

  await page.click("#drive-car");
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.carBay).toBe(2);
  expect(telemetry.carColor).toBe("Yellow");
  await expect(page.locator("#asset")).toContainText("Yellow");

  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
  const persisted = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(persisted.carBay).toBe(2);
  expect(persisted.carColor).toBe("Yellow");
});

test("Deckard paint selected in the garage follows into Drive", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.addInitScript(() => {
    localStorage.setItem("lapster.car.v1", "sc-deckard-conejo");
  });

  await page.goto("/web/garage/?v=cp6&bay=2");
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
  await page.waitForFunction(
    () => JSON.parse((window as any).__garageViewer.bay_info()).loaded === true,
    undefined,
    { timeout: 30_000 },
  );

  const bay = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(bay.carId).toBe("sc-deckard-conejo");

  await page.locator('#paint-options .paint-swatch[aria-label="Blue"]').click();

  const driveHref = await page.locator("#drive-car").getAttribute("href");
  expect(new URL(driveHref!).searchParams.get("paint")).toBe("1");

  await page.click("#drive-car");
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const telemetry = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    car: JSON.parse((window as any).__driveViewer.telemetry()),
  }));
  expect(telemetry.error).toBeUndefined();
  expect(telemetry.car.car).toBe("sc-deckard-conejo");
  expect(telemetry.car.carColor).toBe("Blue");
  expect(telemetry.car.texturePaint).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("Deckard without an explicit paint keeps its factory texture", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.addInitScript(() => {
    localStorage.setItem("lapster.garage.paint.v1", '{"mazda-rx7":1}');
  });

  await page.goto(
    "/web/drive/?car=../../fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima&v=cp6",
  );
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const telemetry = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    car: JSON.parse((window as any).__driveViewer.telemetry()),
  }));
  expect(telemetry.error).toBeUndefined();
  expect(telemetry.car.car).toBe("sc-deckard-conejo");
  expect(telemetry.car.texturePaint).toBe(false);
  expect(pageErrors).toEqual([]);
});
