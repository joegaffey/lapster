import { defineConfig } from "@playwright/test";

const fastArgs = [
  "--enable-unsafe-webgpu",
  "--enable-features=Vulkan",
  "--use-angle=swiftshader",
  "--enable-unsafe-swiftshader",
  "--disable-dev-shm-usage",
];

const trackEditorArgs = process.env.CI
  ? [
      "--use-gl=angle",
      "--use-angle=swiftshader-webgl",
      "--enable-unsafe-swiftshader",
      "--disable-dev-shm-usage",
    ]
  : fastArgs;

export default defineConfig({
  testDir: "./tests",
  testMatch: /.*\.spec\.ts/,
  timeout: 60_000,
  fullyParallel: true,
  workers: Number(process.env.PLAYWRIGHT_WORKERS ?? 1),
  retries: process.env.CI ? 2 : 0,
  outputDir: "test-results",
  use: {
    baseURL: "http://127.0.0.1:8123",
    viewport: { width: 1280, height: 720 },
  },
  projects: [
    {
      name: "track-editor",
      testMatch: /track-editor\.spec\.ts/,
      use: { launchOptions: { args: trackEditorArgs } },
    },
    {
      name: "default",
      testIgnore: /track-editor\.spec\.ts/,
      use: { launchOptions: { args: fastArgs } },
    },
  ],
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.02,
    },
  },
  webServer: {
    command: "python3 -m http.server 8123 --bind 127.0.0.1",
    cwd: "..",
    url: "http://127.0.0.1:8123/web/track-viewer/",
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
