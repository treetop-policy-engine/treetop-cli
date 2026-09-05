# treetop-cli

`treetop-cli` is the standalone command-line client and interactive REPL for
[Treetop](https://github.com/treetop-policy-engine/treetop-rest) authorization servers. It uses the
official [`treetop-client`](https://github.com/treetop-policy-engine/treetop-client) crate for
every HTTP operation.

Version 0.0.1 is extracted from the CLI shipped with
[treetop-rest v0.0.10](https://github.com/treetop-policy-engine/treetop-rest/tree/v0.0.10/src/cli).
The repository
starts with fresh history; that tag is the canonical source provenance for the legacy implementation.

## Installation

Download the archive for your platform from the
[latest release](https://github.com/treetop-policy-engine/treetop-cli/releases/latest), verify the
adjacent
SHA-256 file, and place `treetop-cli` (or `treetop-cli.exe`) on your `PATH`.

Rolling builds from the latest green `main` are published as
[`main-latest`](https://github.com/treetop-policy-engine/treetop-cli/releases/tag/main-latest).
They report a
version such as `v0.0.2+main.g0123456789ab`; stable release builds report `v0.0.2`.

The CLI is distributed as GitHub release binaries only and is not published to crates.io.

## Quick start

The default server is `http://127.0.0.1:9999`:

```bash
treetop-cli status
treetop-cli check \
  --principal alice \
  --action view \
  --resource-type Document \
  --resource-id report-1
treetop-cli policies --raw
treetop-cli schema --raw
treetop-cli metrics
```

Use a complete base URL when the server is remote or uses HTTPS:

```bash
treetop-cli --server-url https://treetop.example.org status
```

Legacy `--host` and `--port` flags and their environment variables remain supported. See
[CLI configuration](docs/config.md) for the exact precedence rules and persistent configuration.

## Commands

| Command | Purpose |
| --- | --- |
| `status` | Show client, server, policy, schema, parallelism, and request-limit status. |
| `version` | Preserve the legacy status/version display command. |
| `check` | Evaluate one request or a matrix-expanded authorization batch. |
| `policies` | Download all policies or list policies applying to one user. |
| `schema` | Download the current Cedar schema. |
| `upload` | Upload policies or a schema using an upload-capable client. |
| `metrics` | Print validated UTF-8 Prometheus exposition text. |
| `repl` | Start the interactive shell with history, completion, and last-used state. |

Use `treetop-cli help <command>` for the complete flag reference.

### Authorization checks

```bash
treetop-cli check \
  --principal 'DNS::User::alice[admins]' \
  --action DNS::Action::create_host \
  --resource-type Host \
  --resource-id host.example.org \
  --resource-attribute ip=192.0.2.10 \
  --context-attribute environment=production \
  --detailed
```

The `--context-file` option accepts a JSON object. Strings, booleans, signed integers, and arrays
can be written directly. Object values must use typed Cedar JSON such as
`{"type":"Ip","value":"192.0.2.10"}`.

Matrix expansion with pipes and bracketed group alternatives is documented in
[docs/matrix.md](docs/matrix.md).

### Policy and schema uploads

Upload clients are created only inside the `upload` command with a validated `UploadToken`, so the
official client's `CanUpload` capability controls access to upload methods:

```bash
treetop-cli upload \
  --file policies.cedar \
  --raw \
  --token "$TREETOP_UPLOAD_TOKEN"
```

Tokens are never written to JSON or debug output, and reflected tokens are redacted from client
errors. HTTPS is required for non-loopback uploads. For an explicitly accepted development server,
add `--danger-allow-insecure-uploads` (or set
`TREETOP_CLI_DANGER_ALLOW_INSECURE_UPLOADS=true`).

### JSON and debug output

`--json` serializes the validated response type returned by `treetop-client`. `--debug` includes
typed requests and validated results or errors and implies JSON output. It never prints upload
tokens or unvalidated raw response bodies.

### Interactive REPL

```bash
treetop-cli repl
```

The REPL preserves the last principal, action, resource type, resource ID, and attributes used by
`check`. It retains the platform-standard config and history locations used by the bundled CLI.
Use `show` to inspect active settings and paths, and `history` to list command history.

## Compatibility

The unreleased CLI uses exactly `treetop-client = "=0.0.4"` and is tested against treetop-rest
v0.0.10 through v0.0.12, and v0.0.16. Version 0.0.2 remains paired with client 0.0.3;
version 0.0.1 remains paired with client 0.0.2. See
[COMPATIBILITY.md](COMPATIBILITY.md) for the tested server matrix.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [RELEASING.md](RELEASING.md) for the complete gates.

## License

MIT
