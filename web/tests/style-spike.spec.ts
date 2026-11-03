import { expect, test } from "@playwright/test";

const waitForReady = (page: import("@playwright/test").Page) =>
  page.waitForFunction(
    () => (window as any).__styleSpikeReady === true || (window as any).__styleSpikeError,
    undefined,
    { timeout: 45_000 },
  );

test("style spike renders a track section with procedural vegetation", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(
    "/web/style-spike/?track=../../fixtures/tracks/drift-sketchfab/pack.sima&quality=high&v=style",
  );
  await waitForReady(page);
  await page.waitForTimeout(1000);

  const state = await page.evaluate(() => {
    const viewer = (window as any).__styleSpike;
    return {
      ready: (window as any).__styleSpikeReady,
      error: (window as any).__styleSpikeError,
      style: JSON.parse(viewer.style_info()),
      debug: JSON.parse(viewer.debug_info()),
      trees: document.querySelector("#trees")?.textContent,
      bushes: document.querySelector("#bushes")?.textContent,
      tier: document.querySelector("#applied-tier")?.textContent,
      status: document.querySelector("#status")?.textContent,
    };
  });

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(state.status).toBe("ready");
  expect(state.style.vegetation.trees).toBeGreaterThan(0);
  expect(state.style.vegetation.bushes).toBeGreaterThan(0);
  expect(state.style.vegetation.nearMeshes).toBeGreaterThan(0);
  expect(state.debug.stats.drawCalls).toBeGreaterThan(6);
  expect(state.trees).not.toBe("—");
  expect(state.bushes).not.toBe("—");
  expect(state.tier).toContain("high");
  expect(errors).toEqual([]);
});

test("style spike defaults to skidpad with grass and vegetation", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto("/web/style-spike/?quality=high&v=style-default");
  await waitForReady(page);
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => ({
    track: (document.querySelector("#track-select") as HTMLSelectElement | null)?.value,
    error: (window as any).__styleSpikeError,
    style: JSON.parse((window as any).__styleSpike.style_info()),
    debug: JSON.parse((window as any).__styleSpike.debug_info()),
  }));

  expect(state.track).toBe("skidpad");
  expect(state.error).toBeUndefined();
  expect(state.style.vegetation.trees).toBeGreaterThan(0);
  expect(state.debug.stats.grassBlades).toBeGreaterThan(200_000);
  expect(errors).toEqual([]);
});

test("style spike exposes independent grass, tree, and bush controls", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(
    "/web/style-spike/?quality=medium&grass=40&treeMass=2&bushMass=1&v=style-mass",
  );
  await waitForReady(page);
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => {
    const viewer = (window as any).__styleSpike;
    return {
      error: (window as any).__styleSpikeError,
      style: JSON.parse(viewer.style_info()),
      grassInput: (document.querySelector("#grass-density") as HTMLInputElement | null)?.value,
      treeInput: (document.querySelector("#tree-mass-slider") as HTMLInputElement | null)?.value,
      bushInput: (document.querySelector("#bush-mass-slider") as HTMLInputElement | null)?.value,
      grass: document.querySelector("#grass")?.textContent,
      treeCards: document.querySelector("#tree-cards")?.textContent,
      bushCards: document.querySelector("#bush-cards")?.textContent,
    };
  });

  expect(state.error).toBeUndefined();
  expect(state.grassInput).toBe("40");
  expect(state.treeInput).toBe("2");
  expect(state.bushInput).toBe("1");
  expect(state.style.mass.trees.maxLayers).toBeGreaterThan(10);
  expect(state.style.mass.bushes.maxLayers).toBeGreaterThan(10);
  expect(state.style.mass.trees.instances).toBeGreaterThan(100_000);
  expect(state.style.mass.bushes.instances).toBeGreaterThan(40_000);
  expect(state.style.mass.trees.activeInstances).toBeGreaterThan(0);
  expect(state.style.mass.bushes.activeInstances).toBeGreaterThan(0);
  expect(state.treeCards).not.toBe("0");
  expect(state.bushCards).not.toBe("0");
  expect(state.grass).not.toBe("off");
  expect(errors).toEqual([]);
});

test("style spike drives in car views and supports camera controls", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(
    "/web/style-spike/?quality=low&treeMass=0&bushMass=0&grass=0&v=style-cameras",
  );
  await waitForReady(page);
  await page.waitForTimeout(500);

  const initial = await page.evaluate(() => {
    const viewer = (window as any).__styleSpike;
    return {
      mode: (document.querySelector("#camera-mode") as HTMLSelectElement | null)?.value,
      car: JSON.parse(viewer.style_car_info()),
      camera: JSON.parse(viewer.camera_info()),
    };
  });
  expect(initial.mode).toBe("orbit");
  expect(initial.car).not.toBeNull();

  await page.selectOption("#camera-mode", "chase");
  await page.evaluate(() => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", { code: "KeyW", key: "w", bubbles: true }),
    );
  });
  await page.waitForTimeout(1600);
  await page.evaluate(() => {
    window.dispatchEvent(
      new KeyboardEvent("keyup", { code: "KeyW", key: "w", bubbles: true }),
    );
  });
  await page.waitForTimeout(300);

  const driven = await page.evaluate(() => {
    const viewer = (window as any).__styleSpike;
    return {
      car: JSON.parse(viewer.style_car_info()),
      speed: document.querySelector("#speed")?.textContent,
      mode: (document.querySelector("#camera-mode") as HTMLSelectElement | null)?.value,
    };
  });
  const distance = Math.hypot(
    driven.car.position[0] - initial.car.position[0],
    driven.car.position[2] - initial.car.position[2],
  );
  expect(driven.mode).toBe("chase");
  expect(distance).toBeGreaterThan(0.01);
  expect(driven.car.speedKph).toBeGreaterThan(0.5);
  expect(driven.speed).not.toBe("0.0 km/h");

  await page.selectOption("#camera-mode", "isometric");
  await page.waitForTimeout(200);
  const iso = await page.evaluate(() => ({
    mode: (document.querySelector("#camera-mode") as HTMLSelectElement | null)?.value,
    camera: JSON.parse((window as any).__styleSpike.camera_info()),
  }));
  expect(iso.mode).toBe("isometric");
  expect(iso.camera.eye[1]).toBeGreaterThan(iso.camera.target[1]);

  await page.selectOption("#camera-mode", "orbit");
  await page.waitForTimeout(200);
  const beforeOrbit = await page.evaluate(() =>
    JSON.parse((window as any).__styleSpike.camera_info()),
  );
  await page.evaluate(() => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        code: "ArrowRight",
        key: "ArrowRight",
        bubbles: true,
      }),
    );
  });
  await page.waitForTimeout(600);
  await page.evaluate(() => {
    window.dispatchEvent(
      new KeyboardEvent("keyup", {
        code: "ArrowRight",
        key: "ArrowRight",
        bubbles: true,
      }),
    );
  });
  await page.waitForTimeout(200);
  const afterOrbit = await page.evaluate(() =>
    JSON.parse((window as any).__styleSpike.camera_info()),
  );
  expect(afterOrbit.eye).not.toEqual(beforeOrbit.eye);
  expect(errors).toEqual([]);
});

test("style spike quality controls drop vegetation tiers", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });

  await page.goto(
    "/web/style-spike/?track=../../fixtures/tracks/drift-sketchfab/pack.sima&quality=high&v=style-low",
  );
  await waitForReady(page);
  await page.selectOption("#quality-select", "low");
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => ({
    tier: document.querySelector("#applied-tier")?.textContent,
    error: (window as any).__styleSpikeError,
  }));

  expect(state.error).toBeUndefined();
  expect(state.tier).toContain("low");
  expect(errors).toEqual([]);
});

test("style spike renders on the WebGL2 fallback", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.addInitScript(() => {
    Object.defineProperty(Navigator.prototype, "gpu", {
      configurable: true,
      get: () => undefined,
    });
  });

  await page.goto(
    "/web/style-spike/?track=../../fixtures/tracks/drift-sketchfab/pack.sima&quality=medium&v=style-webgl2",
  );
  await waitForReady(page);
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => ({
    error: (window as any).__styleSpikeError,
    style: JSON.parse((window as any).__styleSpike.style_info()),
    debug: JSON.parse((window as any).__styleSpike.debug_info()),
    webgl2: Boolean(document.querySelector("#view")?.getContext("webgl2")),
  }));

  expect(state.error).toBeUndefined();
  expect(state.webgl2).toBe(true);
  expect(state.style.vegetation.trees).toBeGreaterThan(0);
  expect(state.debug.stats.grassBlades).toBeGreaterThan(0);
  expect(errors).toEqual([]);
});
