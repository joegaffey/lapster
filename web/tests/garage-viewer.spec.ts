import { expect, test } from "@playwright/test";

test("garage viewer renders the car on the track", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/garage/");
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
  await page.waitForTimeout(750);

  const state = await page.evaluate(() => ({
    ready: (window as any).__garageViewerReady,
    error: (window as any).__garageViewerError,
    status: document.querySelector("#status")?.textContent,
    car: document.querySelector("#car-stats")?.textContent ?? "",
    wheels: document.querySelector("#wheels")?.textContent ?? "",
    stats: document.querySelector("#stats")?.textContent ?? "",
    meshes: document.querySelector("#stats")?.textContent ?? "",
  }));

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(state.status).toBe("ready");
  expect(state.car).toContain("mass1200 kg");
  expect(state.car).toContain("wheelbase2.60 m");
  expect(state.car).toContain("track width1.80 / 1.80");
  expect(state.car).toContain("steering wheelyes");

  for (const wheel of ["FL front", "FR front", "RL rear drive", "RR rear drive"]) {
    expect(state.wheels).toContain(wheel);
  }
  expect(state.stats).toContain("vertices650");
  expect(state.stats).toContain("triangles534");
  expect(state.stats).toContain("track tris74");
  expect(state.stats).toContain("car tris460");
  expect(state.stats).toContain("draw calls13");
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
