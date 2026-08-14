# Changelog

All notable changes to this project are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Move the canonical source repository and release downloads to the `treetop-policy-engine`
  organization, and use organization-owned server images in compatibility checks.
- Verify the CLI command surface against treetop-rest v0.0.10 and the server-only v0.0.11 release,
  and extend the documented stable client contract through v0.0.11.

## [0.0.1] - 2026-08-13

### Added

- Extract the CLI and REPL shipped by treetop-rest v0.0.10 into a fresh-history MIT repository.
- Preserve commands, output modes, exit behavior, matrix syntax, REPL state, configuration keys,
  environment variables, and platform-standard history/config paths.
- Use exactly `treetop-client = "=0.0.2"` for typed requests, validated responses, URL handling,
  bounded response bodies, redirect protection, and token-redacted errors.
- Add `--server-url`, `TREETOP_CLI_SERVER_URL`, and `server_url` configuration with documented
  precedence while retaining legacy host and port settings.
- Create upload-capable clients only within upload commands through `UploadToken` and `CanUpload`.
- Add stable/beta/nightly CI, Markdown lint, rolling `main-latest` binaries, and signed immutable
  release artifacts for static Linux x86_64/ARM64, Apple Silicon macOS, and Windows x86_64.

[Unreleased]: https://github.com/treetop-policy-engine/treetop-cli/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/treetop-policy-engine/treetop-cli/releases/tag/v0.0.1
