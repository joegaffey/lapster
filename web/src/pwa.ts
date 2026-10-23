const SW_URL = new URL("../sw.js", window.location.href);
const SW_SCOPE = new URL("../", window.location.href).pathname;
const RELOAD_KEY = "lapster.coi.reload";

const timeout = (milliseconds: number) =>
  new Promise<void>((resolve) => window.setTimeout(resolve, milliseconds));

export async function setupPwa(): Promise<boolean> {
  if (!("serviceWorker" in navigator)) {
    return false;
  }

  let alreadyAttempted = false;
  try {
    alreadyAttempted = sessionStorage.getItem(RELOAD_KEY) === "1";
  } catch {
    alreadyAttempted = false;
  }
  if (alreadyAttempted && !window.crossOriginIsolated) {
    return false;
  }

  try {
    await Promise.race([
      navigator.serviceWorker.register(SW_URL, {
        scope: SW_SCOPE,
        updateViaCache: "none",
      }),
      timeout(2500),
    ]);
    await Promise.race([navigator.serviceWorker.ready, timeout(2500)]);

    if (window.crossOriginIsolated) {
      return false;
    }

    if (!navigator.serviceWorker.controller) {
      await Promise.race([
        new Promise<void>((resolve) => {
          navigator.serviceWorker.addEventListener("controllerchange", () => resolve(), {
            once: true,
          });
        }),
        timeout(2000),
      ]);
    }

    if (window.crossOriginIsolated) {
      return false;
    }

    if (alreadyAttempted) {
      return false;
    }

    try {
      sessionStorage.setItem(RELOAD_KEY, "1");
    } catch {
      // Private browsing may reject sessionStorage; still reload once.
    }
    window.location.reload();
    return true;
  } catch (error) {
    console.warn("PWA setup failed; continuing without cross-origin isolation", error);
    return false;
  }
}
