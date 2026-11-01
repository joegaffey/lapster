import { findCar, loadSelectedCar } from "../src/ui/car-selection.ts";
import { findTrack, loadSelectedTrack } from "../src/ui/track-selection.ts";
import {
  driveRoomUrl,
  loadPlayerName,
  lobbyInviteUrl,
  makeRoomCode,
  normalizeRoomCode,
  savePlayerName,
  saveRoomPassword,
} from "../src/net/room.ts";

const params = new URLSearchParams(window.location.search);
const selectedCar = findCar(params.get("car")) ?? loadSelectedCar();
const selectedTrack = findTrack(params.get("track")) ?? loadSelectedTrack();
const roomInput = document.querySelector<HTMLInputElement>("#room")!;
const nameInput = document.querySelector<HTMLInputElement>("#name")!;
const passwordInput = document.querySelector<HTMLInputElement>("#password")!;
const carName = document.querySelector<HTMLElement>("#car-name")!;
const trackName = document.querySelector<HTMLElement>("#track-name")!;
const serverName = document.querySelector<HTMLElement>("#server-name")!;
const status = document.querySelector<HTMLElement>("#status")!;
const form = document.querySelector<HTMLFormElement>("#join-form")!;
const newRoomButton = document.querySelector<HTMLButtonElement>("#new-room")!;
const copyInviteButton = document.querySelector<HTMLButtonElement>("#copy-invite")!;

carName.textContent = selectedCar.name;
trackName.textContent = selectedTrack.name;
const relayOverride = params.get("ws");
serverName.textContent = relayOverride
  ? `Server: ${new URL(relayOverride, window.location.href).host}`
  : "Server: this page";

roomInput.value = normalizeRoomCode(params.get("room")) ?? makeRoomCode();
nameInput.value = loadPlayerName();

function currentRoom(): string {
  const room = normalizeRoomCode(roomInput.value);
  if (!room) {
    throw new Error("Room code must be 1–32 letters, numbers, - or _.");
  }
  return room;
}

function syncUrl(): void {
  try {
    const url = new URL(window.location.href);
    url.searchParams.set("room", currentRoom());
    url.searchParams.set("car", selectedCar.id);
    url.searchParams.set("track", selectedTrack.id);
    window.history.replaceState(null, "", url);
  } catch {
    // Keep the editable field when the code is temporarily invalid.
  }
}

function setStatus(message: string, error = false): void {
  status.textContent = message;
  status.className = error ? "status error" : "status";
}

roomInput.addEventListener("input", () => {
  const room = normalizeRoomCode(roomInput.value);
  if (room && room !== roomInput.value) {
    roomInput.value = room;
  }
  syncUrl();
});

newRoomButton.addEventListener("click", () => {
  roomInput.value = makeRoomCode();
  syncUrl();
  roomInput.focus();
  roomInput.select();
  setStatus("New room code generated. Copy the invite link when ready.");
});

copyInviteButton.addEventListener("click", async () => {
  try {
    const room = currentRoom();
    syncUrl();
    const url = lobbyInviteUrl(
      window.location.href,
      room,
      selectedCar.id,
      selectedTrack.id,
      relayOverride,
    );
    await navigator.clipboard.writeText(url);
    setStatus("Invite link copied.");
  } catch (error) {
    setStatus(error instanceof Error ? error.message : String(error), true);
  }
});

form.addEventListener("submit", (event) => {
  event.preventDefault();
  try {
    const room = currentRoom();
    const name = nameInput.value.trim();
    if (!name) {
      throw new Error("Enter a name.");
    }
    const password = passwordInput.value;
    savePlayerName(name);
    saveRoomPassword(password);
    syncUrl();
    const driveUrl = driveRoomUrl(
      new URL("../drive/", window.location.href).toString(),
      room,
      selectedCar.id,
      selectedTrack.id,
      relayOverride,
    );
    window.location.assign(driveUrl);
  } catch (error) {
    setStatus(error instanceof Error ? error.message : String(error), true);
  }
});

syncUrl();
