import { expect, test } from "@playwright/test";

test("track editor renders the procedural example track", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=editor-preview");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );
  await page.waitForTimeout(750);
  await expect(page.locator("#hud")).toHaveClass(/hud-collapsed/);

  const state = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      ready: (window as any).__trackEditorReady,
      error: (window as any).__trackEditorError,
      status: document.querySelector("#status")?.textContent,
      stats: document.querySelector("#stats")?.textContent ?? "",
      draws: document.querySelector("#draws")?.textContent,
      surfaces: document.querySelector("#surfaces")?.textContent ?? "",
      meshes: document.querySelector("#meshes")?.textContent ?? "",
      debug: JSON.parse(viewer.debug_info()),
    };
  });

  const kerbHandlePixels = await page.evaluate(() => {
    const overlay = document.querySelector("#control-points") as HTMLCanvasElement;
    const context = overlay.getContext("2d")!;
    const pixels = context.getImageData(0, 0, overlay.width, overlay.height).data;
    let pink = 0;
    let orange = 0;
    let teal = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      const red = pixels[index];
      const green = pixels[index + 1];
      const blue = pixels[index + 2];
      const alpha = pixels[index + 3];
      if (alpha < 16) continue;
      if (red > 200 && green > 70 && green < 170 && blue > 90 && blue < 190) {
        pink += 1;
      } else if (red > 220 && green > 110 && green < 180 && blue < 100) {
        orange += 1;
      } else if (red < 100 && green > 170 && blue > 150) {
        teal += 1;
      }
    }
    return { pink, orange, teal };
  });

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(kerbHandlePixels.pink).toBeGreaterThan(20);
  expect(kerbHandlePixels.orange).toBeGreaterThan(20);
  expect(kerbHandlePixels.teal).toBeGreaterThan(20);
  expect(state.status).toBe("ready");
  expect(state.debug.asset.id).toBe("procedural-drift-loop");
  expect(state.debug.stats.vertices).toBeGreaterThan(5000);
  expect(state.debug.stats.triangles).toBeGreaterThan(2000);
  expect(state.debug.stats.collisionTriangles).toBeGreaterThan(800);
  expect(state.debug.stats.grassBlades).toBeGreaterThan(0);
  expect(state.debug.meshes.length).toBeGreaterThan(8);
  expect(state.draws).toBe(String(state.debug.stats.drawCalls));
  expect(state.surfaces).toContain("asphalt");
  expect(state.surfaces).toContain("grass");
  expect(state.surfaces).toContain("dirt");
  expect(state.meshes).toContain("asphalt");
  expect(state.meshes).toContain("kerb_red");
  expect(state.meshes).toContain("barriers");
  expect(state.meshes).toContain("tires_blue");

  const overlay = page.locator("#control-points");
  const overlayToggle = page.locator("#toggle-overlay");
  await expect(overlay).toBeVisible();
  await expect(overlayToggle).toHaveText("Hide overlay");
  await overlayToggle.click();
  await expect(overlay).toBeHidden();
  await expect(overlayToggle).toHaveText("Show overlay");
  await expect(overlayToggle).toHaveAttribute("aria-pressed", "false");
  await overlayToggle.click();
  await expect(overlay).toBeVisible();
  await expect(overlayToggle).toHaveAttribute("aria-pressed", "true");

  expect(pageErrors).toEqual([]);
});

test("kerb spans can be trimmed and rebuilt", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=kerb-trim");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const result = await page.evaluate(async () => {
    const viewer = (window as any).__trackEditor;
    const spans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
      start: number;
      end: number;
      side: number;
    }>;
    if (spans.length === 0) {
      return { error: "no kerb spans" };
    }
    const first = spans[0];
    const trimmed = spans.map((span, index) =>
      index === 0 ? { ...span, end: span.start + 1 } : span,
    );
    const kerbVertices = () => {
      const meshes = JSON.parse(viewer.debug_info()).meshes as Array<{
        name: string;
        vertices: number;
      }>;
      return meshes
        .filter((mesh) => mesh.name === "kerb_red" || mesh.name === "kerb_white")
        .reduce((sum, mesh) => sum + mesh.vertices, 0);
    };
    const beforeVertices = kerbVertices();
    const requestFrame = window.requestAnimationFrame;
    window.requestAnimationFrame = () => 0;
    await new Promise((resolve) => window.setTimeout(resolve, 500));
    try {
      await viewer.set_editor_kerb_spans(JSON.stringify(trimmed));
    } finally {
      window.requestAnimationFrame = requestFrame;
    }
    const afterSpans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
      start: number;
      end: number;
      side: number;
    }>;
    return {
      first,
      trimmed: afterSpans[0],
      beforeVertices,
      afterVertices: kerbVertices(),
    };
  });

  expect(result.error).toBeUndefined();
  expect(result.trimmed?.start).toBe(result.first?.start);
  expect(result.trimmed?.end).toBe((result.first?.start ?? 0) + 1);
  expect(result.afterVertices).toBeLessThan(result.beforeVertices);
  expect(pageErrors).toEqual([]);
});

test("prop sections can switch between blocks, tyres, and off", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=prop-kind");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const result = await page.evaluate(async () => {
    const viewer = (window as any).__trackEditor;
    const before = JSON.parse(viewer.editor_prop_lines()) as Array<{
      start: number;
      end: number;
      side: number;
      kind: string;
    }>;
    const acrossSectionVertices = () => {
      const meshes = JSON.parse(viewer.debug_info()).meshes as Array<{
        name: string;
        vertices: number;
      }>;
      return meshes
        .filter((mesh) => mesh.name === "barriers" || mesh.name.startsWith("tires_"))
        .reduce((sum, mesh) => sum + mesh.vertices, 0);
    };
    const beforeVertices = acrossSectionVertices();
    const changed = before.map((line, index) =>
      index === 0 ? { ...line, kind: "off" } : line,
    );
    const requestFrame = window.requestAnimationFrame;
    window.requestAnimationFrame = () => 0;
    await new Promise((resolve) => window.setTimeout(resolve, 500));
    try {
      await viewer.set_editor_prop_lines(JSON.stringify(changed));
    } finally {
      window.requestAnimationFrame = requestFrame;
    }
    const after = JSON.parse(viewer.editor_prop_lines()) as Array<{
      start: number;
      end: number;
      side: number;
      kind: string;
    }>;
    return {
      before,
      after,
      beforeVertices,
      afterVertices: acrossSectionVertices(),
    };
  });

  expect(result.before).toHaveLength(2);
  expect(result.after[0].kind).toBe("off");
  expect(result.after[1].kind).toBe("tyres");
  expect(result.afterVertices).toBeLessThan(result.beforeVertices);
  expect(pageErrors).toEqual([]);
});
