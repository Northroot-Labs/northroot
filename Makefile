# Northroot Makefile
# Provides deterministic targets for common tasks

ADR_DIR=docs/adr
TOOLS=tools/adr
PY?=python3

.PHONY: help adr.index adr.graph adr.link adr.all adr.validate adr.migrate adr.watch

help:
	@echo "Available targets:"
	@echo "  adr.index    - Generate/update ADR index"
	@echo "  adr.graph    - Generate graph files (nodes.jsonl, edges.jsonl)"
	@echo "  adr.link     - Link commits to files and symbols (updates graph)"
	@echo "  adr.all      - Run full pipeline: index → graph → link"
	@echo "  adr.validate - Validate ADR structure and schemas"
	@echo "  adr.migrate  - Migrate existing ADRs to latest format"
	@echo "  adr.watch    - Watch ADR files and auto-update index (requires watchexec)"

adr.index:
	@$(PY) $(TOOLS)/generate_index.py --root $(ADR_DIR) --write

adr.graph:
	@$(PY) $(TOOLS)/generate_graph.py --root $(ADR_DIR)

adr.link:
	@$(PY) $(TOOLS)/linker.py --root $(ADR_DIR)

adr.all: adr.index adr.graph adr.link
	@echo "✓ Full ADR pipeline complete"

adr.validate:
	@$(PY) $(TOOLS)/validate.py --root $(ADR_DIR) --strict

adr.migrate:
	@$(PY) $(TOOLS)/migrate.py --root $(ADR_DIR)

adr.watch:
	@if command -v watchexec > /dev/null; then \
		watchexec -r -e md,yaml,yml 'make adr.index && make adr.validate'; \
	else \
		echo "watchexec not found. Install with: brew install watchexec"; \
		exit 1; \
	fi

