#!/bin/bash
# Automatic changelog generation script
# Updates CHANGELOG.md based on git commit messages since last release

set -euo pipefail

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CHANGELOG_FILE="${CHANGELOG_FILE:-CHANGELOG.md}"
CONFIG_FILE="${CONFIG_FILE:-.changelog.toml}"
TEMP_FILE=$(mktemp)
TEMP_CHANGELOG=$(mktemp)

# Load configuration (simple TOML parsing)
get_config() {
    local key="$1"
    local default="$2"
    if [ -f "$CONFIG_FILE" ]; then
        grep -E "^${key}\s*=" "$CONFIG_FILE" | head -1 | sed -E "s/^${key}\s*=\s*[\"']?([^\"']+)[\"']?/\1/" || echo "$default"
    else
        echo "$default"
    fi
}

UNRELEASED_HEADER=$(get_config "unreleased_header" "## [Unreleased]")
DATE_FORMAT=$(get_config "date_format" "%Y-%m-%d")

# Get the last release tag or commit
get_last_release() {
    local last_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
    if [ -n "$last_tag" ]; then
        echo "$last_tag"
    else
        # If no tags, use the first commit
        git rev-list --max-parents=0 HEAD 2>/dev/null || echo ""
    fi
}

# Get commits since last release
get_commits() {
    local since="$1"
    if [ -z "$since" ]; then
        git log --pretty=format:"%H|%s|%b" HEAD
    else
        git log --pretty=format:"%H|%s|%b" "${since}..HEAD"
    fi
}

# Parse commit message and extract type, scope, and description
parse_commit() {
    local commit_msg="$1"
    local type=""
    local scope=""
    local description=""
    local category=""
    local is_breaking=false

    # Check for breaking change
    if echo "$commit_msg" | grep -q "BREAKING CHANGE:"; then
        is_breaking=true
    fi

    # Try to match conventional commit format: type(scope): description
    if echo "$commit_msg" | grep -qE "^[a-z]+(\([^)]+\))?:"; then
        type=$(echo "$commit_msg" | sed -E 's/^([a-z]+)(\([^)]+\))?:.*/\1/')
        scope=$(echo "$commit_msg" | sed -E 's/^[a-z]+(\(([^)]+)\))?:.*/\2/' | grep -v "^$" || echo "")
        description=$(echo "$commit_msg" | sed -E 's/^[a-z]+(\([^)]+\))?:\s*(.*)/\2/' | sed 's/^BREAKING CHANGE:.*//')
    else
        # Fallback: use first word as type
        type=$(echo "$commit_msg" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')
        description=$(echo "$commit_msg" | sed -E 's/^[A-Za-z]+\s+(.*)/\1/')
    fi

    # Map type to category
    case "$type" in
        feat|feature|add)
            category="Added"
            ;;
        change|changed|refactor|update|modify)
            category="Changed"
            ;;
        deprecate)
            category="Deprecated"
            ;;
        remove|removed|delete)
            category="Removed"
            ;;
        fix|bugfix|bug)
            category="Fixed"
            ;;
        security)
            category="Security"
            ;;
        *)
            # Default to "Changed" for unrecognized types
            category="Changed"
            ;;
    esac

    # Add breaking change indicator
    if [ "$is_breaking" = true ]; then
        description="**BREAKING:** $description"
    fi

    # Add scope if present
    if [ -n "$scope" ]; then
        description="($scope) $description"
    fi

    echo "$category|$description"
}

# Generate changelog entries
generate_entries() {
    local since="$1"
    local commits=$(get_commits "$since")
    
    if [ -z "$commits" ]; then
        echo "No new commits since last release."
        return
    fi

    # Use temporary files for each category
    local added_file=$(mktemp)
    local changed_file=$(mktemp)
    local deprecated_file=$(mktemp)
    local removed_file=$(mktemp)
    local fixed_file=$(mktemp)
    local security_file=$(mktemp)

    echo "$commits" | while IFS='|' read -r hash subject body; do
        # Skip merge commits and ignored patterns
        if echo "$subject" | grep -qE "^(Merge |Revert |chore|ci|test|docs\(changelog\))"; then
            continue
        fi

        # Combine subject and body
        local full_msg="$subject"
        if [ -n "$body" ]; then
            full_msg="$subject

$body"
        fi

        local parsed=$(parse_commit "$full_msg")
        local category=$(echo "$parsed" | cut -d'|' -f1)
        local description=$(echo "$parsed" | cut -d'|' -f2-)

        if [ -n "$description" ] && [ "$description" != "$category" ]; then
            case "$category" in
                Added)
                    echo "- $description" >> "$added_file"
                    ;;
                Changed)
                    echo "- $description" >> "$changed_file"
                    ;;
                Deprecated)
                    echo "- $description" >> "$deprecated_file"
                    ;;
                Removed)
                    echo "- $description" >> "$removed_file"
                    ;;
                Fixed)
                    echo "- $description" >> "$fixed_file"
                    ;;
                Security)
                    echo "- $description" >> "$security_file"
                    ;;
            esac
        fi
    done

    # Output entries by category
    if [ -s "$added_file" ]; then
        echo "### Added"
        cat "$added_file"
        echo ""
    fi
    if [ -s "$changed_file" ]; then
        echo "### Changed"
        cat "$changed_file"
        echo ""
    fi
    if [ -s "$deprecated_file" ]; then
        echo "### Deprecated"
        cat "$deprecated_file"
        echo ""
    fi
    if [ -s "$removed_file" ]; then
        echo "### Removed"
        cat "$removed_file"
        echo ""
    fi
    if [ -s "$fixed_file" ]; then
        echo "### Fixed"
        cat "$fixed_file"
        echo ""
    fi
    if [ -s "$security_file" ]; then
        echo "### Security"
        cat "$security_file"
        echo ""
    fi

    # Cleanup
    rm -f "$added_file" "$changed_file" "$deprecated_file" "$removed_file" "$fixed_file" "$security_file"
}

# Update changelog file
update_changelog() {
    local since="$1"
    
    if [ ! -f "$CHANGELOG_FILE" ]; then
        echo -e "${RED}Error: $CHANGELOG_FILE not found${NC}"
        exit 1
    fi

    # Generate new entries
    local new_entries=$(generate_entries "$since")
    
    if [ -z "$new_entries" ] || [ "$new_entries" = "No new commits since last release." ]; then
        echo -e "${YELLOW}No new entries to add to changelog${NC}"
        return
    fi

    # Find the Unreleased section
    local unreleased_line=$(grep -n "$UNRELEASED_HEADER" "$CHANGELOG_FILE" | head -1 | cut -d: -f1)
    
    if [ -z "$unreleased_line" ]; then
        echo -e "${RED}Error: Could not find '$UNRELEASED_HEADER' in $CHANGELOG_FILE${NC}"
        exit 1
    fi

    # Insert new entries after the Unreleased header
    {
        head -n "$unreleased_line" "$CHANGELOG_FILE"
        echo ""
        echo "$new_entries"
        tail -n +$((unreleased_line + 1)) "$CHANGELOG_FILE"
    } > "$TEMP_CHANGELOG"

    mv "$TEMP_CHANGELOG" "$CHANGELOG_FILE"
    echo -e "${GREEN}✓ Updated $CHANGELOG_FILE with new entries${NC}"
}

# Create a new release section
create_release() {
    local version="$1"
    local date=$(date +"$DATE_FORMAT")
    
    if [ -z "$version" ]; then
        echo -e "${RED}Error: Version required for release${NC}"
        echo "Usage: $0 release <version>"
        exit 1
    fi

    # Replace [Unreleased] with version
    sed -i.bak "s/## \[Unreleased\]/## \[$version\] - $date/" "$CHANGELOG_FILE"
    
    # Add new Unreleased section at the top
    {
        echo "$UNRELEASED_HEADER"
        echo ""
        echo ""
        cat "$CHANGELOG_FILE"
    } > "$TEMP_CHANGELOG"
    
    mv "$TEMP_CHANGELOG" "$CHANGELOG_FILE"
    rm -f "$CHANGELOG_FILE.bak"
    
    echo -e "${GREEN}✓ Created release section for $version${NC}"
}

# Main
main() {
    case "${1:-update}" in
        update)
            local since=$(get_last_release)
            echo -e "${BLUE}Updating changelog since: ${since:-beginning}${NC}"
            update_changelog "$since"
            ;;
        release)
            create_release "${2:-}"
            ;;
        *)
            echo "Usage: $0 [update|release <version>]"
            echo ""
            echo "Commands:"
            echo "  update          Update changelog with commits since last release"
            echo "  release <ver>   Create a new release section with version"
            exit 1
            ;;
    esac
}

main "$@"

