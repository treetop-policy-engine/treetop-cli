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
- Use exactly the `treetop-client` version in `Cargo.toml` for all HTTP operations.
  Coordinated changes may pin an exact unmerged SDK revision for verification;
  switch to its registry release after approval and before publication.
- Prefer correctness and strict, uniform project contracts over compatibility.
  Remove obsolete defaults and deprecated APIs and document breaking migrations.
  Do not merge or release coordinated changes before user approval.
- Do not depend on `treetop-rest` or `treetop-core` and do not duplicate their wire models.
- JSON and debug output operate on typed requests, validated responses, and redacted client errors.
- Construct `Client<CanUpload>` only within upload commands and never log upload tokens.
- Preserve CLI flags, environment variables, output modes, config/history paths, REPL state, and
  matrix syntax unless a change explicitly documents a migration.

## Releases

- The package is GitHub-binary-only and must retain `publish = false`.
- Before preparing a release commit, update all Rust dependencies and GitHub Actions to their
  latest stable versions, and pin every Action to its full commit SHA. Refresh `Cargo.lock`, review
  upstream release notes for compatibility and MSRV changes, and complete the repository's full
  verification and security checks on the resulting dependency set.
- Land dependency and GitHub Actions updates before the version-bump release commit so the signed
  release tag points at a green commit that already contains every update.
- Sign commits and annotated tags. Tags must point to the exact green `main` release commit.
- Keep archive names and checksum files aligned with `RELEASING.md`.
