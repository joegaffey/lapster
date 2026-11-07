import { expect, test, type Page } from "@playwright/test";

async function findKerbGap(page: Page) {
  return page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const spans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
      start: number;
      end: number;
      side: number;
    }>;
    const count = (JSON.parse(viewer.editor_centerline()) as number[][]).length;
    for (const side of [-1, 1]) {
      const ranges = spans
        .filter((span) => span.side === side)
        .sort((a, b) => a.start - b.start);
      let previous = 0;
      for (const range of [...ranges, { start: count, end: count, side }]) {
        const gapStart = previous;
        const gapEnd = range.start;
        if (gapEnd - gapStart >= 14) {
          return { side, index: gapStart + 6, gapStart, gapEnd, count };
        }
        previous = Math.max(previous, range.end);
      }
    }
    return { error: "no kerb gap" };
  });
}

async function findPropGap(page: Page, offset: number) {
  return page.evaluate(({ offset }) => {
    const viewer = (window as any).__trackEditor;
    const lines = JSON.parse(viewer.editor_prop_lines()) as Array<{
      start: number;
      end: number;
      side: number;
      offset: number;
    }>;
    const count = (JSON.parse(viewer.editor_centerline()) as number[][]).length;
    for (const side of [-1, 1]) {
      const ranges = lines
        .filter(
          (line) => line.side === side && Math.abs(line.offset - offset) < 1e-4,
        )
        .sort((a, b) => a.start - b.start);
      let previous = 0;
      for (const range of [...ranges, { start: count, end: count, side, offset }]) {
        const gapStart = previous;
        const gapEnd = range.start;
        if (gapEnd - gapStart >= 3.5) {
          return {
            side,
            index: (gapStart + gapEnd) * 0.5,
            gapStart,
            gapEnd,
            count,
          };
        }
        previous = Math.max(previous, range.end);
      }
    }
    return { error: "no prop gap" };
  }, { offset });
}

async function projectGuideIndex(
  page: Page,
  side: number,
  index: number,
  radius = 6.4,
): Promise<[number, number] | null> {
  const projected = await page.evaluate(
    ({ side, index, radius }) => {
      const viewer = (window as any).__trackEditor;
      const canvas = document.querySelector("#view") as HTMLCanvasElement;
      const centerline = JSON.parse(viewer.editor_centerline()) as number[][];
      const camera = JSON.parse(viewer.camera_info()) as {
        eye: number[];
        target: number[];
        fovY: number;
      };
      const sub = (a: number[], b: number[]) => [
        a[0] - b[0],
        a[1] - b[1],
        a[2] - b[2],
      ];
      const add = (a: number[], b: number[]) => [
        a[0] + b[0],
        a[1] + b[1],
        a[2] + b[2],
      ];
      const scale = (value: number[], amount: number) => [
        value[0] * amount,
        value[1] * amount,
        value[2] * amount,
      ];
      const dot = (a: number[], b: number[]) =>
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
      const cross = (a: number[], b: number[]) => [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
      ];
      const normalize = (value: number[]) => {
        const length = Math.hypot(value[0], value[1], value[2]);
        return length > 1e-8 ? scale(value, 1 / length) : [0, 0, 0];
      };
      const lerp = (a: number[], b: number[], t: number) => [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
      ];
      const binormalAt = (sample: number) => {
        const count = centerline.length;
        const current = ((sample % count) + count) % count;
        const incoming = normalize(
          sub(centerline[current], centerline[(current + count - 1) % count]),
        );
        const outgoing = normalize(
          sub(centerline[(current + 1) % count], centerline[current]),
        );
        const tangent = normalize(add(incoming, outgoing));
        return [-tangent[2], 0, tangent[0]];
      };
      const count = centerline.length;
      const position = Math.max(0, Math.min(count, index));
      const base = Math.floor(position) % count;
      const next = (base + 1) % count;
      const t = position - Math.floor(position);
      const point = lerp(centerline[base], centerline[next], t);
      const binormal = normalize(lerp(binormalAt(base), binormalAt(next), t));
      const guide = add(point, scale(binormal, radius * side));
      const eye = camera.eye;
      const forward = normalize(sub(camera.target, eye));
      const right = normalize(cross(forward, [0, 1, 0]));
      const up = normalize(cross(right, forward));
      const offset = sub(guide, eye);
      const depth = dot(offset, forward);
      if (depth <= 0.01) {
        return null;
      }
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      const tanHalf = Math.tan(camera.fovY * 0.5);
      const aspect = width / Math.max(height, 1);
      const ndcX = dot(offset, right) / (depth * tanHalf * aspect);
      const ndcY = dot(offset, up) / (depth * tanHalf);
      return [(ndcX * 0.5 + 0.5) * width, (0.5 - ndcY * 0.5) * height];
    },
    { side, index, radius },
  );
  return projected as [number, number] | null;
}

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
    let purple = 0;
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
      } else if (red > 160 && red < 220 && green > 90 && green < 170 && blue > 220) {
        purple += 1;
      }
    }
    return { pink, orange, teal, purple };
  });

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(kerbHandlePixels.pink).toBeGreaterThan(20);
  expect(kerbHandlePixels.orange).toBeGreaterThan(20);
  expect(kerbHandlePixels.teal).toBeGreaterThan(20);
  expect(kerbHandlePixels.purple).toBeGreaterThan(20);
  expect(state.status).toBe("ready");
  expect(state.debug.asset.id).toBe("procedural-drift-loop");
  expect(state.debug.stats.vertices).toBeGreaterThan(5000);
  expect(state.debug.stats.triangles).toBeGreaterThan(2000);
  expect(state.debug.stats.collisionTriangles).toBeGreaterThan(800);
  expect(state.debug.stats.grassBlades).toBeGreaterThan(0);
  expect(state.debug.meshes.length).toBeGreaterThanOrEqual(7);
  expect(state.draws).toBe(String(state.debug.stats.drawCalls));
  expect(state.surfaces).toContain("asphalt");
  expect(state.surfaces).toContain("grass");
  expect(state.surfaces).toContain("dirt");
  expect(state.meshes).toContain("asphalt_dither");
  expect(state.meshes).toContain("mountains");
  expect(state.meshes).not.toContain("dither_shoulder");
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
    const render = viewer.render;
    viewer.render = () => {};
    try {
      await viewer.set_editor_kerb_spans(JSON.stringify(trimmed));
    } finally {
      viewer.render = render;
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
  expect(result.trimmed?.end).toBeCloseTo((result.first?.start ?? 0) + 1, 4);
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
      offset: number;
      kind: string;
    }>;
    const blocksIndex = before.findIndex((line) => line.kind === "blocks");
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
      index === blocksIndex ? { ...line, kind: "off" } : line,
    );
    const render = viewer.render;
    viewer.render = () => {};
    try {
      await viewer.set_editor_prop_lines(JSON.stringify(changed));
    } finally {
      viewer.render = render;
    }
    const after = JSON.parse(viewer.editor_prop_lines()) as Array<{
      start: number;
      end: number;
      side: number;
      offset: number;
      kind: string;
    }>;
    return {
      before,
      after,
      blocksIndex,
      beforeVertices,
      afterVertices: acrossSectionVertices(),
    };
  });

  expect(result.blocksIndex).toBeGreaterThanOrEqual(0);
  expect(result.before.length).toBeGreaterThan(4);
  expect(result.before.some((line) => line.kind === "trees")).toBe(true);
  expect(result.after[result.blocksIndex].kind).toBe("off");
  expect(result.after.some((line) => line.kind === "tyres")).toBe(true);
  expect(result.afterVertices).toBeLessThan(result.beforeVertices);
  expect(pageErrors).toEqual([]);
});

test("default tree rings have gaps on both sides", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=prop-trees");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const gap = await findPropGap(page, 19);
  expect(gap.error).toBeUndefined();

  const result = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const lines = JSON.parse(viewer.editor_prop_lines()) as Array<{
      start: number;
      end: number;
      side: number;
      offset: number;
      kind: string;
    }>;
    const meshes = JSON.parse(viewer.debug_info()).meshes as Array<{
      name: string;
      vertices: number;
    }>;
    return {
      trees: lines.filter((line) => line.kind === "trees"),
      treeMeshes: meshes.filter((mesh) => mesh.name.startsWith("tree_")),
    };
  });

  expect(result.trees.length).toBeGreaterThan(1);
  expect(result.trees.every((line) => line.offset === 19)).toBe(true);
  expect([...new Set(result.trees.map((line) => line.side))].sort()).toEqual([
    -1, 1,
  ]);
  expect(result.treeMeshes.map((mesh) => mesh.name)).toEqual([
    "tree_trunks",
    "tree_foliage",
  ]);
  expect(result.treeMeshes.every((mesh) => mesh.vertices > 0)).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("prop type is a single toggle in the main controls", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=prop-toggle");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const toggle = page.locator("#prop-toggle");
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveText("Prop: Blocks");
  await expect(page.locator("#hud [data-prop-kind]")).toHaveCount(0);
  await expect(page.locator("#prop-section-label")).toHaveCount(0);

  await toggle.click();
  await expect(toggle).toHaveText("Prop: Tyres");
  await toggle.click();
  await expect(toggle).toHaveText("Prop: Trees");
  await toggle.click();
  await expect(toggle).toHaveText("Prop: Off");
  await toggle.click();
  await expect(toggle).toHaveText("Prop: Blocks");

  expect(pageErrors).toEqual([]);
});

test("new prop segments span about three metres", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=prop-short");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const gap = await findPropGap(page, 19);
  expect(gap.error).toBeUndefined();
  const index = gap.index!;
  const point = await projectGuideIndex(page, gap.side!, index, 19);
  expect(point).not.toBeNull();
  await page.mouse.click(point![0], point![1]);

  await page.waitForFunction(
    ({ index }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const viewer = (window as any).__trackEditor;
      const lines = JSON.parse(viewer.editor_prop_lines()) as Array<{
        start: number;
        end: number;
        offset: number;
      }>;
      return lines.some(
        (line) =>
          Math.abs(line.offset - 19) < 1e-4 &&
          line.start < index &&
          line.end > index,
      );
    },
    { index },
    { timeout: 15_000 },
  );

  const result = await page.evaluate(
    ({ index }) => {
      const viewer = (window as any).__trackEditor;
      const lines = JSON.parse(viewer.editor_prop_lines()) as Array<{
        start: number;
        end: number;
        side: number;
        offset: number;
        kind: string;
      }>;
      const centerline = JSON.parse(viewer.editor_centerline()) as number[][];
      let perimeter = 0;
      for (let point = 0; point < centerline.length; point += 1) {
        const current = centerline[point];
        const next = centerline[(point + 1) % centerline.length];
        perimeter += Math.hypot(
          next[0] - current[0],
          next[1] - current[1],
          next[2] - current[2],
        );
      }
      return {
        lines,
        sampleSpacing: perimeter / centerline.length,
        created: lines.find(
          (line) =>
            Math.abs(line.offset - 19) < 1e-4 &&
            line.start < index &&
            line.end > index,
        ),
      };
    },
    { index },
  );
  expect(result.created, JSON.stringify(result.lines)).toBeTruthy();
  const created = result.created;
  const length = (created?.end ?? 0) - (created?.start ?? 0);
  const metres = length * result.sampleSpacing;
  expect(metres).toBeGreaterThan(2.5);
  expect(metres).toBeLessThan(3.5);
  expect(pageErrors).toEqual([]);
});

test("delete key and button remove the selected segment", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=segment-delete");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const deleteButton = page.locator("#delete-segment");
  await expect(deleteButton).toBeDisabled();

  const gap = await findKerbGap(page);
  expect(gap.error).toBeUndefined();
  const side = gap.side!;
  const index = gap.index!;
  const kerbPoint = await projectGuideIndex(page, side, index);
  expect(kerbPoint).not.toBeNull();
  await page.mouse.click(kerbPoint![0], kerbPoint![1]);
  await page.waitForFunction(
    ({ side, index }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const spans = JSON.parse(
        (window as any).__trackEditor.editor_kerb_spans(),
      ) as Array<{ start: number; end: number; side: number }>;
      return spans.some(
        (span) => span.side === side && span.start < index && span.end > index,
      );
    },
    { side, index },
    { timeout: 15_000 },
  );
  await expect(deleteButton).toBeEnabled();
  await deleteButton.click();
  await page.waitForFunction(
    ({ side, index }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const spans = JSON.parse(
        (window as any).__trackEditor.editor_kerb_spans(),
      ) as Array<{ start: number; end: number; side: number }>;
      return !spans.some(
        (span) => span.side === side && span.start < index && span.end > index,
      );
    },
    { side, index },
    { timeout: 15_000 },
  );
  await expect(deleteButton).toBeDisabled();

  const propGap = await findPropGap(page, 19);
  expect(propGap.error).toBeUndefined();
  const propIndex = propGap.index!;
  const propPoint = await projectGuideIndex(page, propGap.side!, propIndex, 19);
  expect(propPoint).not.toBeNull();
  await page.mouse.click(propPoint![0], propPoint![1]);
  await page.waitForFunction(
    ({ index }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const lines = JSON.parse(
        (window as any).__trackEditor.editor_prop_lines(),
      ) as Array<{ start: number; end: number; offset: number }>;
      return lines.some(
        (line) =>
          Math.abs(line.offset - 19) < 1e-4 &&
          line.start < index &&
          line.end > index,
      );
    },
    { index: propIndex },
    { timeout: 15_000 },
  );
  await expect(deleteButton).toBeEnabled();
  await page.keyboard.press("Delete");
  await page.waitForFunction(
    ({ index }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const lines = JSON.parse(
        (window as any).__trackEditor.editor_prop_lines(),
      ) as Array<{ start: number; end: number; offset: number }>;
      return !lines.some(
        (line) =>
          Math.abs(line.offset - 19) < 1e-4 &&
          line.start < index &&
          line.end > index,
      );
    },
    { index: propIndex },
    { timeout: 15_000 },
  );
  await expect(deleteButton).toBeDisabled();
  expect(pageErrors).toEqual([]);
});

test("kerb segments are created and dragged without snapping", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=kerb-drag");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const gap = await findKerbGap(page);
  expect(gap.error).toBeUndefined();
  const side = gap.side!;
  const index = gap.index!;
  const clickPoint = await projectGuideIndex(page, side, index);
  expect(clickPoint).not.toBeNull();
  await page.mouse.click(clickPoint![0], clickPoint![1]);

  await page.waitForFunction(
    ({ side, index }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const viewer = (window as any).__trackEditor;
      const spans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
        start: number;
        end: number;
        side: number;
      }>;
      return spans.some(
        (span) => span.side === side && span.start < index && span.end > index,
      );
    },
    { side, index },
    { timeout: 15_000 },
  );

  const created = await page.evaluate(
    ({ side, index }) => {
      const viewer = (window as any).__trackEditor;
      const spans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
        start: number;
        end: number;
        side: number;
      }>;
      return spans.find(
        (span) => span.side === side && span.start < index && span.end > index,
      );
    },
    { side, index },
  );
  expect(created).toBeTruthy();
  expect((created?.end ?? 0) - (created?.start ?? 0)).toBeGreaterThan(4);

  const endPoint = await projectGuideIndex(page, side, created!.end);
  const targetIndex = gap.gapStart! + 11.5;
  const targetPoint = await projectGuideIndex(page, side, targetIndex);
  expect(endPoint).not.toBeNull();
  expect(targetPoint).not.toBeNull();
  await page.mouse.move(endPoint![0], endPoint![1]);
  await page.mouse.down();
  await page.mouse.move(targetPoint![0], targetPoint![1], { steps: 8 });
  await page.mouse.up();

  await page.waitForFunction(
    ({ side, oldEnd }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const viewer = (window as any).__trackEditor;
      const spans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
        start: number;
        end: number;
        side: number;
      }>;
      return spans.some(
        (span) => span.side === side && span.start < oldEnd && span.end > oldEnd + 1,
      );
    },
    { side, oldEnd: created!.end },
    { timeout: 15_000 },
  );

  const moved = await page.evaluate(
    ({ side, oldEnd }) => {
      const viewer = (window as any).__trackEditor;
      const spans = JSON.parse(viewer.editor_kerb_spans()) as Array<{
        start: number;
        end: number;
        side: number;
      }>;
      return spans.find(
        (span) => span.side === side && span.start < oldEnd && span.end > oldEnd + 1,
      );
    },
    { side, oldEnd: created!.end },
  );
  expect(moved).toBeTruthy();
  expect(moved!.end).toBeGreaterThan(created!.end + 1);
  expect(Math.abs(moved!.end - Math.round(moved!.end))).toBeGreaterThan(0.05);
  expect(pageErrors).toEqual([]);
});
