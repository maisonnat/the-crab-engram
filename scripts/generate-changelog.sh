#!/usr/bin/env bash
# generate-changelog.sh — Generates changelog from conventional commits
# Usage: bash scripts/generate-changelog.sh [version]
# If no version, uses git describe --tags

set -euo pipefail

VERSION="${1:-$(git describe --tags --always 2>/dev/null || echo "unreleased")}"
PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
CHANGELOG_FILE="docs/en/changelog.md"

# Get commits since last tag
if [ -n "$PREV_TAG" ]; then
  RANGE="${PREV_TAG}..HEAD"
else
  RANGE="HEAD"
fi

# Classify commits
FEATURES=""
FIXES=""
PERF=""
DOCS=""
OTHER=""

while IFS= read -r line; do
  hash="${line%% *}"
  msg="${line#* }"

  case "$msg" in
    feat\(*\):*|feat:*)
      clean="${msg#feat*: }"
      FEATURES="${FEATURES}\n- ${clean} (\`${hash}\`)" ;;
    fix\(*\):*|fix:*)
      clean="${msg#fix*: }"
      FIXES="${FIXES}\n- ${clean} (\`${hash}\`)" ;;
    perf\(*\):*|perf:*)
      clean="${msg#perf*: }"
      PERF="${PERF}\n- ${clean} (\`${hash}\`)" ;;
    docs\(*\):*|docs:*)
      clean="${msg#docs*: }"
      DOCS="${DOCS}\n- ${clean} (\`${hash}\`)" ;;
    *)
      OTHER="${OTHER}\n- ${msg} (\`${hash}\`)" ;;
  esac
done < <(git log --oneline "$RANGE" 2>/dev/null || echo "")

# Generate version section
DATE=$(date +%Y-%m-%d)
SECTION="## ${VERSION} (${DATE})\n"

if [ -n "$FEATURES" ]; then
  SECTION="${SECTION}\n### Features${FEATURES}\n"
fi
if [ -n "$FIXES" ]; then
  SECTION="${SECTION}\n### Bug Fixes${FIXES}\n"
fi
if [ -n "$PERF" ]; then
  SECTION="${SECTION}\n### Performance${PERF}\n"
fi
if [ -n "$DOCS" ]; then
  SECTION="${SECTION}\n### Documentation${DOCS}\n"
fi
if [ -n "$OTHER" ]; then
  SECTION="${SECTION}\n### Other${OTHER}\n"
fi

# Print to stdout (for release workflow)
echo -e "$SECTION"

# Optionally prepend to changelog file
if [ "${UPDATE_FILE:-false}" = "true" ] && [ -f "$CHANGELOG_FILE" ]; then
  # Insert after the header line
  TEMP=$(mktemp)
  awk -v section="$(echo -e "$SECTION")" '
    /^## / && !inserted { print section; inserted=1 }
    { print }
  ' "$CHANGELOG_FILE" > "$TEMP"
  mv "$TEMP" "$CHANGELOG_FILE"
  echo "Updated $CHANGELOG_FILE"
fi
