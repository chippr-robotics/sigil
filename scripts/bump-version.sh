#!/usr/bin/env bash
#
# Version bump script for Sigil
# Usage: ./scripts/bump-version.sh [major|minor|patch] [alpha|beta|rc]
#
# Examples:
#   ./scripts/bump-version.sh patch         # 0.1.0 -> 0.1.1
#   ./scripts/bump-version.sh minor         # 0.1.0 -> 0.2.0
#   ./scripts/bump-version.sh major         # 0.1.0 -> 1.0.0
#   ./scripts/bump-version.sh minor alpha   # 0.1.0 -> 0.2.0-alpha.1
#   ./scripts/bump-version.sh patch beta    # 0.1.0 -> 0.1.1-beta.1
#   ./scripts/bump-version.sh patch rc      # 0.1.0 -> 0.1.1-rc.1

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"

# Check if we're in the right directory
if [ ! -f "$CARGO_TOML" ]; then
    echo -e "${RED}Error: Could not find Cargo.toml at $CARGO_TOML${NC}"
    exit 1
fi

# Parse arguments
BUMP_TYPE="${1:-}"
PRERELEASE="${2:-}"

if [ -z "$BUMP_TYPE" ]; then
    echo -e "${RED}Error: Missing bump type${NC}"
    echo "Usage: $0 [major|minor|patch] [alpha|beta|rc]"
    exit 1
fi

if [ "$BUMP_TYPE" != "major" ] && [ "$BUMP_TYPE" != "minor" ] && [ "$BUMP_TYPE" != "patch" ]; then
    echo -e "${RED}Error: Invalid bump type '$BUMP_TYPE'${NC}"
    echo "Must be one of: major, minor, patch"
    exit 1
fi

if [ -n "$PRERELEASE" ] && [ "$PRERELEASE" != "alpha" ] && [ "$PRERELEASE" != "beta" ] && [ "$PRERELEASE" != "rc" ]; then
    echo -e "${RED}Error: Invalid prerelease type '$PRERELEASE'${NC}"
    echo "Must be one of: alpha, beta, rc"
    exit 1
fi

# Extract current version from Cargo.toml
CURRENT_VERSION=$(grep -m 1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo -e "${RED}Error: Could not extract current version from Cargo.toml${NC}"
    exit 1
fi

echo -e "${BLUE}Current version: ${CURRENT_VERSION}${NC}"

# Parse current version (handle prerelease versions)
if [[ "$CURRENT_VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-([a-z]+)\.([0-9]+))?$ ]]; then
    MAJOR="${BASH_REMATCH[1]}"
    MINOR="${BASH_REMATCH[2]}"
    PATCH="${BASH_REMATCH[3]}"
    CURRENT_PRERELEASE="${BASH_REMATCH[5]}"
    PRERELEASE_NUM="${BASH_REMATCH[6]}"
else
    echo -e "${RED}Error: Invalid version format in Cargo.toml: $CURRENT_VERSION${NC}"
    exit 1
fi

# Calculate new version
case "$BUMP_TYPE" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
esac

# Build new version string
NEW_VERSION="$MAJOR.$MINOR.$PATCH"

# Handle prerelease
if [ -n "$PRERELEASE" ]; then
    # If same prerelease type and has a number, increment it
    # Otherwise, start at 1 (either new prerelease type or bumping from stable version)
    if [ "$CURRENT_PRERELEASE" = "$PRERELEASE" ] && [ -n "$PRERELEASE_NUM" ]; then
        PRERELEASE_NUM=$((PRERELEASE_NUM + 1))
    else
        PRERELEASE_NUM=1
    fi
    NEW_VERSION="$NEW_VERSION-$PRERELEASE.$PRERELEASE_NUM"
fi

echo -e "${GREEN}New version: ${NEW_VERSION}${NC}"
echo ""

# Confirm with user
read -p "Continue with version bump? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Aborted${NC}"
    exit 0
fi

# Update Cargo.toml
echo -e "${BLUE}Updating Cargo.toml...${NC}"
# Escape dots in version for sed regex
ESCAPED_CURRENT=$(echo "$CURRENT_VERSION" | sed 's/\./\\./g')
sed -i "s/^version = \"$ESCAPED_CURRENT\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

# Update Cargo.lock
echo -e "${BLUE}Updating Cargo.lock...${NC}"
cd "$PROJECT_ROOT"
if ! cargo update -w 2>&1 | grep -v "Updating\|Locking"; then
    # cargo update failed, but check if it's just because there's nothing to update
    if [ $? -ne 0 ] && [ $? -ne 141 ]; then
        echo -e "${RED}Warning: cargo update may have encountered issues${NC}"
    fi
fi

echo ""
echo -e "${GREEN}✓ Version updated successfully!${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Update CHANGELOG.md with changes for version $NEW_VERSION"
echo "2. Review the changes:"
echo "   ${BLUE}git diff Cargo.toml Cargo.lock${NC}"
echo "3. Commit the changes:"
echo "   ${BLUE}git add Cargo.toml Cargo.lock CHANGELOG.md${NC}"
echo "   ${BLUE}git commit -m \"chore: bump version to $NEW_VERSION\"${NC}"
echo "4. Create and push the tag:"
echo "   ${BLUE}git tag -a v$NEW_VERSION -m \"Release version $NEW_VERSION\"${NC}"
echo "   ${BLUE}git push origin main${NC}"
echo "   ${BLUE}git push origin v$NEW_VERSION${NC}"
echo ""
echo -e "${YELLOW}Note: The release workflow will automatically create a GitHub release and build artifacts.${NC}"
