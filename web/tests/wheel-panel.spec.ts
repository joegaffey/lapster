import { expect, test } from "@playwright/test";

async function waitForReady(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

test("dev wheel panel reports WebHID and FFB test controls", async ({ page }) => {
  await page.goto(
    "/web/drive/?dev=1&track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=wheel-panel",
  );
  await waitForReady(page);

  const open = page.locator("#wheel-open");
  await expect(open).toBeVisible();
  await open.click();

  const dialog = page.locator("#wheel-dialog");
  await expect(dialog).toHaveAttribute("open", "");
  await expect(page.locator("#wheel-status")).toContainText(
    /WebHID (available|unavailable)/,
  );
  await expect(page.locator("#wheel-connect")).toBeVisible();
  await expect(page.locator("#wheel-disconnect")).toBeDisabled();
  await expect(page.locator("#wheel-strength")).toHaveValue("0.5");
  await expect(page.locator("#wheel-test-apply")).toBeDisabled();

  await page.locator("#wheel-strength").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "0.8";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(page.locator("#wheel-strength-value")).toHaveText("80%");

  await page.locator("#wheel-invert-force").check();
  await expect(page.locator("#wheel-invert-force")).toBeChecked();
});
