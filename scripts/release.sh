#!/usr/bin/env bash
# release.sh — Prepare a new release
# Usage: bash scripts/release.sh v2.1.0
#
# What it does:
#   1. Validates version format (vX.Y.Z)
#   2. Updates Cargo.toml version
#   3. Generates changelog from conventional commits
#   4. Creates git tag
#   5. Pushes tag (triggers GitHub Actions release workflow)

set -euo pipefail

VERSION="${1:?Usage: bash scripts/release.sh v2.1.0}"
VERSION_CLEAN="${VERSION#v}"

# Validate format
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-.*)?$ ]]; then
  echo "❌ Invalid version format: $VERSION"
  echo "   Expected: vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-PRERELEASE"
  exit 1
fi

echo "🦀 Preparing release: $VERSION"

# 1. Update Cargo.toml versions
echo "  → Updating Cargo.toml versions to $VERSION_CLEAN"
for toml in Cargo.toml crates/*/Cargo.toml; do
  if grep -q '^version' "$toml" 2>/dev/null; then
    sed -i "s/^version = \".*\"/version = \"$VERSION_CLEAN\"/" "$toml"
  fi
done

# 2. Generate changelog section
echo "  → Generating changelog from commits"
SECTION=$(bash scripts/generate-changelog.sh "$VERSION")

# 3. Update changelog file
CHANGELOG="docs/en/changelog.md"
if [ -f "$CHANGELOG" ]; then
  TEMP=$(mktemp)
  # Find first ## line and insert before it
  awk -v section="$SECTION" '
    /^## / && !inserted {
      print section
      print ""
      inserted=1
    }
    { print }
  ' "$CHANGELOG" > "$TEMP"
  mv "$TEMP" "$CHANGELOG"
  echo "  → Updated $CHANGELOG"
fi

# 4. Commit version bump + changelog
git add -A
git commit -m "chore: release $VERSION" || echo "  → Nothing to commit"

# 5. Create and push tag
git tag -a "$VERSION" -m "Release $VERSION

$SECTION"
git push origin main --tags

echo ""
echo "✅ Release $VERSION pushed!"
echo "   GitHub Actions will build binaries and publish the release."
echo "   Watch: https://github.com/$(git remote get-url origin | sed 's|.*github.com/||;s|\.git$||')/actions"
