#!/bin/bash
# Install ADR maintenance hooks

set -euo pipefail

HOOKS_DIR=".git/hooks"

if [ ! -d "$HOOKS_DIR" ]; then
    echo "Error: .git/hooks directory not found. Are you in a git repository?"
    exit 1
fi

# Pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook for ADR maintenance

# Check if any ADR files are staged
if git diff --cached --name-only --quiet -- "docs/adr/**" "schemas/adr/**" "tools/adr/**"; then
    exit 0
fi

# Run ADR validation
make adr.index && make adr.validate
EOF

chmod +x "$HOOKS_DIR/pre-commit"
echo "✓ Installed pre-commit hook"

# Pre-push hook (optional, for extra safety)
cat > "$HOOKS_DIR/pre-push" << 'EOF'
#!/bin/bash
# Pre-push hook for ADR maintenance

# Re-validate ADRs before push
make adr.index
make adr.validate
EOF

chmod +x "$HOOKS_DIR/pre-push"
echo "✓ Installed pre-push hook"

echo ""
echo "ADR hooks installed successfully!"
echo "The hooks will automatically validate ADRs on commit/push."

