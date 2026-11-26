import { expect, test } from "@playwright/test";
import { clickMenuControl, openMenu } from "./menu-helpers";

async function waitForDrive(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () =>
      (window as any).__driveViewerReady === true ||
      (window as any).__driveViewerError,
    undefined,
    { timeout: 30_000 },
  );
}

async function startRaceAudio(
  page: import("@playwright/test").Page,
  query: string,
  aiCount = 2,
) {
  await page.goto(
    `/web/drive/?track=race-loop&race=1&ai=${aiCount}&category=gp67&${query}`,
  );
  await waitForDrive(page);
  await expect(page.locator("#viewer-loading")).toBeHidden();
  const button = page.locator("#audio-toggle");
  await expect(button).toBeEnabled();
  await clickMenuControl(page, "#audio-toggle");
  await page.waitForFunction(
    () => (window as any).__driveAudio?.status === "running",
    undefined,
    { timeout: 20_000 },
  );
}

async function waitForAudibleRemote(page: import("@playwright/test").Page) {
  await page.waitForFunction(
    () => {
      const remote = (window as any).__remoteAudio;
      if (remote?.status !== "running") {
        return false;
      }
      return (remote.voiceSummary?.() ?? []).some(
        (voice: any) => voice.ready && voice.gain > 0,
      );
    },
    undefined,
    { timeout: 20_000 },
  );
}

async function selectAiVoices(
  page: import("@playwright/test").Page,
  voices: number,
): Promise<void> {
  await openMenu(page);
  await page.selectOption("#ai-audio-voices", String(voices));
}

async function waitForVoiceCount(
  page: import("@playwright/test").Page,
  count: number,
) {
  await page.waitForFunction(
    (expected) => {
      const remote = (window as any).__remoteAudio;
      const voices = remote?.voiceSummary?.() ?? [];
      return (
        remote?.status === "running" &&
        voices.length === expected &&
        voices.every((voice: any) => voice.ready)
      );
    },
    count,
    { timeout: 20_000 },
  );
}

test("default AI audio uses one nearest EngineSim voice in its own context", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "v=ai-audio-nearest");
  await expect(page.locator("#ai-audio-voices")).toHaveValue("1");
  await waitForAudibleRemote(page);

  const state = await page.evaluate(() => {
    const remote = (window as any).__remoteAudio;
    return {
      sharesPlayerContext: remote.sharesPlayerContext,
      status: remote.status,
      voices: remote.voiceSummary(),
    };
  });
  expect(state.sharesPlayerContext).toBe(false);
  expect(state.status).toBe("running");
  expect(state.voices.length).toBe(1);
  const [voice] = state.voices;
  expect(voice.slot).toBe(0);
  expect(voice.mode).toBe("full");
  expect(voice.ready).toBe(true);
  expect(voice.gain).toBeGreaterThan(0);
  expect(pageErrors).toEqual([]);
});

test("?aiAudio=lite keeps the low-cost oscillator path", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "aiAudio=lite&v=ai-audio-lite");
  await waitForAudibleRemote(page);

  const state = await page.evaluate(() => {
    const remote = (window as any).__remoteAudio;
    return {
      sharesPlayerContext: remote.sharesPlayerContext,
      status: remote.status,
      voices: remote.voiceSummary(),
    };
  });
  expect(state.sharesPlayerContext).toBe(false);
  expect(state.status).toBe("running");
  expect(state.voices.length).toBeGreaterThanOrEqual(1);
  expect(state.voices.length).toBeLessThanOrEqual(3);
  for (const voice of state.voices) {
    expect(voice.mode).toBe("lite");
    expect(voice.ready).toBe(true);
  }
  expect(state.voices.some((voice: any) => voice.gain > 0)).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("?aiAudio=full keeps the per-slot EngineSim voice path", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "aiAudio=full&v=ai-audio-full");
  await waitForAudibleRemote(page);

  const state = await page.evaluate(() => {
    const remote = (window as any).__remoteAudio;
    return {
      sharesPlayerContext: remote.sharesPlayerContext,
      status: remote.status,
      voices: remote.voiceSummary(),
    };
  });
  expect(state.sharesPlayerContext).toBe(false);
  expect(state.status).toBe("running");
  const ready = state.voices.filter(
    (voice: any) => voice.ready && voice.gain > 0,
  );
  expect(ready.length).toBeGreaterThanOrEqual(1);
  expect(state.voices.every((voice: any) => voice.mode === "full")).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("?aiAudio=off disables remote engine audio", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "aiAudio=off&v=ai-audio-off");

  const state = await page.evaluate(() => {
    const remote = (window as any).__remoteAudio;
    return {
      exists: remote !== undefined && remote !== null,
      voiceCount: remote?.voiceCount ?? 0,
    };
  });
  expect(state.exists).toBe(false);
  expect(state.voiceCount).toBe(0);
  expect(pageErrors).toEqual([]);
});

test("selecting three rebuilds three full EngineSim voices", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "v=ai-audio-select-three", 4);
  await waitForAudibleRemote(page);
  await selectAiVoices(page, 3);
  await waitForVoiceCount(page, 3);
  await waitForAudibleRemote(page);

  const state = await page.evaluate(() => {
    const remote = (window as any).__remoteAudio;
    return {
      sharesPlayerContext: remote.sharesPlayerContext,
      voices: remote.voiceSummary(),
    };
  });
  expect(state.sharesPlayerContext).toBe(false);
  expect(state.voices.length).toBe(3);
  expect(state.voices.every((voice: any) => voice.mode === "full")).toBe(true);
  expect(state.voices.some((voice: any) => voice.gain > 0)).toBe(true);
  expect(pageErrors).toEqual([]);
});

test("selecting zero removes remote audio without stopping the player", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "v=ai-audio-select-off", 4);
  await waitForAudibleRemote(page);
  await selectAiVoices(page, 0);
  await page.waitForFunction(
    () =>
      (window as any).__remoteAudio === undefined ||
      (window as any).__remoteAudio === null,
    undefined,
    { timeout: 10_000 },
  );

  const state = await page.evaluate(() => ({
    exists: Boolean((window as any).__remoteAudio),
    playerStatus: (window as any).__driveAudio?.status,
    stored: localStorage.getItem("lapster.audio.aiVoices.v1"),
    selector: document.querySelector<HTMLSelectElement>("#ai-audio-voices")
      ?.value,
  }));
  expect(state.exists).toBe(false);
  expect(state.playerStatus).toBe("running");
  expect(state.stored).toBe("0");
  expect(state.selector).toBe("0");
  expect(pageErrors).toEqual([]);
});

test("opponent voice setting persists across reload", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await startRaceAudio(page, "v=ai-audio-persist", 4);
  await waitForAudibleRemote(page);
  await selectAiVoices(page, 4);
  await waitForVoiceCount(page, 4);
  expect(
    await page.evaluate(() => localStorage.getItem("lapster.audio.aiVoices.v1")),
  ).toBe("4");

  await page.reload();
  await waitForDrive(page);
  await expect(page.locator("#ai-audio-voices")).toHaveValue("4");
  expect(
    await page.evaluate(() => localStorage.getItem("lapster.audio.aiVoices.v1")),
  ).toBe("4");

  await page.keyboard.press("w");
  await page.waitForFunction(
    () => (window as any).__driveAudio?.status === "running",
    undefined,
    { timeout: 20_000 },
  );
  await waitForVoiceCount(page, 4);
  expect(pageErrors).toEqual([]);
});
