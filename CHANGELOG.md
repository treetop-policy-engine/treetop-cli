# Changelog

All notable changes to this project are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- Update locked h2 to 0.4.19 and chacha20 to 0.10.2. Use tabled 0.22 to remove
  the unmaintained proc-macro-error2 dependency and its future compatibility warning.

### Changed

- Verify the command surface against REST v0.0.16 while retaining the existing
  v0.0.10 through v0.0.12 compatibility checks.
- Correct the advertised minimum Rust version to 1.89, matching the file-lock APIs
  already used by Rustyline 18. CI now verifies that minimum explicitly.

- Preserve label configuration and generation metadata from the updated Rust client in JSON output.
  Human authorization output now shows the engine generation and available label identifier alongside
  the policy hash. Generation is local to one engine instance and can restart on replacement.

## [0.0.2] - 2026-08-15

### Changed

- Move the canonical source repository and release downloads to the `treetop-policy-engine`
  organization, and use organization-owned server images in compatibility checks.
- Update to `treetop-client` 0.0.3, verify the CLI command surface against treetop-rest v0.0.10
  through v0.0.12, and extend the documented stable client contract through v0.0.12.
- Refresh Rust dependencies to the latest releases compatible with Rust 1.88 and pin GitHub
  Actions to their latest stable revisions.

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

[Unreleased]: https://github.com/treetop-policy-engine/treetop-cli/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/treetop-policy-engine/treetop-cli/releases/tag/v0.0.2
[0.0.1]: https://github.com/treetop-policy-engine/treetop-cli/releases/tag/v0.0.1
