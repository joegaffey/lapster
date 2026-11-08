---
description: Deploy the dev preview for a ref without merging anything
agent: architect
---
Deploy the dev preview only.

Arguments: $ARGUMENTS

- Determine the ref: strip an optional `ref=` prefix; if the argument is empty,
  use `dev`.
- Run `gh workflow run pages.yml -R joegaffey/lapster-dev -f ref=<ref>`.
- Watch the newest `pages.yml` run and report the deployed version and URL.
- Do not merge a PR, do not push to `dev` or `main`, and do not treat this as
  approval to merge.
