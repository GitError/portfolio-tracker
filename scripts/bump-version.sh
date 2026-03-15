#!/bin/zsh

set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  echo "Usage: ./scripts/bump-version.sh <version>"
  exit 1
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Version must be in semver format: X.Y.Z"
  exit 1
fi

echo "Bumping project version to $VERSION..."

npm version "$VERSION" --no-git-tag-version

node -e '
  const fs = require("fs");
  const path = "src-tauri/tauri.conf.json";
  const json = JSON.parse(fs.readFileSync(path, "utf8"));
  json.version = process.argv[1];
  fs.writeFileSync(path, JSON.stringify(json, null, 2) + "\n");
' "$VERSION"

perl -0pi -e 's/^version = ".*?"$/version = "'"$VERSION"'"/m' src-tauri/Cargo.toml

echo "Updated:"
echo "  - package.json"
echo "  - src-tauri/tauri.conf.json"
echo "  - src-tauri/Cargo.toml"
echo
echo "Next steps:"
echo "  git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml CHANGELOG.md"
echo "  git commit -m \"chore: bump version to $VERSION\""
echo "  git tag v$VERSION"
echo "  git push && git push --tags"
