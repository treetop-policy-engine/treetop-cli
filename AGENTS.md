# Repository Guidelines

## Verification

- Format Rust with `cargo fmt --all -- --check`.
- Run `cargo clippy --locked --all-targets -- -D warnings`.
- Run `cargo test --locked` and `cargo check --locked --benches`.
- Build docs with `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps`.
- Lint tracked Markdown with the command in `CONTRIBUTING.md`.
- Use ephemeral loopback HTTP servers; tests must not require Docker or external services.

## Boundaries

- Keep `src/main.rs` thin and application behavior in the library.
- Use exactly the published `treetop-client` version in `Cargo.toml` for all HTTP operations.
- Do not depend on `treetop-rest` or `treetop-core` and do not duplicate their wire models.
- JSON and debug output operate on typed requests, validated responses, and redacted client errors.
- Construct `Client<CanUpload>` only within upload commands and never log upload tokens.
- Preserve CLI flags, environment variables, output modes, config/history paths, REPL state, and
  matrix syntax unless a change explicitly documents a migration.

## Releases

- The package is GitHub-binary-only and must retain `publish = false`.
- Sign commits and annotated tags. Tags must point to the exact green `main` release commit.
- Keep archive names and checksum files aligned with `RELEASING.md`.
