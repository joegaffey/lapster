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
  await expect(page.locator('[data-car-id="sc-deckard-conejo"]')).toBeVisible();
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

test("launcher car selection persists into drive and garage", async ({ page }) => {
  await page.goto("/web/");
  const deckard = page.locator('[data-car-id="sc-deckard-conejo"]');
  await expect(deckard).toBeVisible();
  await deckard.click();
  await page.waitForURL(/\/web\/drive\//);

  expect(new URL(page.url()).searchParams.get("car")).toContain("sc-deckard-conejo");
  expect(await page.evaluate(() => localStorage.getItem("lapster.car.v1"))).toBe(
    "sc-deckard-conejo",
  );

  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
  const drive = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    car: JSON.parse((window as any).__driveViewer.telemetry()).car,
    audio: (window as any).__driveAudio?.engineId,
  }));
  expect(drive.error).toBeUndefined();
  expect(drive.car).toBe("sc-deckard-conejo");
  expect(drive.audio).toBe("v8cross");

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
  expect(garage.asset).toContain("sc-deckard-conejo");
});

test("garage car picker loads a catalog car", async ({ page }) => {
  await page.goto("/web/garage/");
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
  await expect(page.locator("#car-select")).toHaveValue("mazda-rx7");

  await page.selectOption("#car-select", "sc-deckard-conejo");
  await page.waitForURL(/car=.*sc-deckard-conejo/);
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
  const state = await page.evaluate(() => ({
    error: (window as any).__garageViewerError,
    asset: document.querySelector("#asset")?.textContent,
    selected: (document.querySelector("#car-select") as HTMLSelectElement).value,
    stored: localStorage.getItem("lapster.car.v1"),
  }));
  expect(state.error).toBeUndefined();
  expect(state.asset).toContain("sc-deckard-conejo");
  expect(state.selected).toBe("sc-deckard-conejo");
  expect(state.stored).toBe("sc-deckard-conejo");
});
