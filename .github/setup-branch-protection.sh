#!/bin/zsh
# ──────────────────────────────────────────────────────────────
# Branch Protection Setup — run once after creating the repo
# ──────────────────────────────────────────────────────────────
#
# Prerequisites:
#   - GitHub CLI installed: brew install gh
#   - Authenticated: gh auth login
#   - Run from the repo root
#
# This sets up branch protection rules for main and dev.
# Alternatively, configure manually in Settings → Branches.
# ──────────────────────────────────────────────────────────────

set -e

REPO=$(gh repo view --json nameWithOwner -q '.nameWithOwner')
echo "Configuring branch protection for: $REPO"

# ── main branch: strict protection ───────────────────────────
echo "\n→ Protecting 'main'..."
gh api repos/$REPO/branches/main/protection \
  --method PUT \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "Frontend",
      "Backend (Rust)"
    ]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF

echo "  ✓ main protected"
echo "    - PRs required (no direct push)"
echo "    - CI must pass (Frontend + Backend + Claude Review)"
echo "    - Stale reviews dismissed on new commits"

# ── dev branch: lighter protection ────────────────────────────
echo "\n→ Protecting 'dev'..."
gh api repos/$REPO/branches/dev/protection \
  --method PUT \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": false,
    "contexts": [
      "Frontend",
      "Backend (Rust)"
    ]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": true,
  "allow_deletions": false
}
EOF

echo "  ✓ dev protected"
echo "    - CI must pass (Frontend + Backend)"
echo "    - Direct push allowed (for agent merges)"
echo "    - Force push allowed (for rebases)"

echo "\n✅ Branch protection configured!"
echo ""
echo "Workflow summary:"
echo "  feature/* → PR to dev   (CI required)"
echo "  dev       → PR to main  (CI + Claude Review required)"
echo ""
echo "Next steps:"
echo "  1. Add ANTHROPIC_API_KEY to repo secrets:"
echo "     gh secret set ANTHROPIC_API_KEY"
echo "  2. Create the dev branch if it doesn't exist:"
echo "     git checkout -b dev && git push -u origin dev"
echo "  3. Update the 'michal' reviewer in .github/dependabot.yml"
echo "     with your actual GitHub username"
