import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";

const hostedIndex = fileURLToPath(
  new URL("../track-ed/index.html", import.meta.url),
);

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

  // Create a junction and confirm the project gained one.
  const junctionCountBefore = afterEdit.junctions.length;
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
    };
  });
  expect(drive.error).toBeUndefined();
  expect(drive.debug.asset.id).toBe("track-ed");
  expect(drive.telemetry.track).toBe("track-ed");
  expect(pageErrors).toEqual([]);
});
