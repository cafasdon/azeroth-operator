# Saved capture coverage verification

The 57 requested section names agree across addon/Core.lua, the collector
definitions and the Rust snapshot registry. This is source coverage.
Native capture qualification remains deferred.

The locked offline build and three scoped integration tests passed.
Independent failure review passed eight real stdio MCP cases covering old
captures, observed empty sections, reported unavailability, filtered panel
scope, malformed captures and retail compatibility. No defects were found.

The advisory shipping judgement selected supported using Jev 1.14.0.
Request SHA-256
`476e9edd597181942086fa52eab6ab5d2f6ef30c9b01eb86dbd5521ab4b39ca3`.
Usage was 469 input and 45 output tokens in 0.302 seconds.
The author accepted the scoped offline result after independent review.

The subsequent comparison extension passed the scoped gossip integration
test and independent MCP probes for all nine status transitions, empty
observed sections, legacy captures, private reason exclusion, scope
appearance/disappearance and retail compatibility.
The advisory shipping result was supported using Jev 1.14.0, request
`946c8ee551b7f4334e7d8b43926cf0f6f41671d550d4434a692037a253e405ea`.
Usage was 491 input and 45 output tokens in 0.289 seconds.

Exact status discovery passed two scoped integration tests against the
rebuilt MCP. Independent failure review passed 24 stdio calls covering
all statuses, legacy empty captures, latest selection, time windows,
paging, other filters, malformed input and unavailable-reason exclusion.
The advisory shipping result was supported using Jev 1.14.0, request
`c77a50da812911bc886a5fa5850823c50f67154bb17f2c9e0f7b9e0433d8f753`.
Usage was 467 input and 45 output tokens in 0.349 seconds.
