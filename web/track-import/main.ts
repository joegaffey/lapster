import init, { compile_glb_track } from "../pkg/track-pack/track_pack_wasm.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import {
  TRACK_ED_TRACK_CACHE_KEY,
  TRACK_ED_TRACK_ID,
} from "../src/assets/editor-track.ts";
import { versionedWasmUrl } from "../src/wasm-init.ts";

declare global {
  interface Window {
    __trackImportReady?: boolean;
    __trackImportError?: string;
  }
}

const form = document.querySelector<HTMLFormElement>("#import-form")!;
const jsonInput = document.querySelector<HTMLInputElement>("#track-json")!;
const glbInput = document.querySelector<HTMLInputElement>("#track-glb")!;
const button = document.querySelector<HTMLButtonElement>("#import-button")!;
const status = document.querySelector<HTMLDivElement>("#status")!;

function setStatus(message: string, error = false): void {
  status.textContent = message;
  status.classList.toggle("error", error);
}

async function readFile(input: HTMLInputElement, label: string): Promise<Uint8Array> {
  const file = input.files?.[0];
  if (!file) {
    throw new Error(`Choose ${label}.`);
  }
  return new Uint8Array(await file.arrayBuffer());
}

async function boot(): Promise<void> {
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
        readFile(glbInput, "track.glb"),
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
