import { expect, test } from "@playwright/test";

test.use({ permissions: ["clipboard-read", "clipboard-write"] });

async function waitForReady(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function setup(page: import("@playwright/test").Page) {
  return page.evaluate(() => JSON.parse((window as any).__driveViewer.car_setup()));
}

test("tuning dialog applies, resets, persists, imports, and copies", async ({ page }) => {
  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=cp6",
  );
  await waitForReady(page);
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await waitForReady(page);

  const toggle = page.locator("#setup-open");
  await toggle.click();
  const dialog = page.locator("#car-setup-dialog");
  await expect(dialog).toHaveAttribute("open", "");
  expect(await page.evaluate(() => document.querySelector("#car-setup-dialog")?.matches(":modal"))).toBe(true);

  const initial = await setup(page);
  await page.locator("summary", { hasText: "Front suspension" }).click();
  const spring = page.locator("#setup-front-suspension-springRate");
  await spring.fill("60000");
  await page.locator("#car-setup-apply").click();
  const applied = await setup(page);
  expect(applied.front.suspension.springRate).toBe(60000);
  expect(applied.rear.suspension.springRate).toBe(initial.rear.suspension.springRate);

  await page.reload();
  await waitForReady(page);
  expect((await setup(page)).front.suspension.springRate).toBe(60000);

  await page.locator("#setup-open").click();
  await page.locator("#car-setup-copy").click();
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  const payload = JSON.parse(copied);
  const car = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()).car,
  );
  expect(payload.schema).toBe("lapster/setup@1");
  expect(payload.car).toBe(car);
  expect(payload.setup.front.suspension.springRate).toBe(60000);

  const imported = structuredClone(payload.setup);
  imported.front.suspension.springRate = 62000;
  await page.locator("summary", { hasText: "Paste shared settings" }).click();
  await page.locator("#car-setup-import-text").fill(JSON.stringify(imported));
  await page.locator("#car-setup-import-apply").click();
  expect((await setup(page)).front.suspension.springRate).toBe(62000);

  await page.locator("#car-setup-reset").click();
  const reset = await setup(page);
  expect(reset.front.suspension.springRate).toBe(initial.front.suspension.springRate);
});
