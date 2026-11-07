import { expect, test, type Page } from "@playwright/test";

async function waitForEditor(page: Page) {
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );
}

async function waitForDrive(page: Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 45_000 },
  );
}

test("track editor can test drive the procedural track", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=editor-drive");
  await waitForEditor(page);
  const startDirection = await page.evaluate(() => {
    const centerline = JSON.parse(
      (window as any).__trackEditor.editor_centerline(),
    ) as number[][];
    const dx = centerline[1][0] - centerline[0][0];
    const dz = centerline[1][2] - centerline[0][2];
    const length = Math.hypot(dx, dz);
    return [dx / length, dz / length];
  });
  await expect(page.locator("#test-drive")).toBeEnabled();
  await page.locator("#tree-detail").click();
  await page.locator("#tree-detail").click();
  await expect(page.locator("#tree-detail")).toHaveText("Trees: Cards");
  await page.locator("#test-drive").click();
  await page.waitForURL(/\/web\/drive\/\?[^#]*track=editor/, { timeout: 30_000 });
  await waitForDrive(page);
  await page.waitForTimeout(1000);

  const result = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      error: (window as any).__driveViewerError,
      status: document.querySelector("#status")?.textContent,
      debug: JSON.parse(viewer.debug_info()),
      telemetry: JSON.parse(viewer.telemetry()),
    };
  });

  expect(result.error).toBeUndefined();
  expect(result.status).toBe("ready");
  expect(
    await page.evaluate(() => (window as any).__driveViewer.tree_detail()),
  ).toBe("cards");
  expect(result.debug.asset.id).toBe("procedural-drift-loop");
  expect(result.telemetry.track).toBe("procedural-drift-loop");
  expect(
    result.debug.surfaces.some((surface: { name: string }) => surface.name === "kerb"),
  ).toBe(true);
  expect(
    result.debug.surfaces.some((surface: { name: string }) => surface.name === "wall"),
  ).toBe(true);
  expect(result.debug.stats.collisionTriangles).toBeGreaterThan(800);
  expect(result.telemetry.contacts).toBe(4);

  const motion = await page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    await viewer.reset();
    const start = JSON.parse(viewer.telemetry()).position;
    await viewer.simulate(1.5, 1.0, 0.0, 0.0, 1);
    const end = JSON.parse(viewer.telemetry()).position;
    return { dx: end[0] - start[0], dz: end[2] - start[2] };
  });
  const distance = Math.hypot(motion.dx, motion.dz);
  const alignment =
    (motion.dx * startDirection[0] + motion.dz * startDirection[1]) / distance;
  expect(distance).toBeGreaterThan(1);
  expect(alignment).toBeGreaterThan(0.9);
  expect(pageErrors).toEqual([]);
});
