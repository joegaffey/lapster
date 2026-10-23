const COOP = "same-origin";
const COEP = "require-corp";

self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET" || request.mode !== "navigate") {
    return;
  }

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) {
    return;
  }

  event.respondWith(
    (async () => {
      const response = await fetch(request, { cache: "no-store" });
      if (response.type === "opaque" || response.status === 0) {
        return response;
      }
      const headers = new Headers(response.headers);
      headers.set("Cross-Origin-Opener-Policy", COOP);
      headers.set("Cross-Origin-Embedder-Policy", COEP);
      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers,
      });
    })(),
  );
});
