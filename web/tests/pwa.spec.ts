import { expect, test } from "@playwright/test";

test("pwa installs a service worker and enables cross-origin isolation", async ({ page }) => {
  await page.goto("/web/drive/");
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );

  const state = await page.evaluate(async () => {
    const link = document.querySelector<HTMLLinkElement>('link[rel="manifest"]');
    const manifest = link ? await (await fetch(link.href)).json() : null;
    const icon = manifest?.icons?.[0]?.src
      ? await fetch(new URL(manifest.icons[0].src, link?.href).toString())
      : null;
    return {
      error: (window as any).__driveViewerError,
      isolated: window.crossOriginIsolated,
      sharedArrayBuffer: typeof SharedArrayBuffer,
      controlled: Boolean(navigator.serviceWorker?.controller),
      manifestName: manifest?.name,
      iconStatus: icon?.status,
    };
  });

  expect(state.error).toBeUndefined();
  expect(state.isolated).toBe(true);
  expect(state.sharedArrayBuffer).toBe("function");
  expect(state.controlled).toBe(true);
  expect(state.manifestName).toBe("Lapster");
  expect(state.iconStatus).toBe(200);
});
