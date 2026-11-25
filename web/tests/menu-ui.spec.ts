import { expect, test, type Page } from "@playwright/test";
import { clickMenuControl, closeMenu, openMenu } from "./menu-helpers";

async function waitForDrive(page: Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function waitForGarage(page: Page) {
  await page.waitForFunction(
    () => (window as any).__garageViewerReady === true || (window as any).__garageViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function waitForTrack(page: Page) {
  await page.waitForFunction(
    () => (window as any).__trackViewerReady === true || (window as any).__trackViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

test.describe("drive app menu", () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test("collapses the toolbar and runs menu actions", async ({ page }) => {
    await page.goto(
      "/web/drive/?track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=menu-desktop",
    );
    await waitForDrive(page);

    const visibleChildren = await page.evaluate(() =>
      Array.from(document.querySelectorAll("#viewer-controls > *"))
        .filter((element) => (element as HTMLElement).getClientRects().length > 0)
        .map((element) => (element as HTMLElement).id),
    );
    expect(visibleChildren).toEqual(["menu-open"]);

    await openMenu(page);
    await expect(page.locator("#app-menu")).toBeVisible();
    for (const selector of [
      "#assists-open",
      "#setup-open",
      "#hud-toggle",
      "#camera-mode",
      "#body-toggle",
      "#audio-toggle",
      "#reset",
    ]) {
      await expect(page.locator(selector)).toBeVisible();
    }

    await clickMenuControl(page, "#assists-open");
    await expect(page.locator("#app-menu")).toBeHidden();
    await expect(page.locator("#assists-dialog")).toHaveAttribute("open", "");
    await page.locator("#assists-close").click();
    await expect(page.locator("#assists-dialog")).toBeHidden();

    await clickMenuControl(page, "#setup-open");
    await expect(page.locator("#app-menu")).toBeHidden();
    await expect(page.locator("#car-setup-dialog")).toHaveAttribute("open", "");
    await page.locator("#car-setup-close").click();
    await expect(page.locator("#car-setup-dialog")).toBeHidden();

    await clickMenuControl(page, "#hud-toggle");
    await expect(page.locator("#app-menu")).toBeHidden();
    await expect(page.locator("#hud")).not.toHaveClass(/hud-collapsed/);
    await expect(page.locator("#speed")).toBeVisible();

    await openMenu(page);
    await expect(page.locator(".hud-exit")).toBeVisible();
    await expect(page.locator(".hud-exit")).toHaveAttribute("href", "../");
    await closeMenu(page);
  });
});

test.describe("viewer app menus", () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test("garage exposes reset camera and exit through the menu", async ({ page }) => {
    await page.goto("/web/garage/?track=../../fixtures/tracks/minimal/pack.sima&v=cp6");
    await waitForGarage(page);

    await openMenu(page);
    await expect(page.locator("#app-menu")).toBeVisible();
    await expect(page.locator("#reset-camera")).toBeVisible();
    await expect(page.locator(".hud-exit")).toBeVisible();
    await expect(page.locator(".hud-exit")).toHaveAttribute("href", "../");
    await closeMenu(page);

    await clickMenuControl(page, "#reset-camera");
    await expect(page.locator("#app-menu")).toBeHidden();
  });

  test("track viewer exposes reset camera and exit through the menu", async ({ page }) => {
    await page.goto("/web/track-viewer/?track=../../fixtures/tracks/minimal/pack.sima&v=cp6");
    await waitForTrack(page);

    await openMenu(page);
    await expect(page.locator("#app-menu")).toBeVisible();
    await expect(page.locator("#reset-camera")).toBeVisible();
    await expect(page.locator(".hud-exit")).toBeVisible();
    await expect(page.locator(".hud-exit")).toHaveAttribute("href", "../");
    await closeMenu(page);

    await clickMenuControl(page, "#reset-camera");
    await expect(page.locator("#app-menu")).toBeHidden();
  });
});

test.describe("race panel desktop", () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test("sits near the top of the drive view", async ({ page }) => {
    await page.goto("/web/drive/?track=race-loop&ai=3&v=menu-race-desktop");
    await waitForDrive(page);
    await expect(page.locator("#race-panel")).toBeVisible();

    const panel = await page.locator("#race-panel").boundingBox();
    expect(panel).not.toBeNull();
    if (!panel) {
      return;
    }
    expect(panel.y).toBeGreaterThanOrEqual(0);
    expect(panel.y).toBeLessThanOrEqual(24);
  });
});

test.describe("race panel mobile portrait", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

  test("wraps into two rows and clears the hamburger", async ({ page }) => {
    await page.goto("/web/drive/?track=race-loop&ai=3&v=menu-race-mobile");
    await waitForDrive(page);
    await expect(page.locator("#race-panel")).toBeVisible();
    await expect(page.getByTestId("race-ai")).toBeVisible();

    const layout = await page.evaluate(() => {
      const panel = document.querySelector<HTMLElement>("#race-panel");
      const menu = document.querySelector<HTMLElement>("#menu-open");
      if (!panel || !menu) {
        return null;
      }
      const style = getComputedStyle(panel);
      const stats = Array.from(panel.querySelectorAll<HTMLElement>(".race-stat"))
        .filter((element) => !element.hidden && getComputedStyle(element).display !== "none")
        .map((element) => {
          const rect = element.getBoundingClientRect();
          return {
            label: element.querySelector(".race-label")?.textContent?.trim() ?? "",
            top: Math.round(rect.top),
            left: rect.left,
            right: rect.right,
          };
        });
      const panelRect = panel.getBoundingClientRect();
      const menuRect = menu.getBoundingClientRect();
      return {
        display: style.display,
        columns: style.gridTemplateColumns.split(" ").filter(Boolean).length,
        stats,
        panel: {
          top: panelRect.top,
          left: panelRect.left,
          right: panelRect.right,
          bottom: panelRect.bottom,
        },
        menu: {
          top: menuRect.top,
          left: menuRect.left,
          right: menuRect.right,
          bottom: menuRect.bottom,
        },
      };
    });

    expect(layout).not.toBeNull();
    if (!layout) {
      return;
    }

    expect(layout.display).toBe("grid");
    expect(layout.columns).toBe(4);
    expect(layout.stats.map((stat) => stat.label)).toEqual([
      "Position",
      "Lap",
      "Sector",
      "Current",
      "Last",
      "Best",
      "AI",
    ]);

    const rowTops = Array.from(new Set(layout.stats.map((stat) => stat.top)));
    expect(rowTops).toHaveLength(2);
    const firstRow = layout.stats.filter((stat) => stat.top === rowTops[0]);
    const secondRow = layout.stats.filter((stat) => stat.top === rowTops[1]);
    expect(firstRow).toHaveLength(4);
    expect(secondRow).toHaveLength(layout.stats.length - 4);
    expect(rowTops[0]).toBeLessThan(rowTops[1]);

    expect(layout.panel.top).toBeGreaterThanOrEqual(0);
    expect(layout.panel.top).toBeLessThanOrEqual(24);
    expect(layout.panel.right).toBeLessThanOrEqual(layout.menu.left + 1);
  });
});
