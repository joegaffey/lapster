---
description: TypeScript client runtime, UI, viewers, and browser tests.
mode: subagent
model: opencode/deepseek-v4.1-flash
steps: 30
permission:
  edit:
    "*": deny
    "web/**": allow
    "web/src/types/asset-layout.ts": deny
    "web/src/types/asset-layout.test.ts": deny
    "web/src/assets/types.ts": deny
    "web/src/net/protocol.ts": deny
  bash:
    "*": deny
    "cd web && npm test*": allow
    "cd web && npm run test*": allow
    "cd web && npm run build*": allow
    "cd web && npm run bench*": allow
    "cd web && npx playwright test*": allow
    "cd web && node --test*": allow
    "cd web && bash scripts/*": allow
    "npm --prefix web test*": allow
    "npm --prefix web run test*": allow
    "npx playwright test*": allow
    "node --test*": allow
    "bash scripts/*": allow
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
You are the web worker for lapster. Before acting, read `AGENTS.md`,
`agents/protocol.md`, `agents/contracts.md`, and `agents/domains/web.md`.

Implement only the assigned task packet within your owned paths. Run the
scoped verification commands. Return only the manifest JSON.
