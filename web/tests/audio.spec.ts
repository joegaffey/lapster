import { expect, test } from "@playwright/test";

async function waitForReady(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

test("audio worklet starts and mutes from the toolbar", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=audio",
  );
  await waitForReady(page);

  const button = page.locator("#audio-toggle");
  await expect(button).toBeEnabled();
  await expect(button).toHaveText("Audio: Off");
  await button.click();
  await page.waitForFunction(
    () => (window as any).__driveAudio?.status === "running",
    undefined,
    { timeout: 10_000 },
  );
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => {
    const audio = (window as any).__driveAudio;
    return {
      status: audio.status,
      contextState: audio.contextState,
      muted: audio.muted,
      level: audio.level(),
    };
  });
  expect(state.status).toBe("running");
  expect(state.contextState).toBe("running");
  expect(state.muted).toBe(false);
  expect(state.level).toBeGreaterThan(0);
  await expect(button).toHaveText("Audio: On");
  await expect(button).toHaveAttribute("aria-pressed", "true");

  await button.click();
  await expect(button).toHaveText("Audio: Muted");
  await expect(button).toHaveAttribute("aria-pressed", "false");
  expect(pageErrors).toEqual([]);
});

test("audio follows the vehicle rpm and speed", async ({ page }) => {
  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=audio-sync",
  );
  await waitForReady(page);
  await page.locator("#audio-toggle").click();
  await page.waitForFunction(
    () => (window as any).__driveAudio?.status === "running",
    undefined,
    { timeout: 10_000 },
  );

  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  await page.waitForFunction(
    () => JSON.parse((window as any).__driveViewer.telemetry()).speedKph > 15,
    undefined,
    { timeout: 10_000 },
  );
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => {
    const game = JSON.parse((window as any).__driveViewer.telemetry());
    const audio = (window as any).__driveAudio.state();
    return {
      gameRpm: game.rpm,
      gameSpeedKph: game.speedKph,
      gameGear: game.gear,
      audio,
    };
  });
  await page.keyboard.up("w");

  expect(Math.abs(state.audio.rpm - state.gameRpm)).toBeLessThan(150);
  expect(Math.abs(state.audio.speedKph - state.gameSpeedKph)).toBeLessThan(1);
  expect(state.audio.gear).toBe(state.gameGear);
  expect(state.audio.clutch).toBe(1);
  expect(Math.abs(state.audio.clutchSlip)).toBeLessThan(50);
});
