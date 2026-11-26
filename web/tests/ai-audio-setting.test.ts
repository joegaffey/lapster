import test from "node:test";
import assert from "node:assert/strict";

import {
  AI_VOICE_DEFAULT,
  AI_VOICE_STORAGE_KEY,
  aiAudioFromVoices,
  clampAiVoices,
  loadAiVoices,
  parseAiAudioOverride,
  saveAiVoices,
  type AiVoiceStorage,
} from "../src/audio/ai-audio-setting.ts";

function memoryStorage(initial: Record<string, string> = {}): AiVoiceStorage {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

test("voice count clamps to the 0..5 range", () => {
  assert.equal(clampAiVoices(-3), 0);
  assert.equal(clampAiVoices(0), 0);
  assert.equal(clampAiVoices(3), 3);
  assert.equal(clampAiVoices(5), 5);
  assert.equal(clampAiVoices(9), 5);
  assert.equal(clampAiVoices("4"), 4);
  assert.equal(clampAiVoices("nope"), AI_VOICE_DEFAULT);
});

test("voice count maps to off, nearest, or full modes", () => {
  assert.deepEqual(aiAudioFromVoices(0), { mode: "off", voices: 0 });
  assert.deepEqual(aiAudioFromVoices(1), { mode: "nearest", voices: 1 });
  for (const voices of [2, 3, 4, 5]) {
    assert.deepEqual(aiAudioFromVoices(voices), { mode: "full", voices });
  }
  assert.deepEqual(aiAudioFromVoices(99), { mode: "full", voices: 5 });
});

test("missing or corrupt storage loads the one-voice default", () => {
  assert.equal(loadAiVoices(memoryStorage()), 1);
  assert.equal(
    loadAiVoices(memoryStorage({ [AI_VOICE_STORAGE_KEY]: "bad" })),
    1,
  );
  assert.equal(
    loadAiVoices(memoryStorage({ [AI_VOICE_STORAGE_KEY]: "4" })),
    4,
  );
});

test("saving clamps, persists, and round-trips through load", () => {
  const storage = memoryStorage();
  assert.equal(saveAiVoices(9, storage), 5);
  assert.equal(storage.getItem(AI_VOICE_STORAGE_KEY), "5");
  assert.equal(loadAiVoices(storage), 5);

  assert.equal(saveAiVoices(-2, storage), 0);
  assert.equal(storage.getItem(AI_VOICE_STORAGE_KEY), "0");
  assert.equal(loadAiVoices(storage), 0);
});

test("the ?aiAudio override maps each diagnostic mode", () => {
  assert.deepEqual(parseAiAudioOverride("?aiAudio=off"), {
    mode: "off",
    voices: 0,
  });
  assert.deepEqual(parseAiAudioOverride("aiAudio=nearest"), {
    mode: "nearest",
    voices: 1,
  });
  assert.deepEqual(parseAiAudioOverride("?aiAudio=full&v=1"), {
    mode: "full",
    voices: 5,
  });
  assert.deepEqual(parseAiAudioOverride("?aiAudio=lite"), {
    mode: "lite",
    voices: 3,
  });
  assert.equal(parseAiAudioOverride("?aiAudio=bogus"), null);
  assert.equal(parseAiAudioOverride(""), null);
  assert.equal(parseAiAudioOverride(null), null);
});
