import { APP_VERSION } from "./version.generated.ts";

export function versionedWasmUrl(path: string): URL {
  const url = new URL(path, import.meta.url);
  url.searchParams.set("v", APP_VERSION);
  return url;
}
