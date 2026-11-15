# Managed world generation acceptance — 15 November 2026

The world builder now consumes the managed incremental source cache directly.
Previously it needed a separate source directory. The new adapter preserves
that original interface and connects the installed bounded source cache to
the existing offline generation and MCP reader path.

`ReferenceCache.snapshot` validates the exact provider, immutable revision and
complete file hash map under one OS-held cache lock. `verify_source` keeps that
lock while it copies each validated input into the existing temporary build
directory. Source inputs retain the 64 MiB aggregate bound. The existing exporter
and generated-output checksum remain unchanged. A cache update cannot replace
objects during staging. No downloader is called by generation or a query.

## Real-data acceptance

The actual CLI built a new output from
`local-data/reference-cache/world`, using all 148 pinned source files. The
source cache's status before and after matched. The output was 20,278,256 bytes
with SHA-256:

```text
f535e174c7ce519442d91d9888bc81e5e5bc47dac436f72297c7664741cc15e6
```

The build, source checks and full reader load took 14.761 seconds on this host.
This single acceptance run is not a comparative benchmark. The validated reader
contained 23,858 items, 15,559 NPCs, 7,258 objects and 5,682 quests. These are
provider record counts, not proof of complete or current game coverage.

An actual MCP SDK client launched from an independent working directory read
item 2318 as Light Leather from the newly generated output. The response kept
`live: false` and `game_execution: false`. The extra acceptance output was then
removed; the installed compiled cache and private captures stayed unchanged.
The small acceptance report remains in ignored local data.

Thirty-four focused tests passed, including seven adapter tests and the original
source-directory build. One independent review round covered callers/failures
and security/data with two Astra/high reviewers. Both ran the adapter tests and
separate probes. Wrong revisions, invalid manifests, missing/corrupt objects,
busy locks and source byte overruns stopped before publication. Existing output
bytes survived rejection. No medium, high or low finding remained. Infrastructure
drift review was unnecessary because no service configuration changed.

## Scope

The managed cache's quota does not include the temporary source copy or generated
output. No automatic lock update, patch choice, reader promotion or private
inventory operation was added. Future reference updates still need an explicit
reviewed source lock and expected compiled hash.

The planning advisory used Jev `jev-1.14.0`, request hash
`829b0c5e7842552ce75b6e4e19efe98bc7585d6eff49b7dde4602fab8597aba9`,
522 input and 47 output tokens, 0.301 seconds. It selected proceed; the advisory
was not test evidence or authority to publish.

The release advisory used the same Jev version, request hash
`0a4979dcb551d9818fed3a7be3181e1096e6845ea7ca929f2c0c9ead07935606`,
603 input and 44 output tokens, 0.293 seconds. It selected ship. The main session
accepted the deterministic checks and independent review as the release basis.
