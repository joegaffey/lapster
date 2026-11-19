import path from "node:path";
import { expect, test } from "@playwright/test";

test("track-ed files compile in the browser and open Test drive", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  const fixture = path.resolve("..", "fixtures", "tracks", "drift-sketchfab");
  await page.goto("/web/track-import/?v=track-import");
  await page.waitForFunction(
    () => (window as any).__trackImportReady === true || (window as any).__trackImportError,
    undefined,
    { timeout: 30_000 },
  );
  await page.setInputFiles("#track-json", path.join(fixture, "track.json"));
  await page.setInputFiles("#track-glb", path.join(fixture, "drift_tracks.glb"));
  await page.locator("#import-button").click();
  await page.waitForURL(/\/web\/drive\/\?[^#]*track=track-ed/, {
    timeout: 60_000,
  });
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 45_000 },
  );

  const result = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    asset: JSON.parse((window as any).__driveViewer.info()).asset,
    surfaces: JSON.parse((window as any).__driveViewer.debug_info()).surfaces,
  }));

  expect(result.error).toBeUndefined();
  expect(result.asset.id).toBe("track-ed");
  expect(result.surfaces.some((surface: { name: string }) => surface.name === "kerb")).toBe(
    true,
  );
  expect(pageErrors).toEqual([]);
});
