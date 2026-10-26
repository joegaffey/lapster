const COOP = "same-origin";
const COEP = "require-corp";

self.addEventListener("install", (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

async function networkFirst(request, extraHeaders = {}) {
  const response = await fetch(request, { cache: "no-store" });
  if (response.type === "opaque" || response.status === 0) {
    return response;
  }
  const headers = new Headers(response.headers);
  headers.set("Cache-Control", "no-store");
  for (const [name, value] of Object.entries(extraHeaders)) {
    headers.set(name, value);
  }
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(
      networkFirst(request, {
        "Cross-Origin-Opener-Policy": COOP,
        "Cross-Origin-Embedder-Policy": COEP,
      }),
    );
    return;
  }

  if (url.pathname.endsWith("/web/version.json")) {
    event.respondWith(networkFirst(request));
    return;
  }

  if (url.pathname.includes("/worker/") || request.destination === "worker") {
    event.respondWith(
      networkFirst(request, {
        "Cross-Origin-Opener-Policy": COOP,
        "Cross-Origin-Embedder-Policy": COEP,
        "Cross-Origin-Resource-Policy": "same-origin",
      }),
    );
  }
});
