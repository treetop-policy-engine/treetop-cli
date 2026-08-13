# Compatibility

| CLI | Client | Full server target | Stable server contract |
| --- | --- | --- | --- |
| 0.0.1 | exactly 0.0.2 | treetop-rest 0.0.10 and 0.0.11 | treetop-rest 0.0.4 through 0.0.11 |

The stable contract covers health, version, policy download, and authorization. The full target also
exercises status metadata, request context, schemas, uploads, metrics, liveness/readiness probes, and
the generated OpenAPI document through the official client.

CLI CI exercises the command surface against both full server targets. The official client's CI
verifies the stable contract across every server release in the documented range.

The CLI was extracted from the implementation bundled in
[treetop-rest v0.0.10](https://github.com/terjekv/treetop-rest/tree/v0.0.10/src/cli). The standalone
repository does not depend on `treetop-rest` or `treetop-core`.
