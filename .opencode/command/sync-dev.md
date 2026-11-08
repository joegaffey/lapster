---
description: Sync dev with main after a release without deploying or merging
agent: architect
---
Sync `dev` with `main`.

1. Run `git fetch origin`.
2. Switch to `dev` and run `git pull origin main --no-rebase`.
3. Resolve release-only docs/config conflicts by keeping `dev`'s current
   content. If a conflict touches code, stop and escalate to the owning worker.
4. Run `git diff --check`. If the merge introduced code changes, run the scoped
   code gate; otherwise skip cargo and web builds.
5. Push `dev`.
6. Confirm `git log origin/main..origin/dev` only shows new work, not the
   already-released commits.

Do not merge a PR, deploy, or push to `main`.
