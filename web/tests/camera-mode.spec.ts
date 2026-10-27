import { expect, test } from "@playwright/test";

test("drive camera cycles through chase, bonnet, cockpit, and isometric", async ({ page }) => {
  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const camera = page.locator("#camera-mode");
  const body = page.locator("#body-toggle");
  await expect(camera).toHaveText("Camera: Chase");
  await expect(body).toHaveText("Body: Visible");
  expect(
    await page.evaluate(() => JSON.parse((window as any).__driveViewer.telemetry()).cameraMode),
  ).toBe("chase");

  await camera.click();
  await expect(camera).toHaveText("Camera: Bonnet");
  await expect(body).toHaveText("Body: Visible");
  expect(
    await page.evaluate(() => JSON.parse((window as any).__driveViewer.telemetry()).cameraMode),
  ).toBe("bonnet");

  await camera.click();
  await expect(camera).toHaveText("Camera: Cockpit");
  expect(
    await page.evaluate(() => JSON.parse((window as any).__driveViewer.telemetry()).cameraMode),
  ).toBe("cockpit");

  await camera.click();
  await expect(camera).toHaveText("Camera: Isometric");
  expect(
    await page.evaluate(() => JSON.parse((window as any).__driveViewer.telemetry()).cameraMode),
  ).toBe("isometric");

  await body.click();
  await expect(body).toHaveText("Body: Hidden");
  await body.click();
  await expect(body).toHaveText("Body: Visible");

  await page.keyboard.press("c");
  await expect(camera).toHaveText("Camera: Chase");
  await expect(body).toHaveText("Body: Visible");
  expect(
    await page.evaluate(() => JSON.parse((window as any).__driveViewer.telemetry()).cameraMode),
  ).toBe("chase");
});
