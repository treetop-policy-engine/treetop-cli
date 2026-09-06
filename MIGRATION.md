# Breaking 0.1.0 migration

Upgrade CLI, the Rust SDK, and REST to the coordinated 0.1.0 contract. Early
releases prioritize correctness and one strict contract over compatibility.

The CLI uses the official SDK for every HTTP operation. Status now requires
schema metadata, request limits, and context capabilities. Batch size is always
reported. Incomplete old-server responses fail parsing instead of displaying
inferred defaults. Authorization versions include required `hash`, `loaded_at`,
`label_set` (nullable), and unsigned `generation`. Policy listings remain
non-authoritative candidates; use `check` to request an authorization decision.

## Declared labels and format 2

Bundle and REST configurations now use:

```json
{
  "target": {"resource_type": "App::Host", "attribute": "labels"},
  "field": "name",
  "patterns": [{"name": "prod", "regex": "^prod"}]
}
```

Replace `kind`/`output`, set module and bundle manifests to format 2, rebuild
archives, and re-sign them. Each exact resource-type/attribute tuple has one
owner. Different resource types can own the same attribute name. Sanitization
also follows scope, so constrain resource types before trusting derived labels.

Use `/livez`, `/readyz`, and `/openapi.json` in scripts that previously used the
removed health or OpenAPI aliases. CLI command names and matrix syntax still
use the documented command surface; the SDK provides the strict wire contract.

## Published dependencies and release order

CLI requires the exact Rust SDK 0.1.0 package from crates.io. Its lockfile uses the
registry release without candidate Git patches. CI exercises all commands against
the immutable REST 0.1.0 release image. Release Core, Bundle, REST, and the Rust SDK
in that order before releasing CLI 0.1.0.
