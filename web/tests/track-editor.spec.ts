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

interface DecorationData {
  id: string;
  kind: "kerb" | "prop";
  side: number;
  lane?: string;
  chain: Array<{
    sectionId: string;
    entryFraction: number;
    exitFraction: number;
    wrap?: boolean;
  }>;
  elements?: number[];
  phase?: number;
  propKind?: string;
  spacing?: number;
}

interface KerbSpanData {
  start: number;
  end: number;
  side: number;
}

interface ProjectData {
  schema?: string;
  name?: string;
  points?: Array<{ id: string; position: [number, number]; kind?: string }>;
  sections?: Array<{ id: string; closed?: boolean; points: string[] }>;
  layouts?: Array<{ id: string; route: unknown[]; startFinish: string }>;
  decorations?: DecorationData[];
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

async function readDecorations(page: Page): Promise<DecorationData[]> {
  const project = await readProject(page);
  return project.decorations ?? [];
}

async function readKerbSpans(page: Page): Promise<KerbSpanData[]> {
  return page.evaluate(
    () =>
      JSON.parse(
        (window as any).__trackEditor.editor_kerb_spans(),
      ) as KerbSpanData[],
  );
}

type EditorMode = "points" | "kerbs" | "props";

interface RailData {
  kind: "kerb" | "prop";
  side: number;
  offset: number;
  lane: string | null;
}

interface RailSpec {
  kind: "kerb" | "prop";
  side: number;
  lane?: string;
  offset?: number;
}

async function readGuideRails(page: Page): Promise<RailData[]> {
  return page.evaluate(
    () =>
      JSON.parse((window as any).__trackEditor.editor_guide_rails()) as RailData[],
  );
}

function railSpecForDecoration(
  decoration: DecorationData,
  rails: RailData[],
): RailSpec | null {
  const side = decoration.side < 0 ? -1 : 1;
  if (decoration.kind === "kerb") {
    const rail = rails.find((item) => item.kind === "kerb" && item.side === side);
    return rail ? { kind: "kerb", side, offset: rail.offset } : null;
  }
  const lane = decoration.lane ?? "";
  let rail = rails.find(
    (item) =>
      item.kind === "prop" &&
      item.side === side &&
      (item.lane ?? "") === lane,
  );
  const numeric = Number(lane);
  if (!rail && Number.isFinite(numeric) && lane.trim() !== "") {
    rail = rails.find(
      (item) =>
        item.kind === "prop" &&
        item.side === side &&
        Math.abs(item.offset - numeric) < 1e-4,
    );
  }
  if (!rail) {
    const offset = lane === "far" ? 19 : 14;
    rail = rails.find(
      (item) =>
        item.kind === "prop" &&
        item.side === side &&
        Math.abs(item.offset - offset) < 1e-4,
    );
  }
  return rail ? { kind: "prop", side, lane: lane || undefined, offset: rail.offset } : null;
}

async function railWorldAt(
  page: Page,
  spec: RailSpec,
  fraction: number,
): Promise<[number, number, number] | null> {
  return page.evaluate(
    ({ spec, fraction }) => {
      const viewer = (window as any).__trackEditor;
      const rails = JSON.parse(viewer.editor_guide_rails()) as RailData[];
      const rail = rails.find(
        (item) =>
          item.kind === spec.kind &&
          item.side === spec.side &&
          ((spec.lane !== undefined && (item.lane ?? "") === spec.lane) ||
            (spec.offset !== undefined &&
              Math.abs(item.offset - spec.offset) < 1e-4)),
      );
      if (!rail) {
        return null;
      }
      const route = JSON.parse(viewer.editor_route()) as Array<{
        sectionId: string;
      }>;
      const sectionId = route[0]?.sectionId;
      if (!sectionId) {
        return null;
      }
      const project = JSON.parse(viewer.editor_project()) as ProjectData;
      const section = (project.sections ?? []).find(
        (item) => item.id === sectionId,
      );
      const closed = section?.closed !== false;
      const samples = (
        JSON.parse(viewer.editor_section_centerlines()) as Record<
          string,
          number[][]
        >
      )[sectionId];
      if (!samples || samples.length < 2) {
        return null;
      }
      const count = samples.length;
      const offsetAt = (index: number): number[] => {
        const wrapped = ((index % count) + count) % count;
        const previousIndex = closed
          ? (wrapped + count - 1) % count
          : Math.max(0, wrapped - 1);
        const nextIndex = closed
          ? (wrapped + 1) % count
          : Math.min(count - 1, wrapped + 1);
        const previous = samples[previousIndex];
        const next = samples[nextIndex];
        const dx = next[0] - previous[0];
        const dz = next[2] - previous[2];
        const length = Math.hypot(dx, dz);
        let tangentX = length > 1e-8 ? dx / length : 0;
        let tangentZ = length > 1e-8 ? dz / length : -1;
        if (Math.abs(tangentX) < 1e-8 && Math.abs(tangentZ) < 1e-8) {
          tangentX = 0;
          tangentZ = -1;
        }
        const binormalX = -tangentZ;
        const binormalZ = tangentX;
        const sample = samples[wrapped];
        const lateral = rail.offset * rail.side;
        return [
          sample[0] + binormalX * lateral,
          sample[1] + 0.05,
          sample[2] + binormalZ * lateral,
        ];
      };
      const clamped = Math.max(0, Math.min(1, fraction));
      const position = clamped * (count - 1);
      const first = Math.floor(position);
      const second = Math.min(count - 1, first + 1);
      const t = position - first;
      const a = offsetAt(first);
      const b = offsetAt(second);
      return [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
      ];
    },
    { spec, fraction },
  );
}

async function railScreenAt(
  page: Page,
  spec: RailSpec,
  fraction: number,
): Promise<[number, number] | null> {
  const world = await railWorldAt(page, spec, fraction);
  return world ? projectWorldPoint(page, world) : null;
}

async function decorationScreen(
  page: Page,
  decoration: DecorationData,
  rails: RailData[],
  fraction: number,
): Promise<[number, number] | null> {
  const spec = railSpecForDecoration(decoration, rails);
  return spec ? railScreenAt(page, spec, fraction) : null;
}

async function readModeState(page: Page) {
  return page.evaluate(() => {
    const pressed = (id: string) =>
      document.querySelector<HTMLButtonElement>(id)?.getAttribute("aria-pressed") ??
      "";
    const inspector = document.querySelector<HTMLDialogElement>(
      "#decoration-inspector",
    );
    const pointEditor = document.querySelector<HTMLDialogElement>("#point-editor");
    return {
      points: pressed("#mode-points"),
      kerbs: pressed("#mode-kerbs"),
      props: pressed("#mode-props"),
      inspectorOpen: inspector?.open ?? false,
      inspectorVisible: inspector ? inspector.getClientRects().length > 0 : false,
      pointEditorOpen: pointEditor?.open ?? false,
    };
  });
}

async function readDecorationInspector(page: Page) {
  return page.evaluate(() => {
    const field = (id: string) => {
      const element = document.querySelector<HTMLInputElement | HTMLSelectElement>(
        id,
      );
      const wrapper = element?.closest(".decoration-field") as HTMLElement | null;
      return {
        value: element?.value ?? "",
        hidden: wrapper?.hidden ?? true,
      };
    };
    const apply = document.querySelector<HTMLButtonElement>("#decoration-apply");
    const remove = document.querySelector<HTMLButtonElement>("#decoration-delete");
    return {
      open:
        document.querySelector<HTMLDialogElement>("#decoration-inspector")?.open ??
        false,
      title:
        document.querySelector("#decoration-inspector-title")?.textContent ?? "",
      hint:
        document.querySelector("#decoration-inspector-hint")?.textContent ?? "",
      status: document.querySelector("#decoration-status")?.textContent ?? "",
      side: field("#decoration-side"),
      lane: field("#decoration-lane"),
      propKind: field("#decoration-prop-kind"),
      spacing: field("#decoration-spacing"),
      phase: field("#decoration-phase"),
      elements: field("#decoration-elements"),
      applyDisabled: apply?.disabled ?? true,
      deleteHidden: remove?.hidden ?? true,
      deleteVisible: remove ? remove.getClientRects().length > 0 : false,
    };
  });
}

async function setEditorMode(
  page: Page,
  mode: EditorMode,
  options: { imported?: boolean } = {},
): Promise<void> {
  await page.locator(`#mode-${mode}`).click();
  await expect(page.locator(`#mode-${mode}`)).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  if (mode === "points") {
    await expect(
      page.locator('[data-testid="decoration-inspector"]'),
    ).toBeHidden();
    return;
  }
  await expect(
    page.locator('[data-testid="decoration-inspector"]'),
  ).toBeVisible();
  if (options.imported) {
    await expect
      .poll(
        async () => page.locator("#decoration-status").textContent(),
        { timeout: 30_000 },
      )
      .toContain("imported");
  }
}

async function selectDecoration(
  page: Page,
  candidates: DecorationData[],
  rails: RailData[],
): Promise<DecorationData | null> {
  for (const decoration of candidates) {
    const leg = decoration.chain[0];
    if (!leg) {
      continue;
    }
    for (const ratio of [0.5, 0.35, 0.65]) {
      const fraction =
        leg.entryFraction + (leg.exitFraction - leg.entryFraction) * ratio;
      const screen = await decorationScreen(page, decoration, rails, fraction);
      if (!screen) {
        continue;
      }
      await page.mouse.click(screen[0], screen[1]);
      const inspector = await readDecorationInspector(page);
      if (inspector.hint.includes("selected")) {
        return decoration;
      }
    }
  }
  return null;
}

async function railScreenPoint(
  page: Page,
  offset: number,
  side: number,
  index: number,
): Promise<[number, number] | null> {
  const world = await page.evaluate(
    ({ offset, side, index }) => {
      const viewer = (window as any).__trackEditor;
      const route = JSON.parse(viewer.editor_route()) as Array<{
        sectionId: string;
      }>;
      const sectionId = route[0]?.sectionId;
      const centerlines = JSON.parse(
        viewer.editor_section_centerlines(),
      ) as Record<string, number[][]>;
      const samples = centerlines[sectionId];
      const count = samples.length;
      const wrapped = ((index % count) + count) % count;
      const previous = samples[(wrapped + count - 1) % count];
      const next = samples[(wrapped + 1) % count];
      const dx = next[0] - previous[0];
      const dz = next[2] - previous[2];
      const length = Math.hypot(dx, dz) || 1;
      const tangentX = dx / length;
      const tangentZ = dz / length;
      const binormalX = -tangentZ;
      const binormalZ = tangentX;
      const sample = samples[wrapped];
      return [
        sample[0] + binormalX * offset * side,
        sample[1] + 0.05,
        sample[2] + binormalZ * offset * side,
      ] as [number, number, number];
    },
    { offset, side, index },
  );
  return projectWorldPoint(page, world);
}

async function paintRail(
  page: Page,
  offset: number,
  side: number,
  fromIndex: number,
  toIndex: number,
): Promise<void> {
  const start = await railScreenPoint(page, offset, side, fromIndex);
  const end = await railScreenPoint(page, offset, side, toIndex);
  expect(start, `rail start ${fromIndex} is visible`).not.toBeNull();
  expect(end, `rail end ${toIndex} is visible`).not.toBeNull();
  await page.mouse.move(start![0], start![1]);
  await page.mouse.down();
  await page.mouse.move(end![0], end![1], { steps: 12 });
  await page.mouse.up();
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

test("making a junction adds a capped branch without splitting the host", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const pageErrors = collectPageErrors(page);

  await page.goto("/web/track-editor/?v=make-junction");
  await waitForEditor(page);
  await waitForReady(page);

  const before = await readProject(page);
  const host = before.sections!.find((section) => section.id === "section-0")!;
  const hostPoints = [...host.points];

  // Make junction is not offered on the layout start/finish point.
  const startScreen = await pointScreen(page, "point-0");
  expect(startScreen).not.toBeNull();
  await page.mouse.click(startScreen![0], startScreen![1]);
  await expect(page.locator("#point-editor")).toBeVisible();
  await expect(page.locator("#make-junction")).toBeHidden();

  // Make a junction on an eligible interior point.
  const targetId = "point-3";
  const screen = await pointScreen(page, targetId);
  expect(screen).not.toBeNull();
  await page.mouse.click(screen![0], screen![1]);
  await expect(page.locator("#make-junction")).toBeVisible();
  await page.locator("#make-junction").click();
  await waitForReady(page);

  const after = await readProject(page);
  expect(after.sections).toHaveLength(2);
  const hostAfter = after.sections!.find((section) => section.id === "section-0")!;
  expect(hostAfter.points).toEqual(hostPoints);
  const branch = after.sections!.find(
    (section) => section.id !== "section-0",
  )!;
  expect(branch.closed).toBe(false);
  expect(branch.points).toHaveLength(2);
  expect(branch.points).toContain(targetId);

  const points = await readPoints(page);
  const junction = points.find((point) => point.id === targetId)!;
  expect(junction.junction).toBe("normal");
  const looseId = branch.points.find((id) => id !== targetId)!;
  const loose = points.find((point) => point.id === looseId)!;
  expect(loose.end).toBe("cap");
  expect(loose.degree).toBe(1);
  expect(loose.sections).toEqual([branch.id]);

  // The new branch section is selected.
  const rows = await readSelectionRows(page);
  expect(rows.kind).toBe("section");
  expect(rows.id).toBe(branch.id);
  expect(rows.state).toBe("open");
  expect(rows.points).toBe("2");

  // The loose end reads as a loose end and stays draggable.
  const looseScreen = await pointScreen(page, looseId);
  expect(looseScreen).not.toBeNull();
  await page.mouse.click(looseScreen![0], looseScreen![1]);
  await expect
    .poll(async () => (await readPointEditor(page)).pointId)
    .toBe(looseId);
  const looseEditor = await readPointEditor(page);
  expect(looseEditor.rows.role).toBe("loose end");

  const beforeLoose = (await readPoints(page)).find(
    (point) => point.id === looseId,
  )!;
  const moved = await dragPoint(page, looseId);
  expect(
    Math.abs(moved.position[0] - beforeLoose.position[0]) +
      Math.abs(moved.position[1] - beforeLoose.position[1]),
  ).toBeGreaterThan(1e-3);

  // Dragging the branch does not split or reshape the host point list.
  const afterDrag = await readProject(page);
  expect(afterDrag.sections).toHaveLength(2);
  expect(
    afterDrag.sections!.find((section) => section.id === "section-0")!.points,
  ).toEqual(hostPoints);

  // Test drive export still produces a pack.
  const packLength = await page.evaluate(
    () => ((window as any).__trackEditor.editor_track_pack() as Uint8Array).length,
  );
  expect(packLength).toBeGreaterThan(0);

  // Save and reload keeps the branch.
  await page.locator("#save-project").click();
  await page.waitForFunction(
    ({ key, expectedJson }) => {
      const stored = localStorage.getItem(key);
      return (
        stored !== null && JSON.stringify(JSON.parse(stored)) === expectedJson
      );
    },
    {
      key: PROJECT_STORAGE_KEY,
      expectedJson: JSON.stringify(afterDrag),
    },
  );
  await page.reload();
  await waitForEditor(page);
  await waitForProject(page, afterDrag);
  const reloaded = await readProject(page);
  expect(
    reloaded.sections!.some((section) => section.id === branch.id),
  ).toBe(true);
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

test("decoration guide rails expose the default kerb and prop lanes", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-rails");
  await waitForEditor(page);

  const rails = await page.evaluate(
    () =>
      JSON.parse((window as any).__trackEditor.editor_guide_rails()) as Array<{
        kind: string;
        side: number;
        offset: number;
        lane: string | null;
      }>,
  );

  expect(
    rails
      .filter((rail) => rail.kind === "kerb")
      .map((rail) => [rail.side, rail.offset]),
  ).toEqual([
    [-1, 6.4],
    [1, 6.4],
  ]);
  expect(
    rails
      .filter((rail) => rail.kind === "prop" && rail.lane === "near")
      .map((rail) => [rail.side, rail.offset]),
  ).toEqual([
    [-1, 14],
    [1, 14],
  ]);
  expect(
    rails
      .filter((rail) => rail.kind === "prop" && rail.lane === "far")
      .map((rail) => [rail.side, rail.offset]),
  ).toEqual([
    [-1, 19],
    [1, 19],
  ]);
});

function orderedSpans(
  decorations: DecorationData[],
  kind: "kerb" | "prop",
  side?: number,
): DecorationData[] {
  return decorations
    .filter(
      (decoration) =>
        decoration.kind === kind &&
        (side === undefined || decoration.side === side) &&
        decoration.chain[0],
    )
    .sort(
      (a, b) =>
        a.chain[0].entryFraction - b.chain[0].entryFraction,
    );
}

function spanWithFollower(
  spans: DecorationData[],
): { current: DecorationData; next: DecorationData } | null {
  for (let index = 0; index < spans.length - 1; index += 1) {
    const current = spans[index];
    const next = spans[index + 1];
    if (next.chain[0].entryFraction >= current.chain[0].exitFraction - 1e-6) {
      return { current, next };
    }
  }
  return null;
}

test("editor modes switch and gate point editing", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=mode-switch");
  await waitForEditor(page);
  await waitForReady(page);

  const initial = await readModeState(page);
  expect(initial.points).toBe("true");
  expect(initial.kerbs).toBe("false");
  expect(initial.props).toBe("false");
  expect(initial.inspectorOpen).toBe(false);
  expect(initial.pointEditorOpen).toBe(false);

  await setEditorMode(page, "kerbs", { imported: true });
  let state = await readModeState(page);
  expect(state.points).toBe("false");
  expect(state.kerbs).toBe("true");
  expect(state.props).toBe("false");
  expect(state.inspectorOpen).toBe(true);
  expect(state.inspectorVisible).toBe(true);
  expect(state.pointEditorOpen).toBe(false);

  const before = (await readPoints(page)).length;
  const section = await clearSectionScreen(page);
  await page.mouse.click(section.screen[0], section.screen[1]);
  await page.waitForTimeout(150);
  expect((await readPoints(page)).length).toBe(before);
  expect((await readModeState(page)).pointEditorOpen).toBe(false);

  await setEditorMode(page, "props", { imported: false });
  state = await readModeState(page);
  expect(state.kerbs).toBe("false");
  expect(state.props).toBe("true");
  expect(state.inspectorOpen).toBe(true);
  expect((await readDecorationInspector(page)).title).toBe("Prop");

  await setEditorMode(page, "points");
  state = await readModeState(page);
  expect(state.points).toBe("true");
  expect(state.inspectorOpen).toBe(false);
  const point = await pointScreen(page, "point-3");
  expect(point).not.toBeNull();
  await page.mouse.click(point![0], point![1]);
  expect((await readPointEditor(page)).open).toBe(true);
});

test("kerb mode imports the baked defaults on first entry", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-import");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });

  await expect
    .poll(async () => (await readDecorations(page)).length, {
      timeout: 30_000,
    })
    .toBe(28);
  const decorations = await readDecorations(page);
  expect(
    decorations.some(
      (decoration) =>
        decoration.kind === "prop" &&
        decoration.lane === "far" &&
        decoration.propKind === "trees",
    ),
  ).toBe(true);

  const inspector = await readDecorationInspector(page);
  expect(inspector.title).toBe("Kerb");
  expect(inspector.hint).toContain("Drag a kerb rail");
  expect(inspector.applyDisabled).toBe(true);
  expect(inspector.deleteHidden).toBe(true);
});

test("selecting a default kerb shows the selected inspector", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-select");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });

  const decorations = await readDecorations(page);
  const rails = await readGuideRails(page);
  const kerbs = orderedSpans(decorations, "kerb");
  const selected = await selectDecoration(page, kerbs, rails);
  expect(selected, "a default kerb is selectable").not.toBeNull();

  const inspector = await readDecorationInspector(page);
  expect(inspector.title).toBe("Kerb");
  expect(inspector.hint).toContain("selected");
  expect(inspector.applyDisabled).toBe(false);
  expect(inspector.deleteHidden).toBe(false);
  expect(inspector.deleteVisible).toBe(true);
  expect((await readProject(page)).decorations?.find(
    (decoration) => decoration.id === selected!.id,
  )?.kind).toBe("kerb");
});

test("resizing a kerb handle updates the chain fraction", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-resize");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });

  const decorations = await readDecorations(page);
  const rails = await readGuideRails(page);
  const pair = spanWithFollower(orderedSpans(decorations, "kerb"));
  expect(pair, "a kerb span with a following span exists").not.toBeNull();
  const current = pair!.current;
  const next = pair!.next;
  const selected = await selectDecoration(page, [current], rails);
  expect(selected, "the kerb is selectable").not.toBeNull();

  const spec = railSpecForDecoration(current, rails);
  expect(spec).not.toBeNull();
  const original = current.chain[0];
  const exitScreen = await railScreenAt(page, spec!, original.exitFraction);
  const targetFraction =
    original.exitFraction +
    (next.chain[0].entryFraction - original.exitFraction) * 0.5;
  const targetScreen = await railScreenAt(page, spec!, targetFraction);
  expect(exitScreen, "exit handle is visible").not.toBeNull();
  expect(targetScreen, "resize target is visible").not.toBeNull();

  await page.mouse.move(exitScreen![0], exitScreen![1]);
  await page.mouse.down();
  await page.mouse.move(targetScreen![0], targetScreen![1], { steps: 12 });
  await page.mouse.up();

  await expect(page.locator("#decoration-status")).toContainText("resized", {
    timeout: 30_000,
  });
  await expect(page.locator("#status")).toHaveText("ready");
  const after = (await readDecorations(page)).find(
    (decoration) => decoration.id === current.id,
  );
  expect(after?.chain[0].exitFraction).toBeGreaterThan(
    original.exitFraction + 1e-3,
  );
  expect(after?.chain[0].entryFraction).toBeCloseTo(
    original.entryFraction,
    5,
  );
});

test("editing kerb properties applies the change", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-edit");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });

  const decorations = await readDecorations(page);
  const rails = await readGuideRails(page);
  const selected = await selectDecoration(
    page,
    orderedSpans(decorations, "kerb"),
    rails,
  );
  expect(selected).not.toBeNull();

  await page.locator("#decoration-phase").fill("2.5");
  await page.locator("#decoration-elements").fill("1.2, 2.4");
  await page.locator("#decoration-apply").click();

  await expect(page.locator("#decoration-status")).toContainText("updated", {
    timeout: 30_000,
  });
  await expect(page.locator("#status")).toHaveText("ready");
  const updated = (await readDecorations(page)).find(
    (decoration) => decoration.id === selected!.id,
  );
  expect(updated?.phase).toBe(2.5);
  expect(updated?.elements).toEqual([1.2, 2.4]);
});

test("deleting a selected kerb removes it", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-delete");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });

  const decorations = await readDecorations(page);
  const rails = await readGuideRails(page);
  const selected = await selectDecoration(
    page,
    orderedSpans(decorations, "kerb"),
    rails,
  );
  expect(selected).not.toBeNull();

  await page.locator("#decoration-delete").click();
  await expect(page.locator("#decoration-status")).toContainText("deleted", {
    timeout: 30_000,
  });
  await expect(page.locator("#status")).toHaveText("ready");
  const after = await readDecorations(page);
  expect(after.length).toBe(decorations.length - 1);
  expect(
    after.some((decoration) => decoration.id === selected!.id),
  ).toBe(false);
});

test("prop mode selects a tree and paints a replacement prop", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-props");
  await waitForEditor(page);
  await setEditorMode(page, "props", { imported: true });

  const decorations = await readDecorations(page);
  const rails = await readGuideRails(page);
  const tree = decorations.find(
    (decoration) =>
      decoration.kind === "prop" &&
      decoration.lane === "far" &&
      decoration.propKind === "trees",
  );
  expect(tree, "a far tree prop exists").toBeDefined();
  const selected = await selectDecoration(page, [tree!], rails);
  expect(selected, "the tree prop is selectable").not.toBeNull();

  const inspector = await readDecorationInspector(page);
  expect(inspector.title).toBe("Prop");
  expect(inspector.lane.value).toBe("far");
  expect(inspector.propKind.value).toBe("trees");

  const spec = railSpecForDecoration(tree!, rails);
  expect(spec).not.toBeNull();
  const leg = tree!.chain[0];
  const length = leg.exitFraction - leg.entryFraction;
  const start = await railScreenAt(
    page,
    spec!,
    leg.entryFraction + length * 0.1,
  );
  const end = await railScreenAt(
    page,
    spec!,
    leg.exitFraction - length * 0.1,
  );
  expect(start).not.toBeNull();
  expect(end).not.toBeNull();

  const before = decorations.length;
  await page.locator("#decoration-delete").click();
  // Reading editor_project() while set_editor_project() is in flight triggers
  // a wasm reentrancy panic, so wait for the rebuild to settle first.
  await waitForReady(page);
  const afterDelete = await readDecorations(page);
  expect(afterDelete.length).toBe(before - 1);

  await page.mouse.move(start![0], start![1]);
  await page.mouse.down();
  await page.mouse.move(end![0], end![1], { steps: 12 });
  await page.mouse.up();

  await expect(page.locator("#decoration-status")).toContainText("painted", {
    timeout: 30_000,
  });
  const after = await readDecorations(page);
  expect(after.length).toBe(before);
  const painted = after.find((decoration) =>
    decoration.id.startsWith("paint-prop"),
  );
  expect(painted).toBeDefined();
  expect(painted!.kind).toBe("prop");
  expect(painted!.side).toBe(tree!.side);
  expect(painted!.lane).toBe("far");
  expect(painted!.propKind).toBe("trees");
  expect(painted!.chain).toHaveLength(1);
  expect(painted!.chain[0].entryFraction).toBeLessThan(
    painted!.chain[0].exitFraction,
  );
});

test("resizing a kerb into another span is rejected", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-overlap");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });

  const decorations = await readDecorations(page);
  const rails = await readGuideRails(page);
  const pair = spanWithFollower(orderedSpans(decorations, "kerb"));
  expect(pair).not.toBeNull();
  const current = pair!.current;
  const next = pair!.next;
  const selected = await selectDecoration(page, [current], rails);
  expect(selected).not.toBeNull();

  const spec = railSpecForDecoration(current, rails);
  expect(spec).not.toBeNull();
  const original = current.chain[0];
  const exitScreen = await railScreenAt(page, spec!, original.exitFraction);
  const overlapFraction =
    next.chain[0].entryFraction +
    (next.chain[0].exitFraction - next.chain[0].entryFraction) * 0.4;
  const overlapScreen = await railScreenAt(page, spec!, overlapFraction);
  expect(exitScreen).not.toBeNull();
  expect(overlapScreen).not.toBeNull();

  await page.mouse.move(exitScreen![0], exitScreen![1]);
  await page.mouse.down();
  await page.mouse.move(overlapScreen![0], overlapScreen![1], { steps: 12 });
  await page.mouse.up();

  await expect
    .poll(
      async () => page.locator("#decoration-status").textContent(),
      { timeout: 30_000 },
    )
    .toContain("overlap");
  const after = (await readDecorations(page)).find(
    (decoration) => decoration.id === current.id,
  );
  expect(after?.chain[0].exitFraction).toBeCloseTo(
    original.exitFraction,
    5,
  );
  expect((await readDecorations(page)).length).toBe(decorations.length);
});

test("a painted decoration survives save, reload, and load", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await page.goto("/web/track-editor/?v=decoration-persist");
  await waitForEditor(page);
  await setEditorMode(page, "kerbs", { imported: true });
  const before = await readDecorations(page);

  const spec: RailSpec = { kind: "kerb", side: -1, offset: 6.4 };
  const start = await railScreenAt(page, spec, 25 / 119);
  const end = await railScreenAt(page, spec, 35 / 119);
  expect(start).not.toBeNull();
  expect(end).not.toBeNull();
  await page.mouse.move(start![0], start![1]);
  await page.mouse.down();
  await page.mouse.move(end![0], end![1], { steps: 12 });
  await page.mouse.up();
  await expect(page.locator("#decoration-status")).toContainText("painted", {
    timeout: 30_000,
  });
  expect((await readDecorations(page)).length).toBe(before.length + 1);
  const painted = (await readDecorations(page)).find((decoration) =>
    decoration.id.startsWith("paint-kerb"),
  );
  expect(painted).toBeDefined();

  await page.locator("#save-project").click();
  await expect
    .poll(async () =>
      page.evaluate(
        (key) => localStorage.getItem(key) !== null,
        PROJECT_STORAGE_KEY,
      ),
    )
    .toBe(true);

  await page.reload();
  await waitForEditor(page);
  await page.locator("#load-project").click();
  await waitForReady(page);
  await expect
    .poll(async () =>
      (await readDecorations(page)).some(
        (decoration) => decoration.id === painted!.id,
      ),
    )
    .toBe(true);
});
