#!/usr/bin/env bash
# EA Code Release Script (Bash)
# Usage: ./scripts/release.sh [version|patch|minor|major] [--auto-tag]

set -euo pipefail

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "Error: Not in a git repository"
  exit 1
}

TAURI_PATH="$ROOT/frontend/desktop/src-tauri/tauri.conf.json"
CARGO_PATH="$ROOT/frontend/desktop/src-tauri/Cargo.toml"
PACKAGE_PATH="$ROOT/frontend/desktop/package.json"

CURRENT=$(grep -o '"version": "[^"]*"' "$TAURI_PATH" | head -1 | cut -d'"' -f4)
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

ARG="patch"
AUTO_TAG=false
for INPUT in "$@"; do
  case "$INPUT" in
    --auto-tag)
      AUTO_TAG=true
      ;;
    patch|minor|major)
      ARG="$INPUT"
      ;;
    *)
      if echo "$INPUT" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        ARG="$INPUT"
      else
        echo "Usage: ./scripts/release.sh [version|patch|minor|major] [--auto-tag]"
        exit 1
      fi
      ;;
  esac
done

if echo "$ARG" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  VERSION="$ARG"
elif [ "$ARG" = "patch" ]; then
  VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
elif [ "$ARG" = "minor" ]; then
  VERSION="$MAJOR.$((MINOR + 1)).0"
else
  VERSION="$((MAJOR + 1)).0.0"
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

replace_in_file "$CARGO_PATH" "0,/^version = \"$CURRENT\"/s//version = \"$VERSION\"/"
echo "[2/6] Bumped frontend/desktop/src-tauri/Cargo.toml"

replace_in_file "$PACKAGE_PATH" "s/\"version\": \"$CURRENT\"/\"version\": \"$VERSION\"/"
echo "[3/6] Bumped frontend/desktop/package.json"

git add "$TAURI_PATH" "$CARGO_PATH" "$PACKAGE_PATH"
git commit -m "chore: bump version to $VERSION"
echo "[4/7] Committed version bump"

git push origin main
echo "[5/7] Pushed version bump commit to origin/main"

if [ "$AUTO_TAG" != "true" ]; then
  echo
  echo "Wait for the CI workflow to finish successfully before pushing the release tag."
  echo "CI: https://github.com/eatbas/ea-code/actions/workflows/ci.yml"
  read -r -p "Push release tag $TAG now? [y/N] " PUSH_TAG_NOW
  if [ -z "$PUSH_TAG_NOW" ] || { [ "$PUSH_TAG_NOW" != "y" ] && [ "$PUSH_TAG_NOW" != "Y" ]; }; then
    echo
    echo "Tag not pushed."
    echo "After CI passes, run:"
    echo "  git tag $TAG"
    echo "  git push origin $TAG"
    exit 0
  fi
fi

git tag "$TAG"
echo "[6/7] Created tag $TAG"

git push origin "$TAG"
echo "[7/7] Pushed tag $TAG"

echo
echo "Release $TAG triggered. Monitor at:"
echo "  https://github.com/eatbas/ea-code/actions"
