import { expect, test } from "@playwright/test";

const waitForGarage = (page: import("@playwright/test").Page) =>
  page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );

test("garage viewer defaults to the concrete scene", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/garage/?v=cp6");
  await waitForGarage(page);
  await page.waitForTimeout(750);
  await expect(page.locator("#hud")).toHaveClass(/hud-collapsed/);

  const state = await page.evaluate(() => ({
    ready: (window as any).__garageViewerReady,
    error: (window as any).__garageViewerError,
    status: document.querySelector("#status")?.textContent,
    car: document.querySelector("#car-stats")?.textContent ?? "",
    wheels: document.querySelector("#wheels")?.textContent ?? "",
    stats: document.querySelector("#stats")?.textContent ?? "",
    meshes: document.querySelector("#stats")?.textContent ?? "",
    bay: JSON.parse((window as any).__garageViewer.bay_info()),
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
  expect(state.stats).toContain("track tris0");
  expect(state.stats).toContain("car tris");
  expect(state.debug.cars.map((car: any) => car.id)).toEqual([
    "mazda-rx7",
    "67gp-cavallo-123",
    "67gp-guthrie-eagle",
    "67gp-kuiper-t81",
    "67gp-mango-ms7",
    "67gp-murasama-37a",
    "67gp-upping-ut19",
    "sc-deckard-conejo",
  ]);
  expect(state.debug.stats.carDrawCalls).toBeGreaterThan(50);
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

  const initial = await page.evaluate(() =>
    JSON.parse((window as any).__garageViewer.bay_info()),
  );
  expect(initial.carId).toBe("mazda-rx7");
  expect(initial.paint).toBe(0);
  await expect(page.locator("#paint-panel")).toBeVisible();
  await expect(page.locator("#paint-options .paint-swatch")).toHaveCount(10);
  await expect(page.locator("#paint-label")).toHaveText("Red");

  await page.locator('#paint-options .paint-swatch[aria-label="Blue"]').click();

  const blue = await page.evaluate(() => ({
    bay: JSON.parse((window as any).__garageViewer.bay_info()),
    paint: localStorage.getItem("lapster.garage.paint.v1"),
  }));
  expect(blue.bay.paint).toBe(1);
  expect(blue.bay.paintName).toBe("Blue");
  expect(blue.paint).toBe('{"mazda-rx7":1}');
  await expect(page.locator("#paint-label")).toHaveText("Blue");
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
  await expect(page.locator("#paint-label")).toHaveText("Factory livery");
  await expect(page.locator("#paint-options")).toBeHidden();

  expect(pageErrors).toEqual([]);
});
