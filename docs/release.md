# EA Code Release Guide

This guide documents signed updater releases for Windows and macOS.

## Release Flow

- GitHub Actions builds desktop installers only when a tag matching `v*` is pushed.
- Windows build publishes NSIS installer assets.
- macOS build publishes universal DMG assets.
- Updater metadata is published as `latest.json` in the same GitHub Release.
- The app checks `https://github.com/eatbas/ea-code/releases/latest/download/latest.json` and silently installs newer versions.

## One-Time Setup

1. Generate an updater keypair locally:
   - `npx tauri signer generate --write-keys "$HOME/.tauri/ea-code.key"`
2. Add GitHub repository secrets:
   - `TAURI_SIGNING_PRIVATE_KEY`: contents of `~/.tauri/ea-code.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: key password used during generation
3. Keep `~/.tauri/ea-code.key.pub` committed in `src-tauri/tauri.conf.json` as `plugins.updater.pubkey`.

## Creating a Release

Use one of the release scripts:

- PowerShell:
  - `.\scripts\release.ps1`
  - `.\scripts\release.ps1 minor`
  - `.\scripts\release.ps1 major`
  - `.\scripts\release.ps1 0.2.0`
- Bash:
  - `./scripts/release.sh`
  - `./scripts/release.sh minor`
  - `./scripts/release.sh major`
  - `./scripts/release.sh 0.2.0`

Each script:

1. Validates clean git status.
2. Validates tag does not already exist.
3. Bumps version in:
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - `package.json`
4. Commits with `chore: bump version to X.Y.Z`.
5. Creates tag `vX.Y.Z`.
6. Pushes commit and tag to `origin/main`.

## Key Rotation

When rotating signing keys:

1. Generate a new keypair with `tauri signer generate`.
2. Update `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` GitHub secrets.
3. Replace `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`.
4. Release a new version after rotation.

Do not delete old private keys until the new release has been validated on both platforms.
