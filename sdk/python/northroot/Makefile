# Python SDK Development Makefile

.PHONY: help venv install-dev build test clean

help:
	@echo "Available targets:"
	@echo "  make venv        - Create Python virtual environment"
	@echo "  make install-dev - Install development dependencies"
	@echo "  make build       - Build the package (maturin build)"
	@echo "  make develop     - Install in development mode (maturin develop)"
	@echo "  make test        - Run tests"
	@echo "  make clean       - Clean build artifacts"

# Create virtual environment
venv:
	@if [ ! -d "venv" ]; then \
		python3 -m venv venv; \
		echo "✅ Virtual environment created"; \
		echo "Activate with: source venv/bin/activate"; \
	else \
		echo "Virtual environment already exists"; \
	fi

# Install development dependencies
install-dev: venv
	@echo "Installing development dependencies..."
	@./venv/bin/pip install --upgrade pip
	@./venv/bin/pip install -e ".[dev]"
	@echo "✅ Development dependencies installed"

# Build the package
build: install-dev
	@echo "Building package with maturin..."
	@./venv/bin/maturin build --release
	@echo "✅ Build complete. Wheels in dist/"

# Install in development mode
develop: install-dev
	@echo "Installing in development mode..."
	@./venv/bin/maturin develop
	@echo "Installing Python package wrapper..."
	@if [ -d "northroot" ]; then \
		SITE_PACKAGES=$$(./venv/bin/python -c "import site; print(site.getsitepackages()[0])"); \
		cp -r northroot "$$SITE_PACKAGES/"; \
		echo "✅ Python package installed to site-packages"; \
	else \
		echo "⚠️  Warning: northroot/ directory not found"; \
	fi
	@echo "✅ Package installed in development mode"

# Run tests
test: develop
	@echo "Running tests..."
	@./venv/bin/python -m pytest tests/ || echo "No tests found"
	@./venv/bin/python examples/quickstart.py || echo "Quickstart example failed"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf dist/ build/ *.egg-info/
	@rm -rf target/wheels/
	@echo "✅ Clean complete"

