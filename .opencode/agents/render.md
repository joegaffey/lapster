---
description: wgpu renderer and wasm rendering entry points.
mode: subagent
model: opencode-go/deepseek-v4.1-flash
steps: 30
permission:
  edit:
    "*": deny
    "crates/sim-render/**": allow
  bash:
    "*": deny
    "cargo test -p sim-render*": allow
    "cargo clippy -p sim-render*": allow
    "cargo check -p sim-render*": allow
    "cargo fmt --check*": allow
    "git status*": allow
    "git diff*": allow
    "git log*": allow
  task: deny
  read:
    "*": allow
    "target/**": deny
    "web/node_modules/**": deny
    "tools/speed-dreams/node_modules/**": deny
    "web/pkg/**": deny
    "web/.cache/**": deny
    "dist/**": deny
    "fixtures/real/**": deny
    "web/test-results/**": deny
    "web/playwright-report/**": deny
    "**/*.kn5": deny
    "**/*.log": deny
    ".env*": deny
    "web/src/version.generated.ts": deny
    "web/version.json": deny
    "web/drive/main.js": deny
    "web/garage/main.js": deny
    "web/multiplayer/main.js": deny
    "web/track-viewer/main.js": deny
    "web/style-spike/main.js": deny
    "web/worker/physics-worker.js": deny
    "web/audio/combustion-worklet.js": deny
    "Cargo.lock": deny
    "web/package-lock.json": deny
    "tools/speed-dreams/package-lock.json": deny
---
You are the render worker for lapster. Before acting, read `AGENTS.md`,
`agents/protocol.md`, `agents/contracts.md`, and `agents/domains/render.md`.

Implement only the assigned task packet within your owned paths. Run the
scoped verification commands. Return only the manifest JSON.
