import { expect, test } from "@playwright/test";

test("multiplayer lobby displays and carries the selected category", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto(
    "/web/multiplayer/?category=gp67&car=mazda-rx7&track=skidpad05",
  );

  await expect(page.locator("#category-name")).toHaveText("GP 67");
  await expect(page.locator("#car-name")).toHaveText("Cavallo 123");
  await expect(page.locator("#track-name")).toHaveText("Skidpad 0.5");

  const url = new URL(page.url());
  expect(url.searchParams.get("category")).toBe("gp67");
  expect(url.searchParams.get("car")).toBe("67gp-cavallo-123");
  expect(url.searchParams.get("track")).toBe("skidpad05");
  expect(
    await page.evaluate(() => localStorage.getItem("lapster.category.v1")),
  ).toBe("gp67");
  expect(pageErrors).toEqual([]);
});

test("multiplayer lobby defaults to the road category", async ({ page }) => {
  await page.goto("/web/multiplayer/?car=67gp-cavallo-123");

  await expect(page.locator("#category-name")).toHaveText("Road");
  await expect(page.locator("#car-name")).toHaveText("Mazda RX-7");
  expect(
    await page.evaluate(() => localStorage.getItem("lapster.category.v1")),
  ).toBe("road");
});
