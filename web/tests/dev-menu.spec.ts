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

const TRACK = "../../fixtures/tracks/minimal/pack.sima";
const CAR = "../../fixtures/cars/minimal/pack.sima";

test("dev menu is absent without the dev query parameter", async ({ page }) => {
  await page.goto(`/web/drive/?track=${TRACK}&car=${CAR}&v=dev-off`);
  await waitForReady(page);
  await expect(page.locator("#dev-drawer")).toHaveCount(0);
  await expect(page.locator("#dev-open")).toHaveCount(0);
});

test("dev menu adjusts live settings and car tuning", async ({ page }) => {
  await page.goto(`/web/drive/?dev=1&track=${TRACK}&car=${CAR}&v=dev-menu`);
  await waitForReady(page);

  const drawer = page.locator("#dev-drawer");
  const sideButton = page.locator("#dev-open");
  await expect(drawer).not.toHaveClass(/dev-open/);
  await expect(sideButton).toBeVisible();
  await expect(sideButton).toHaveAttribute("aria-expanded", "false");
  await expect(drawer.locator("details[open]")).toHaveCount(0);

  await sideButton.click();
  await expect(drawer).toHaveClass(/dev-open/);
  await expect(drawer).toBeVisible();
  await expect(sideButton).toBeHidden();
  await expect(drawer.locator("details[open]")).toHaveCount(0);

  const resetSettings = page.getByRole("button", { name: "Reset settings" });
  await expect(resetSettings).toBeVisible();
  await drawer.locator(".dev-body").evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(resetSettings).toBeVisible();

  await drawer.locator("summary", { hasText: "Driving assists" }).click();
  await drawer.locator("summary", { hasText: "View" }).click();
  await drawer.locator("summary", { hasText: /^Camera$/ }).click();
  await drawer.locator("summary", { hasText: /^Chase$/ }).click();
  await drawer.locator("summary", { hasText: /^Particles$/ }).click();
  await drawer.locator("summary", { hasText: /^Asphalt$/ }).click();
  await drawer.locator("summary", { hasText: /^Physics$/ }).click();
  await drawer.locator("summary", { hasText: /^World$/ }).click();
  await drawer.locator("summary", { hasText: /^GPU$/ }).click();
  await drawer.locator("summary", { hasText: /^Textures$/ }).click();
  await drawer.locator("summary", { hasText: /^Rendering$/ }).click();
  await drawer.locator("summary", { hasText: /^Shadows$/ }).click();
  await page.locator("#dev-environment-shadows").selectOption("high");
  await expect
    .poll(async () => (await telemetry(page)).environmentShadowMode)
    .toBe("high");
  await drawer.locator("summary", { hasText: /^Car tuning$/ }).click();
  await drawer.locator("summary", { hasText: /^Chassis$/ }).click();

  const tc = page.locator("#dev-tc");
  await expect(tc).toBeChecked();
  await tc.uncheck();
  await expect.poll(async () => (await telemetry(page)).tractionControl).toBe(false);

  await page.locator("#dev-tc-strength").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "0.8";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect
    .poll(async () => (await telemetry(page)).tractionStrength)
    .toBeCloseTo(0.8);

  await page.locator("#dev-tune-camera-chase-fovY").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "0.9";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect
    .poll(async () =>
      page.evaluate(
        () => JSON.parse((window as any).__driveViewer.tuning()).values.camera.chase.fovY,
      ),
    )
    .toBeCloseTo(0.9);

  await page.locator("#dev-tune-particles-surfaces-asphalt-rateScale").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "1200";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect
    .poll(async () =>
      page.evaluate(
        () =>
          JSON.parse((window as any).__driveViewer.tuning()).values.particles.surfaces.asphalt
            .rateScale,
      ),
    )
    .toBeCloseTo(1200);

  await page.locator("#dev-tune-physics-gravity").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "9.8";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(page.getByRole("button", { name: /Apply physics/ })).toBeVisible();

  await page.locator("#dev-tune-shadows-fullTrack").check();
  await expect
    .poll(async () =>
      page.evaluate(
        () => JSON.parse((window as any).__driveViewer.tuning()).values.shadows.fullTrack,
      ),
    )
    .toBe(true);

  await page.locator("#dev-tune-gpu-shadowMapSize").evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "1024";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect(page.getByRole("button", { name: /Apply GPU/ })).toBeVisible();
  await expect(page.getByRole("button", { name: "Restart audio" })).toBeVisible();

  await page.getByRole("button", { name: /Apply physics/ }).click();
  await expect(page.locator("#dev-status")).toContainText("Physics restarted");

  await page.locator("#dev-camera-mode").selectOption("isometric");
  await expect(page.locator("#camera-mode")).toHaveText("Camera: Isometric");
  await expect.poll(async () => (await telemetry(page)).cameraMode).toBe("isometric");

  await page.locator("#dev-body-visible").uncheck();
  await expect(page.locator("#body-toggle")).toHaveText("Body: Hidden");

  const cgHeight = page.locator("#dev-setup-chassis-centerOfGravity-1");
  await cgHeight.evaluate((element) => {
    const input = element as HTMLInputElement;
    input.value = "0.12";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect
    .poll(async () =>
      page.evaluate(
        () => JSON.parse((window as any).__driveViewer.car_setup()).chassis.centerOfGravity[1],
      ),
    )
    .toBeCloseTo(0.12);
  await expect
    .poll(async () =>
      page.evaluate(() => {
        const raw = localStorage.getItem("lapster.setup.fixtures/car/minimal");
        return raw ? JSON.parse(raw).chassis.centerOfGravity[1] : null;
      }),
    )
    .toBeCloseTo(0.12);

  await expect
    .poll(async () =>
      page.evaluate(() => localStorage.getItem("lapster.tuning.dev.v1") !== null),
    )
    .toBe(true);

  await page.getByRole("button", { name: "Reset tuning" }).click();
  await expect
    .poll(async () =>
      page.evaluate(
        () => JSON.parse((window as any).__driveViewer.tuning()).values.camera.chase.fovY,
      ),
    )
    .toBeCloseTo(0.95);
  await expect
    .poll(async () =>
      page.evaluate(() => localStorage.getItem("lapster.tuning.dev.v1")),
    )
    .toBeNull();

  await page.click("#dev-close");
  await expect(drawer).not.toHaveClass(/dev-open/);
  await expect(sideButton).toBeVisible();
  await expect(sideButton).toHaveAttribute("aria-expanded", "false");

  await page.keyboard.press("\\");
  await expect(drawer).toHaveClass(/dev-open/);
  await expect(sideButton).toBeHidden();
});

test("GPU tuning persists across a reload", async ({ page }) => {
  await page.goto(`/web/drive/?dev=1&track=${TRACK}&car=${CAR}&v=dev-gpu-reload`);
  await waitForReady(page);
  await page.locator("#dev-open").click();
  await page.locator("summary", { hasText: /^GPU$/ }).click();
  await page.locator("summary", { hasText: /^Textures$/ }).click();
  for (const [id, value] of [
    ["#dev-tune-gpu-shadowMapSize", "1024"],
    ["#dev-tune-gpu-carShadowMapSize", "1024"],
    ["#dev-tune-gpu-bakedResolution", "1024"],
    ["#dev-tune-gpu-bakedHalfExtent", "60"],
  ]) {
    await page.locator(id).evaluate((element, next) => {
      const input = element as HTMLInputElement;
      input.value = next;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }, value);
  }
  const apply = page.getByRole("button", { name: /Apply GPU/ });
  await expect(apply).toBeVisible();
  await Promise.all([page.waitForNavigation(), apply.click()]);
  await waitForReady(page);
  await expect
    .poll(async () => {
      return page.evaluate(() => {
        const tuning = JSON.parse((window as any).__driveViewer.tuning()).values;
        const telemetry = JSON.parse((window as any).__driveViewer.telemetry());
        return {
          shadowMapSize: tuning.gpu.shadowMapSize,
          carShadowMapSize: tuning.gpu.carShadowMapSize,
          bakedResolution: tuning.gpu.bakedResolution,
          bakedHalfExtent: tuning.gpu.bakedHalfExtent,
          actualShadowMapSize: telemetry.shadowMapSize,
          actualCarShadowMapSize: telemetry.carShadowMapSize,
          actualBakedResolution: telemetry.bakedShadowResolution,
          actualBakedHalfExtent: telemetry.bakedRegionHalfExtent,
        };
      });
    })
    .toEqual({
      shadowMapSize: 1024,
      carShadowMapSize: 1024,
      bakedResolution: 1024,
      bakedHalfExtent: 60,
      actualShadowMapSize: 1024,
      actualCarShadowMapSize: 1024,
      actualBakedResolution: 1024,
      actualBakedHalfExtent: 60,
    });
});
