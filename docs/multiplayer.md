# Relay Multiplayer

Basic multiplayer is a relay-only prototype. Clients keep their existing local
physics; the server only manages rooms, rosters, passwords, and the latest
dashboard state for each player. It does not simulate, validate, or arbitrate
vehicle physics.

## Scope

- One car model and one track per room.
- Up to four players.
- Optional room password.
- Local physics at 500 Hz, state upload at 30 Hz.
- Remote cars interpolate a 100 ms tick-based snapshot timeline.
- Cars are ghosts: no car-to-car collision and no server correction.
- Offline Pages deployment and the single-player physics worker are unchanged.

## Run Locally

```bash
cd web
npm run build:viewer
bash scripts/build-pages.sh
npm run serve:multiplayer
```

The script prints the URL. It starts at port 3000 and uses the next free port if
3000 is occupied. For example:

```text
http://127.0.0.1:3010/web/multiplayer/?room=TEST&car=mazda-rx7&track=skidpad05
http://127.0.0.1:3010/web/drive/?mp=1&room=TEST&car=mazda-rx7&track=skidpad05
```

Do not use a plain static server for the shared URL: `/ws` must be served by
`relay-server`. If `/ws` returns 404, the page is being served by the wrong
process.

The relay server can also be run directly:

```bash
cargo run -p relay-server -- --static-dir dist --bind 0.0.0.0:3000
```

## External Relay

The dev/static Pages build cannot host WebSockets itself. Host `relay-server`
separately and point the browser at it with the `ws` query parameter:

```text
https://joegaffey.com/lapster-dev/web/multiplayer/?ws=wss%3A%2F%2Frelay.example.test%2Fws
```

The lobby preserves `ws` when redirecting to Drive and when copying an invite.
The relay URL must use `wss://` when the page is served over HTTPS. The relay
server itself still needs no track or car assets in this mode; use any existing
directory for `--static-dir`.

## Codespaces

Codespaces is used as a **relay-only** host to keep startup fast. The browser
client comes from the dev Pages build.

1. Create a Codespace from this branch.
2. The devcontainer builds only `relay-server` and starts it on port `3000`.
   If it is not running, use `bash web/scripts/serve-relay.sh`.
3. Set port `3000` to **Public** in the Ports panel, or run
   `gh codespace ports visibility 3000:public`.
4. Open `https://<codespace-name>-3000.app.github.dev/`. It redirects to the dev
   multiplayer client with the correct `ws=` relay URL.
5. Check `/tmp/lapster-relay.log` inside the Codespace if the relay does not start.

This avoids installing Node, wasm-bindgen, and the full viewer build in the
Codespace. Only Rust and the small relay binary are required.

The invite link contains the room, car, and track. Share the password separately
when one is set. Codespaces is suitable for development and short playtests; it
sleeps when idle and is not a persistent production host.

## Room Behaviour

- The first join creates the room and fixes its car/track pair.
- Later joins must use the same car/track.
- Slots follow the garage bay layout, spaced 8 m apart across five bays per row.
- Empty rooms are removed immediately.
- The room password is compared in memory and is never sent back to clients.
- If someone joins without the password, the room rejects them.

## Wire Protocol

Control and state messages are JSON over `/ws`.

Client to server:

```json
{"type":"join","room":"TEST","password":"","name":"Driver","car":"mazda-rx7","track":"skidpad05"}
{"type":"state","seq":1,"tick":123,"state":[0.0,0.0,0.0,0.0]}
{"type":"ping"}
```

Server to client:

```json
{"type":"welcome","player_id":1,"slot":0,"room":"TEST","car":"mazda-rx7","track":"skidpad05","max_players":4,"players":[],"states":[]}
{"type":"roster","players":[]}
{"type":"state","player_id":1,"seq":1,"tick":123,"state":[]}
{"type":"player_left","player_id":1}
{"type":"error","message":"wrong room password"}
{"type":"pong"}
```

The `state` array is the existing 64-float simulation snapshot. The relay only
checks length, finiteness, and monotonic sequence numbers.

## Manual Verification

Use two separate browser profiles or two machines so both pages remain active.

1. Open the multiplayer lobby and create or enter a room code.
2. Set a name and optional password, then join.
3. Share the invite URL; give the password separately.
4. Confirm both players appear in the room badge and roster count.
5. Confirm each car starts in a different garage bay, 8 m apart across the first row.
6. Drive one car and confirm the other client sees it move and turn.
7. Confirm brake and reverse lights update on the remote car.
8. Drift or spin a car and confirm the other client sees remote tyre smoke.
9. Enable Audio and confirm the remote engine and tyre noise get quieter as the car moves away.
10. Leave one client and confirm the other roster drops back to one player.
11. Join with a wrong password and confirm the room rejects it.
12. Reload the offline Drive page and confirm single-player behaviour is unchanged.

The automated suite covers the relay room logic and client protocol helpers. The
two-browser game test is kept manual for now because headless Chromium throttles
background tabs, which makes keyboard input timing unreliable.

## Security and Limits

This is not an authoritative or cheat-resistant implementation. A client can
send arbitrary positions, teleport, or impersonate a room member if it knows the
password. Do not use it for competitive results. The room password is access
control only, not authentication.

The websocket must be reached over `wss://` when the page is served over HTTPS.
Serving the page and relay from the same origin avoids CORS and mixed-content
issues.
