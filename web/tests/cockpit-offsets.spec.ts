import { expect, test } from "@playwright/test";

const CAVALLO = "../../fixtures/cars/67gp-cavallo-123/67gp-cavallo-123.sima";

test("67GP cockpit RPM offset is applied from the car pack id", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(`/web/drive/?car=${CAVALLO}`);
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const state = await page.evaluate(() => ({
    error: (window as any).__driveViewerError,
    car: JSON.parse((window as any).__driveViewer.telemetry()).car,
    offsets: JSON.parse((window as any).__driveViewer.cockpit_dial_offsets()),
  }));

  expect(state.error).toBeUndefined();
  expect(state.car).toBe("67gp-cavallo-123");
  expect(state.offsets.rpm[0]).toBeCloseTo(0.002, 5);
  expect(state.offsets.rpm[1]).toBeCloseTo(-0.024, 5);
  expect(state.offsets.rpm[2]).toBeCloseTo(-0.01, 5);
  expect(state.offsets.speed).toEqual([0, 0, 0]);
  expect(state.offsets.gear).toEqual([0, 0, 0]);
  expect(pageErrors).toEqual([]);
});
