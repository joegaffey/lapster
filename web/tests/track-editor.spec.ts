import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";

const PROJECT_STORAGE_KEY = "lapster.track-editor.project.v6";
const PROJECT_SCHEMA = "lapster/track-project@6";

interface PointData {
  id: string;
  position: [number, number];
  kind: "anchor" | "handle";
  junction: string | null;
  end: string | null;
  degree: number;
  sections: string[];
  startFinish: boolean;
}

interface ProjectData {
  schema?: string;
  name?: string;
  points?: Array<{ id: string; position: [number, number]; kind?: string }>;
  sections?: Array<{ id: string; closed?: boolean; points: string[] }>;
  layouts?: Array<{ id: string; route: unknown[]; startFinish: string }>;
}

async function waitForEditor(page: Page) {
  await page.waitForFunction(
    () =>
      (window as any).__trackEditorReady === true ||
      (window as any).__trackEditorError,
    undefined,
    { timeout: 45_000 },
  );
}

async function waitForReady(page: Page) {
  await page.waitForFunction(
    () => document.querySelector("#status")?.textContent === "ready",
    undefined,
    { timeout: 30_000 },
  );
}

async function readProject(page: Page): Promise<ProjectData> {
  return page.evaluate(
    () =>
      JSON.parse((window as any).__trackEditor.editor_project()) as ProjectData,
  );
}

async function readPoints(page: Page): Promise<PointData[]> {
  return page.evaluate(
    () =>
      JSON.parse((window as any).__trackEditor.editor_points()) as PointData[],
  );
}

async function readRouteCenterline(page: Page): Promise<number[][]> {
  return page.evaluate(
    () =>
      JSON.parse(
        (window as any).__trackEditor.editor_route_centerline(),
      ) as number[][],
  );
}

async function readSectionCenterlines(
  page: Page,
): Promise<Record<string, number[][]>> {
  return page.evaluate(
    () =>
      JSON.parse(
        (window as any).__trackEditor.editor_section_centerlines(),
      ) as Record<string, number[][]>,
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

async function readSelectionRows(page: Page): Promise<Record<string, string>> {
  return page.evaluate(() => {
    const details = document.querySelector("#selection-details");
    const rows: Record<string, string> = {};
    details?.querySelectorAll("dt").forEach((dt) => {
      rows[dt.textContent ?? ""] = dt.nextElementSibling?.textContent ?? "";
    });
    return rows;
  });
}

interface PointEditorState {
  open: boolean;
  visible: boolean;
  modal: boolean;
  pointId: string;
  rows: Record<string, string>;
}

async function readPointEditor(page: Page): Promise<PointEditorState> {
  return page.evaluate(() => {
    const dialog = document.querySelector<HTMLDialogElement>("#point-editor");
    const rows: Record<string, string> = {};
    dialog?.querySelectorAll("#point-editor-rows dt").forEach((dt) => {
      rows[dt.textContent ?? ""] = dt.nextElementSibling?.textContent ?? "";
    });
    return {
      open: dialog?.open ?? false,
      visible: dialog ? dialog.getClientRects().length > 0 : false,
      modal: dialog?.matches(":modal") ?? false,
      pointId: document.querySelector("#point-editor-id")?.textContent ?? "",
      rows,
    };
  });
}

/**
 * Projects a world point through the editor camera using the same maths as
 * `web/track-editor/main.ts`. Returns viewport coordinates.
 */
async function projectWorldPoint(
  page: Page,
  point: [number, number, number],
): Promise<[number, number] | null> {
  const projected = await page.evaluate(
    ({ point }) => {
      const viewer = (window as any).__trackEditor;
      const canvas = document.querySelector("#view") as HTMLCanvasElement;
      const rect = canvas.getBoundingClientRect();
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
      return [
        (ndcX * 0.5 + 0.5) * width + rect.left,
        (0.5 - ndcY * 0.5) * height + rect.top,
      ];
    },
    { point },
  );
  return projected as [number, number] | null;
}

/**
 * Screen position of a control point. The editor derives each point's height
 * from the nearest section-centerline sample, so mirror that here.
 */
async function pointScreen(
  page: Page,
  id: string,
): Promise<[number, number] | null> {
  const world = await page.evaluate((pointId) => {
    const viewer = (window as any).__trackEditor;
    const points = JSON.parse(viewer.editor_points()) as PointData[];
    const point = points.find((item) => item.id === pointId);
    if (!point) {
      return null;
    }
    const centerlines = JSON.parse(viewer.editor_section_centerlines()) as Record<
      string,
      number[][]
    >;
    let best = Infinity;
    let height = 0;
    for (const sectionId of point.sections) {
      for (const sample of centerlines[sectionId] ?? []) {
        const dx = sample[0] - point.position[0];
        const dz = sample[2] - point.position[1];
        const distance = dx * dx + dz * dz;
        if (distance < best) {
          best = distance;
          height = sample[1];
        }
      }
    }
    return [point.position[0], height, point.position[1]] as [
      number,
      number,
      number,
    ];
  }, id);
  if (!world) {
    return null;
  }
  return projectWorldPoint(page, world);
}

/**
 * Finds a route-centerline sample whose screen position is clear of every
 * control-point handle and the draggable car marker, so a click selects the
 * section rather than a point.
 */
async function clearSectionScreen(
  page: Page,
): Promise<{ screen: [number, number]; index: number }> {
  return page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const canvas = document.querySelector("#view") as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    const camera = JSON.parse(viewer.camera_info()) as {
      eye: number[];
      target: number[];
      fovY: number;
    };
    const route = JSON.parse(viewer.editor_route_centerline()) as number[][];
    const points = JSON.parse(viewer.editor_points()) as PointData[];
    const centerlines = JSON.parse(viewer.editor_section_centerlines()) as Record<
      string,
      number[][]
    >;
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
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const tanHalf = Math.tan(camera.fovY * 0.5);
    const aspect = width / Math.max(height, 1);
    const project = (point: number[]): [number, number] | null => {
      const offset = sub(point, eye);
      const depth = dot(offset, forward);
      if (depth <= 0.01) {
        return null;
      }
      const ndcX = dot(offset, right) / (depth * tanHalf * aspect);
      const ndcY = dot(offset, up) / (depth * tanHalf);
      return [
        (ndcX * 0.5 + 0.5) * width + rect.left,
        (0.5 - ndcY * 0.5) * height + rect.top,
      ];
    };

    const pointScreens = points
      .map((point) => {
        let best = Infinity;
        let pointHeight = 0;
        for (const sectionId of point.sections) {
          for (const sample of centerlines[sectionId] ?? []) {
            const dx = sample[0] - point.position[0];
            const dz = sample[2] - point.position[1];
            const distance = dx * dx + dz * dz;
            if (distance < best) {
              best = distance;
              pointHeight = sample[1];
            }
          }
        }
        return project([point.position[0], pointHeight, point.position[1]]);
      })
      .filter((screen): screen is [number, number] => screen !== null);

    const carScreen = route.length
      ? project([route[0][0], route[0][1] + 3.2, route[0][2]])
      : null;

    let bestIndex = -1;
    let bestDistance = -Infinity;
    let bestScreen: [number, number] | null = null;
    route.forEach((sample, index) => {
      const screen = project(sample);
      if (!screen) {
        return;
      }
      let nearest = Infinity;
      for (const handle of pointScreens) {
        nearest = Math.min(
          nearest,
          Math.hypot(screen[0] - handle[0], screen[1] - handle[1]),
        );
      }
      if (carScreen) {
        nearest = Math.min(
          nearest,
          Math.hypot(screen[0] - carScreen[0], screen[1] - carScreen[1]),
        );
      }
      if (nearest > bestDistance) {
        bestDistance = nearest;
        bestIndex = index;
        bestScreen = screen;
      }
    });
    return { screen: bestScreen!, index: bestIndex };
  });
}

async function emptyCanvasScreen(
  page: Page,
): Promise<{ screen: [number, number]; distance: number }> {
  return page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const canvas = document.querySelector("#view") as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    const camera = JSON.parse(viewer.camera_info()) as {
      eye: number[];
      target: number[];
      fovY: number;
    };
    const route = JSON.parse(viewer.editor_route_centerline()) as number[][];
    const points = JSON.parse(viewer.editor_points()) as PointData[];
    const centerlines = JSON.parse(viewer.editor_section_centerlines()) as Record<
      string,
      number[][]
    >;
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
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const tanHalf = Math.tan(camera.fovY * 0.5);
    const aspect = width / Math.max(height, 1);
    const project = (point: number[]): [number, number] | null => {
      const offset = sub(point, eye);
      const depth = dot(offset, forward);
      if (depth <= 0.01) {
        return null;
      }
      const ndcX = dot(offset, right) / (depth * tanHalf * aspect);
      const ndcY = dot(offset, up) / (depth * tanHalf);
      return [
        (ndcX * 0.5 + 0.5) * width + rect.left,
        (0.5 - ndcY * 0.5) * height + rect.top,
      ];
    };

    const obstacles: Array<[number, number]> = [];
    for (const sample of route) {
      const screen = project(sample);
      if (screen) obstacles.push(screen);
    }
    for (const point of points) {
      let best = Infinity;
      let pointHeight = 0;
      for (const sectionId of point.sections) {
        for (const sample of centerlines[sectionId] ?? []) {
          const dx = sample[0] - point.position[0];
          const dz = sample[2] - point.position[1];
          const distance = dx * dx + dz * dz;
          if (distance < best) {
            best = distance;
            pointHeight = sample[1];
          }
        }
      }
      const screen = project([
        point.position[0],
        pointHeight,
        point.position[1],
      ]);
      if (screen) obstacles.push(screen);
    }

    const margin = 24;
    const top = 110;
    let bestScreen: [number, number] = [
      rect.left + width - margin,
      rect.top + height - margin,
    ];
    let bestDistance = -1;
    for (let gx = 0; gx <= 12; gx += 1) {
      for (let gy = 0; gy <= 12; gy += 1) {
        const x = rect.left + margin + ((width - 2 * margin) * gx) / 12;
        const y = rect.top + top + ((height - top - 2 * margin) * gy) / 12;
        // Keep clear of the bottom-left HUD/dialog panel.
        if (x < rect.left + 380 && y > rect.top + height - 320) {
          continue;
        }
        let nearest = Infinity;
        for (const obstacle of obstacles) {
          nearest = Math.min(
            nearest,
            Math.hypot(x - obstacle[0], y - obstacle[1]),
          );
        }
        if (nearest > bestDistance) {
          bestDistance = nearest;
          bestScreen = [x, y];
        }
      }
    }
    return { screen: bestScreen, distance: bestDistance };
  });
}

async function dragPoint(
  page: Page,
  id: string,
  dx = 44,
  dy = 22,
): Promise<PointData> {
  const points = await readPoints(page);
  const target = points.find((point) => point.id === id);
  expect(target, `point ${id} exists`).toBeDefined();
  const screen = await pointScreen(page, id);
  expect(screen, `point ${id} is visible`).not.toBeNull();
  await page.mouse.move(screen![0], screen![1]);
  await page.mouse.down();
  await page.mouse.move(screen![0] + dx, screen![1] + dy, { steps: 12 });
  await page.mouse.up();
  await page.waitForFunction(
    ({ pointId, start }) => {
      if (document.querySelector("#status")?.textContent !== "ready") {
        return false;
      }
      const points = JSON.parse(
        (window as any).__trackEditor.editor_points(),
      ) as PointData[];
      const point = points.find((item) => item.id === pointId);
      return (
        !!point &&
        (Math.abs(point.position[0] - start[0]) > 1e-3 ||
          Math.abs(point.position[1] - start[1]) > 1e-3)
      );
    },
    { pointId: id, start: target!.position },
    { timeout: 30_000 },
  );
  const updated = await readPoints(page);
  return updated.find((point) => point.id === id)!;
}

async function findPropGap(page: Page, offset: number) {
  return page.evaluate(
    ({ offset }) => {
      const viewer = (window as any).__trackEditor;
      const lines = JSON.parse(viewer.editor_prop_lines()) as Array<{
        start: number;
        end: number;
        side: number;
        offset: number;
      }>;
      const count = (JSON.parse(viewer.editor_route_centerline()) as number[][])
        .length;
      for (const side of [-1, 1]) {
        const ranges = lines
          .filter(
            (line) =>
              line.side === side && Math.abs(line.offset - offset) < 1e-4,
          )
          .sort((a, b) => a.start - b.start);
        let previous = 0;
        for (const range of [
          ...ranges,
          { start: count, end: count, side, offset },
        ]) {
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
    },
    { offset },
  );
}

function collectPageErrors(page: Page): string[] {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });
  return pageErrors;
}

test("track editor boots the @6 single-section B-spline loop", async ({ page }) => {
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=boot-v6");
  await waitForEditor(page);
  await page.waitForTimeout(750);
  await expect(page.locator("#hud")).toHaveClass(/hud-collapsed/);

  const state = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const project = JSON.parse(viewer.editor_project());
    const points = JSON.parse(viewer.editor_points()) as PointData[];
    const sections = JSON.parse(viewer.editor_sections());
    const route = JSON.parse(viewer.editor_route());
    const routeCenterline = JSON.parse(
      viewer.editor_route_centerline(),
    ) as number[][];
    const sectionCenterlines = JSON.parse(
      viewer.editor_section_centerlines(),
    ) as Record<string, number[][]>;
    return {
      ready: (window as any).__trackEditorReady,
      error: (window as any).__trackEditorError,
      status: document.querySelector("#status")?.textContent,
      stats: document.querySelector("#stats")?.textContent ?? "",
      draws: document.querySelector("#draws")?.textContent,
      surfaces: document.querySelector("#surfaces")?.textContent ?? "",
      meshes: document.querySelector("#meshes")?.textContent ?? "",
      debug: JSON.parse(viewer.debug_info()),
      project,
      points,
      sections,
      route,
      routeCenterline,
      sectionCenterlines,
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
  expect(state.status).toBe("ready");
  expect(state.project.schema).toBe(PROJECT_SCHEMA);
  expect(state.activeLayout).toBe("procedural-drift-loop");

  // Boot model: one closed section, eight all-handle points, no anchors or
  // junctions, and point-0 as the layout start/finish.
  expect(state.points.length).toBe(8);
  expect(state.points.every((point) => point.kind === "handle")).toBe(true);
  expect(state.points.filter((point) => point.kind === "anchor").length).toBe(0);
  expect(state.points.filter((point) => point.junction !== null).length).toBe(0);
  expect(state.sections.length).toBe(1);
  expect(state.sections[0].id).toBe("section-0");
  expect(state.sections[0].closed).toBe(true);
  expect(state.sections[0].points).toEqual([
    "point-0",
    "point-1",
    "point-2",
    "point-3",
    "point-4",
    "point-5",
    "point-6",
    "point-7",
  ]);
  expect(state.project.layouts?.length).toBe(1);
  expect(state.project.layouts?.[0].startFinish).toBe("point-0");
  const startFinish = state.points.find((point) => point.id === "point-0");
  expect(startFinish?.startFinish).toBe(true);
  expect(state.route.length).toBe(1);
  expect(state.route[0].sectionId).toBe("section-0");
  expect(state.routeCenterline.length).toBe(120);
  expect(state.sectionCenterlines["section-0"].length).toBeGreaterThan(1);

  expect(markerPixels.markers).toBeGreaterThan(20);
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

test("selecting a handle shows its id, role, and off-curve contact", async ({
  page,
}) => {
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-selection");
  await waitForEditor(page);

  const screen = await pointScreen(page, "point-0");
  expect(screen).not.toBeNull();
  await page.mouse.click(screen![0], screen![1]);
  await waitForReady(page);

  const editor = await readPointEditor(page);
  expect(editor.open).toBe(true);
  expect(editor.visible).toBe(true);
  expect(editor.pointId).toBe("point-0");
  expect(editor.rows.kind).toBe("point");
  expect(editor.rows.id).toBe("point-0");
  expect(editor.rows["point kind"]).toBe("handle");
  expect(editor.rows.role).toBe("start/finish");
  expect(editor.rows.sections).toBe("section-0");

  // point-0 is an off-road start/finish handle: its contact is > 1 m away.
  const contact = Number.parseFloat(editor.rows.contact);
  expect(Number.isFinite(contact)).toBe(true);
  expect(contact).toBeGreaterThan(0.5);

  // The HUD keeps only a short hint; point rows live in the dialog.
  const selection = await readSelectionRows(page);
  expect(selection.kind).toBeUndefined();
  expect(selection.hint).toContain("point-0");

  expect(pageErrors).toEqual([]);
});

test("Alt-clicking the route centerline selects the closed section", async ({
  page,
}) => {
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=section-selection");
  await waitForEditor(page);

  const target = await clearSectionScreen(page);
  expect(target.index).toBeGreaterThanOrEqual(0);
  await page.keyboard.down("Alt");
  await page.mouse.click(target.screen[0], target.screen[1]);
  await page.keyboard.up("Alt");
  await waitForReady(page);

  const rows = await readSelectionRows(page);
  expect(rows.kind).toBe("section");
  expect(rows.id).toBe("section-0");
  expect(rows.state).toBe("closed");
  expect(rows.points).toBe("8");
  expect((await readPointEditor(page)).open).toBe(false);

  expect(pageErrors).toEqual([]);
});

test("clicking the road inserts and selects a new point", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-insert");
  await waitForEditor(page);
  await waitForReady(page);

  const beforePoints = await readPoints(page);
  const beforeIds = new Set(beforePoints.map((point) => point.id));
  const beforeCenterlines = await readSectionCenterlines(page);
  const target = await clearSectionScreen(page);
  expect(target.index).toBeGreaterThanOrEqual(0);

  await page.mouse.move(target.screen[0], target.screen[1]);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (document.querySelector("#view") as HTMLCanvasElement).style.cursor,
      ),
    )
    .toBe("copy");

  await page.mouse.click(target.screen[0], target.screen[1]);
  await waitForReady(page);
  await expect.poll(async () => (await readPoints(page)).length).toBe(
    beforePoints.length + 1,
  );

  const afterPoints = await readPoints(page);
  const inserted = afterPoints.find((point) => !beforeIds.has(point.id));
  expect(inserted, "inserted point").toBeDefined();
  const editor = await readPointEditor(page);
  expect(editor.open).toBe(true);
  expect(editor.pointId).toBe(inserted!.id);
  expect(editor.rows.kind).toBe("point");
  expect(editor.rows.id).toBe(inserted!.id);

  const project = await readProject(page);
  expect(project.points?.some((point) => point.id === inserted!.id)).toBe(true);
  const afterCenterlines = await readSectionCenterlines(page);
  expect(afterCenterlines["section-0"]).not.toEqual(
    beforeCenterlines["section-0"],
  );

  expect(pageErrors).toEqual([]);
});

test("an inserted point can be dragged and deleted", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-insert-drag-delete");
  await waitForEditor(page);
  await waitForReady(page);

  const beforePoints = await readPoints(page);
  const beforeIds = new Set(beforePoints.map((point) => point.id));
  const target = await clearSectionScreen(page);
  await page.mouse.click(target.screen[0], target.screen[1]);
  await waitForReady(page);
  await expect.poll(async () => (await readPoints(page)).length).toBe(
    beforePoints.length + 1,
  );

  const inserted = (await readPoints(page)).find(
    (point) => !beforeIds.has(point.id),
  )!;
  const moved = await dragPoint(page, inserted.id);
  expect(
    Math.abs(moved.position[0] - inserted.position[0]) +
      Math.abs(moved.position[1] - inserted.position[1]),
  ).toBeGreaterThan(1e-3);

  await page.locator("#delete-point").click();
  await waitForReady(page);
  await expect.poll(async () => (await readPoints(page)).length).toBe(
    beforePoints.length,
  );

  const editor = await readPointEditor(page);
  expect(editor.open).toBe(false);
  expect(editor.visible).toBe(false);
  const rows = await readSelectionRows(page);
  expect(rows.kind).toBe("none");
  expect((await readPoints(page)).some((point) => point.id === inserted.id)).toBe(
    false,
  );

  expect(pageErrors).toEqual([]);
});

test("a layout start/finish point cannot be deleted", async ({ page }) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-delete-blocked");
  await waitForEditor(page);
  await waitForReady(page);

  const beforePoints = await readPoints(page);
  const screen = await pointScreen(page, "point-0");
  expect(screen).not.toBeNull();
  await page.mouse.click(screen![0], screen![1]);
  await waitForReady(page);

  await page.locator("#delete-point").click();
  await page.waitForFunction(() => {
    const status = document.querySelector("#status");
    return (
      status?.classList.contains("error") === true &&
      /cannot delete point 'point-0'/.test(status.textContent ?? "")
    );
  });

  expect((await readPoints(page)).length).toBe(beforePoints.length);
  expect((await readPoints(page)).some((point) => point.id === "point-0")).toBe(
    true,
  );
  const editor = await readPointEditor(page);
  expect(editor.open).toBe(true);
  expect(editor.visible).toBe(true);
  expect(editor.rows.kind).toBe("point");
  expect(editor.rows.id).toBe("point-0");
  expect((await readSelectionRows(page)).hint).toContain("point-0");
  expect((await readProject(page)).layouts?.[0].startFinish).toBe("point-0");
  expect(pageErrors).toEqual([]);
});

test("setting start/finish updates the active layout and point", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-start-finish");
  await waitForEditor(page);
  await waitForReady(page);

  const screen = await pointScreen(page, "point-3");
  expect(screen).not.toBeNull();
  await page.mouse.click(screen![0], screen![1]);
  const editor = await readPointEditor(page);
  expect(editor.open).toBe(true);
  expect(editor.rows.kind).toBe("point");
  expect(editor.rows.id).toBe("point-3");

  await page.locator("#set-start-finish").click();
  await waitForReady(page);
  await expect
    .poll(async () => (await readProject(page)).layouts?.[0].startFinish)
    .toBe("point-3");

  const points = await readPoints(page);
  expect(points.find((point) => point.id === "point-3")?.startFinish).toBe(true);
  expect(points.find((point) => point.id === "point-0")?.startFinish).toBe(
    false,
  );
  const after = await readPointEditor(page);
  expect(after.open).toBe(true);
  expect(after.rows.role).toBe("start/finish");
  expect(pageErrors).toEqual([]);
});

test("point editor stays open while non-modal and closes with selection", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-editor-lifecycle");
  await waitForEditor(page);
  await waitForReady(page);

  const selectPoint = async (id: string) => {
    const screen = await pointScreen(page, id);
    expect(screen, `point ${id} is visible`).not.toBeNull();
    await page.mouse.click(screen![0], screen![1]);
    const editor = await readPointEditor(page);
    expect(editor.open).toBe(true);
    expect(editor.visible).toBe(true);
    expect(editor.pointId).toBe(id);
    return editor;
  };

  const editor = await selectPoint("point-3");
  expect(editor.modal).toBe(false);

  // A non-modal dialog has no backdrop: canvas is still the hit target.
  expect(
    await page.evaluate(() => {
      const dialog = document.querySelector<HTMLDialogElement>("#point-editor")!;
      const canvas = document.querySelector("#view") as HTMLCanvasElement;
      const rect = canvas.getBoundingClientRect();
      const hit = document.elementFromPoint(
        rect.left + rect.width * 0.75,
        rect.top + rect.height * 0.5,
      );
      return { modal: dialog.matches(":modal"), hit: hit?.id ?? "" };
    }),
  ).toEqual({ modal: false, hit: "view" });

  await page.keyboard.press("Escape");
  expect((await readPointEditor(page)).open).toBe(false);
  expect((await readSelectionRows(page)).kind).toBe("none");

  await selectPoint("point-4");
  const section = await clearSectionScreen(page);
  expect(section.index).toBeGreaterThanOrEqual(0);
  await page.keyboard.down("Alt");
  await page.mouse.click(section.screen[0], section.screen[1]);
  await page.keyboard.up("Alt");
  expect((await readPointEditor(page)).open).toBe(false);
  const sectionRows = await readSelectionRows(page);
  expect(sectionRows.kind).toBe("section");
  expect(sectionRows.id).toBe("section-0");

  await selectPoint("point-5");
  const empty = await emptyCanvasScreen(page);
  expect(empty.distance).toBeGreaterThan(20);
  await page.mouse.click(empty.screen[0], empty.screen[1]);
  expect((await readPointEditor(page)).open).toBe(false);
  expect((await readSelectionRows(page)).kind).toBe("none");

  await selectPoint("point-3");
  const before = (await readPoints(page)).find(
    (point) => point.id === "point-3",
  )!;
  const dragged = await dragPoint(page, "point-3");
  expect(
    Math.abs(dragged.position[0] - before.position[0]) +
      Math.abs(dragged.position[1] - before.position[1]),
  ).toBeGreaterThan(1e-3);
  const afterDrag = await readPointEditor(page);
  expect(afterDrag.open).toBe(true);
  expect(afterDrag.pointId).toBe("point-3");

  expect(pageErrors).toEqual([]);
});

test("dragging a handle moves the point and rebuilds the route", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=point-drag");
  await waitForEditor(page);

  const beforePoints = await readPoints(page);
  const beforeRoute = await readRouteCenterline(page);
  const dragged = await dragPoint(page, "point-3");
  const afterRoute = await readRouteCenterline(page);
  const original = beforePoints.find((point) => point.id === "point-3")!;

  expect(
    Math.abs(dragged.position[0] - original.position[0]) +
      Math.abs(dragged.position[1] - original.position[1]),
  ).toBeGreaterThan(1e-3);
  expect(afterRoute).not.toEqual(beforeRoute);
  expect(await page.evaluate(() => (window as any).__trackEditorReady)).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("default project exports a non-empty track pack", async ({ page }) => {
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=test-drive-pack");
  await waitForEditor(page);

  const pack = await page.evaluate(() => {
    const bytes = (window as any).__trackEditor.editor_track_pack() as Uint8Array;
    return { length: bytes.length, bytes: Array.from(bytes.slice(0, 4)) };
  });

  expect(pack.length).toBeGreaterThan(0);
  expect(pack.bytes.some((value) => value !== 0)).toBe(true);
  expect(page.url()).toContain("/web/track-editor/");
  expect(pageErrors).toEqual([]);
});

test("project export and import round-trip the normalized project", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=project-round-trip");
  await waitForEditor(page);
  await waitForReady(page);

  const defaultProject = await readProject(page);
  await dragPoint(page, "point-5", 32, 20);
  const modified = await readProject(page);

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

test("project saves to localStorage, reloads on boot, and reset clears it", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=project-storage");
  await waitForEditor(page);
  await waitForReady(page);

  const defaultProject = await readProject(page);
  await dragPoint(page, "point-6", 30, -24);
  const modified = await readProject(page);

  await page.locator("#save-project").click();
  await page.waitForFunction(
    ({ key, expectedJson }) => {
      const stored = localStorage.getItem(key);
      return (
        stored !== null && JSON.stringify(JSON.parse(stored)) === expectedJson
      );
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

test("importing an invalid project reports an error without breaking the editor", async ({
  page,
}) => {
  const pageErrors = collectPageErrors(page);

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
  expect(await page.evaluate(() => (window as any).__trackEditorReady)).toBe(
    true,
  );
  expect(await readPoints(page)).toHaveLength(8);
  expect(pageErrors).toEqual([]);
});

test("camera view cycles and the draggable car follows the route", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=camera-view");
  await waitForEditor(page);

  const toggle = page.locator("#camera-view");
  await expect(toggle).toHaveText("View: Front");
  const centerline = await readRouteCenterline(page);

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
      () =>
        JSON.parse((window as any).__trackEditor.camera_info()) as {
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

test("default tree rings have gaps on both sides", async ({ page }) => {
  const pageErrors = collectPageErrors(page);

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

test("tree detail toggle switches between high, low, and card trees", async ({
  page,
}) => {
  const pageErrors = collectPageErrors(page);

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
