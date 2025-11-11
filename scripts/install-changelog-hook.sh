#!/bin/bash
# Install git hook for automatic changelog updates (optional)

set -euo pipefail

HOOK_FILE=".git/hooks/post-commit"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPDATE_SCRIPT="$SCRIPT_DIR/update-changelog.sh"

if [ ! -d ".git" ]; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Create post-commit hook
cat > "$HOOK_FILE" << 'EOF'
#!/bin/bash
# Post-commit hook: Update changelog automatically

# Only update if changelog script exists
if [ -f "scripts/update-changelog.sh" ]; then
    # Run in background to not block commits
    bash scripts/update-changelog.sh update > /dev/null 2>&1 &
fi
EOF

chmod +x "$HOOK_FILE"
echo "✓ Installed post-commit hook for automatic changelog updates"
echo ""
echo "Note: The hook runs in the background and won't block commits."
echo "To disable, remove .git/hooks/post-commit"

