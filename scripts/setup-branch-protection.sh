#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}"
BRANCH="${BRANCH:-main}"
REQUIRED_REVIEWS="${REQUIRED_REVIEWS:-0}"
REQUIRE_CODEOWNERS="${REQUIRE_CODEOWNERS:-false}"

echo "Protecting $REPO branch $BRANCH (reviews=$REQUIRED_REVIEWS codeowners=$REQUIRE_CODEOWNERS)"

payload="$(
  REQUIRED_REVIEWS="$REQUIRED_REVIEWS" REQUIRE_CODEOWNERS="$REQUIRE_CODEOWNERS" python3 - <<'PY'
import json
import os

reviews = int(os.environ["REQUIRED_REVIEWS"])
codeowners = os.environ["REQUIRE_CODEOWNERS"].lower() == "true"

payload = {
    "required_status_checks": {"strict": True, "contexts": ["gate"]},
    "enforce_admins": True,
    "required_pull_request_reviews": None,
    "restrictions": None,
    "allow_force_pushes": False,
    "allow_deletions": False,
    "required_conversation_resolution": True,
}
if reviews > 0:
    payload["required_pull_request_reviews"] = {
        "dismiss_stale_reviews": True,
        "require_code_owner_reviews": codeowners,
        "required_approving_review_count": reviews,
    }
print(json.dumps(payload))
PY
)"

printf '%s\n' "$payload" | gh api -X PUT "repos/$REPO/branches/$BRANCH/protection" --input -
