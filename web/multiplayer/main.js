// web/src/ui/car-selection.ts
var CAR_OPTIONS = [
  {
    id: "mazda-rx7",
    name: "Mazda RX-7",
    url: "../../fixtures/cars/mazda-rx7/mazda-rx7.sima",
    engine: "rotary2",
    vehicle: "sports"
  },
  {
    id: "sc-deckard-conejo",
    name: "Deckard Conejo",
    url: "../../fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima",
    engine: "v8cross",
    vehicle: "sports"
  }
];
var DEFAULT_CAR = CAR_OPTIONS[0];
var CAR_STORAGE_KEY = "lapster.car.v1";
function findCar(id) {
  return CAR_OPTIONS.find((car) => car.id === id) ?? null;
}
function loadSelectedCar() {
  try {
    return findCar(localStorage.getItem(CAR_STORAGE_KEY)) ?? DEFAULT_CAR;
  } catch {
    return DEFAULT_CAR;
  }
}

// web/src/ui/track-selection.ts
var TRACK_OPTIONS = [
  {
    id: "skidpad05",
    name: "Skidpad 0.5",
    url: "../../fixtures/tracks/skidpad05/pack.sima"
  },
  {
    id: "drift-sketchfab",
    name: "Drift Tracks",
    url: "../../fixtures/tracks/drift-sketchfab/pack.sima"
  }
];
var DEFAULT_TRACK = TRACK_OPTIONS[0];
var TRACK_STORAGE_KEY = "lapster.track.v1";
function findTrack(id) {
  return TRACK_OPTIONS.find((track) => track.id === id) ?? null;
}
function loadSelectedTrack() {
  try {
    return findTrack(localStorage.getItem(TRACK_STORAGE_KEY)) ?? DEFAULT_TRACK;
  } catch {
    return DEFAULT_TRACK;
  }
}

// web/src/net/room.ts
var NAME_STORAGE_KEY = "lapster.net.name.v1";
var PASSWORD_STORAGE_KEY = "lapster.net.password.v1";
var ROOM_ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
function makeRoomCode(length = 6) {
  const bytes = new Uint8Array(length);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  return Array.from(bytes, (value) => ROOM_ALPHABET[value % ROOM_ALPHABET.length]).join("");
}
function normalizeRoomCode(value) {
  const code = value?.trim().toUpperCase() ?? "";
  if (!code || code.length > 32 || !/^[A-Z0-9_-]+$/.test(code)) {
    return null;
  }
  return code;
}
function loadPlayerName() {
  try {
    return localStorage.getItem(NAME_STORAGE_KEY)?.trim() || "";
  } catch {
    return "";
  }
}
function savePlayerName(name) {
  try {
    localStorage.setItem(NAME_STORAGE_KEY, name.trim());
  } catch {
  }
}
function saveRoomPassword(password) {
  try {
    if (password) {
      sessionStorage.setItem(PASSWORD_STORAGE_KEY, password);
    } else {
      sessionStorage.removeItem(PASSWORD_STORAGE_KEY);
    }
  } catch {
  }
}
function lobbyInviteUrl(baseHref, room, car, track) {
  const url = new URL(baseHref);
  url.search = "";
  url.hash = "";
  url.searchParams.set("room", room);
  url.searchParams.set("car", car);
  url.searchParams.set("track", track);
  return url.toString();
}
function driveRoomUrl(baseHref, room, car, track) {
  const url = new URL(baseHref);
  url.search = "";
  url.hash = "";
  url.searchParams.set("mp", "1");
  url.searchParams.set("room", room);
  url.searchParams.set("car", car);
  url.searchParams.set("track", track);
  return url.toString();
}

// web/multiplayer/main.ts
var params = new URLSearchParams(window.location.search);
var selectedCar = findCar(params.get("car")) ?? loadSelectedCar();
var selectedTrack = findTrack(params.get("track")) ?? loadSelectedTrack();
var roomInput = document.querySelector("#room");
var nameInput = document.querySelector("#name");
var passwordInput = document.querySelector("#password");
var carName = document.querySelector("#car-name");
var trackName = document.querySelector("#track-name");
var serverName = document.querySelector("#server-name");
var status = document.querySelector("#status");
var form = document.querySelector("#join-form");
var newRoomButton = document.querySelector("#new-room");
var copyInviteButton = document.querySelector("#copy-invite");
carName.textContent = selectedCar.name;
trackName.textContent = selectedTrack.name;
var relayOverride = params.get("ws");
serverName.textContent = relayOverride ? `Server: ${new URL(relayOverride, window.location.href).host}` : "Server: this page";
roomInput.value = normalizeRoomCode(params.get("room")) ?? makeRoomCode();
nameInput.value = loadPlayerName();
function currentRoom() {
  const room = normalizeRoomCode(roomInput.value);
  if (!room) {
    throw new Error("Room code must be 1\u201332 letters, numbers, - or _.");
  }
  return room;
}
function syncUrl() {
  try {
    const url = new URL(window.location.href);
    url.searchParams.set("room", currentRoom());
    url.searchParams.set("car", selectedCar.id);
    url.searchParams.set("track", selectedTrack.id);
    window.history.replaceState(null, "", url);
  } catch {
  }
}
function setStatus(message, error = false) {
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
      selectedTrack.id
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
      selectedTrack.id
    );
    window.location.assign(driveUrl);
  } catch (error) {
    setStatus(error instanceof Error ? error.message : String(error), true);
  }
});
syncUrl();
