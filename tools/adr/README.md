# ADR Tools

Tools for managing Architectural Decision Records (ADRs).

## Tools

- **`generate_index.py`** - Generate/update machine-readable ADR index (version 0.4)
- **`generate_graph.py`** - Generate graph files (nodes.jsonl, edges.jsonl) from index
- **`linker.py`** - Link commits to files and symbols, populate code references
- **`validate.py`** - Validate ADR structure, schemas, and graph consistency
- **`migrate.py`** - Migrate existing ADRs to new format

## Usage

### Via Makefile (Recommended)

```bash
make adr.index      # Generate/update index
make adr.graph      # Generate graph files
make adr.link       # Link commits to code
make adr.all        # Run full pipeline (index → graph → link)
make adr.validate   # Validate structure and graph
make adr.migrate    # Migrate existing ADRs
make adr.watch      # Watch for changes (requires watchexec)
```

### Direct Usage

```bash
# Generate index
python3 tools/adr/generate_index.py --root docs/adr --write

# Generate graph
python3 tools/adr/generate_graph.py --root docs/adr

# Link commits to code
python3 tools/adr/linker.py --root docs/adr
python3 tools/adr/linker.py --root docs/adr --no-symbols  # Skip symbol extraction

# Validate
python3 tools/adr/validate.py --root docs/adr --strict

# Migrate
python3 tools/adr/migrate.py --root docs/adr
```

## Linker Tool

The `linker.py` tool maps implementation commits to files and symbols:

1. **Parses phases**: Reads all phase YAML files for accepted/implemented phases
2. **Reads git metadata**: Extracts commit messages, timestamps, and file changes
3. **Maps commits → files → symbols**: 
   - Uses `git diff-tree` to get files changed per commit
   - Optionally uses `ctags` for symbol extraction (can be disabled with `--no-symbols`)
4. **Updates graph files**: Adds COMMIT, FILE, SYMBOL nodes and TOUCHES_FILE, TOUCHES_SYMBOL edges
5. **Populates code_refs**: Updates `code_refs` in the index with discovered references

### Dependencies

- **git**: Required for commit/file mapping
- **ctags**: Optional, for symbol extraction (falls back to file-level mapping if unavailable)

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

