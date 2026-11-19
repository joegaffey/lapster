// web/track-import/main.ts
import init, { compile_glb_track } from "../pkg/track-pack/track_pack_wasm.js";

// web/src/assets/cache.ts
var DB_NAME = "sim-assets";
var DB_VERSION = 1;
var STORE_NAME = "packs";
var IndexedDbPackCache = class {
  async get(key) {
    const db = await openDatabase();
    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, "readonly");
      const request = transaction.objectStore(STORE_NAME).get(key);
      request.onsuccess = () => {
        const value = request.result;
        resolve(value instanceof Uint8Array ? value : void 0);
      };
      request.onerror = () => reject(request.error);
    });
  }
  async put(key, bytes) {
    const db = await openDatabase();
    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, "readwrite");
      transaction.objectStore(STORE_NAME).put(bytes, key);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }
  async delete(key) {
    const db = await openDatabase();
    return new Promise((resolve, reject) => {
      const transaction = db.transaction(STORE_NAME, "readwrite");
      transaction.objectStore(STORE_NAME).delete(key);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }
};
function openDatabase() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE_NAME)) {
        request.result.createObjectStore(STORE_NAME);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

// web/src/assets/editor-track.ts
var TRACK_ED_TRACK_ID = "track-ed";
var TRACK_ED_TRACK_CACHE_KEY = "lapster.track-ed.v1";

// web/src/version.generated.ts
var APP_VERSION = "fe488bf-1795112462";

// web/src/wasm-init.ts
function versionedWasmUrl(path) {
  const url = new URL(path, import.meta.url);
  url.searchParams.set("v", APP_VERSION);
  return url;
}

// web/track-import/main.ts
var form = document.querySelector("#import-form");
var jsonInput = document.querySelector("#track-json");
var glbInput = document.querySelector("#track-glb");
var button = document.querySelector("#import-button");
var status = document.querySelector("#status");
function setStatus(message, error = false) {
  status.textContent = message;
  status.classList.toggle("error", error);
}
async function readFile(input, label) {
  const file = input.files?.[0];
  if (!file) {
    throw new Error(`Choose ${label}.`);
  }
  return new Uint8Array(await file.arrayBuffer());
}
async function boot() {
  try {
    await init(versionedWasmUrl("../pkg/track-pack/track_pack_wasm_bg.wasm"));
    window.__trackImportReady = true;
    setStatus("Ready.");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    window.__trackImportError = message;
    setStatus(`Could not load the track compiler: ${message}`, true);
    button.disabled = true;
  }
}
form.addEventListener("submit", (event) => {
  event.preventDefault();
  void (async () => {
    button.disabled = true;
    try {
      setStatus("Reading track files...");
      const [trackJson, glb] = await Promise.all([
        readFile(jsonInput, "track.json"),
        readFile(glbInput, "track.glb")
      ]);
      setStatus("Compiling track...");
      const pack = compile_glb_track(trackJson, glb, TRACK_ED_TRACK_ID);
      await new IndexedDbPackCache().put(TRACK_ED_TRACK_CACHE_KEY, pack);
      setStatus(`Compiled ${(pack.byteLength / 1024 / 1024).toFixed(1)} MB. Opening Test drive...`);
      const target = new URL("../drive/", window.location.href);
      target.searchParams.set("track", TRACK_ED_TRACK_ID);
      const car = new URLSearchParams(window.location.search).get("car");
      if (car) {
        target.searchParams.set("car", car);
      }
      window.location.href = target.toString();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus(message, true);
      button.disabled = false;
    }
  })();
});
void boot();
