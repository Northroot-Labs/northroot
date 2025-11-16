# PyPI Release Checklist

**Version:** 0.1.1  
**Date:** 2025-11-16  
**Status:** Ready for release (cross-platform support added)

## Pre-Release Checklist

### ✅ Metadata (pyproject.toml)

- [x] Package name: `northroot`
- [x] Version: `0.1.1`
- [x] Description: Updated with clear value proposition
- [x] Keywords: Added relevant keywords
- [x] Classifiers: Updated to match Python 3.10+ requirement
- [x] License: Apache-2.0
- [x] README: Updated with PyPI installation instructions
- [x] Authors: Northroot Contributors
- [x] URLs: Homepage, Documentation, Repository

### ✅ Build Configuration

- [x] Maturin build system configured
- [x] Storage feature enabled by default
- [x] Module name: `_northroot` (internal)
- [x] Python source: `.`

### ⏳ Testing (To Do)

- [ ] Install maturin: `pip install maturin`
- [ ] Test local build: `maturin build`
- [ ] Test wheel installation: `pip install dist/*.whl`
- [ ] Test in clean virtual environment
- [ ] Run quickstart example: `python examples/quickstart.py`
- [ ] Verify all imports work: `from northroot import Client`
- [ ] Test storage functionality
- [ ] Test listing functionality

### ✅ PyPI Account Setup

- [x] PyPI account created
- [x] TestPyPI account created (for testing)
- [ ] Generate API token
- [ ] Add secrets to GitHub repository:
  - `PYPI_API_TOKEN` - PyPI API token
  - `TESTPYPI_API_TOKEN` - TestPyPI API token (optional)

### ⏳ Release Process (To Do)

#### Option 1: GitHub Actions (Recommended)

1. **Create a GitHub release:**
   - Go to: https://github.com/Northroot-Labs/Northroot/releases/new
   - Tag: `v0.1.0` (must match version in `pyproject.toml`)
   - Title: `v0.1.0`
   - Description: Release notes
   - Click "Publish release"

2. **Workflow will automatically:**
   - Build the package
   - Verify the wheel
   - Publish to PyPI
   - Upload artifacts

3. **For TestPyPI testing:**
   - Use "Run workflow" → Select "Publish to TestPyPI"
   - Or manually trigger: Actions → "Publish to PyPI" → Run workflow → Check "testpypi"

#### Option 2: Manual Publishing

1. **Test on TestPyPI first:**
   ```bash
   cd sdk/python/northroot
   maturin publish --repository testpypi
   ```

2. **Test installation from TestPyPI:**
   ```bash
   pip install -i https://test.pypi.org/simple/ northroot
   ```

3. **Publish to PyPI:**
   ```bash
   cd sdk/python/northroot
   maturin publish
   ```

4. **Verify on PyPI:**
   - Check package page: https://pypi.org/project/northroot/
   - Verify metadata is correct
   - Test installation: `pip install northroot`

### ⏳ Post-Release (To Do)

- [ ] Update README with actual PyPI link
- [ ] Create GitHub release tag: `v0.1.0`
- [ ] Update CHANGELOG.md
- [ ] Announce release (if applicable)

## Local Testing (Recommended First Step)

Before using GitHub Actions, test locally:

```bash
cd sdk/python/northroot
./test-publish.sh
```

This script will:
- Build the package
- Verify artifacts are created
- Test wheel installation
- Check package metadata
- Validate with twine (without uploading)

## Build Commands

### Development Build
```bash
cd sdk/python/northroot
maturin develop
```

### Production Build
```bash
cd sdk/python/northroot
maturin build --release --out dist
```

### Test Installation
```bash
pip install dist/northroot-0.1.0-*.whl
```

### Manual Publish to TestPyPI (using twine)
```bash
cd sdk/python/northroot
export TWINE_USERNAME=__token__
export TWINE_PASSWORD='your-testpypi-token'  # No quotes needed, just the token
twine upload --repository testpypi dist/*
```

### Manual Publish to PyPI (using twine)
```bash
cd sdk/python/northroot
export TWINE_USERNAME=__token__
export TWINE_PASSWORD='your-pypi-token'
twine upload dist/*
```

## GitHub Actions Setup

### Required Secrets

Add these secrets to your GitHub repository (Settings → Secrets and variables → Actions):

1. **PYPI_API_TOKEN** (required)
   - Generate at: https://pypi.org/manage/account/token/
   - Scope: Entire account (or project-specific)
   - Add as repository secret: `PYPI_API_TOKEN`

2. **TESTPYPI_API_TOKEN** (optional, for testing)
   - Generate at: https://test.pypi.org/manage/account/token/
   - Add as repository secret: `TESTPYPI_API_TOKEN`

### Optional: Environment Protection

To add environment protection (requires approval before publishing):

1. Go to: Settings → Environments → New environment
2. Name: `pypi`
3. Add protection rules (reviewers, wait timer, etc.)
4. Uncomment the `environment:` section in `.github/workflows/pypi-publish.yml`

## Notes

- Package name: `northroot`
- Internal module: `_northroot` (wrapped by `northroot/__init__.py`)
- Minimum Python: 3.10
- License: Apache-2.0
- Features: `storage` enabled by default
- GitHub Actions workflow: `.github/workflows/pypi-publish.yml`

## Known Issues

None at this time.

## Next Steps

1. Install maturin and test local build
2. Test in clean environment
3. Publish to TestPyPI for validation
4. Publish to PyPI

