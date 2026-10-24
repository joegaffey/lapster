import { expect, test } from "@playwright/test";

test("selected garage car becomes the driving car", async ({ page }) => {
  const query =
    "?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6";
  await page.goto(`/web/garage/${query}`);
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
  expect(new URL(driveHref!).searchParams.get("bay")).toBe("3");
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
