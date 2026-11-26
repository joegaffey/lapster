import { expect, test } from "@playwright/test";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function driveState(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      error: (window as any).__driveViewerError,
      slots: JSON.parse(viewer.info()).slots as string[],
      car: JSON.parse(viewer.telemetry()).car as string,
    };
  });
}

test("gp67 race uses different same-category AI car packs", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&race=1&ai=2&category=gp67");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();

  const state = await driveState(page);
  expect(state.error).toBeUndefined();
  expect(state.slots).toHaveLength(3);
  expect(state.slots[0]).toBe(state.car);
  expect(state.slots[0]).toMatch(/^67gp-/);
  expect(state.slots[1]).toMatch(/^67gp-/);
  expect(state.slots[2]).toMatch(/^67gp-/);
  expect(state.slots[1]).not.toBe(state.slots[2]);
  expect(state.slots[1]).not.toBe(state.slots[0]);
  expect(state.slots[2]).not.toBe(state.slots[0]);
  expect(pageErrors).toEqual([]);
});

test("road race mixes the other road car with the player", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/drive/?track=race-loop&race=1&ai=1&category=road");
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();

  const state = await driveState(page);
  expect(state.error).toBeUndefined();
  expect(state.slots).toHaveLength(2);
  expect(state.slots[0]).toBe(state.car);
  expect(["mazda-rx7", "sc-deckard-conejo"]).toContain(state.slots[0]);
  expect(["mazda-rx7", "sc-deckard-conejo"]).toContain(state.slots[1]);
  expect(state.slots[1]).not.toBe(state.slots[0]);
  expect(pageErrors).toEqual([]);
});
