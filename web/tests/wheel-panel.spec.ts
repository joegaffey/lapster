import { expect, test } from "@playwright/test";

async function waitForReady(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

test("wheel controls live in the default settings dialog", async ({ page }) => {
  await page.goto(
    "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=wheel-settings",
  );
  await waitForReady(page);

  await expect(page.locator("#wheel-open")).toHaveCount(0);
  await page.locator("#assists-open").click();

  const controlsTab = page.locator("#tab-controls");
  await expect(controlsTab).toBeVisible();
  await controlsTab.click();
  await expect(page.locator("#settings-controls")).toBeVisible();
  await expect(page.locator("#wheel-status")).toContainText(
    /WebHID (available|unavailable)/,
  );
  await expect(page.locator("#wheel-connect")).toBeVisible();
  await expect(page.locator("#wheel-disconnect")).toBeDisabled();
  await expect(page.locator("#wheel-strength")).toHaveValue("0.5");
  await expect(page.locator("#wheel-test-apply")).toBeDisabled();
  await expect(page.locator("#wheel-rear-lightening")).toHaveValue("0.7");
  await expect(page.locator("#wheel-rear-threshold")).toHaveValue("0.08");
  await expect(page.locator("#wheel-front-shudder")).toHaveValue("0.8");
  await expect(page.locator("#wheel-shudder-hz")).toHaveValue("22");
  await expect(page.locator("#wheel-road-noise")).toHaveValue("0.55");
  await expect(page.locator("#wheel-roughness")).toHaveValue("0.65");

  await page.locator("#wheel-strength").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "0.8";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(page.locator("#wheel-strength-value")).toHaveText("80%");

  await page.locator("#wheel-invert-force").check();
  await expect(page.locator("#wheel-invert-force")).toBeChecked();

  await page.locator("#wheel-rear-lightening").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "0.9";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(page.locator("#wheel-rear-lightening-value")).toHaveText("90%");

  await page.locator("#wheel-reset").click();
  await expect(page.locator("#wheel-strength")).toHaveValue("0.5");
  await expect(page.locator("#wheel-invert-force")).not.toBeChecked();
  await expect(page.locator("#wheel-rear-lightening")).toHaveValue("0.7");
});
