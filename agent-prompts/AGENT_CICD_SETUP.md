# CI/CD Pipeline Setup

Read the CLAUDE.md in the project root for full project context.

## Task

Set up the CI/CD pipeline and local quality gates for this Tauri v2 + React/TypeScript + Rust portfolio tracker. The workflow files are pre-written in the `ci-cd/` folder. No API keys needed — code review runs locally through Claude Code CLI via git hooks.

## Steps

### 1. Move workflow files into place

Copy the pre-written files from `ci-cd/` to their correct locations:

- `ci-cd/ci.yml` → `.github/workflows/ci.yml`
- `ci-cd/dependabot.yml` → `.github/dependabot.yml`
- `ci-cd/setup-branch-protection.sh` → `.github/setup-branch-protection.sh`

Skip `claude-review.yml` — we're doing reviews locally instead.

Make `setup-branch-protection.sh` executable.

### 2. Update the reviewer username

In `.github/dependabot.yml`, replace all instances of `"michal"` in the reviewers field with the actual GitHub username from the git config (`git config user.name` or check the remote origin URL).

### 3. Frontend tooling setup

Ensure the following dev dependencies are installed and configured:

**ESLint** (if not already set up):
- Install: `eslint`, `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, `eslint-plugin-react-refresh`
- Create `eslint.config.js` (flat config format) with TypeScript + React rules
- No `console.log` in production code (warn level)
- Unused imports/vars as errors

**Prettier**:
- Install: `prettier`
- Create `.prettierrc` with: `{ "semi": true, "singleQuote": true, "tabWidth": 2, "trailingComma": "es5", "printWidth": 100 }`
- Create `.prettierignore`: `dist/`, `src-tauri/target/`, `node_modules/`

**Vitest**:
- Install: `vitest`, `@testing-library/react`, `@testing-library/jest-dom`, `jsdom`
- Create `vitest.config.ts` with jsdom environment, coverage via v8
- Add a basic smoke test: `src/__tests__/App.test.tsx` that renders App and checks it mounts without crashing

**package.json scripts** — ensure these exist:
```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "lint": "eslint src/ --ext .ts,.tsx --max-warnings 0",
    "format": "prettier --write \"src/**/*.{ts,tsx,css}\"",
    "format:check": "prettier --check \"src/**/*.{ts,tsx,css}\"",
    "test": "vitest run",
    "test:watch": "vitest",
    "test:coverage": "vitest run --coverage",
    "review": "claude -p 'Review the staged changes for code quality, type alignment between Rust and TypeScript, error handling, and adherence to the dark terminal design system. Be concise, flag real issues only.'",
    "review:diff": "claude -p \"Review this diff for bugs, type mismatches, and security issues. Be concise.\\n\\n$(git diff HEAD~1)\""
  }
}
```

### 4. Rust tooling verification

In `src-tauri/`, verify:
- `cargo fmt` runs without errors (if it reformats, commit the changes)
- `cargo clippy` passes with no warnings (fix any issues)
- `cargo test` passes (add a basic test in `stress.rs` or `db.rs` if none exist)

If there are no Rust tests yet, add at least these:
- `db.rs`: test that `init_db` creates tables without error on an in-memory SQLite connection
- `stress.rs`: test that applying a zero-shock scenario returns the same values as input
- `fx.rs`: test that `convert_to_cad("CAD", amount)` returns the amount unchanged

### 5. Git hooks

Create `.githooks/` with three hooks:

**`.githooks/pre-commit`** — fast lint + format gate (runs on every commit):
```bash
#!/bin/zsh
set -e
echo "⚡ Pre-commit: lint + format check"

# Frontend
npm run lint --silent
npm run format:check --silent

# Backend
cd src-tauri
cargo fmt --check 2>/dev/null

echo "✓ Pre-commit passed"
```

**`.githooks/pre-push`** — full test suite + Claude review before pushing to main or dev:
```bash
#!/bin/zsh
set -e

BRANCH=$(git rev-parse --abbrev-ref HEAD)
REMOTE_REF=$1
PUSH_TARGET=$2

echo "🚀 Pre-push: verifying before push to $BRANCH"

# ── Always run tests ──
echo "→ Running frontend tests..."
npm run test --silent

echo "→ Running backend tests..."
cd src-tauri
cargo test --quiet 2>/dev/null
cd ..

echo "→ Running clippy..."
cd src-tauri
cargo clippy --quiet -- -D warnings 2>/dev/null
cd ..

# ── Claude review only on pushes to main or dev ──
if [[ "$BRANCH" == "main" || "$BRANCH" == "dev" ]]; then
  echo ""
  echo "🔍 Running Claude Code review (pushing to protected branch)..."
  echo ""

  # Get the diff of commits being pushed
  DIFF=$(git log --oneline --no-merges origin/$BRANCH..HEAD 2>/dev/null)

  if [ -n "$DIFF" ]; then
    # Run Claude review on the diff
    claude -p "You are reviewing code about to be pushed to the '$BRANCH' branch of a Tauri v2 portfolio tracker (Rust + React/TypeScript).

Review the following changes for:
1. Bugs or logic errors
2. Type alignment issues between Rust (serde camelCase) and TypeScript interfaces
3. Security issues (SQL injection, hardcoded secrets, unvalidated input)
4. Missing error handling
5. Style consistency with the dark terminal/Bloomberg design system

Only flag real issues. Be concise. If everything looks good, say so.

Changes being pushed:
$(git diff origin/$BRANCH..HEAD)

Files changed:
$(git diff --name-only origin/$BRANCH..HEAD)"

    echo ""
    read "REPLY?Claude review complete. Continue pushing? [y/N] "
    if [[ ! "$REPLY" =~ ^[Yy]$ ]]; then
      echo "Push aborted."
      exit 1
    fi
  fi
fi

echo "✓ Pre-push passed"
```

**`.githooks/post-merge`** — auto-install deps after pulling:
```bash
#!/bin/zsh
echo "📦 Post-merge: checking for dependency changes..."

CHANGED_FILES=$(git diff-tree -r --name-only --no-commit-id ORIG_HEAD HEAD)

if echo "$CHANGED_FILES" | grep -q "package-lock.json"; then
  echo "→ package-lock.json changed, running npm install..."
  npm install
fi

if echo "$CHANGED_FILES" | grep -q "Cargo.lock"; then
  echo "→ Cargo.lock changed, running cargo build..."
  cd src-tauri && cargo build
fi

echo "✓ Post-merge complete"
```

Make all hooks executable and configure git:
```bash
chmod +x .githooks/*
git config core.hooksPath .githooks
```

### 6. Update branch protection

Edit `.github/setup-branch-protection.sh` — remove the `"review"` entry from the `main` branch required status checks (since we're not using the Claude GitHub Action). The required checks should only be:
```json
"contexts": [
  "Frontend",
  "Backend (Rust)"
]
```

### 7. Create the dev branch

If it doesn't exist yet:
```bash
git checkout -b dev
git push -u origin dev
```

### 8. Verify everything

Run the full local pipeline:
```bash
npm run lint
npm run format:check
npm run test
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test && cd ..
```

Fix any issues until all checks pass.

### 9. Commit and push

```bash
git add .github/ .githooks/ eslint.config.js .prettierrc .prettierignore vitest.config.ts src/__tests__/
git commit -m "ci: add CI pipeline, local Claude review hooks, and dependabot"
git push
```

Then remind me to run:
```bash
./.github/setup-branch-protection.sh
```
