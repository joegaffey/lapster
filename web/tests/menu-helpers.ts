import { expect, type Page } from "@playwright/test";

/** Open the collapsed app menu and wait for the dialog to be shown. */
export async function openMenu(page: Page): Promise<void> {
  const menu = page.locator("#app-menu");
  const isOpen = await menu.isVisible().catch(() => false);
  if (!isOpen) {
    await page.locator("#menu-open").click();
  }
  await expect(menu).toBeVisible();
  await expect(menu).toHaveAttribute("open", "");
}

/** Close the app menu if it is currently open. */
export async function closeMenu(page: Page): Promise<void> {
  const menu = page.locator("#app-menu");
  const isOpen = await menu.isVisible().catch(() => false);
  if (isOpen) {
    await page.locator("#menu-close").click();
  }
  await expect(menu).toBeHidden();
}

/**
 * Open the app menu, activate a control that now lives inside it, and wait for
 * the menu to close. The menu closes before the control's own handler runs, so
 * dialogs opened by the control are a second layer.
 */
export async function clickMenuControl(
  page: Page,
  selector: string,
): Promise<void> {
  await openMenu(page);
  const menu = page.locator("#app-menu");
  const control = page.locator(selector);
  await expect(control).toBeVisible();
  await control.click();
  await expect(menu).toBeHidden();
}
