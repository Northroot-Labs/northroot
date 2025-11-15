# PyPI Release Checklist

**Version:** 0.1.0  
**Date:** 2025-11-15  
**Status:** Pre-release

## Pre-Release Checklist

### ✅ Metadata (pyproject.toml)

- [x] Package name: `northroot`
- [x] Version: `0.1.0`
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

### ⏳ PyPI Account Setup (To Do)

- [ ] Create PyPI account (if needed)
- [ ] Create TestPyPI account (for testing)
- [ ] Generate API token
- [ ] Configure credentials: `~/.pypirc` or environment variables

### ⏳ Release Process (To Do)

1. **Test on TestPyPI first:**
   ```bash
   maturin publish --repository testpypi
   ```

2. **Test installation from TestPyPI:**
   ```bash
   pip install -i https://test.pypi.org/simple/ northroot
   ```

3. **Publish to PyPI:**
   ```bash
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

## Build Commands

### Development Build
```bash
cd sdk/python/northroot
maturin develop
```

### Production Build
```bash
cd sdk/python/northroot
maturin build --release
```

### Test Installation
```bash
pip install dist/northroot-0.1.0-*.whl
```

### Publish to TestPyPI
```bash
maturin publish --repository testpypi
```

### Publish to PyPI
```bash
maturin publish
```

## Notes

- Package name: `northroot`
- Internal module: `_northroot` (wrapped by `northroot/__init__.py`)
- Minimum Python: 3.10
- License: Apache-2.0
- Features: `storage` enabled by default

## Known Issues

None at this time.

## Next Steps

1. Install maturin and test local build
2. Test in clean environment
3. Publish to TestPyPI for validation
4. Publish to PyPI

