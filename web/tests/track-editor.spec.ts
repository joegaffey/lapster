import { expect, test, type Page } from "@playwright/test";

test("editor loads a schema 4 project with routes", async ({ page }) => {
  test.setTimeout(90_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/");
  await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });

  const project = await page.evaluate(
    () => JSON.parse((window as any).__trackEditor.editor_project()) as {
      schema: string;
      layouts: Array<{ id: string }>;
      edges: unknown[];
      controlPoints: unknown[];
    },
  );

  expect(project.schema).toBe("lapster/track-project@4");
  expect(Array.isArray(project.layouts) && project.layouts.length > 0).toBe(true);
  expect(Array.isArray(project.edges) && project.edges.length > 0).toBe(true);
  expect(
    Array.isArray(project.controlPoints) && project.controlPoints.length > 0,
  ).toBe(true);

  const routeIds = project.layouts.map((layout) => layout.id);
  const initialActive = await page.evaluate(
    () => (window as any).__trackEditor.editor_active_layout() as string,
  );
  await expect(page.locator("#layout-select option")).toHaveCount(
    project.layouts.length,
  );
  expect(routeIds).toContain(initialActive);

  if (project.layouts.length >= 2) {
    await page.locator("#hud-toggle").click();
    const otherId = routeIds.find((id) => id !== initialActive)!;
    await page.locator("#layout-select").selectOption(otherId);
    await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });
    const switched = await page.evaluate(
      () => (window as any).__trackEditor.editor_active_layout() as string,
    );
    expect(switched).toBe(otherId);
    expect(switched).not.toBe(initialActive);
  } else {
    const active = await page.evaluate(
      () => (window as any).__trackEditor.editor_active_layout() as string,
    );
    expect(active).toBe(initialActive);
  }

  expect(pageErrors).toEqual([]);
});

test("selecting a road segment shows the segment panel", async ({ page }) => {
  test.setTimeout(90_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=segment-select");
  await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });

  const segmentId = await page.evaluate(() => {
    const project = JSON.parse((window as any).__trackEditor.editor_project()) as {
      layouts: Array<{ route: string[] }>;
    };
    return project.layouts[0]?.route[0] ?? "";
  });
  expect(segmentId).not.toBe("");

  await page.locator("#hud-toggle").click();
  await page.evaluate(
    (id) => (window as any).__editorSelectSegment(id),
    segmentId,
  );
  await expect(page.locator("#segment-hud")).toContainText(segmentId);
  const makeUnique = page.locator("#segment-make-unique");
  await expect(makeUnique).toBeVisible();
  await expect(makeUnique).toBeEnabled();
  expect(pageErrors).toEqual([]);
});

test("duplicating a route shares canonical segments", async ({ page }) => {
  test.setTimeout(90_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/");
  await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });

  await page.locator("#hud-toggle").click();

  const before = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as {
        layouts: Array<{ id: string; route: string[] }>;
      },
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(before.project.layouts.length).toBeGreaterThanOrEqual(1);
  const source =
    before.project.layouts.find((layout) => layout.id === before.active) ??
    before.project.layouts[0];
  const sourceRoute = [...source.route];
  const existingIds = before.project.layouts.map((layout) => layout.id);

  await page.locator("#duplicate-layout").click();
  await waitForEditorRebuild(page);

  const after = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as {
        layouts: Array<{ id: string; route: string[] }>;
      },
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(after.project.layouts.length).toBe(before.project.layouts.length + 1);
  const duplicate = after.project.layouts.find(
    (layout) => !existingIds.includes(layout.id),
  );
  expect(duplicate).toBeTruthy();
  expect(duplicate!.route).toEqual(sourceRoute);
  expect(after.active).toBe(duplicate!.id);

  expect(pageErrors).toEqual([]);
});

test("shared segment make unique diverges one route", async ({ page }) => {
  test.setTimeout(90_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  const readEditor = () =>
    page.evaluate(() => {
      const viewer = (window as any).__trackEditor;
      return {
        project: JSON.parse(viewer.editor_project()) as {
          edges: Array<{ id: string; points: unknown[] }>;
          layouts: Array<{
            id: string;
            route: string[];
            startFinish?: { edge: string; fraction: number };
          }>;
        },
        active: viewer.editor_active_layout() as string,
      };
    });

  await page.goto("/web/track-editor/?v=shared-segment-unique");
  await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });

  if (
    await page
      .locator("#hud")
      .evaluate((hud) => hud.classList.contains("hud-collapsed"))
  ) {
    await page.locator("#hud-toggle").click();
  }

  const before = await readEditor();

  await page.locator("#duplicate-layout").click();
  await waitForEditorRebuild(page);

  const duplicated = await readEditor();
  const source = duplicated.project.layouts.find(
    (layout) => layout.id === before.active,
  );
  const duplicate = duplicated.project.layouts.find(
    (layout) => layout.id === duplicated.active,
  );
  expect(source).toBeTruthy();
  expect(duplicate).toBeTruthy();
  expect(duplicate!.startFinish?.edge).toBeTruthy();
  const segmentId = duplicate!.route.find(
    (id) => id !== duplicate!.startFinish!.edge,
  );
  expect(segmentId).toBeTruthy();
  const routeIndex = duplicate!.route.indexOf(segmentId!);
  expect(routeIndex).toBeGreaterThanOrEqual(0);
  expect(source!.route).toContain(segmentId!);

  await page.evaluate(
    (id) => (window as any).__editorSelectSegment(id),
    segmentId,
  );
  await expect(page.locator("#segment-hud")).toContainText(segmentId!);
  const makeUnique = page.locator("#segment-make-unique");
  await expect(makeUnique).toBeEnabled();
  await makeUnique.click();
  await waitForEditorRebuild(page);

  const unique = await readEditor();
  const uniqueSource = unique.project.layouts.find(
    (layout) => layout.id === before.active,
  );
  const uniqueDuplicate = unique.project.layouts.find(
    (layout) => layout.id === duplicated.active,
  );
  expect(uniqueSource).toBeTruthy();
  expect(uniqueDuplicate).toBeTruthy();
  const newSegmentId = uniqueDuplicate!.route[routeIndex];
  expect(newSegmentId).toBeTruthy();
  expect(newSegmentId).not.toBe(segmentId);
  expect(uniqueSource!.route).toContain(segmentId!);
  expect(uniqueSource!.route[routeIndex]).toBe(segmentId);
  const edgeIds = unique.project.edges.map((edge) => edge.id);
  expect(edgeIds).toContain(segmentId!);
  expect(edgeIds).toContain(newSegmentId);
  await expect(page.locator("#segment-hud")).toContainText(newSegmentId);

  const newSegment = unique.project.edges.find(
    (edge) => edge.id === newSegmentId,
  );
  expect(newSegment).toBeTruthy();
  const originalPointCount = newSegment!.points.length;

  await page.locator("#segment-add-point").click();
  await waitForEditorRebuild(page);
  const afterAdd = await readEditor();
  const addedSegment = afterAdd.project.edges.find(
    (edge) => edge.id === newSegmentId,
  );
  expect(addedSegment).toBeTruthy();
  expect(addedSegment!.points.length).toBe(originalPointCount + 1);

  await page.locator("#segment-delete-point").click();
  await waitForEditorRebuild(page);
  const afterDelete = await readEditor();
  const restoredSegment = afterDelete.project.edges.find(
    (edge) => edge.id === newSegmentId,
  );
  expect(restoredSegment).toBeTruthy();
  expect(restoredSegment!.points.length).toBe(originalPointCount);

  expect(pageErrors).toEqual([]);
});

type ProjectLoop = {
  id: string;
  name: string;
  controlPoints: string[];
  sectors: unknown[];
  cameras: unknown[];
  spawns: unknown[];
};

type ProjectV3 = {
  schema: string;
  settings: {
    width: number;
    kerb: { width: number; depth: number; elementLength: number };
    pit?: unknown;
  };
  loops: ProjectLoop[];
  controlPoints: Array<{
    id: string;
    position: [number, number];
    kind: string;
    tangent?: [number, number] | null;
  }>;
  segments: Array<{
    from: string;
    to: string;
    role: "main" | "branch";
    kerbs: unknown[];
    propSpans: unknown[];
  }>;
  pitLane?: unknown;
};

function trackWidth(project: ProjectV3): number {
  return project.settings.width;
}

async function waitForEditorRebuild(page: Page, timeout = 20_000) {
  await page
    .waitForFunction(
      () => document.querySelector("#status")?.textContent !== "ready",
      undefined,
      { timeout: 5_000 },
    )
    .catch(() => {});
  await expect(page.locator("#status")).toHaveText("ready", { timeout });
}

async function readLoopCenterlines(page: Page, restoreId?: string) {
  return page.evaluate(async (targetId) => {
    const viewer = (window as any).__trackEditor;
    const switchLayout = (window as any).__setEditorActiveLayout as (
      id: string,
    ) => Promise<void>;
    const project = JSON.parse(viewer.editor_project()) as {
      loops: Array<{ id: string }>;
    };
    const previous = viewer.editor_active_layout() as string;
    const centerlines: Record<string, string> = {};
    for (const loop of project.loops) {
      await switchLayout(loop.id);
      centerlines[loop.id] = viewer.editor_centerline() as string;
    }
    const restore = targetId && centerlines[targetId] !== undefined
      ? targetId
      : previous;
    await switchLayout(restore);
    return { centerlines, active: viewer.editor_active_layout() as string };
  }, restoreId);
}

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

async function projectWorldPoint(
  page: Page,
  point: [number, number, number],
): Promise<[number, number] | null> {
  const projected = await page.evaluate(
    ({ point }) => {
      const viewer = (window as any).__trackEditor;
      const canvas = document.querySelector("#view") as HTMLCanvasElement;
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
      const dot = (a: number[], b: number[]) =>
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
      const cross = (a: number[], b: number[]) => [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
      ];
      const normalize = (value: number[]) => {
        const length = Math.hypot(value[0], value[1], value[2]);
        return length > 1e-8
          ? value.map((component) => component / length)
          : [0, 0, 0];
      };
      const eye = camera.eye;
      const forward = normalize(sub(camera.target, eye));
      const right = normalize(cross(forward, [0, 1, 0]));
      const up = normalize(cross(right, forward));
      const offset = sub(point, eye);
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
    { point },
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
  expect(state.meshes).toContain("tree_trunks_high");

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

test("camera view cycles and the draggable car follows the spline", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=camera-view");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const toggle = page.locator("#camera-view");
  await expect(toggle).toHaveText("View: Front");
  const centerline = await page.evaluate(
    () =>
      JSON.parse((window as any).__trackEditor.editor_centerline()) as number[][],
  );

  const targetIndex = 30;
  const markerStart = await projectWorldPoint(page, [
    centerline[0][0],
    centerline[0][1] + 3.2,
    centerline[0][2],
  ]);
  const dragEnd = await projectWorldPoint(page, [
    centerline[targetIndex][0],
    centerline[targetIndex][1],
    centerline[targetIndex][2],
  ]);
  expect(markerStart).not.toBeNull();
  expect(dragEnd).not.toBeNull();
  await page.mouse.move(markerStart![0], markerStart![1]);
  await page.mouse.down();
  await page.mouse.move(dragEnd![0], dragEnd![1], { steps: 12 });
  await page.mouse.up();

  await toggle.click();
  await expect(toggle).toHaveText("View: Back");
  await toggle.click();
  await expect(toggle).toHaveText("View: Left");
  await toggle.click();
  await expect(toggle).toHaveText("View: Right");
  await toggle.click();
  await expect(toggle).toHaveText("View: Car");

  const nearest = async () => {
    const camera = await page.evaluate(
      () => JSON.parse((window as any).__trackEditor.camera_info()) as {
        eye: number[];
      },
    );
    let bestDistance = Infinity;
    let bestIndex = -1;
    for (let index = 0; index < centerline.length; index += 1) {
      const point = centerline[index];
      const distance = Math.hypot(
        point[0] - camera.eye[0],
        point[1] + 1.4 - camera.eye[1],
        point[2] - camera.eye[2],
      );
      if (distance < bestDistance) {
        bestDistance = distance;
        bestIndex = index;
      }
    }
    return { index: bestIndex, distance: bestDistance, camera };
  };

  const dragged = await nearest();
  expect(dragged.distance).toBeLessThan(2);
  expect(dragged.index).toBeGreaterThan(20);
  expect(dragged.index).toBeLessThan(40);

  await page.keyboard.press("ArrowUp");
  const advanced = await nearest();
  expect(advanced.distance).toBeLessThan(2);
  expect(advanced.index).toBeGreaterThan(dragged.index);
  expect(advanced.index - dragged.index).toBeLessThanOrEqual(2);
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
    };
  });

  expect(result.blocksIndex).toBeGreaterThanOrEqual(0);
  expect(result.before.length).toBeGreaterThan(4);
  expect(result.before.some((line) => line.kind === "trees")).toBe(true);
  expect(result.after.length).toBeGreaterThanOrEqual(result.before.length);
  const blocksBefore = result.before[result.blocksIndex];
  const blocksAfter = result.after[result.blocksIndex];
  expect(blocksAfter.kind).toBe("off");
  expect(blocksAfter.start).toBe(blocksBefore.start);
  expect(blocksAfter.side).toBe(blocksBefore.side);
  expect(blocksAfter.offset).toBe(blocksBefore.offset);
  expect(result.after.some((line) => line.kind === "blocks")).toBe(true);
  expect(result.after.some((line) => line.kind === "tyres")).toBe(true);
  expect(result.after.some((line) => line.kind === "trees")).toBe(true);
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
      triangles: number;
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
    "tree_trunks_high",
    "tree_foliage_high",
    "tree_trunks_low",
    "tree_foliage_low",
    "tree_cards",
  ]);
  expect(result.treeMeshes.every((mesh) => mesh.vertices > 0)).toBe(true);
  const highTriangles = result.treeMeshes
    .filter((mesh) => mesh.name.endsWith("_high"))
    .reduce((sum, mesh) => sum + mesh.triangles, 0);
  const lowTriangles = result.treeMeshes
    .filter((mesh) => mesh.name.endsWith("_low"))
    .reduce((sum, mesh) => sum + mesh.triangles, 0);
  expect(highTriangles).toBeGreaterThan(lowTriangles * 1.5);
  const cardTriangles = result.treeMeshes
    .filter((mesh) => mesh.name === "tree_cards")
    .reduce((sum, mesh) => sum + mesh.triangles, 0);
  expect(cardTriangles).toBeGreaterThan(0);
  expect(cardTriangles).toBeLessThan(lowTriangles);
  expect(pageErrors).toEqual([]);
});

test("tree detail toggle switches between high, low, and card trees", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=tree-detail");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const button = page.locator("#tree-detail");
  await expect(button).toHaveText("Trees: High");
  expect(
    await page.evaluate(() => (window as any).__trackEditor.tree_detail()),
  ).toBe("high");

  await button.click();
  await expect(button).toHaveText("Trees: Low");
  expect(
    await page.evaluate(() => (window as any).__trackEditor.tree_detail()),
  ).toBe("low");

  await button.click();
  await expect(button).toHaveText("Trees: Cards");
  expect(
    await page.evaluate(() => (window as any).__trackEditor.tree_detail()),
  ).toBe("cards");

  await button.click();
  await expect(button).toHaveText("Trees: High");
  expect(
    await page.evaluate(() => (window as any).__trackEditor.tree_detail()),
  ).toBe("high");
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

test("editing a kerb segment preserves the camera view", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=camera-preserve");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const canvas = page.locator("#view");
  const bounds = await canvas.boundingBox();
  expect(bounds).not.toBeNull();
  await page.mouse.move(
    bounds!.x + bounds!.width * 0.5,
    bounds!.y + bounds!.height * 0.5,
  );
  await page.mouse.wheel(0, -300);

  const cameraBefore = await page.evaluate(
    () => JSON.parse((window as any).__trackEditor.camera_info()) as {
      eye: number[];
      target: number[];
      fovY: number;
    },
  );

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

  const assertCameraPreserved = async (label: string) => {
    const cameraAfter = await page.evaluate(
      () => JSON.parse((window as any).__trackEditor.camera_info()) as {
        eye: number[];
        target: number[];
        fovY: number;
      },
    );
    expect(cameraAfter.eye, `${label} eye`).toHaveLength(3);
    expect(cameraAfter.target, `${label} target`).toHaveLength(3);
    for (let axis = 0; axis < 3; axis += 1) {
      expect(
        Math.abs(cameraAfter.eye[axis] - cameraBefore.eye[axis]),
        `${label} eye[${axis}]`,
      ).toBeLessThanOrEqual(0.01);
      expect(
        Math.abs(cameraAfter.target[axis] - cameraBefore.target[axis]),
        `${label} target[${axis}]`,
      ).toBeLessThanOrEqual(0.01);
    }
    expect(
      Math.abs(cameraAfter.fovY - cameraBefore.fovY),
      `${label} fovY`,
    ).toBeLessThanOrEqual(0.01);
  };

  await assertCameraPreserved("after create");

  const deleteButton = page.locator("#delete-segment");
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

  await assertCameraPreserved("after delete");
  expect(pageErrors).toEqual([]);
});

test("legacy track projects migrate from @1 to @3", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=legacy-migration");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  const legacyProject = {
    schema: "lapster/track-project@1",
    name: "Legacy Loop",
    defaultLayout: "legacy-layout",
    layouts: [
      {
        id: "legacy-layout",
        name: "Legacy Layout",
        width: 14,
        controlPoints: [
          [0, 0],
          [0, 30],
          [30, 30],
          [30, 0],
        ],
        kerbs: [],
        propLines: [],
        startFinish: 0,
        spawns: [],
        pitBoxes: [],
        cameras: [],
        sectors: [],
      },
    ],
  };

  const project = await page.evaluate(async (legacy) => {
    await (window as any).__setEditorProject(JSON.stringify(legacy));
    const viewer = (window as any).__trackEditor;
    return JSON.parse(viewer.editor_project()) as ProjectV3;
  }, legacyProject);

  expect(project.schema).toBe("lapster/track-project@3");
  expect(project.loops).toHaveLength(1);
  expect(project.loops[0].id).toBe("legacy-layout");
  expect(project.loops[0].controlPoints.length).toBeGreaterThanOrEqual(4);
  expect(project.controlPoints.length).toBeGreaterThanOrEqual(4);
  expect(trackWidth(project)).toBe(14);
  expect(pageErrors).toEqual([]);
});

test("layout panel adds, switches, resizes, duplicates, deletes, and test drives a circuit", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=multi-layout");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  await page.locator("#hud-toggle").click();
  await expect(page.locator("#hud")).toBeVisible();

  const initial = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
      centerline: viewer.editor_centerline() as string,
    };
  });
  expect(initial.project.schema).toBe("lapster/track-project@3");
  expect(initial.project.loops).toHaveLength(1);
  expect(Array.isArray(initial.project.controlPoints)).toBe(true);
  expect(initial.project.controlPoints.length).toBeGreaterThan(0);
  const firstLoop = initial.project.loops[0];
  expect(firstLoop).not.toHaveProperty("width");
  expect(firstLoop).not.toHaveProperty("route");
  expect(firstLoop.controlPoints.length).toBeGreaterThan(0);
  expect(firstLoop.controlPoints.every((id) => typeof id === "string")).toBe(true);
  const firstId = firstLoop.id;
  expect(initial.active).toBe(firstId);
  expect(trackWidth(initial.project)).toBeCloseTo(12, 4);
  await expect(page.locator("#layout-hud")).toContainText("control points");
  await expect(page.locator("#layout-hud")).toContainText("shared");

  await page.locator("#add-layout").click();
  await waitForEditorRebuild(page);

  const added = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
      centerline: viewer.editor_centerline() as string,
    };
  });
  expect(added.project.schema).toBe("lapster/track-project@3");
  expect(added.project.loops).toHaveLength(2);
  expect(added.project.controlPoints.length).toBeGreaterThan(0);
  const secondId = added.active;
  expect(secondId).not.toBe(firstId);
  expect(added.centerline).not.toBe(initial.centerline);
  await expect(page.locator("#layout-hud")).toContainText("control points");
  await expect(page.locator("#layout-hud")).toContainText("shared");

  await page.locator("#layout-select").selectOption(firstId);
  await waitForEditorRebuild(page);
  const switched = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      active: viewer.editor_active_layout() as string,
      centerline: viewer.editor_centerline() as string,
    };
  });
  expect(switched.active).toBe(firstId);
  expect(switched.centerline).toBe(initial.centerline);

  await page.locator("#layout-width").fill("18");
  await page.locator("#layout-width").dispatchEvent("change");
  await waitForEditorRebuild(page);
  const resized = await page.evaluate(
    () =>
      JSON.parse(
        (window as any).__trackEditor.editor_project(),
      ) as ProjectV3,
  );
  expect(resized.schema).toBe("lapster/track-project@3");
  expect(trackWidth(resized)).toBe(18);

  await page.locator("#duplicate-layout").click();
  await waitForEditorRebuild(page);
  const duplicated = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(duplicated.project.loops).toHaveLength(3);
  const duplicateId = duplicated.active;
  expect(duplicateId).not.toBe(firstId);
  const duplicateLoop = duplicated.project.loops.find(
    (loop) => loop.id === duplicateId,
  );
  expect(duplicateLoop).toBeTruthy();
  expect(duplicateLoop!.controlPoints).toEqual(firstLoop.controlPoints);

  await page.locator("#delete-layout").click();
  await waitForEditorRebuild(page);
  const deleted = await page.evaluate((id) => {
    const viewer = (window as any).__trackEditor;
    return {
      loops: (
        JSON.parse(viewer.editor_project()) as {
          loops: Array<{ id: string }>;
        }
      ).loops,
      active: viewer.editor_active_layout() as string,
      deletedId: id,
    };
  }, duplicateId);
  expect(deleted.loops).toHaveLength(2);
  expect(deleted.active).not.toBe(deleted.deletedId);
  const activeAfterDelete = deleted.active;

  const driveRequest = page.waitForRequest(
    (request) => request.url().includes("/web/drive/"),
    { timeout: 20_000 },
  );
  await page.route("**/web/drive/**", (route) =>
    route.fulfill({ status: 200, contentType: "text/html", body: "<!doctype html><title>drive</title>" }),
  );
  await page.locator("#test-drive").click();
  const requestUrl = new URL((await driveRequest).url());
  expect(requestUrl.searchParams.get("layout")).toBe(activeAfterDelete);
  expect(requestUrl.searchParams.get("track")).toBe("editor");

  expect(pageErrors).toEqual([]);
});

test("loops share control points and can be made unique", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=loop-shared");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  await page.locator("#hud-toggle").click();
  await expect(page.locator("#hud")).toBeVisible();
  await expect(page.locator("#status")).toHaveText("ready");

  const initial = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(initial.project.loops).toHaveLength(1);
  const originalId = initial.active;
  const originalLoop = initial.project.loops.find(
    (loop) => loop.id === originalId,
  );
  expect(originalLoop).toBeTruthy();
  expect(originalLoop!.controlPoints.length).toBeGreaterThan(2);

  await page.locator("#duplicate-layout").click();
  await waitForEditorRebuild(page);

  const duplicated = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(duplicated.project.loops).toHaveLength(2);
  const newLoopId = duplicated.active;
  expect(newLoopId).not.toBe(originalId);
  const duplicatedLoop = duplicated.project.loops.find(
    (loop) => loop.id === newLoopId,
  );
  const originalLoopAfterDuplicate = duplicated.project.loops.find(
    (loop) => loop.id === originalId,
  );
  expect(duplicatedLoop).toBeTruthy();
  expect(originalLoopAfterDuplicate).toBeTruthy();
  expect(duplicatedLoop!.controlPoints).toEqual(
    originalLoopAfterDuplicate!.controlPoints,
  );

  const pointIndex = 2;
  const sharedPointId = duplicatedLoop!.controlPoints[pointIndex];
  const sharedPoint = duplicated.project.controlPoints.find(
    (point) => point.id === sharedPointId,
  );
  expect(sharedPoint).toBeTruthy();

  const beforeMove = await readLoopCenterlines(page, newLoopId);
  await waitForEditorRebuild(page);
  const initialCenterline = beforeMove.centerlines[originalId];
  expect(beforeMove.centerlines[newLoopId]).toBe(initialCenterline);

  await page.evaluate(
    async ({ id, x, z }) =>
      await (window as any).__editorMovePoint(id, x, z),
    {
      id: sharedPointId,
      x: sharedPoint!.position[0] + 5,
      z: sharedPoint!.position[1],
    },
  );
  await waitForEditorRebuild(page);
  const afterSharedMove = await readLoopCenterlines(page, newLoopId);
  await waitForEditorRebuild(page);
  expect(afterSharedMove.centerlines[originalId]).not.toBe(initialCenterline);
  expect(afterSharedMove.centerlines[newLoopId]).not.toBe(initialCenterline);
  expect(afterSharedMove.centerlines[newLoopId]).toBe(
    afterSharedMove.centerlines[originalId],
  );

  await page.evaluate(
    async ({ loopId, pointId }) =>
      await (window as any).__editorMakeUnique(loopId, pointId),
    { loopId: newLoopId, pointId: sharedPointId },
  );
  await waitForEditorRebuild(page);

  const unique = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return JSON.parse(viewer.editor_project()) as ProjectV3;
  });
  const uniqueLoop = unique.loops.find((loop) => loop.id === newLoopId);
  const originalLoopAfterUnique = unique.loops.find(
    (loop) => loop.id === originalId,
  );
  expect(uniqueLoop).toBeTruthy();
  expect(originalLoopAfterUnique).toBeTruthy();
  expect(uniqueLoop!.controlPoints).not.toContain(sharedPointId);
  expect(originalLoopAfterUnique!.controlPoints).toContain(sharedPointId);
  const newPointId = uniqueLoop!.controlPoints[pointIndex];
  expect(newPointId).not.toBe(sharedPointId);
  const newPoint = unique.controlPoints.find((point) => point.id === newPointId);
  expect(newPoint).toBeTruthy();

  await page.evaluate((id) => {
    (window as any).__editorSelectPoint(id);
  }, newPointId);
  await expect(page.locator("#make-unique")).toBeEnabled();

  const beforeUniqueMove = await readLoopCenterlines(page, newLoopId);
  await waitForEditorRebuild(page);
  await page.evaluate(
    async ({ id, x, z }) =>
      await (window as any).__editorMovePoint(id, x, z),
    {
      id: newPointId,
      x: newPoint!.position[0] + 7,
      z: newPoint!.position[1],
    },
  );
  await waitForEditorRebuild(page);
  const afterUniqueMove = await readLoopCenterlines(page, newLoopId);
  await waitForEditorRebuild(page);
  expect(afterUniqueMove.centerlines[originalId]).toBe(
    beforeUniqueMove.centerlines[originalId],
  );
  expect(afterUniqueMove.centerlines[newLoopId]).not.toBe(
    beforeUniqueMove.centerlines[newLoopId],
  );

  const driveRequest = page.waitForRequest(
    (request) => request.url().includes("/web/drive/"),
    { timeout: 20_000 },
  );
  await page.route("**/web/drive/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<!doctype html><title>drive</title>",
    }),
  );
  await page.locator("#test-drive").click();
  const requestUrl = new URL((await driveRequest).url());
  expect(requestUrl.searchParams.get("layout")).toBe(newLoopId);
  expect(requestUrl.searchParams.get("track")).toBe("editor");

  expect(pageErrors).toEqual([]);
});

test("a branch creates a new loop whose shared arc keeps the original ids", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=branch-create");
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );

  await page.locator("#hud-toggle").click();
  await expect(page.locator("#hud")).toBeVisible();
  await expect(page.locator("#status")).toHaveText("ready");
  await expect(page.locator("#branch-hint")).toContainText(
    "Creates a new loop with a perpendicular bypass",
  );

  const initial = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(initial.project.loops).toHaveLength(1);
  const originalId = initial.active;
  const originalLoop = initial.project.loops.find(
    (loop) => loop.id === originalId,
  );
  expect(originalLoop).toBeTruthy();
  const originalRoute = originalLoop!.controlPoints;
  expect(originalRoute.length).toBeGreaterThanOrEqual(4);

  const startIndex = 1;
  const endIndex = 3;
  const startId = originalRoute[startIndex];
  const endId = originalRoute[endIndex];
  await page.locator("#branch-start").selectOption(startId);
  await page.locator("#branch-end").selectOption(endId);

  await page.locator("#create-branch").click();
  await waitForEditorRebuild(page);

  const branched = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as ProjectV3,
      active: viewer.editor_active_layout() as string,
    };
  });
  expect(branched.project.loops).toHaveLength(2);
  const newLoopId = branched.active;
  expect(newLoopId).not.toBe(originalId);
  const newLoop = branched.project.loops.find((loop) => loop.id === newLoopId);
  expect(newLoop).toBeTruthy();
  expect(newLoop!.name).toMatch(/^Branch \d+$/);

  const newRoute = newLoop!.controlPoints;
  const p1Id = newRoute[1];
  const p2Id = newRoute[2];
  expect(newRoute.slice(0, 4)).toEqual([startId, p1Id, p2Id, endId]);
  expect(p1Id).toMatch(new RegExp(`^${newLoopId}-cp-1$`));
  expect(p2Id).toMatch(new RegExp(`^${newLoopId}-cp-2$`));
  expect(originalRoute).not.toContain(p1Id);
  expect(originalRoute).not.toContain(p2Id);

  const expectedAfterEnd: string[] = [];
  for (
    let index = (endIndex + 1) % originalRoute.length;
    index !== startIndex;
    index = (index + 1) % originalRoute.length
  ) {
    expectedAfterEnd.push(originalRoute[index]);
  }
  expect(newRoute.slice(4)).toEqual(expectedAfterEnd);

  const segment = (from: string, to: string) =>
    branched.project.segments.find(
      (entry) => entry.from === from && entry.to === to,
    );
  for (const [from, to] of [
    [startId, p1Id],
    [p1Id, p2Id],
    [p2Id, endId],
  ]) {
    const entry = segment(from, to);
    expect(entry).toBeTruthy();
    expect(entry!.role).toBe("branch");
    expect(entry!.kerbs).toEqual([]);
    expect(entry!.propSpans).toEqual([]);
  }
  for (let index = 3; index < newRoute.length; index += 1) {
    const entry = segment(
      newRoute[index],
      newRoute[(index + 1) % newRoute.length],
    );
    expect(entry).toBeTruthy();
    expect(entry!.role).toBe("main");
  }

  const originalAfterBranch = branched.project.loops.find(
    (loop) => loop.id === originalId,
  );
  expect(originalAfterBranch).toBeTruthy();
  expect(originalAfterBranch!.controlPoints).toEqual(originalRoute);

  const branchPoint = branched.project.controlPoints.find(
    (point) => point.id === p1Id,
  );
  expect(branchPoint).toBeTruthy();
  expect(branchPoint!.kind).toBe("normal");

  const loopPositions = originalRoute.map((id) => {
    const point = branched.project.controlPoints.find(
      (candidate) => candidate.id === id,
    );
    expect(point).toBeTruthy();
    return point!.position;
  });
  const centroid: [number, number] = [
    loopPositions.reduce((sum, position) => sum + position[0], 0) /
      loopPositions.length,
    loopPositions.reduce((sum, position) => sum + position[1], 0) /
      loopPositions.length,
  ];
  const startPoint = branched.project.controlPoints.find(
    (point) => point.id === startId,
  );
  expect(startPoint).toBeTruthy();
  const distanceToCentroid = (position: [number, number]): number =>
    Math.hypot(position[0] - centroid[0], position[1] - centroid[1]);
  expect(distanceToCentroid(branchPoint!.position)).toBeGreaterThan(
    distanceToCentroid(startPoint!.position),
  );

  const driveRequest = page.waitForRequest(
    (request) => request.url().includes("/web/drive/"),
    { timeout: 20_000 },
  );
  await page.route("**/web/drive/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<!doctype html><title>drive</title>",
    }),
  );
  await page.locator("#test-drive").click();
  const requestUrl = new URL((await driveRequest).url());
  expect(requestUrl.searchParams.get("layout")).toBe(newLoopId);
  expect(requestUrl.searchParams.get("track")).toBe("editor");

  expect(pageErrors).toEqual([]);
});
