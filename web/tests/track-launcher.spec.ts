import { expect, test } from "@playwright/test";

test("launcher track selection persists into drive and garage", async ({ page }) => {
  await page.goto("/web/");
  await expect(page.locator('a[href="./drive/"]')).toBeVisible();
  await expect(page.locator('[data-track-id="drift-sketchfab"]')).toBeVisible();
  await expect(page.locator('[data-track-id="skidpad05"]')).toHaveAttribute(
    "aria-current",
    "true",
  );

  await page.click('[data-track-id="drift-sketchfab"]');
  await page.waitForURL(/\/web\/drive\//);
  expect(new URL(page.url()).searchParams.get("track")).toContain("drift-sketchfab");
  expect(await page.evaluate(() => localStorage.getItem("lapster.track.v1"))).toBe(
    "drift-sketchfab",
  );

  await page.goto("/web/garage/");
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
  const garage = await page.evaluate(() => ({
    error: (window as any).__garageViewerError,
    asset: document.querySelector("#asset")?.textContent,
  }));
  expect(garage.error).toBeUndefined();
  expect(garage.asset).toContain("drift-sketchfab");

  const driveHref = await page.locator("#drive-car").getAttribute("href");
  expect(new URL(driveHref!).searchParams.get("track")).toContain("drift-sketchfab");
});

test("public launcher exposes the drift track", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator('[data-track-id="skidpad05"]')).toHaveAttribute(
    "aria-current",
    "true",
  );
  await page.click('[data-track-id="drift-sketchfab"]');
  await page.waitForURL(/\/web\/drive\//);
  expect(new URL(page.url()).searchParams.get("track")).toContain("drift-sketchfab");
  expect(await page.evaluate(() => localStorage.getItem("lapster.track.v1"))).toBe(
    "drift-sketchfab",
  );
});
