# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-11-15

### Added
- Initial release of Northroot Python SDK (v0.1.0)
- Core receipt lifecycle: record, store, retrieve, verify, list
- Filesystem-based receipt storage with querying and filtering
- Async and sync API support
- OpenTelemetry integration for span-to-receipt conversion
- JSON deserialization with base64url byte array support

### Fixed
- JSON deserialization errors for `[u8; 32]` byte arrays in ExecutionPayload
- Receipt loading from filesystem storage
- PyPI publishing workflow authentication issues

### Changed
- Enhanced PyPI publish workflow to use twine for reliable authentication
- Updated project description to be more developer-friendly
- Improved error handling and token validation in CI/CD workflows

---

## Changelog Format

Entries are categorized as:
- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security-related changes

