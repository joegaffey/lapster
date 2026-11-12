import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";

const PROJECT_STORAGE_KEY = "lapster.track-editor.project.v5";
const PROJECT_SCHEMA = "lapster/track-project@5";

interface LoopPointData {
  id: string;
  position: [number, number];
  kind: string;
  degree: number;
}

async function waitForEditor(page: Page) {
  await page.waitForFunction(
    () => (window as any).__trackEditorReady === true || (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );
}

async function readProject(page: Page) {
  return page.evaluate(
    () => JSON.parse((window as any).__trackEditor.editor_project()) as Record<string, any>,
  );
}

async function readLoopPoints(page: Page): Promise<LoopPointData[]> {
  return page.evaluate(
    () => JSON.parse((window as any).__trackEditor.editor_loop_points()) as LoopPointData[],
  );
}

async function waitForProject(page: Page, expected: unknown) {
  await page.waitForFunction(
    (expectedJson) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      try {
        return (
          JSON.stringify(
            JSON.parse((window as any).__trackEditor.editor_project()),
          ) === expectedJson
        );
      } catch {
        return false;
      }
    },
    JSON.stringify(expected),
    { timeout: 30_000 },
  );
}

async function loopPointWorld(
  page: Page,
  id: string,
): Promise<[number, number, number] | null> {
  return page.evaluate((junctionId) => {
    const viewer = (window as any).__trackEditor;
    const points = JSON.parse(viewer.editor_loop_points()) as LoopPointData[];
    const point = points.find((item) => item.id === junctionId);
    if (!point) {
      return null;
    }
    const project = JSON.parse(viewer.editor_project()) as {
      segments?: Array<{ id: string; from: string; to: string }>;
    };
    const centerlines = JSON.parse(viewer.editor_segment_centerlines()) as Record<
      string,
      number[][]
    >;
    const heights = new Map<string, number>();
    for (const segment of project.segments ?? []) {
      const samples = centerlines[segment.id];
      if (!samples || samples.length === 0) {
        continue;
      }
      if (!heights.has(segment.from)) {
        heights.set(segment.from, samples[0][1]);
      }
      if (!heights.has(segment.to)) {
        heights.set(segment.to, samples[samples.length - 1][1]);
      }
    }
    return [point.position[0], heights.get(junctionId) ?? 0, point.position[1]] as [
      number,
      number,
      number,
    ];
  }, id);
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

async function dragJunction(
  page: Page,
  index = 2,
  dx = 44,
  dy = 18,
): Promise<Record<string, any>> {
  const loops = await readLoopPoints(page);
  expect(loops.length).toBeGreaterThan(index);
  const target = loops[index];
  const world = await loopPointWorld(page, target.id);
  expect(world).not.toBeNull();
  const screen = await projectWorldPoint(page, world!);
  expect(screen).not.toBeNull();
  await page.mouse.move(screen![0], screen![1]);
  await page.mouse.down();
  await page.mouse.move(screen![0] + dx, screen![1] + dy, { steps: 12 });
  await page.mouse.up();
  await page.waitForFunction(
    ({ id, start }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const project = JSON.parse((window as any).__trackEditor.editor_project()) as {
        junctions?: Array<{ id: string; position: [number, number] }>;
      };
      const junction = project.junctions?.find((item) => item.id === id);
      return (
        !!junction &&
        (Math.abs(junction.position[0] - start[0]) > 1e-3 ||
          Math.abs(junction.position[1] - start[1]) > 1e-3)
      );
    },
    { id: target.id, start: target.position },
    { timeout: 30_000 },
  );
  return readProject(page);
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

test("track editor renders the procedural example track", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=editor-preview");
  await waitForEditor(page);
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
      loopPoints: JSON.parse(viewer.editor_loop_points()) as unknown[],
      project: JSON.parse(viewer.editor_project()) as { schema?: string },
      activeLayout: viewer.editor_active_layout(),
    };
  });

  const markerPixels = await page.evaluate(() => {
    const canvas = document.querySelector("#control-points") as HTMLCanvasElement;
    const context = canvas.getContext("2d")!;
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let markers = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      const red = pixels[index];
      const green = pixels[index + 1];
      const blue = pixels[index + 2];
      const alpha = pixels[index + 3];
      if (alpha < 16) continue;
      if (red > 220 && green > 150 && green < 220 && blue < 90) {
        markers += 1;
      }
    }
    return { markers };
  });

  expect(state.error).toBeUndefined();
  expect(state.ready).toBe(true);
  expect(state.project.schema).toBe(PROJECT_SCHEMA);
  expect(state.activeLayout).toBe("procedural-drift-loop");
  expect(state.loopPoints.length).toBe(8);
  expect(markerPixels.markers).toBeGreaterThan(20);
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

test("camera view cycles and the draggable car follows the spline", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=camera-view");
  await waitForEditor(page);

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

test("dragging a junction updates the project and rerenders the track", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=junction-drag");
  await waitForEditor(page);

  const before = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      centerline: JSON.parse(viewer.editor_centerline()) as number[][],
      camera: JSON.parse(viewer.camera_info()) as {
        eye: number[];
        target: number[];
        fovY: number;
      },
    };
  });

  const modified = await dragJunction(page);
  const after = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      centerline: JSON.parse(viewer.editor_centerline()) as number[][],
      camera: JSON.parse(viewer.camera_info()) as {
        eye: number[];
        target: number[];
        fovY: number;
      },
      status: document.querySelector("#status")?.textContent,
    };
  });

  expect(after.status).toBe("ready");
  expect(after.centerline).not.toEqual(before.centerline);
  expect(modified.schema).toBe(PROJECT_SCHEMA);
  expect(modified.junctions.length).toBe(8);
  for (let axis = 0; axis < 3; axis += 1) {
    expect(
      Math.abs(after.camera.eye[axis] - before.camera.eye[axis]),
      `camera eye[${axis}]`,
    ).toBeLessThanOrEqual(0.01);
    expect(
      Math.abs(after.camera.target[axis] - before.camera.target[axis]),
      `camera target[${axis}]`,
    ).toBeLessThanOrEqual(0.01);
  }
  expect(Math.abs(after.camera.fovY - before.camera.fovY)).toBeLessThanOrEqual(0.01);
  expect(pageErrors).toEqual([]);
});

test("project export and import round-trip the normalized project", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=project-round-trip");
  await waitForEditor(page);

  const defaultProject = await readProject(page);
  const modified = await dragJunction(page);

  const downloadPromise = page.waitForEvent("download");
  await page.locator("#export-project").click();
  const download = await downloadPromise;
  const downloadPath = await download.path();
  expect(downloadPath).not.toBeNull();
  const exported = JSON.parse(await readFile(downloadPath!, "utf8"));
  expect(exported).toEqual(modified);
  expect(exported.schema).toBe(PROJECT_SCHEMA);

  await page.locator("#reset-project").click();
  await waitForProject(page, defaultProject);

  await page.locator("#import-project-file").setInputFiles(downloadPath!);
  await waitForProject(page, modified);
  expect(await readProject(page)).toEqual(modified);
  expect(pageErrors).toEqual([]);
});

test("project saves to localStorage, reloads on boot, and reset clears it", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=project-storage");
  await waitForEditor(page);

  const defaultProject = await readProject(page);
  const modified = await dragJunction(page);

  await page.locator("#save-project").click();
  await page.waitForFunction(
    ({ key, expectedJson }) => {
      const stored = localStorage.getItem(key);
      return stored !== null && JSON.stringify(JSON.parse(stored)) === expectedJson;
    },
    { key: PROJECT_STORAGE_KEY, expectedJson: JSON.stringify(modified) },
  );

  await page.reload();
  await waitForEditor(page);
  await waitForProject(page, modified);

  await page.locator("#reset-project").click();
  await waitForProject(page, defaultProject);
  await page.waitForFunction(
    (key) => localStorage.getItem(key) === null,
    PROJECT_STORAGE_KEY,
  );

  await page.reload();
  await waitForEditor(page);
  await waitForProject(page, defaultProject);
  expect(pageErrors).toEqual([]);
});

test("importing an invalid project reports an error without breaking the editor", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=project-invalid-import");
  await waitForEditor(page);
  const before = await readProject(page);

  await page.locator("#import-project-file").setInputFiles({
    name: "invalid-project.json",
    mimeType: "application/json",
    buffer: Buffer.from("{ not valid json"),
  });

  await page.waitForFunction(() => {
    const status = document.querySelector("#status");
    return (
      status?.classList.contains("error") === true &&
      /import failed/i.test(status.textContent ?? "")
    );
  });

  expect(await readProject(page)).toEqual(before);
  expect(
    await page.evaluate(() => (window as any).__trackEditorError),
  ).toBeUndefined();
  expect(await page.evaluate(() => (window as any).__trackEditorReady)).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("default tree rings have gaps on both sides", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=prop-trees");
  await waitForEditor(page);

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
  await waitForEditor(page);

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
