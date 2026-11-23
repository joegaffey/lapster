import { expect, test } from "@playwright/test";

// Let the selected bay's 0.9 s flicker finish before screenshotting.
const GARAGE_SETTLE_MS = 1_500;

const waitForGarage = (page: import("@playwright/test").Page) =>
  page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );

const waitForSelectedBay = (page: import("@playwright/test").Page) =>
  page.waitForFunction(
    () => JSON.parse((window as any).__garageViewer.bay_info()).loaded === true,
    undefined,
    { timeout: 30_000 },
  );

test("garage viewer defaults to the concrete scene", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/garage/?v=cp6");
  await waitForGarage(page);
  await waitForSelectedBay(page);
  await page.waitForTimeout(GARAGE_SETTLE_MS);
  await expect(page.locator("#hud")).toHaveClass(/hud-collapsed/);
  await expect(page.locator("#car-loading")).toHaveAttribute("aria-hidden", "true");
  await expect(page.locator("#car-loading")).not.toHaveClass(/car-loading-visible/);

  const state = await page.evaluate(() => ({
    ready: (window as any).__garageViewerReady,
    error: (window as any).__garageViewerError,
    status: document.querySelector("#status")?.textContent,
    car: document.querySelector("#car-stats")?.textContent ?? "",
    wheels: document.querySelector("#wheels")?.textContent ?? "",
    stats: document.querySelector("#stats")?.textContent ?? "",
    meshes: document.querySelector("#stats")?.textContent ?? "",
    bay: JSON.parse((window as any).__garageViewer.bay_info()),
    bays: JSON.parse((window as any).__garageViewer.bay_list()),
    debug: JSON.parse((window as any).__garageViewer.debug_info()),
  }));

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(state.status).toBe("ready");
  expect(state.car).toContain("mass1270 kg");
  expect(state.car).toContain("wheelbase2.45 m");
  expect(state.car).toContain("track width1.73 / 1.73");
  expect(state.car).toContain("steering wheelno");

  for (const wheel of ["FL front", "FR front", "RL rear drive", "RR rear drive"]) {
    expect(state.wheels).toContain(wheel);
  }
  expect(state.debug.asset.id).toBe("concrete-garage");
  expect(state.debug.garage.mode).toBe("concrete");
  expect(state.debug.garage.layout).toBe("line");
  expect(state.debug.garage.spotLights).toBe(8);
  expect(state.debug.garage.floorTriangles).toBeGreaterThan(0);
  expect(state.bay.layout).toBe("line");
  expect(state.bay.loaded).toBe(true);
  expect(state.bay.carId).toBe("mazda-rx7");
  expect(state.stats).toContain("track tris0");
  expect(state.stats).toContain("car tris");
  expect(state.bays.map((bay: any) => bay.carId)).toEqual([
    "mazda-rx7",
    "67gp-cavallo-123",
    "67gp-guthrie-eagle",
    "67gp-kuiper-t81",
    "67gp-mango-ms7",
    "67gp-murasama-37a",
    "67gp-upping-ut19",
    "sc-deckard-conejo",
  ]);
  expect(state.debug.car.id).toBe("mazda-rx7");
  expect(pageErrors).toEqual([]);

  if (!process.env.CI) {
    await expect(page).toHaveScreenshot("garage-viewer.png", {
      mask: [page.locator("#timing")],
      maskColor: "#0a0d11",
    });
  }

  const angles = await page.evaluate(() => {
    const viewer = (window as any).__garageViewer;
    viewer.set_wheel_angles(0.4, 1.0);
    return { steer: viewer.steer_angle(), spin: viewer.spin_angle() };
  });
  expect(angles.steer).toBeCloseTo(0.4, 4);
  expect(angles.spin).toBeCloseTo(1.0, 4);
});

test("garage paint picker updates the selected car and drive link", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/garage/?v=cp6");
  await waitForGarage(page);
  await waitForSelectedBay(page);

  const initial = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(initial.carId).toBe("mazda-rx7");
  expect(initial.paint).toBe(0);

  const panel = page.locator("#paint-panel");
  await expect(panel).toHaveClass(/paint-panel-visible/);
  await expect(panel).not.toHaveAttribute("aria-hidden", "true");
  await expect(panel).not.toHaveAttribute("inert", "");
  await expect(panel).toHaveCSS("opacity", "1");
  expect((await panel.innerText()).trim()).toBe("");
  await expect(page.locator("#paint-options .paint-swatch")).toHaveCount(10);
  await expect(
    page.locator('#paint-options .paint-swatch[aria-checked="true"]'),
  ).toHaveAttribute("aria-label", "Red");

  await page.locator('#paint-options .paint-swatch[aria-label="Blue"]').click();

  const blue = await page.evaluate(() => ({
    bay: JSON.parse((window as any).__garageViewer.bay_info()),
    paint: localStorage.getItem("lapster.garage.paint.v1"),
  }));
  expect(blue.bay.paint).toBe(1);
  expect(blue.bay.paintName).toBe("Blue");
  expect(blue.paint).toBe('{"mazda-rx7":1}');
  await expect(
    page.locator('#paint-options .paint-swatch[aria-checked="true"]'),
  ).toHaveAttribute("aria-label", "Blue");
  const driveHref = await page.locator("#drive-car").getAttribute("href");
  expect(new URL(driveHref!).searchParams.get("paint")).toBe("1");

  await page.mouse.move(640, 360);
  await page.click("#bay-next");
  const cavallo = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(cavallo.selected).toBe(1);
  expect(cavallo.carId).toBe("67gp-cavallo-123");
  expect(cavallo.recolorable).toBe(false);
  expect(cavallo.paintName).toBe("Factory livery");
  await expect(panel).toHaveAttribute("aria-hidden", "true");
  await expect(panel).toHaveAttribute("inert", "");
  await expect(panel).not.toHaveClass(/paint-panel-visible/);
  await expect(panel).toHaveCSS("opacity", "0");
  await expect(
    page.locator('#paint-options .paint-swatch[aria-checked="true"]'),
  ).toHaveCount(0);

  expect(pageErrors).toEqual([]);
});

test("garage enables textured paint for the Deckard only", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.addInitScript(() => {
    localStorage.setItem("lapster.car.v1", "sc-deckard-conejo");
  });

  await page.goto("/web/garage/?v=cp6&bay=8");
  await waitForGarage(page);
  await waitForSelectedBay(page);

  const initial = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(initial.carId).toBe("sc-deckard-conejo");
  expect(initial.recolorable).toBe(true);
  expect(initial.paint).toBe(0);
  expect(initial.paintName).toBe("Red");

  const panel = page.locator("#paint-panel");
  await expect(panel).toHaveClass(/paint-panel-visible/);
  await expect(panel).not.toHaveAttribute("aria-hidden", "true");
  await expect(page.locator("#paint-options .paint-swatch")).toHaveCount(10);
  await expect(
    page.locator('#paint-options .paint-swatch[aria-checked="true"]'),
  ).toHaveAttribute("aria-label", "Red");

  await page.locator('#paint-options .paint-swatch[aria-label="Blue"]').click();

  const blue = await page.evaluate(() => ({
    bay: JSON.parse((window as any).__garageViewer.bay_info()),
    paint: localStorage.getItem("lapster.garage.paint.v1"),
  }));
  expect(blue.bay.paint).toBe(1);
  expect(blue.bay.paintName).toBe("Blue");
  expect(blue.paint).toBe('{"sc-deckard-conejo":1}');
  await expect(
    page.locator('#paint-options .paint-swatch[aria-checked="true"]'),
  ).toHaveAttribute("aria-label", "Blue");

  const driveHref = await page.locator("#drive-car").getAttribute("href");
  expect(new URL(driveHref!).searchParams.get("paint")).toBe("1");

  expect(pageErrors).toEqual([]);
});

test("garage hides the paint picker and shows a loader for pending cars", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.addInitScript(() => {
    const marker = "67gp-cavallo-123.sima";
    let released = false;
    const waiters: Array<() => void> = [];
    (window as any).__releaseCavalloLoaderTest = () => {
      released = true;
      for (const resolve of waiters.splice(0)) {
        resolve();
      }
    };
    const originalFetch = window.fetch.bind(window);
    window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof Request
            ? input.url
            : String(input);
      if (!released && url.includes(marker)) {
        await new Promise<void>((resolve) => waiters.push(resolve));
      }
      return originalFetch(input, init);
    };
  });

  await page.goto("/web/garage/?v=cp6");
  await waitForGarage(page);

  const panel = page.locator("#paint-panel");
  await page.mouse.move(640, 360);
  await page.click("#bay-next");

  try {
    const pending = await page.evaluate(() => ({
      bay: JSON.parse((window as any).__garageViewer.bay_info()),
      loaded: (window as any).__garageCarsLoaded,
      total: (window as any).__garageCarsTotal,
    }));
    expect(pending.bay.selected).toBe(1);
    expect(pending.bay.loaded).toBe(false);
    expect(pending.loaded).toBeLessThan(pending.total);

    await expect(panel).toHaveAttribute("aria-hidden", "true");
    await expect(panel).toHaveAttribute("inert", "");
    await expect(panel).not.toHaveClass(/paint-panel-visible/);
    await expect(panel).toHaveCSS("opacity", "0");
    await expect(page.locator("#car-loading")).toHaveClass(/car-loading-visible/);
    await expect(page.locator("#car-loading")).not.toHaveAttribute("aria-hidden", "true");
    await expect(page.locator("#car-loading")).toContainText("Loading Cavallo 123");
  } finally {
    await page.evaluate(() => (window as any).__releaseCavalloLoaderTest?.());
  }

  await page.waitForFunction(
    () => JSON.parse((window as any).__garageViewer.bay_info()).loaded === true,
    undefined,
    { timeout: 15_000 },
  );
  await expect(panel).toHaveAttribute("aria-hidden", "true");
  await expect(panel).not.toHaveClass(/paint-panel-visible/);

  await expect(page.locator("#car-loading")).toHaveAttribute("aria-hidden", "true");

  expect(pageErrors).toEqual([]);
});
