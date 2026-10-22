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

  await page.goto("/web/drive/");
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
  await page.keyboard.down("w");
  await page.waitForTimeout(1500);
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

test("60 second scripted soak stays finite", async ({ page }) => {
  await page.goto("/web/drive/");
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
  await page.goto("/web/drive/");
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(250);

  const driveFor = async (steerKey: string) => {
    await page.keyboard.down("w");
    await page.keyboard.down(steerKey);
    await page.waitForTimeout(1500);
    await page.keyboard.up(steerKey);
    await page.keyboard.up("w");
    return telemetry(page);
  };

  const left = await driveFor("ArrowLeft");
  expect(left.position[0]).toBeLessThan(-0.3);

  await page.keyboard.press("r");
  await page.waitForTimeout(150);
  const right = await driveFor("ArrowRight");
  expect(right.position[0]).toBeGreaterThan(0.3);

  await page.keyboard.press("r");
  await page.waitForTimeout(150);
  const aKey = await driveFor("a");
  expect(aKey.position[0]).toBeLessThan(-0.3);

  await page.keyboard.press("r");
  await page.waitForTimeout(150);
  const dKey = await driveFor("d");
  expect(dKey.position[0]).toBeGreaterThan(0.3);
});

test("arrow throttle, brake, and auto reverse", async ({ page }) => {
  await page.goto("/web/drive/");
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
  await page.waitForTimeout(450);
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
  await page.waitForTimeout(2600);
  await page.keyboard.up("ArrowUp");
  const forwardAgain = await telemetry(page);
  expect(forwardAgain.gear).toBe(1);
  expect(forwardAgain.position[2]).toBeLessThan(reverse.position[2]);
});
