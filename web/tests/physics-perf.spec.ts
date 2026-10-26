import { expect, test } from "@playwright/test";

const enabled = process.env.RUN_PHYSICS_PERF === "1";

async function waitForReady(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => (window as any).__driveViewerReady === true || (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function measureTickRate(page: import("@playwright/test").Page): Promise<number> {
  return page.evaluate(async () => {
    const viewer = (window as any).__driveViewer;
    await viewer.reset();
    await new Promise((resolve) => setTimeout(resolve, 500));
    const before = JSON.parse(viewer.telemetry()).tick;
    const started = performance.now();
    await new Promise((resolve) => setTimeout(resolve, 3_000));
    const elapsed = (performance.now() - started) / 1000;
    const after = JSON.parse(viewer.telemetry()).tick;
    return (after - before) / elapsed;
  });
}

async function measureFrames(page: import("@playwright/test").Page): Promise<{
  fps: number;
  p95FrameMs: number;
  p99FrameMs: number;
  maxFrameMs: number;
}> {
  await page.locator("#view").click({ position: { x: 640, y: 360 } });
  await page.keyboard.down("w");
  const result = await page.evaluate(async () => {
    const times: number[] = [];
    let last = performance.now();
    const started = last;
    await new Promise<void>((resolve) => {
      const frame = (now: number) => {
        times.push(now - last);
        last = now;
        if (now - started < 6_000) {
          requestAnimationFrame(frame);
        } else {
          resolve();
        }
      };
      requestAnimationFrame(frame);
    });
    const sorted = [...times].sort((a, b) => a - b);
    const mean = times.reduce((a, b) => a + b, 0) / times.length;
    const percentile = (p: number) =>
      sorted[Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * p))];
    return {
      fps: 1000 / mean,
      p95FrameMs: percentile(0.95),
      p99FrameMs: percentile(0.99),
      maxFrameMs: sorted[sorted.length - 1],
    };
  });
  await page.keyboard.up("w");
  await page.evaluate(async () => {
    await (window as any).__driveViewer.reset();
  });
  return result;
}

async function measureMode(
  page: import("@playwright/test").Page,
  mode: "main" | "worker",
): Promise<{ mode: string; physicsHz: number; fps: number; p95FrameMs: number; p99FrameMs: number; maxFrameMs: number }> {
  await page.goto(
    `/web/drive/?physics=${mode}&track=../../fixtures/tracks/minimal/pack.sima&car=../../fixtures/cars/minimal/pack.sima&v=perf`,
  );
  await page.bringToFront();
  await waitForReady(page);
  await page.waitForTimeout(1_000);
  const physicsHz = await measureTickRate(page);
  const frames = await measureFrames(page);
  const telemetry = await page.evaluate(() =>
    JSON.parse((window as any).__driveViewer.telemetry()),
  );
  return {
    mode: telemetry.physicsMode ?? "main",
    physicsHz,
    ...frames,
  };
}

test("SAB physics worker matches the main thread", async ({ page }) => {
  test.skip(!enabled, "set RUN_PHYSICS_PERF=1 to run the physics A/B benchmark");
  test.setTimeout(180_000);
  const throttle = Number(process.env.PERF_CPU_THROTTLE ?? 0);
  const cdp = await page.context().newCDPSession(page);
  if (throttle > 1) {
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: throttle });
  }
  const main = await measureMode(page, "main");
  const worker = await measureMode(page, "worker");
  if (throttle > 1) {
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
  }
  const results = { throttle, main, worker };
  console.log(`PHYSICS_PERF ${JSON.stringify(results)}`);

  expect(worker.physicsHz).toBeGreaterThan(480);
  if (process.env.PERF_REQUIRE_MAIN_HZ === "1") {
    expect(main.physicsHz).toBeGreaterThan(480);
  }
  if (process.env.PERF_ASSERT_FPS === "1") {
    expect(worker.fps).toBeGreaterThan(main.fps * 0.9);
  }
});
