# Contributing to treetop-cli

## Verification

Run the focused test while iterating, then complete these gates before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo check --locked --benches
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
git ls-files -z '*.md' | xargs -0 npx --yes markdownlint-cli2@0.23.0 --config .markdownlint.json
```

Tests that exercise HTTP behavior must bind an ephemeral loopback server and must not require Docker
or a public service. Add command-level coverage for output, configuration precedence, failure exit
status, and secret redaction when changing those contracts.

CI separately exercises the built CLI against the published full-target treetop-rest images listed
in [COMPATIBILITY.md](COMPATIBILITY.md). Keep that job and the compatibility documentation aligned
when adding a supported server release; ordinary Rust tests must remain Docker-free.

## Architecture

Keep `src/main.rs` as a thin process boundary. Command parsing and execution belong in `src/app.rs`;
REPL, completion, configuration, matrix expansion, rendering, and paths remain focused modules in the
library. Use `treetop-client` for every server operation. Do not add a second HTTP client or duplicate
wire request/response models.

Upload-capable clients must be constructed only inside upload commands. Never log tokens or raw
unvalidated response bodies. JSON output must serialize client-returned types.

## Commits and pull requests

Sign commits, keep changes scoped, and include the behavior, compatibility impact, and validation in
the pull request description. Review `CHANGELOG.md` for every user-facing change.
