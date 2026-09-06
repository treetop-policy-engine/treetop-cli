# Releasing treetop-cli

Stable releases are GitHub-binary-only and use signed annotated `vMAJOR.MINOR.PATCH` tags. Never
publish this package to crates.io.

## Release gates

1. Confirm the exact `treetop-client` version is published and the lockfile contains it.
2. Update `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and `MIGRATION.md` together.
3. Run the complete checks from [CONTRIBUTING.md](CONTRIBUTING.md).
4. Exercise all commands against the full target server and run the current contract suite.
5. Merge a signed commit to a green `main` with no unresolved release-related pull requests.
6. Create and push a signed annotated tag pointing to that exact `main` commit:

   ```bash
   git tag -s vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   ```

The release workflow rejects a version/tag mismatch and any tag whose commit has not already passed
the `main` workflow. Treat published tags and artifacts as immutable.

## Artifacts

Artifact names use `treetop-cli-<platform>-<version>` and include a SHA-256 file beside every
archive:

- `treetop-cli-linux-x86_64-musl-vX.Y.Z.tar.gz`
- `treetop-cli-linux-aarch64-musl-vX.Y.Z.tar.gz`
- `treetop-cli-macos-aarch64-vX.Y.Z.tar.gz`
- `treetop-cli-windows-x86_64-vX.Y.Z.zip`

The workflow verifies Linux archives have no dynamic runtime dependencies with `readelf`, rejects
package-manager crypto dependencies on macOS, uses the Windows static CRT, and starts every binary
with `--version` before publishing.

Successful `main` builds update the mutable `main-latest` release and use build metadata such as
`v0.0.2+main.g0123456789ab`. Stable tag builds report the plain tagged version.
