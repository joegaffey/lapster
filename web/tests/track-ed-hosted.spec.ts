import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

const hostedIndex = fileURLToPath(
  new URL("../track-ed/index.html", import.meta.url),
);

const AUTOSAVE_KEY = "track-ed.autosave.v1";

interface HostedProjectSnapshot {
  schema: string;
  name: string;
  centerlines: Array<{
    id: string;
    closed: boolean;
    points: Array<{ id: string; position: [number, number] }>;
  }>;
  junctions: Array<{ id: string }>;
  errors: string[];
}

async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForFunction(
    () => Boolean((window as any).__trackEd),
    undefined,
    { timeout: 30_000 },
  );
  await page.evaluate(
    () => (window as any).__trackEd.whenReady() as Promise<void>,
  );
}

async function readProject(page: Page): Promise<HostedProjectSnapshot> {
  const project = await page.evaluate(
    () => (window as any).__trackEd.project() as HostedProjectSnapshot | null,
  );
  expect(project).not.toBeNull();
  return project!;
}

async function clearAutosaveBeforeFirstNavigation(page: Page): Promise<void> {
  await page.addInitScript(() => {
    try {
      if (sessionStorage.getItem("track-ed-e2e-autosave-cleared") !== "1") {
        localStorage.clear();
        sessionStorage.setItem("track-ed-e2e-autosave-cleared", "1");
      }
    } catch {
      // Storage may be unavailable in restricted contexts.
    }
  });
}

async function waitForStoredAutosave(
  page: Page,
  expected?: { index: number; position: [number, number] },
): Promise<void> {
  await expect
    .poll(async () => {
      const raw = await page.evaluate(
        (key) => localStorage.getItem(key),
        AUTOSAVE_KEY,
      );
      if (raw === null) {
        return false;
      }
      if (!expected) {
        return true;
      }
      const envelope = JSON.parse(raw) as {
        project?: {
          sections?: Array<{
            points?: Array<{ position?: [number, number] }>;
          }>;
          centerlines?: Array<{
            points?: Array<{ position?: [number, number] }>;
          }>;
        };
      };
      const section =
        envelope.project?.sections?.[0] ?? envelope.project?.centerlines?.[0];
      return (
        JSON.stringify(section?.points?.[expected.index]?.position) ===
        JSON.stringify(expected.position)
      );
    })
    .toBe(true);
}

test("hosted track-ed boots, edits, creates a junction, exports and test drives", async ({
  page,
}) => {
  test.skip(
    !existsSync(hostedIndex),
    "hosted track-ed is not built; run `bash web/scripts/build-track-ed.sh`",
  );
  test.setTimeout(180_000);

  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await clearAutosaveBeforeFirstNavigation(page);

  // Force the unversioned track-pack fallback built by `build:viewer`.
  await page.route(/\/version\.json(?:\?|$)/, (route) =>
    route.fulfill({ status: 404, body: "not found" }),
  );

  // The hook is opt-in: a normal hosted load must not expose it.
  await page.goto("/web/track-ed/");
  await page.waitForFunction(() =>
    document.querySelector("#project-meta")?.textContent?.includes(
      "Loop Spline Point Terrain",
    ),
  );
  expect(await page.evaluate(() => (window as any).__trackEd)).toBeUndefined();

  await page.goto("/web/track-ed/?e2e=1");
  await waitForAppReady(page);
  expect(pageErrors).toEqual([]);

  // Hidden legacy authoring modes must be gone.
  await expect(page.locator("#mode-kerbs")).toHaveCount(0);
  await expect(page.locator("#mode-walls")).toHaveCount(0);
  await expect(page.locator("#mode-props")).toHaveCount(0);

  // Roadside authoring smoke.
  await page.locator("#mode-roadside").click();
  const kerbButton = page.locator("#roadside-kind-kerb");
  const treeButton = page.locator("#roadside-kind-tree");
  await expect(treeButton).toHaveClass(/active/);
  await kerbButton.click();
  await expect(kerbButton).toHaveClass(/active/);
  await expect(treeButton).not.toHaveClass(/active/);
  await treeButton.click();
  await expect(treeButton).toHaveClass(/active/);
  await expect(page.locator("#roadside-status")).toBeVisible();

  // Edit a control point through the explicit same-path e2e edit.
  await page.locator("#mode-points").click();
  const pointIndex = 1;
  const before = await readProject(page);
  const pointBefore = before.centerlines[0]!.points[pointIndex]!.position;
  const selected = await page.evaluate(
    (index) => (window as any).__trackEd.selectPoint(index),
    pointIndex,
  );
  expect(selected).toEqual({ centerlineIndex: 0, pointIndex });
  await expect(page.locator("#add-junction-button")).toBeEnabled();

  const moved = await page.evaluate(
    ([index, position]) =>
      (window as any).__trackEd.movePoint(index, position) as boolean,
    [pointIndex, [pointBefore[0] + 6, pointBefore[1] + 4]] as const,
  );
  expect(moved).toBe(true);
  const afterEdit = await readProject(page);
  const pointAfter = afterEdit.centerlines[0]!.points[pointIndex]!.position;
  expect(pointAfter).not.toEqual(pointBefore);
  expect(
    Math.hypot(pointAfter[0] - pointBefore[0], pointAfter[1] - pointBefore[1]),
  ).toBeGreaterThan(1);
  expect(afterEdit.errors).toEqual([]);

  // The debounced autosave lands, then a reload restores the edited project.
  await waitForStoredAutosave(page, {
    index: pointIndex,
    position: pointAfter,
  });
  await page.reload();
  await waitForAppReady(page);
  const restored = await readProject(page);
  expect(restored.centerlines[0]!.points[pointIndex]!.position).toEqual(
    pointAfter,
  );
  expect(restored.errors).toEqual([]);
  await expect(page.locator("#edit-status")).toHaveText(
    "Restored autosaved project.",
  );

  // Re-select after restore so the junction action has a target.
  const reselected = await page.evaluate(
    (index) => (window as any).__trackEd.selectPoint(index),
    pointIndex,
  );
  expect(reselected).toEqual({ centerlineIndex: 0, pointIndex });
  await expect(page.locator("#add-junction-button")).toBeEnabled();

  // Create a junction and confirm the project gained one.
  const junctionCountBefore = restored.junctions.length;
  await page.locator("#add-junction-button").click();
  await expect
    .poll(async () => (await readProject(page)).junctions.length)
    .toBe(junctionCountBefore + 1);
  const afterJunction = await readProject(page);
  expect(afterJunction.errors).toEqual([]);
  expect(pageErrors).toEqual([]);

  // Export produces the two-file Lapster bundle.
  await expect(page.locator("#export-button")).toBeEnabled();
  const downloadNames: string[] = [];
  page.on("download", (download) =>
    downloadNames.push(download.suggestedFilename()),
  );
  await page.locator("#export-button").click();
  await expect.poll(() => downloadNames.length).toBe(2);
  expect(downloadNames.sort()).toEqual(["track.glb", "track.json"]);

  // Test drive compiles the pack and boots Drive with the track-ed asset.
  await expect(page.locator("#test-drive-button")).toBeEnabled();
  await page.locator("#test-drive-button").click();
  await page.waitForURL(/\/web\/drive\/\?[^#]*track=track-ed/, {
    timeout: 120_000,
  });
  await page.waitForFunction(
    () =>
      (window as any).__driveViewerReady === true ||
      Boolean((window as any).__driveViewerError),
    undefined,
    { timeout: 120_000 },
  );
  const drive = await page.evaluate(() => {
    const viewer = (window as any).__driveViewer;
    return {
      error: (window as any).__driveViewerError as string | undefined,
      debug: JSON.parse(viewer.debug_info()),
      telemetry: JSON.parse(viewer.telemetry()),
      race: JSON.parse(viewer.race()),
    };
  });
  expect(drive.error).toBeUndefined();
  expect(drive.debug.asset.id).toBe("track-ed");
  expect(drive.telemetry.track).toBe("track-ed");
  expect(drive.telemetry.raceAvailable).toBe(true);
  expect(drive.race.available).toBe(true);
  // The MVP layout_id is the spawn section id; the hosted default is "section-1".
  expect(drive.race.layout.layout_id).toBe(restored.centerlines[0]!.id);
  expect(drive.race.grid.length).toBe(8);
  expect(drive.race.sectors.length).toBe(4);
  expect(pageErrors).toEqual([]);
});

test("new loop clears autosave and reloads the default project", async ({
  page,
}) => {
  test.skip(
    !existsSync(hostedIndex),
    "hosted track-ed is not built; run `bash web/scripts/build-track-ed.sh`",
  );
  test.setTimeout(180_000);

  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await clearAutosaveBeforeFirstNavigation(page);

  await page.goto("/web/track-ed/?e2e=1");
  await waitForAppReady(page);

  const pointIndex = 1;
  const defaultProject = await readProject(page);
  const defaultPoint = defaultProject.centerlines[0]!.points[pointIndex]!.position;

  const moved = await page.evaluate(
    ([index, position]) =>
      (window as any).__trackEd.movePoint(index, position) as boolean,
    [pointIndex, [defaultPoint[0] + 9, defaultPoint[1] + 5]] as const,
  );
  expect(moved).toBe(true);
  await waitForStoredAutosave(page);

  page.once("dialog", (dialog) => void dialog.accept());
  await page.locator("#new-button").click();
  await expect
    .poll(() =>
      page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY),
    )
    .toBeNull();

  const reset = await readProject(page);
  expect(reset.name).toBe(defaultProject.name);
  expect(reset.centerlines[0]!.points[pointIndex]!.position).toEqual(
    defaultPoint,
  );

  // Reloading must not re-save the default project.
  await page.reload();
  await waitForAppReady(page);
  const reloaded = await readProject(page);
  expect(reloaded.name).toBe(defaultProject.name);
  expect(reloaded.centerlines[0]!.points[pointIndex]!.position).toEqual(
    defaultPoint,
  );
  expect(
    await page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY),
  ).toBeNull();
  expect(pageErrors).toEqual([]);
});

test("invalid autosave data is ignored with a warning and the default loads", async ({
  page,
}) => {
  test.skip(
    !existsSync(hostedIndex),
    "hosted track-ed is not built; run `bash web/scripts/build-track-ed.sh`",
  );
  test.setTimeout(120_000);

  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.addInitScript(() => {
    localStorage.setItem("track-ed.autosave.v1", "{ this is not valid json");
  });

  await page.goto("/web/track-ed/?e2e=1");
  await waitForAppReady(page);

  await expect(page.locator("#edit-status")).toHaveText(
    "Ignored invalid autosaved project; loaded the default.",
  );
  const project = await readProject(page);
  expect(project.name).toBe("Loop Spline Point Terrain");
  expect(project.errors).toEqual([]);
  expect(
    await page.evaluate((key) => localStorage.getItem(key), AUTOSAVE_KEY),
  ).toBeNull();
  expect(pageErrors).toEqual([]);
});
