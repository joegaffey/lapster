import { expect, test } from "@playwright/test";

async function waitForReady(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function telemetry(page: import("@playwright/test").Page) {
  return page.evaluate(() => JSON.parse((window as any).__driveViewer.telemetry()));
}

test("drive viewer settles and accepts keyboard input", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(750);

  const settled = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
    status: document.querySelector("#status")?.textContent,
    speed: document.querySelector("#speed")?.textContent,
    contacts: document.querySelector("#contacts")?.textContent,
  }));

  expect(settled.error).toBeUndefined();
  expect(settled.status).toBe("ready");
  expect(Number(settled.speed)).toBeLessThan(1);
  expect(settled.contacts).toBe("4/4");
  expect(settled.telemetry.tick).toBeGreaterThan(0);
  expect(settled.telemetry.physicsMode).toBe("sab");
  expect(["BrowserWebGpu", "Gl"]).toContain(settled.telemetry.rendererBackend);
  if (!process.env.CI) {
    expect(settled.telemetry.rendererBackend).toBe("BrowserWebGpu");
  }
  expect(settled.telemetry.position[1]).toBeGreaterThan(-0.2);
  expect(settled.telemetry.position[1]).toBeLessThan(0.2);
  expect([1024, 2048, 4096]).toContain(settled.telemetry.shadowMapSize);
  expect(settled.telemetry.shadowFocus).toHaveLength(3);
  expect(pageErrors).toEqual([]);

  const tickBefore = settled.telemetry.tick;
  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 6,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("w");
  const moving = await telemetry(page);
  expect(moving.speedKph).toBeGreaterThan(5);
  expect(moving.tick).toBeGreaterThan(tickBefore);
  expect(moving.contacts).toBe(4);
  expect(moving.shadowFocus).not.toEqual(settled.telemetry.shadowFocus);

  await page.keyboard.press("r");
  await page.waitForFunction(
    () => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      return (
        state.speedKph < 0.1 &&
        Math.abs(state.position[0]) < 0.01 &&
        Math.abs(state.position[2]) < 0.01
      );
    },
    undefined,
    { timeout: 10_000 },
  );
  const reset = await telemetry(page);
  expect(reset.speedKph).toBeLessThan(2);
  expect(reset.contacts).toBe(4);

  const reverse = await page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    const start = JSON.parse(viewer.telemetry()).position;
    await viewer.simulate(2.0, 0.6, 0.0, 0.0, -1);
    const end = JSON.parse(viewer.telemetry());
    return {
      ...end,
      distance: Math.hypot(end.position[0] - start[0], end.position[2] - start[2]),
    };
  });
  expect(reverse.gear).toBe(-1);
  expect(reverse.distance).toBeGreaterThan(0.25);
  expect(reverse.contacts).toBe(4);
  expect(pageErrors).toEqual([]);
});

test("main-thread physics remains available as a fallback", async ({ page }) => {
  await page.goto("/web/drive/?physics=main&track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await waitForReady(page);
  await page.waitForTimeout(250);

  const state = await page.evaluate(() => ({
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
    isolated: window.crossOriginIsolated,
  }));
  expect(state.telemetry.physicsMode).toBeUndefined();
  expect(state.telemetry.tick).toBeGreaterThan(0);
  expect(state.isolated).toBe(true);

  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 2,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("w");
});

test("info starts hidden and can be opened", async ({ page }) => {
  await page.goto("/web/drive/");
  await waitForReady(page);

  const hud = page.locator("#hud");
  const toggle = page.locator("#hud-toggle");
  await expect(hud).toHaveClass(/hud-collapsed/);
  await expect(toggle).toHaveText("Info");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("#speed")).toBeHidden();

  await toggle.click();
  await expect(hud).not.toHaveClass(/hud-collapsed/);
  await expect(toggle).toHaveText("Hide info");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#speed")).toBeVisible();
});

test("settings tabs tune assists, graphics, and time of day", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await waitForReady(page);
  await page.evaluate(() => {
    localStorage.removeItem("lapster.settings.v3");
    localStorage.removeItem("lapster.settings.v2");
    localStorage.removeItem("lapster.assists.v1");
  });
  await page.reload();
  await waitForReady(page);

  await page.locator("#assists-open").click();
  await expect(page.locator("#assists-dialog")).toHaveAttribute("open", "");

  const gameplayTab = page.locator("#tab-gameplay");
  const graphicsTab = page.locator("#tab-graphics");
  await expect(gameplayTab).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#settings-gameplay")).toBeVisible();
  await expect(page.locator("#settings-graphics")).toBeHidden();
  await graphicsTab.click();
  await expect(graphicsTab).toHaveAttribute("aria-selected", "true");
  await expect(gameplayTab).toHaveAttribute("aria-selected", "false");
  await expect(page.locator("#settings-graphics")).toBeVisible();

  const tc = page.locator("#tc-toggle");
  const abs = page.locator("#abs-toggle");
  const esc = page.locator("#esc-toggle");
  const debug = page.locator("#debug-lines-toggle");
  const fpsToggle = page.locator("#fps-counter-toggle");
  const fpsCounter = page.locator("#fps-counter");
  const environment = page.locator("#environment-shadows");
  const carShadow = page.locator("#car-shadow");
  const timeRange = page.locator("#time-of-day");
  const timeValue = page.locator("#time-of-day-value");
  const timeApply = page.locator("#time-of-day-apply");
  const msaa = page.locator("#msaa-mode");
  const renderScale = page.locator("#render-scale");
  const renderScaleValue = page.locator("#render-scale-value");
  const view = page.locator("#view");

  await expect(environment).toHaveValue("baked");
  await expect(carShadow).toHaveValue("dynamic");
  await expect(environment.locator('option[value="auto"]')).toHaveCount(0);
  await expect(carShadow.locator('option[value="auto"]')).toHaveCount(0);
  await expect(environment.locator('option[value="baked"]')).toHaveText("Static");
  await expect(environment.locator('option[value="high"]')).toHaveText("Dynamic");
  await expect(timeValue).toHaveText("15:00");
  await expect(debug).toHaveText("Debug: Off");
  await expect(fpsToggle).toHaveText("FPS: Off");
  await expect(fpsCounter).toBeHidden();
  await expect(renderScale).toHaveValue("1");
  await expect(renderScaleValue).toHaveText("100%");
  const fullWidth = await view.evaluate((element) => (element as HTMLCanvasElement).width);

  await fpsToggle.click();
  await expect(fpsToggle).toHaveText("FPS: On");
  await expect(fpsToggle).toHaveAttribute("aria-pressed", "true");
  await expect(fpsCounter).toBeVisible();
  await expect(fpsCounter).toHaveText(/\d+ fps/, { timeout: 10_000 });

  await renderScale.selectOption("0.5");
  await expect(renderScaleValue).toHaveText("50%");
  await expect
    .poll(() => view.evaluate((element) => (element as HTMLCanvasElement).width))
    .toBe(Math.max(1, Math.floor(fullWidth * 0.5)));

  await environment.selectOption("high");
  await carShadow.selectOption("dynamic");
  let telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.environmentShadowMode).toBe("high");
  expect(telemetry.carShadowMode).toBe("dynamic");

  await carShadow.selectOption("silhouette");
  await environment.selectOption("baked");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.environmentShadowMode).toBe("baked");
  expect(telemetry.carShadowMode).toBe("silhouette");
  expect(telemetry.bakedShadowResolution).toBe(4096);

  await carShadow.selectOption("dynamic");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.environmentShadowMode).toBe("baked");
  expect(telemetry.carShadowMode).toBe("dynamic");

  await environment.selectOption("off");
  await carShadow.selectOption("off");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.environmentShadowMode).toBe("off");
  expect(telemetry.carShadowMode).toBe("off");

  await environment.selectOption("baked");
  await carShadow.selectOption("silhouette");
  await expect(tc).toHaveText("TC: On");
  await expect(abs).toHaveText("ABS: On");
  await expect(esc).toHaveText("ESC: On");
  await expect(tc).toHaveAttribute("aria-pressed", "true");
  await expect(abs).toHaveAttribute("aria-pressed", "true");
  await expect(esc).toHaveAttribute("aria-pressed", "true");

  await msaa.selectOption("off");
  let msaaTelemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(msaaTelemetry.msaaSamples).toBe(1);
  expect(await page.evaluate(() => (window as any).__driveViewerError)).toBeUndefined();
  await expect(page.locator("#msaa-applied")).toHaveText("1×");

  await debug.click();
  await expect(debug).toHaveText("Debug: On");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.debugLinesVisible).toBe(true);

  await page.locator("#tab-gameplay").click();
  await tc.click();
  await expect(tc).toHaveText("TC: Off");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.tractionControl).toBe(false);
  expect(telemetry.stabilityControl).toBe(true);

  await abs.click();
  await expect(abs).toHaveText("ABS: Off");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.absControl).toBe(false);

  await esc.click();
  await expect(esc).toHaveText("ESC: Off");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.tractionControl).toBe(false);
  expect(telemetry.absControl).toBe(false);
  expect(telemetry.stabilityControl).toBe(false);

  const setRange = async (id: string, value: string) => {
    await page.locator(id).evaluate((element, next) => {
      const input = element as HTMLInputElement;
      input.value = next;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, value);
  };
  await setRange("#tc-strength", "0.5");
  await setRange("#esc-strength", "0.6");
  await setRange("#steering-sensitivity", "1.25");
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.tractionStrength).toBeCloseTo(0.5);
  expect(telemetry.stabilityStrength).toBeCloseTo(0.6);
  expect(telemetry.steeringSensitivity).toBeCloseTo(1.25);

  await page.locator("#tab-graphics").click();
  await timeRange.evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "18";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(timeValue).toHaveText("18:00");
  await timeApply.click();
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.timeOfDay).toBeCloseTo(18);
  expect(telemetry.sunElevation).toBeLessThan(0.75);

  await page.locator("#assists-reset").click();
  telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.tractionControl).toBe(true);
  expect(telemetry.absControl).toBe(true);
  expect(telemetry.stabilityControl).toBe(true);
  expect(telemetry.tractionStrength).toBe(1);
  expect(telemetry.stabilityStrength).toBe(1);
  expect(telemetry.steeringSensitivity).toBe(1);
  expect(telemetry.debugLinesVisible).toBe(false);
  expect(telemetry.environmentShadowMode).toBe("baked");
  expect(telemetry.carShadowMode).toBe("dynamic");
  expect(telemetry.timeOfDay).toBeCloseTo(15);
  await expect(msaa).toHaveValue("auto");
  await expect(environment).toHaveValue("baked");
  await expect(carShadow).toHaveValue("dynamic");
  await expect(environment.locator('option[value="auto"]')).toHaveCount(0);
  await expect(carShadow.locator('option[value="auto"]')).toHaveCount(0);
  await expect(environment.locator('option[value="baked"]')).toHaveText("Static");
  await expect(environment.locator('option[value="high"]')).toHaveText("Dynamic");
  await expect(timeValue).toHaveText("15:00");
  await expect(fpsToggle).toHaveText("FPS: Off");
  await expect(fpsCounter).toBeHidden();
  await expect(renderScale).toHaveValue("1");
  await expect(renderScaleValue).toHaveText("100%");
  await expect
    .poll(() => view.evaluate((element) => (element as HTMLCanvasElement).width))
    .toBe(fullWidth);
});

test("migrates automatic shadow settings to baked and dynamic", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await waitForReady(page);
  await page.evaluate(() => {
    localStorage.removeItem("lapster.settings.v3");
    localStorage.setItem(
      "lapster.settings.v2",
      JSON.stringify({
        tc: false,
        environmentShadows: "auto",
        carShadow: "auto",
        timeOfDay: 18,
        antiAliasing: "off",
      }),
    );
  });
  await page.reload();
  await waitForReady(page);

  const migrated = await page.evaluate(() => ({
    telemetry: JSON.parse((window as any).__driveViewer.telemetry()),
    stored: JSON.parse(localStorage.getItem("lapster.settings.v3") ?? "null"),
  }));
  expect(migrated.telemetry.environmentShadowMode).toBe("baked");
  expect(migrated.telemetry.carShadowMode).toBe("dynamic");
  expect(migrated.telemetry.tractionControl).toBe(false);
  expect(migrated.telemetry.timeOfDay).toBeCloseTo(18);
  expect(migrated.stored.environmentShadows).toBe("baked");
  expect(migrated.stored.carShadow).toBe("dynamic");
});

test("60 second scripted soak stays finite", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  const result = await page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    const started = performance.now();
    for (let index = 0; index < 60; index += 1) {
      const phase = index % 4;
      const throttle = phase === 0 ? 1.0 : phase === 1 ? 0.4 : 0.0;
      const brake = phase === 2 ? 0.5 : 0.0;
      const steering = phase === 1 ? 0.4 : phase === 3 ? -0.4 : 0.0;
      await viewer.simulate(1.0, throttle, brake, steering, 1);
    }
    return {
      elapsedMs: performance.now() - started,
      telemetry: JSON.parse(viewer.telemetry()),
      error: (window as any).__driveViewerError,
    };
  });

  expect(result.error).toBeUndefined();
  expect(result.telemetry.tick).toBeGreaterThan(29_500);
  expect(Number.isFinite(result.telemetry.speedKph)).toBe(true);
  expect(Number.isFinite(result.telemetry.position[0])).toBe(true);
  expect(Number.isFinite(result.telemetry.position[1])).toBe(true);
  expect(Number.isFinite(result.telemetry.position[2])).toBe(true);
  expect(result.telemetry.contacts).toBe(4);
  expect(result.elapsedMs).toBeLessThan(10_000);
});

test("steering keys turn in the expected direction", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  const resetToSpawn = async () => {
    await page.keyboard.press("r");
    await page.waitForFunction(
      () => {
        const state = JSON.parse((window as any).__driveViewer.telemetry());
        return (
          state.speedKph < 0.1 &&
          Math.abs(state.position[0]) < 0.01 &&
          Math.abs(state.position[2]) < 0.01
        );
      },
      undefined,
      { timeout: 10_000 },
    );
  };

  const driveFor = async (steerKey: string) => {
    await page.keyboard.down("w");
    await page.keyboard.down(steerKey);
    await page.waitForFunction(
      () => Math.abs(JSON.parse((window as any).__driveViewer.telemetry()).position[0]) > 0.05,
      undefined,
      { timeout: 10_000 },
    );
    await page.keyboard.up(steerKey);
    await page.keyboard.up("w");
    return telemetry(page);
  };

  const left = await driveFor("ArrowLeft");
  expect(left.position[0]).toBeGreaterThan(0.05);

  await resetToSpawn();
  const right = await driveFor("ArrowRight");
  expect(right.position[0]).toBeLessThan(-0.05);

  await resetToSpawn();
  const aKey = await driveFor("a");
  expect(aKey.position[0]).toBeGreaterThan(0.05);

  await resetToSpawn();
  const dKey = await driveFor("d");
  expect(dKey.position[0]).toBeLessThan(-0.05);
});

test("arrow throttle, brake, and auto reverse", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 5,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");
  const forward = await telemetry(page);
  expect(forward.speedKph).toBeGreaterThan(5);
  expect(forward.gear).toBe(1);

  await page.keyboard.down("ArrowDown");
  await page.waitForFunction(
    (before) =>
      JSON.parse((window as any).__driveViewer.telemetry()).speedKph < before - 0.25,
    forward.speedKph,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowDown");
  const braking = await telemetry(page);
  expect(braking.speedKph).toBeLessThan(forward.speedKph);

  await page.keyboard.press("r");
  await page.waitForFunction(
    () => {
      const state = JSON.parse((window as any).__driveViewer.telemetry());
      return (
        state.speedKph < 0.1 &&
        Math.abs(state.position[0]) < 0.01 &&
        Math.abs(state.position[2]) < 0.01
      );
    },
    undefined,
    { timeout: 10_000 },
  );
  const reverseStart = await telemetry(page);
  await page.keyboard.down("ArrowDown");
  await page.waitForFunction(
    (start) => {
      const telemetry = JSON.parse((window as any).__driveViewer.telemetry());
      return (
        telemetry.gear === -1 &&
        Math.hypot(
          telemetry.position[0] - start[0],
          telemetry.position[2] - start[2],
        ) > 0.2
      );
    },
    reverseStart.position,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowDown");
  const reverse = await telemetry(page);
  expect(reverse.gear).toBe(-1);
  expect(
    Math.hypot(
      reverse.position[0] - reverseStart.position[0],
      reverse.position[2] - reverseStart.position[2],
    ),
  ).toBeGreaterThan(0.2);

  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph < 0.05,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");
  const stoppedFromReverse = await telemetry(page);
  expect(stoppedFromReverse.gear).toBe(-1);

  await page.waitForFunction(
    (tick) => JSON.parse((window as any).__driveViewer.telemetry()).tick > tick + 5,
    stoppedFromReverse.tick,
    { timeout: 10_000 },
  );
  await page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
      }),
  );
  await page.waitForTimeout(500);
  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    (before) => {
      const telemetry = JSON.parse((window as any).__driveViewer.telemetry());
      return (
        telemetry.gear === 1 &&
        Math.hypot(
          telemetry.position[0] - before[0],
          telemetry.position[2] - before[2],
        ) > 0.1
      );
    },
    stoppedFromReverse.position,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");
  const forwardAgain = await telemetry(page);
  expect(forwardAgain.gear).toBe(1);
  expect(
    Math.hypot(
      forwardAgain.position[0] - stoppedFromReverse.position[0],
      forwardAgain.position[2] - stoppedFromReverse.position[2],
    ),
  ).toBeGreaterThan(0.1);
});

test("reverse requires a second brake press after stopping", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 5,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");

  await page.keyboard.down("ArrowDown");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph < 0.05,
    undefined,
    { timeout: 10_000 },
  );
  await page.waitForTimeout(300);
  const held = await telemetry(page);
  expect(held.gear).toBe(1);
  expect(held.speedKph).toBeLessThan(0.05);

  await page.keyboard.up("ArrowDown");
  await page.waitForFunction(
    (tick) => JSON.parse((window as any).__driveViewer.telemetry()).tick > tick + 5,
    held.tick,
    { timeout: 10_000 },
  );
  await page.waitForTimeout(500);
  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("ArrowDown");
  await page.waitForFunction(
    () => {
      const telemetry = JSON.parse((window as any).__driveViewer.telemetry());
      return telemetry.gear === -1 && telemetry.speedKph > 2;
    },
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowDown");
  const reverse = await telemetry(page);
  expect(reverse.gear).toBe(-1);
  expect(reverse.speedKph).toBeGreaterThan(2);

  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    (before) =>
      JSON.parse((window as any).__driveViewer.telemetry()).speedKph < before - 0.2,
    reverse.speedKph,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");
  const braking = await telemetry(page);
  expect(braking.gear).toBe(-1);
  expect(braking.speedKph).toBeLessThan(reverse.speedKph);
});

test("brake input controls brake lights", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await waitForReady(page);

  const states = await page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    await viewer.simulate(0.1, 0.0, 1.0, 0.0, 1);
    const on = JSON.parse(viewer.telemetry()).brakeLightsOn;
    await viewer.simulate(0.1, 0.0, 0.0, 0.0, 1);
    const off = JSON.parse(viewer.telemetry()).brakeLightsOn;
    return { on, off };
  });

  expect(states.on).toBe(true);
  expect(states.off).toBe(false);
});

test("reverse gear controls reverse lights", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await waitForReady(page);

  const states = await page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    await viewer.simulate(0.1, 0.0, 0.0, 0.0, -1);
    const reverse = JSON.parse(viewer.telemetry());
    await viewer.simulate(0.1, 0.0, 0.0, 0.0, 1);
    const forward = JSON.parse(viewer.telemetry());
    return {
      reverseOn: reverse.reverseLightsOn,
      reverseBrake: reverse.brakeLightsOn,
      forwardOn: forward.reverseLightsOn,
    };
  });

  expect(states.reverseOn).toBe(true);
  expect(states.reverseBrake).toBe(false);
  expect(states.forwardOn).toBe(false);
});

test("wheels do not idle-spin when stopped", async ({ page }) => {
  await page.goto("/web/drive/");
  await page.bringToFront();
  await waitForReady(page);
  await page.evaluate(() => (window as any).__driveViewer.reset());
  await page.waitForTimeout(1200);

  const state = await telemetry(page);
  expect(state.speedKph).toBeLessThan(0.2);
  for (const wheel of state.wheels) {
    expect(Math.abs(wheel.angularVelocity)).toBeLessThan(0.01);
  }
});
