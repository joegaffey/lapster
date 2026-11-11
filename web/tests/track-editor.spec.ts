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

async function openHud(page: Page) {
  if (
    await page
      .locator("#hud")
      .evaluate((hud) => hud.classList.contains("hud-collapsed"))
  ) {
    await page.locator("#hud-toggle").click();
  }
}

test("segment kerbs and props are shared by every route", async ({ page }) => {
  test.setTimeout(90_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=shared-decoration");
  await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });
  await openHud(page);

  const before = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    return {
      project: JSON.parse(viewer.editor_project()) as {
        layouts: Array<{
          id: string;
          route: string[];
          startFinish?: { edge: string };
        }>;
        settings: {
          splines: Array<{
            id: string;
            role?: string;
            base?: string | null;
            offset?: number;
          }>;
        };
      },
      active: viewer.editor_active_layout() as string,
    };
  });
  const source = before.project.layouts.find(
    (layout) => layout.id === before.active,
  )!;

  await page.locator("#duplicate-layout").click();
  await waitForEditorRebuild(page);

  const shared = await page.evaluate(
    (sourceId) => {
      const viewer = (window as any).__trackEditor;
      const project = JSON.parse(viewer.editor_project()) as {
        layouts: Array<{
          id: string;
          route: string[];
          startFinish?: { edge: string };
        }>;
        settings: {
          splines: Array<{
            id: string;
            role?: string;
            base?: string | null;
            offset?: number;
          }>;
        };
        edges: Array<{
          id: string;
          kerbs: Array<{ start: number; end: number; side: number }>;
          propSpans: Array<{ spline: string; kind: string }>;
        }>;
      };
      const source = project.layouts.find((layout) => layout.id === sourceId)!;
      const duplicate = project.layouts.find(
        (layout) => layout.id !== sourceId,
      )!;
      const segmentId = source.route.find(
        (id) => id !== source.startFinish?.edge,
      )!;
      const splines = project.settings.splines;
      const spline = splines.find((candidate) => {
        if (candidate.role !== "prop") {
          return false;
        }
        const base = candidate.base
          ? splines.find((item) => item.id === candidate.base)
          : undefined;
        return (base?.offset ?? 0) + (candidate.offset ?? 0) > 0;
      })!;
      return {
        segmentId,
        sourceId,
        duplicateId: duplicate.id,
        spline: spline.id,
      };
    },
    source.id,
  );

  await page.evaluate(
    async ({ segmentId, spline }) => {
      const viewer = (window as any).__trackEditor;
      const render = viewer.render;
      viewer.render = () => {};
      try {
        await viewer.set_editor_segment_kerbs(
          segmentId,
          JSON.stringify([{ start: 0.2, end: 0.4, side: 1 }]),
        );
        await viewer.set_editor_segment_props(
          segmentId,
          JSON.stringify([
            { spline, start: 0.1, end: 0.3, kind: "blocks" },
          ]),
        );
      } finally {
        viewer.render = render;
      }
    },
    shared,
  );

  const read = () =>
    page.evaluate((segmentId) => {
      const viewer = (window as any).__trackEditor;
      const project = JSON.parse(viewer.editor_project()) as {
        edges: Array<{
          id: string;
          kerbs: Array<{ start: number; end: number; side: number }>;
          propSpans: Array<{ spline: string; kind: string }>;
        }>;
        layouts: Array<{ id: string; route: string[] }>;
      };
      return {
        edge: project.edges.find((edge) => edge.id === segmentId),
        layouts: project.layouts,
        kerbSpans: JSON.parse(viewer.editor_kerb_spans()) as unknown[],
        propLines: JSON.parse(viewer.editor_prop_lines()) as Array<{
          kind: string;
        }>,
        active: viewer.editor_active_layout() as string,
      };
    }, shared.segmentId);

  const sourceState = await read();
  expect(sourceState.edge?.kerbs).toEqual([
    { start: 0.2, end: 0.4, side: 1 },
  ]);
  expect(sourceState.edge?.propSpans[0]?.spline).toBe(shared.spline);
  expect(sourceState.kerbSpans.length).toBeGreaterThan(0);
  expect(sourceState.propLines[0]?.kind).toBe("blocks");
  expect(sourceState.layouts.map((layout) => layout.route)).toEqual([
    expect.arrayContaining([shared.segmentId]),
    expect.arrayContaining([shared.segmentId]),
  ]);

  await page.locator("#layout-select").selectOption(shared.duplicateId);
  await waitForEditorRebuild(page);

  const duplicateState = await read();
  expect(duplicateState.active).toBe(shared.duplicateId);
  expect(duplicateState.edge).toEqual(sourceState.edge);
  expect(duplicateState.kerbSpans.length).toBeGreaterThan(0);
  expect(duplicateState.propLines[0]?.kind).toBe("blocks");

  expect(pageErrors).toEqual([]);
});

test("pit path UI adds, appends, removes, and deletes a path", async ({ page }) => {
  test.setTimeout(90_000);
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto("/web/track-editor/?v=pit-path");
  await expect(page.locator("#status")).toHaveText("ready", { timeout: 20_000 });
  await openHud(page);

  const initialState = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const project = JSON.parse(viewer.editor_project()) as {
      layouts: Array<{ id: string; route: string[] }>;
      paths: unknown[];
    };
    return {
      layouts: project.layouts.map((layout) => layout.id),
      paths: project.paths.length,
      route: project.layouts[0].route,
    };
  });
  expect(initialState.paths).toBe(0);

  await page.locator("#add-path").click();
  await waitForEditorRebuild(page);
  const added = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const project = JSON.parse(viewer.editor_project()) as {
      layouts: Array<{ id: string }>;
      paths: Array<{ id: string; kind: string; route: string[] }>;
    };
    return { project };
  });
  expect(added.project.paths).toHaveLength(1);
  expect(added.project.paths[0]?.kind).toBe("pit_lane");
  expect(added.project.layouts.map((layout) => layout.id)).toEqual(
    initialState.layouts,
  );

  for (const segmentId of initialState.route.slice(0, 2)) {
    await page.evaluate(
      (id) => (window as any).__editorSelectSegment(id),
      segmentId,
    );
    await expect(page.locator("#append-path-segment")).toBeEnabled();
    await page.locator("#append-path-segment").click();
    await waitForEditorRebuild(page);
  }

  const appended = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const project = JSON.parse(viewer.editor_project()) as {
      paths: Array<{ route: string[] }>;
    };
    return project.paths[0]?.route ?? [];
  });
  expect(appended).toEqual(initialState.route.slice(0, 2));

  await expect(page.locator("#remove-path-segment")).toBeEnabled();
  await page.locator("#remove-path-segment").click();
  await waitForEditorRebuild(page);
  const removed = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const project = JSON.parse(viewer.editor_project()) as {
      paths: Array<{ route: string[] }>;
    };
    return project.paths[0]?.route ?? [];
  });
  expect(removed).toEqual(initialState.route.slice(0, 1));

  await page.locator("#delete-path").click();
  await waitForEditorRebuild(page);
  const deleted = await page.evaluate(() => {
    const viewer = (window as any).__trackEditor;
    const project = JSON.parse(viewer.editor_project()) as {
      paths: unknown[];
    };
    return project.paths.length;
  });
  expect(deleted).toBe(0);
  expect(pageErrors).toEqual([]);
});

test("legacy projects migrate to schema 4 routes", async ({ page }) => {
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
    return JSON.parse(viewer.editor_project()) as {
      schema: string;
      layouts: Array<{ route: string[] }>;
      edges: unknown[];
      paths: unknown[];
    };
  }, legacyProject);

  expect(project.schema).toBe("lapster/track-project@4");
  expect(project.layouts).toHaveLength(1);
  expect(project.layouts[0]?.route.length).toBeGreaterThanOrEqual(4);
  expect(project.edges.length).toBeGreaterThanOrEqual(4);
  expect(project.paths).toEqual([]);
  expect(pageErrors).toEqual([]);
});

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
