/**
 * True when the page was loaded with `?sw=0`.
 *
 * This only disables service-worker registration/caching for local testing; it
 * is not a security bypass. Cross-origin isolation is still provided by the
 * local dev server headers.
 */
export function isServiceWorkerBypass(search: string): boolean {
  return new URLSearchParams(search).get("sw") === "0";
}
