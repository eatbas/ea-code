#!/usr/bin/env bash
# Maestro Release Script (Bash)
# Usage: ./scripts/release.sh [version|patch|minor|major]

set -euo pipefail

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "Error: Not in a git repository"
  exit 1
}

TAURI_PATH="$ROOT/frontend/desktop/src-tauri/tauri.conf.json"
CARGO_PATH="$ROOT/frontend/desktop/src-tauri/Cargo.toml"
PACKAGE_PATH="$ROOT/frontend/desktop/package.json"

CURRENT=$(grep -o '"version": "[^"]*"' "$TAURI_PATH" | head -1 | cut -d'"' -f4)
CARGO_CURRENT=$(grep -E '^version = "[^"]*"' "$CARGO_PATH" | head -1 | cut -d'"' -f2)
PACKAGE_CURRENT=$(grep -o '"version": "[^"]*"' "$PACKAGE_PATH" | head -1 | cut -d'"' -f4)

# Fail-fast if the three version files disagree. Silent drift is how a previous
# release shipped with Cargo.toml pinned to an older version — the sed
# replacement below only fires when the "from" version matches, so mismatched
# sources silently produced no change.
if [ -z "$CURRENT" ] || [ -z "$CARGO_CURRENT" ] || [ -z "$PACKAGE_CURRENT" ]; then
  echo "Error: Could not read a version from one of the sources:" >&2
  echo "  tauri.conf.json : '${CURRENT:-<missing>}'" >&2
  echo "  Cargo.toml      : '${CARGO_CURRENT:-<missing>}'" >&2
  echo "  package.json    : '${PACKAGE_CURRENT:-<missing>}'" >&2
  exit 1
fi

if [ "$CURRENT" != "$CARGO_CURRENT" ] || [ "$CURRENT" != "$PACKAGE_CURRENT" ]; then
  echo "Error: Version sources are out of sync — aborting to avoid a partial bump." >&2
  echo "  tauri.conf.json : $CURRENT" >&2
  echo "  Cargo.toml      : $CARGO_CURRENT" >&2
  echo "  package.json    : $PACKAGE_CURRENT" >&2
  echo "Sync them manually to the same value, commit, then re-run this script." >&2
  exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

ARG="${1:-patch}"
if echo "$ARG" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  VERSION="$ARG"
elif [ "$ARG" = "patch" ]; then
  VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
elif [ "$ARG" = "minor" ]; then
  VERSION="$MAJOR.$((MINOR + 1)).0"
elif [ "$ARG" = "major" ]; then
  VERSION="$((MAJOR + 1)).0.0"
else
  echo "Usage: ./scripts/release.sh [version|patch|minor|major]"
  exit 1
fi

TAG="v$VERSION"
echo "Current version: $CURRENT"
echo "New version:     $VERSION  (tag: $TAG)"
echo

if [ -n "$(git status --porcelain)" ]; then
  echo "Error: You have uncommitted changes. Commit or stash them first."
  git status --short
  exit 1
fi

if git tag -l "$TAG" | grep -q "$TAG"; then
  echo "Error: Tag $TAG already exists"
  exit 1
fi

read -r -p "Proceed? [Y/n] " CONFIRM
if [ -n "$CONFIRM" ] && [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
  echo "Aborted."
  exit 0
fi

replace_in_file() {
  local file="$1"
  local pattern="$2"
  if sed --version >/dev/null 2>&1; then
    sed -i "$pattern" "$file"
  else
    sed -i '' "$pattern" "$file"
  fi
}

replace_in_file "$TAURI_PATH" "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/"
echo "[1/6] Bumped frontend/desktop/src-tauri/tauri.conf.json"

# Scope the substitution to the [package] section so we never clobber a
# workspace or dependency version. BSD sed (macOS default) silently accepts
# the GNU-only `0,/regex/s//...` form without substituting — that was the
# source of the 1.7.7 partial-bump failure.
replace_in_file "$CARGO_PATH" "/^\[package\]/,/^\[/ s/^version = \"$CURRENT\"\$/version = \"$VERSION\"/"
echo "[2/6] Bumped frontend/desktop/src-tauri/Cargo.toml"

replace_in_file "$PACKAGE_PATH" "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/"
echo "[3/6] Bumped frontend/desktop/package.json"

# Verify all three files actually landed on $VERSION before committing.
TAURI_AFTER=$(grep -o '"version": "[^"]*"' "$TAURI_PATH" | head -1 | cut -d'"' -f4)
CARGO_AFTER=$(grep -E '^version = "[^"]*"' "$CARGO_PATH" | head -1 | cut -d'"' -f2)
PACKAGE_AFTER=$(grep -o '"version": "[^"]*"' "$PACKAGE_PATH" | head -1 | cut -d'"' -f4)

if [ "$TAURI_AFTER" != "$VERSION" ] || [ "$CARGO_AFTER" != "$VERSION" ] || [ "$PACKAGE_AFTER" != "$VERSION" ]; then
  echo "Error: Post-bump verification failed — files did not land on $VERSION:" >&2
  echo "  tauri.conf.json : $TAURI_AFTER" >&2
  echo "  Cargo.toml      : $CARGO_AFTER" >&2
  echo "  package.json    : $PACKAGE_AFTER" >&2
  echo "No commit or tag created. Restore and investigate." >&2
  exit 1
fi

git add "$TAURI_PATH" "$CARGO_PATH" "$PACKAGE_PATH"
git commit -m "chore: bump version to $VERSION"
echo "[4/6] Committed version bump"

git tag "$TAG"
echo "[5/6] Created tag $TAG"

git push origin main --tags
echo "[6/6] Pushed commit and tag to origin/main"

echo
echo "Release $TAG triggered. Monitor at:"
echo "  https://github.com/eatbas/maestro/actions"
