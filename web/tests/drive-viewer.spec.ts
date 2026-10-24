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
  expect(["BrowserWebGpu", "Gl"]).toContain(settled.telemetry.rendererBackend);
  if (!process.env.CI) {
    expect(settled.telemetry.rendererBackend).toBe("BrowserWebGpu");
  }
  expect(settled.telemetry.position[1]).toBeGreaterThan(-0.2);
  expect(settled.telemetry.position[1]).toBeLessThan(0.2);
  expect(pageErrors).toEqual([]);

  const tickBefore = settled.telemetry.tick;
  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 5,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("w");
  const moving = await telemetry(page);
  expect(moving.speedKph).toBeGreaterThan(5);
  expect(moving.tick).toBeGreaterThan(tickBefore);
  expect(moving.contacts).toBe(4);

  await page.keyboard.press("r");
  await page.waitForTimeout(750);
  const reset = await telemetry(page);
  expect(reset.speedKph).toBeLessThan(2);
  expect(reset.contacts).toBe(4);

  const reverse = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    viewer.simulate(2.0, 0.6, 0.0, 0.0, -1);
    return JSON.parse(viewer.telemetry());
  });
  expect(reverse.gear).toBe(-1);
  expect(reverse.position[2]).toBeGreaterThan(0.25);
  expect(reverse.contacts).toBe(4);
  expect(pageErrors).toEqual([]);
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

test("settings dialog toggles and tunes TC, ESC, and steering", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await waitForReady(page);
  await page.evaluate(() => localStorage.removeItem("lapster.assists.v1"));

  await page.locator("#assists-open").click();
  await expect(page.locator("#assists-dialog")).toHaveAttribute("open", "");

  const tc = page.locator("#tc-toggle");
  const abs = page.locator("#abs-toggle");
  const esc = page.locator("#esc-toggle");
  await expect(tc).toHaveText("TC: On");
  await expect(abs).toHaveText("ABS: On");
  await expect(esc).toHaveText("ESC: On");
  await expect(tc).toHaveAttribute("aria-pressed", "true");
  await expect(abs).toHaveAttribute("aria-pressed", "true");
  await expect(esc).toHaveAttribute("aria-pressed", "true");
  const debug = page.locator("#debug-lines-toggle");
  await expect(debug).toHaveText("Debug: Off");
  await expect(debug).toHaveAttribute("aria-pressed", "false");
  const msaa = page.locator("#msaa-mode");
  await msaa.selectOption("off");
  let msaaTelemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(msaaTelemetry.msaaSamples).toBe(1);
  await expect(page.locator("#msaa-applied")).toHaveText("1×");

  await debug.click();
  await expect(debug).toHaveText("Debug: On");
  let telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  expect(telemetry.debugLinesVisible).toBe(true);

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
  await expect(page.locator("#msaa-mode")).toHaveValue("auto");
});

test("60 second scripted soak stays finite", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  const result = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    viewer.reset();
    const started = performance.now();
    for (let index = 0; index < 60; index += 1) {
      const phase = index % 4;
      const throttle = phase === 0 ? 1.0 : phase === 1 ? 0.4 : 0.0;
      const brake = phase === 2 ? 0.5 : 0.0;
      const steering = phase === 1 ? 0.4 : phase === 3 ? -0.4 : 0.0;
      viewer.simulate(1.0, throttle, brake, steering, 1);
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
  expect(left.position[0]).toBeLessThan(-0.05);

  await page.keyboard.press("r");
  await page.waitForTimeout(150);
  const right = await driveFor("ArrowRight");
  expect(right.position[0]).toBeGreaterThan(0.05);

  await page.keyboard.press("r");
  await page.waitForTimeout(150);
  const aKey = await driveFor("a");
  expect(aKey.position[0]).toBeLessThan(-0.05);

  await page.keyboard.press("r");
  await page.waitForTimeout(150);
  const dKey = await driveFor("d");
  expect(dKey.position[0]).toBeGreaterThan(0.05);
});

test("arrow throttle, brake, and auto reverse", async ({ page }) => {
  await page.goto("/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  await page.keyboard.down("ArrowUp");
  await page.waitForTimeout(1200);
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
  await page.waitForTimeout(150);
  await page.keyboard.down("ArrowDown");
  await page.waitForTimeout(1500);
  await page.keyboard.up("ArrowDown");
  const reverse = await telemetry(page);
  expect(reverse.gear).toBe(-1);
  expect(reverse.position[2]).toBeGreaterThan(0.2);

  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph < 0.3,
    undefined,
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");
  const stoppedFromReverse = await telemetry(page);
  expect(stoppedFromReverse.gear).toBe(-1);

  await page.waitForTimeout(100);
  await page.keyboard.down("ArrowUp");
  await page.waitForFunction(
    (before) => {
      const telemetry = JSON.parse((window as any).__driveViewer.telemetry());
      return telemetry.gear === 1 && telemetry.position[2] < before - 0.1;
    },
    stoppedFromReverse.position[2],
    { timeout: 10_000 },
  );
  await page.keyboard.up("ArrowUp");
  const forwardAgain = await telemetry(page);
  expect(forwardAgain.gear).toBe(1);
  expect(forwardAgain.position[2]).toBeLessThan(stoppedFromReverse.position[2]);
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
    () => JSON.parse((window as any).__driveViewer.telemetry()).gear === -1,
    undefined,
    { timeout: 10_000 },
  );
  await page.waitForTimeout(500);
  await page.keyboard.up("ArrowDown");
  const reverse = await telemetry(page);
  expect(reverse.gear).toBe(-1);
  expect(reverse.speedKph).toBeGreaterThan(1);

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
