#!/bin/sh
# Configure git to use the project's hooks in .githooks/
# Run this once after cloning, or it runs automatically via `npm install`.
set -e
git config core.hooksPath .githooks
echo "✅ Git hooks configured (core.hooksPath = .githooks)"
