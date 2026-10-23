import { clearPackCache } from "./assets/cache.ts";
import { APP_VERSION } from "./version.generated.ts";

const RELOAD_KEY = "lapster.update.reload";
const VERSION_URL = new URL("../../web/version.json", window.location.href).toString();
let checking = false;

function checkVersion() {
  if (checking || document.hidden) {
    return;
  }
  checking = true;
  fetch(`${VERSION_URL}?t=${Date.now()}`, { cache: "no-store" })
    .then((response) => (response.ok ? response.json() : null))
    .then((info: { version?: string } | null) => {
      if (!info?.version || info.version === APP_VERSION) {
        return;
      }
      let alreadyReloaded = false;
      try {
        alreadyReloaded = sessionStorage.getItem(RELOAD_KEY) === info.version;
      } catch {
        alreadyReloaded = false;
      }
      if (alreadyReloaded) {
        return;
      }
      try {
        sessionStorage.setItem(RELOAD_KEY, info.version);
      } catch {
        // Private browsing may reject sessionStorage; reload once anyway.
      }
      void hardReload(info.version);
    })
    .catch(() => {
      // Offline or missing version metadata: keep the current build.
    })
    .finally(() => {
      checking = false;
    });
}

export function checkForUpdate() {
  checkVersion();
  window.setInterval(checkVersion, 60_000);
  window.addEventListener("focus", checkVersion);
  window.addEventListener("pageshow", checkVersion);
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) {
      checkVersion();
    }
  });
}

export async function hardReload(version?: string) {
  try {
    await clearPackCache();
  } catch {
    // IndexedDB may be unavailable; the versioned URL still bypasses stale packs.
  }
  try {
    const keys = await caches.keys();
    await Promise.all(keys.map((key) => caches.delete(key)));
  } catch {
    // Cache Storage is optional.
  }
  const url = new URL(window.location.href);
  url.searchParams.set("v", version ?? String(Date.now()));
  url.searchParams.set("t", String(Date.now()));
  window.location.replace(url.toString());
}
