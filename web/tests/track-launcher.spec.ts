import { expect, test } from "@playwright/test";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function waitForGarage(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

test("drive launcher lists the solo tracks", async ({ page }) => {
  await page.goto("/web/");
  const driveTrack = page.locator("#drive-track");
  await expect(driveTrack).toBeVisible();

  const values = await driveTrack
    .locator("option")
    .evaluateAll((options) =>
      options.map((option) => (option as HTMLOptionElement).value),
    );
  expect(values).toEqual([
    "skidpad05",
    "drift-sketchfab",
    "loop-spline-point-terrain",
  ]);

  await expect(driveTrack.locator('option[value="skidpad05"]')).toHaveText(
    "Skidpad 0.5",
  );
  await expect(driveTrack.locator('option[value="drift-sketchfab"]')).toHaveText(
    "Drift Tracks",
  );
  await expect(
    driveTrack.locator('option[value="loop-spline-point-terrain"]'),
  ).toContainText("Loop Spline");
  await expect(driveTrack.locator('option[value="race-loop"]')).toHaveCount(0);
});

test("race launcher offers only the loop and flags it under development", async ({
  page,
}) => {
  await page.goto("/web/");
  const raceForm = page.locator("#race-form");
  await expect(raceForm.locator(".panel-badge")).toHaveText(/Under development/i);

  const raceTrack = page.locator("#race-track");
  await expect(raceTrack).toBeVisible();
  const values = await raceTrack
    .locator("option")
    .evaluateAll((options) =>
      options.map((option) => (option as HTMLOptionElement).value),
    );
  expect(values).toEqual(["loop-spline-point-terrain"]);
  await expect(
    raceTrack.locator('option[value="loop-spline-point-terrain"]'),
  ).toContainText("Loop Spline");
  await expect(raceTrack).toHaveValue("loop-spline-point-terrain");
});

test("drive launcher starts a solo drive on the drift tracks", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/");
  await page.locator("#drive-track").selectOption("drift-sketchfab");
  await page.locator("#drive-form button[type=submit]").click();
  await page.waitForURL(/\/web\/drive\//);

  const url = new URL(page.url());
  expect(url.searchParams.get("track")).toBe("drift-sketchfab");
  expect(url.searchParams.get("race")).toBe("0");
  expect(await page.evaluate(() => localStorage.getItem("lapster.track.v1"))).toBe(
    "drift-sketchfab",
  );

  await waitForDrive(page);
  const drive = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
  }));
  expect(drive.error).toBeUndefined();
  expect(drive.telemetry.track).toBe("drift-sketchfab");
  expect(drive.telemetry.aiCount).toBe(0);
  await expect(page.locator("#race-panel")).toBeHidden();
  await expect(page.locator("#race-countdown")).toBeHidden();
  await expect(page.locator("#race-results")).toBeHidden();
  expect(pageErrors).toEqual([]);
});

test("race launcher starts a three-AI race on the loop", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/");
  await page.locator("#race-track").selectOption("loop-spline-point-terrain");
  await page.locator("#race-form button[type=submit]").click();
  await page.waitForURL(/\/web\/drive\//);

  const url = new URL(page.url());
  expect(url.searchParams.get("track")).toBe("loop-spline-point-terrain");
  expect(url.searchParams.get("race")).toBe("1");
  expect(
    await page.evaluate(() => localStorage.getItem("lapster.race.track.v1")),
  ).toBe("loop-spline-point-terrain");

  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  await page.waitForFunction(
    () => {
      const label = document.querySelector("#race-countdown");
      return label !== null && !label.hasAttribute("hidden");
    },
    undefined,
    { timeout: 10_000 },
  );

  const drive = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
  }));
  expect(drive.error).toBeUndefined();
  expect(drive.telemetry.track).toBe("loop-spline-point-terrain");
  expect(drive.telemetry.raceAvailable).toBe(true);
  expect(drive.telemetry.aiCount).toBe(3);
  await expect(page.locator("#race-panel")).toBeVisible();
  await expect(page.locator("#race-countdown")).toBeVisible();
  expect(pageErrors).toEqual([]);
});

test("launcher shows the stored car and opens the garage with it selected", async ({
  page,
}) => {
  await page.goto("/web/");
  await page.evaluate(() =>
    localStorage.setItem("lapster.car.v1", "sc-deckard-conejo"),
  );
  await page.reload();

  await expect(
    page.locator("#drive-form [data-selected-car-name]"),
  ).toHaveText("Deckard Conejo");
  await expect(
    page.locator("#race-form [data-selected-car-name]"),
  ).toHaveText("Deckard Conejo");
  await expect(page.locator("#drive-form .car-summary")).toHaveAttribute(
    "href",
    "./garage/",
  );

  await page.locator("#drive-form .car-summary").click();
  await page.waitForURL(/\/web\/garage\//);
  await waitForGarage(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();

  const garage = await page.evaluate(() => ({
    error: (window as any).__garageViewerError,
    selected: JSON.parse((window as any).__garageViewer.bay_info()),
  }));
  expect(garage.error).toBeUndefined();
  expect(garage.selected.carId).toBe("sc-deckard-conejo");
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

test("garage mixes catalog cars across bays", async ({ page }) => {
  await page.goto("/web/garage/");
  await waitForGarage(page);
  await page.waitForFunction(
    () => JSON.parse((window as any).__garageViewer.bay_info()).loaded === true,
    undefined,
    { timeout: 30_000 },
  );

  const initial = await page.evaluate(() => {
    const viewer = (window as any).__garageViewer;
    return {
      error: (window as any).__garageViewerError,
      bays: JSON.parse(viewer.bay_list()),
      selected: JSON.parse(viewer.bay_info()),
      menuButtons: document.querySelectorAll("#car-list button").length,
    };
  });
  expect(initial.error).toBeUndefined();
  expect(initial.bays.map((bay: any) => bay.carId)).toEqual([
    "mazda-rx7",
    "67gp-cavallo-123",
    "67gp-guthrie-eagle",
    "67gp-kuiper-t81",
    "67gp-mango-ms7",
    "67gp-murasama-37a",
    "67gp-upping-ut19",
    "sc-deckard-conejo",
  ]);
  expect(initial.menuButtons).toBe(0);
  expect(initial.selected.carId).toBe("mazda-rx7");
  expect(initial.selected.loaded).toBe(true);

  await page.keyboard.press("ArrowRight");
  await page.waitForFunction(
    () => JSON.parse((window as any).__garageViewer.bay_info()).loaded === true,
    undefined,
    { timeout: 30_000 },
  );
  const state = await page.evaluate(() => {
    const viewer = (window as any).__garageViewer;
    return {
      info: JSON.parse(viewer.bay_info()),
      asset: document.querySelector("#asset")?.textContent,
      drive: document.querySelector("#drive-car")?.getAttribute("href"),
      stored: localStorage.getItem("lapster.car.v1"),
    };
  });
  expect(state.info.carId).toBe("67gp-cavallo-123");
  expect(state.info.car.id).toBe("67gp-cavallo-123");
  expect(state.asset).toContain("67gp-cavallo-123");
  expect(state.drive).toContain("67gp-cavallo-123");
  expect(state.stored).toBe("67gp-cavallo-123");

  await page.keyboard.press("ArrowRight");
  await page.waitForFunction(
    () => JSON.parse((window as any).__garageViewer.bay_info()).loaded === true,
    undefined,
    { timeout: 30_000 },
  );
  const after = await page.evaluate(() => {
    const viewer = (window as any).__garageViewer;
    return {
      info: JSON.parse(viewer.bay_info()),
      asset: document.querySelector("#asset")?.textContent,
      drive: document.querySelector("#drive-car")?.getAttribute("href"),
      stored: localStorage.getItem("lapster.car.v1"),
    };
  });
  expect(after.info.carId).toBe("67gp-guthrie-eagle");
  expect(after.info.car.id).toBe("67gp-guthrie-eagle");
  expect(after.asset).toContain("67gp-guthrie-eagle");
  expect(after.drive).toContain("67gp-guthrie-eagle");
  expect(after.stored).toBe("67gp-guthrie-eagle");
});
