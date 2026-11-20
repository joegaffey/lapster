---
description: TypeScript client runtime, UI, viewers, and browser tests.
mode: subagent
model: opencode-go/deepseek-v4.1-flash
steps: 80
permission:
  edit:
    "*": deny
    "web/**": allow
    "web/track-ed-src/**": deny
    "web/track-import/**": deny
    "web/track-editor/**": deny
    "web/tests/track-editor.spec.ts": deny
    "web/tests/editor-drive.spec.ts": deny
    "web/tests/track-import.spec.ts": deny
    "web/tests/track-ed*.spec.ts": deny
    "web/src/types/asset-layout.ts": deny
    "web/src/types/asset-layout.test.ts": deny
    "web/src/assets/types.ts": deny
    "web/src/net/protocol.ts": deny
  bash:
    "*": ask
    "cd web && npm test*": allow
    "cd web && npm run test*": allow
    "cd web && npm run build*": allow
    "cd web && npm run bench*": allow
    "cd web && npx playwright test*": allow
    "cd web && node --test*": allow
    "cd web && bash scripts/*": allow
    "cd web && npx playwright install*": allow
    "npm --prefix web test*": allow
    "npm --prefix web run test*": allow
    "npm --prefix web run build*": allow
    "npm --prefix web run bench*": allow
    "npx playwright test*": allow
    "npx playwright install*": allow
    "node --test*": allow
    "bash scripts/*": allow
    "bash web/scripts/*": allow
    "git status*": allow
    "git diff*": allow
    "git log*": allow
  task: deny
---
You are the web worker for lapster. Before acting, read `AGENTS.md`,
`agents/protocol.md`, `agents/contracts.md`, and `agents/domains/web.md`.

Implement only the assigned task packet within your owned paths. Run the
scoped verification commands. Return only the manifest JSON.
