# ADR Tools

Tools for managing Architectural Decision Records (ADRs).

## Tools

- **`generate_index.py`** - Generate/update machine-readable ADR index
- **`validate.py`** - Validate ADR structure and schemas
- **`migrate.py`** - Migrate existing ADRs to new format

## Usage

### Via Makefile (Recommended)

```bash
make adr.index      # Generate/update index
make adr.validate   # Validate structure
make adr.migrate    # Migrate existing ADRs
make adr.watch      # Watch for changes (requires watchexec)
```

### Direct Usage

```bash
# Generate index
python3 tools/adr/generate_index.py --root docs/adr --write

# Validate
python3 tools/adr/validate.py --root docs/adr --strict

# Migrate
python3 tools/adr/migrate.py --root docs/adr
```

## Dependencies

Install with:

```bash
pip install -r tools/adr/requirements.txt
```

## Integration

- **Pre-commit**: Automatically validates on ADR file changes
- **CI/CD**: GitHub Actions workflow validates on PR/push
- **Cursor**: Auto-enforcement rule triggers on ADR edits

See `.cursor/rules/adr-maintenance.mdc` for agent guidance.

