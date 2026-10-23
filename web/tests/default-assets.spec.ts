import { expect, test } from "@playwright/test";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

test("default demos use skidpad and the rx7 without url params", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/");
  await waitForDrive(page);
  await page.waitForTimeout(500);
  const drive = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
  }));
  expect(drive.error).toBeUndefined();
  expect(drive.telemetry.car).toBe("mazda-rx7");
  expect(drive.telemetry.track).toBe("skidpad05");
  expect(drive.telemetry.contacts).toBe(4);

  await page.goto("/web/garage/");
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
  const garage = await page.evaluate(() => ({
    error: (window as any).__garageViewerError,
    asset: document.querySelector("#asset")?.textContent,
    stats: document.querySelector("#stats")?.textContent,
  }));
  expect(garage.error).toBeUndefined();
  expect(garage.asset).toContain("skidpad05 + mazda-rx7");
  expect(garage.stats).toContain("draw calls35");

  await page.goto("/web/track-viewer/");
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );
  const track = await page.evaluate(() => ({
    error: (window as any).__trackViewerError,
    asset: document.querySelector("#asset")?.textContent,
    stats: document.querySelector("#stats")?.textContent,
  }));
  expect(track.error).toBeUndefined();
  expect(track.asset).toContain("skidpad05");
  expect(track.stats).toContain("draw calls30");
  expect(pageErrors).toEqual([]);
});
