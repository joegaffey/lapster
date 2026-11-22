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
  expect(state.debug.garage.spotLights).toBe(10);
  expect(state.debug.garage.floorTriangles).toBeGreaterThan(0);
  expect(state.bay.layout).toBe("line");
  expect(state.stats).toContain("track tris0");
  expect(state.stats).toContain("car tris");
  expect(state.debug.cars.map((car: any) => car.id)).toEqual([
    "mazda-rx7",
    "sc-deckard-conejo",
    "67gp-guthrie-eagle",
  ]);
  expect(state.debug.stats.carDrawCalls).toBeGreaterThan(100);
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

test("garage viewer renders the track-backed showroom on request", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(
    "/web/garage/?garage=showroom&track=../../fixtures/tracks/minimal/pack.sima&v=cp6",
  );
  await waitForGarage(page);
  await page.waitForTimeout(750);

  const state = await page.evaluate(() => ({
    error: (window as any).__garageViewerError,
    bay: JSON.parse((window as any).__garageViewer.bay_info()),
    debug: JSON.parse((window as any).__garageViewer.debug_info()),
  }));

  expect(state.error).toBeUndefined();
  expect(state.bay.layout).toBe("grid");
  expect(state.debug.stats.trackTriangles).toBe(6230);
  expect(state.debug.stats.carDrawCalls).toBeGreaterThan(100);
  expect(pageErrors).toEqual([]);
});
