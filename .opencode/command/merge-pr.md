---
description: Merge a PR into its base without deploying
agent: architect
---
Merge a pull request only.

Arguments: $ARGUMENTS

- Require a PR number. If it is missing, ask for one.
- Run `gh pr view <n> --json baseRefName,state,mergeable` and
  `gh pr checks <n>`.
- If the base is `main`, do not merge; comment and hand off to @joegaffey.
- If checks are not green or the PR is not mergeable, stop and report.
- Restate: "Merging PR #<n> into <base>; this does not deploy."
- Run `gh pr merge <n> --squash --delete-branch` after checks pass.
- Do not trigger the dev preview or push to `main`.
