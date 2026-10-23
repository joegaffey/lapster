import { expect, test } from "@playwright/test";

test("track viewer renders the baked minimal track", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/web/track-viewer/?track=../../fixtures/tracks/minimal/pack.sima&v=cp6");
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );
  await page.waitForTimeout(750);

  const state = await page.evaluate(() => ({
    ready: (window as any).__trackViewerReady,
    error: (window as any).__trackViewerError,
    status: document.querySelector("#status")?.textContent,
    stats: document.querySelector("#stats")?.textContent ?? "",
    draws: document.querySelector("#draws")?.textContent,
    surfaces: document.querySelector("#surfaces")?.textContent ?? "",
    meshes: document.querySelector("#meshes")?.textContent ?? "",
    debug: JSON.parse((window as any).__trackViewer.debug_info()),
  }));

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(state.status).toBe("ready");
  expect(state.draws).toBe("8");
  expect(state.stats).toContain("vertices172");
  expect(state.stats).toContain("triangles86");
  expect(state.stats).toContain("draw calls8");
  expect(state.stats).toContain("textures1");
  expect(state.stats).toContain("collision tris26");
  expect(state.debug.stats.texturedMeshes).toBe(1);
  expect(state.debug.stats.outlinedMeshes).toBe(3);
  expect(state.surfaces).toContain("asphalt");
  expect(state.surfaces).toContain("wall");
  expect(state.surfaces).toContain("line");
  expect(state.meshes).toContain("ground");
  expect(state.meshes).toContain("wall_left");
  expect(state.meshes).toContain("gauge_sign");
  expect(state.meshes).toContain("axis_neg_z");
  expect(pageErrors).toEqual([]);

  if (!process.env.CI) {
    await expect(page).toHaveScreenshot("track-viewer.png", {
      mask: [page.locator("#timing")],
      maskColor: "#0a0d11",
    });
  }
});

test("reload uses the cached pack", async ({ page }) => {
  let packRequests = 0;
  page.on("request", (request) => {
    if (request.resourceType() === "fetch" && request.url().includes("pack.sima")) {
      packRequests += 1;
    }
  });

  const waitForReady = () =>
    page.waitForFunction(
      () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
      undefined,
      { timeout: 30_000 },
    );

  await page.goto("/web/track-viewer/?track=../../fixtures/tracks/minimal/pack.sima&v=cp6");
  await waitForReady();
  expect(packRequests).toBe(1);

  await page.reload();
  await waitForReady();
  expect(packRequests).toBe(1);
});
