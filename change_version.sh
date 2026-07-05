#!/usr/bin/env bash
set -e

NEW_VERSION="$1"
DESCRIPTION_MD="$2"
PRE_RELEASE_LABEL="$3"
IS_PRE_RELEASE=false

GREEN='\033[38;5;40m'
RED='\x1b[31m'
RESET='\033[0m'

log() {
  echo -e "\n${GREEN}[PROGRAM] $1${RESET}"
}

error() {
  echo -e "\n\n\n${RED}[ERROR] $1${RESET}"
}

# Checks if it pasts the tests
log "Running tests..."

run_cargo_test() {
  cargo test "$@" || { error "Tests failed!"; exit 1; }
}

run_cargo_test
run_cargo_test --lib -p board_lexer
run_cargo_test --lib -p board_website
run_cargo_test --lib -p board_settings

log "Tests passed!"

if [[ -z "$NEW_VERSION" || -z "$DESCRIPTION_MD" ]]; then
  echo -e "Usage:\n  $0 \"<version>\" \"<description.md>\" [--pre-release]"
  exit 1
fi

# If DESCRIPTION_MD is a file path, read it
if [ -f "$DESCRIPTION_MD" ]; then
  DESCRIPTION_MD="$(cat "$DESCRIPTION_MD")"
fi

# Detect pre-release flag
if [[ "$PRE_RELEASE_LABEL" == "-p" || "$PRE_RELEASE_LABEL" == "--pre-release" || "$PRE_RELEASE_LABEL" == "--prerelease" ]]; then
  IS_PRE_RELEASE=true
  if [[ "$NEW_VERSION" != *-pre ]]; then
    NEW_VERSION="${NEW_VERSION}-pre"
  fi
fi

# Remove leading "v"
if [[ "$NEW_VERSION" == v* ]]; then
  NEW_VERSION="${NEW_VERSION#v}"
fi

# Checks if version is already in use
WEBSITE_VERSION_FILE="../home/docs/board/json/versions.json"

TODAY_YEAR=$(date +%Y)
TODAY_MONTH=$(date +%-m)
TODAY_DAY=$(date +%-d)

# Check if version already exists and put it in website JSON
if jq -e --arg version "$NEW_VERSION" \
  '.[] | select(.version == $version)' \
  "$WEBSITE_VERSION_FILE" > /dev/null; then
  error "Version '$NEW_VERSION' already exists!"
  exit 1
fi

log "Updating version to: $NEW_VERSION"

update_cargo_toml() {
  local file="$1"
  sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"$NEW_VERSION\"/" "$file"
  rm -f "$file.bak"
}

update_cargo_toml Cargo.toml
update_cargo_toml dependencies/lexer/Cargo.toml
update_cargo_toml dependencies/website/Cargo.toml
update_cargo_toml dependencies/settings/Cargo.toml

# -----------------------
# Commit core repo
# -----------------------
commit_changes() {
  if [[ -n "$(git status --porcelain)" ]]; then
    git add Cargo.toml dependencies/lexer/Cargo.toml dependencies/website/Cargo.toml dependencies/settings/Cargo.toml

    git commit -m "chore: release v$NEW_VERSION" -m "$DESCRIPTION_MD"
  fi
}

# -----------------------
# Create GitHub release
# -----------------------
create_release() {
  TAG="v$NEW_VERSION"

  if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Tag $TAG already exists"
    exit 1
  fi

  git tag "$TAG"
  git push origin HEAD
  git push origin "$TAG"

  GH_FLAGS=()

  if [[ "$IS_PRE_RELEASE" == true ]]; then
    GH_FLAGS+=(--prerelease)
    echo "Creating pre-release on GitHub"
  else
    echo "Creating stable release on GitHub"
  fi

  gh release create "$TAG" \
    --title "$TAG" \
    --notes "$DESCRIPTION_MD" \
    "${GH_FLAGS[@]}"
}

# -----------------------
# Update website JSON
# -----------------------

NEW_ENTRY=$(jq -n \
  --arg version "$NEW_VERSION" \
  --arg description "$DESCRIPTION_MD" \
  --argjson year "$TODAY_YEAR" \
  --argjson month "$TODAY_MONTH" \
  --argjson day "$TODAY_DAY" \
  '{
    version: $version,
    date: { year: $year, month: $month, day: $day },
    description: $description
  }'
)

jq ". |= [$NEW_ENTRY] + ." "$WEBSITE_VERSION_FILE" > "${WEBSITE_VERSION_FILE}.tmp"
mv "${WEBSITE_VERSION_FILE}.tmp" "$WEBSITE_VERSION_FILE"

commit_website() {
  git add "$WEBSITE_VERSION_FILE"
  git commit -m "docs: update board version to v$NEW_VERSION" -m "$DESCRIPTION_MD"
  git push origin HEAD
}

# -----------------------
# Create Markdown map
# -----------------------

create_markdown_map() {
  echo "Creating Markdown map"
  MARKDOWN_MAP_FILE="docs/map.json"
}

# -----------------------
# Run pipeline
# -----------------------

# commit_changes
# create_release
# commit_website
# create_markdown_map

echo "Release process complete"
