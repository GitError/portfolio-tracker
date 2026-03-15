# Releases

## Download

Published installers are attached to GitHub Releases:

- Latest release: https://github.com/GitError/portfolio-tracker/releases/latest
- All releases: https://github.com/GitError/portfolio-tracker/releases

Draft releases are created automatically from tags that match `v*.*.*`.

## Versioning

Keep these three files in sync for every release:

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

Use `./scripts/bump-version.sh X.Y.Z` to update them together.

## Release flow

1. Update `CHANGELOG.md`
2. Run `./scripts/bump-version.sh X.Y.Z`
3. Commit the version bump
4. Tag the release with `vX.Y.Z`
5. Push the branch and tag
6. GitHub Actions builds installers and creates a draft release
7. Review the draft release and publish it manually

## Code signing

### macOS

Unsigned macOS builds work for testing, but users will see Gatekeeper warnings.
To enable Developer ID signing and notarization, configure these repository secrets:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

The release workflow is already wired to use those environment variables when they are present.

### Windows

Windows signing is optional for now. Without a signing certificate, SmartScreen may warn on first launch.

## Expected artifacts

- macOS Apple Silicon: `.dmg`
- macOS Intel: `.dmg`
- Windows: `.exe`, `.msi`
- Linux: `.AppImage`, `.deb`
