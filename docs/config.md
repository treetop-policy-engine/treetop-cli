# CLI configuration

The CLI accepts a complete server URL and retains the bundled CLI's host/port settings, output
flags, config paths, and environment variables.

## Server resolution

The server base URL is selected in this exact order:

1. Explicit `--server-url` command-line value.
2. `TREETOP_CLI_SERVER_URL`.
3. Explicit command-line or environment host or port.
4. Config-file `server_url`.
5. Config-file `host` and `port`.
6. `http://127.0.0.1:9999`.

When either the legacy host or port is explicit, the other component comes from the config file if
present and otherwise uses its built-in default. Host/port settings construct an `http://` URL; use
`server_url` for HTTPS or a path-prefixed service.

## Config file location

The existing platform-standard locations are unchanged:

- Linux: `~/.config/treetop-cli/config.toml`
- macOS: `~/Library/Application Support/treetop-cli/config.toml`
- Windows: `%APPDATA%/treetop-cli/config.toml`

The history file remains in the platform data directory under `treetop-cli/history`.

## Config file format

```toml
server_url = "https://treetop.example.org"
json = false
debug = false
timing = false
table_style = "unicode"
```

For legacy host/port configuration:

```toml
host = "127.0.0.1"
port = 9999
```

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `server_url` | string | unset | Complete HTTP(S) server base URL. |
| `host` | string | `127.0.0.1` | Legacy server host. |
| `port` | integer | `9999` | Legacy server port. |
| `json` | boolean | `false` | Serialize validated results as pretty JSON. |
| `debug` | boolean | `false` | Show typed diagnostics and imply JSON output. |
| `timing` | boolean | `false` | Print command execution time. |
| `table_style` | string | `rounded` | `rounded`, `ascii`, `unicode`, or `markdown`. |

## Environment variables

| Variable | Corresponding option |
| --- | --- |
| `TREETOP_CLI_SERVER_URL` | `--server-url` |
| `TREETOP_CLI_SERVER_ADDRESS` | `--host` |
| `TREETOP_CLI_SERVER_PORT` | `--port` |
| `TREETOP_CLI_JSON` | `--json` |
| `TREETOP_CLI_DEBUG` | `--debug` |
| `TREETOP_CLI_TIMING` | `--timing` |
| `TREETOP_CLI_TABLE_STYLE` | `--table-style` |
| `TREETOP_CLI_DANGER_ALLOW_INSECURE_UPLOADS` | Upload-only insecure HTTP opt-in. |

Upload tokens intentionally have no config-file key. Supply `--token` to the upload command through
a secret-aware launcher or shell expansion. Debug output never prints the token value.
