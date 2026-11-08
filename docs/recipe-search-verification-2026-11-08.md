# Local recipe search — 8 November 2026

Baseline `0daf28b54eefc70a83c5872d245a82384320bbc7`.
Source/offline verification only; no live endpoint, native client, install or
deployment checks. The broad MCP coverage goal remains incomplete.

## Contract and authority

The new `search_forever_recipes` route reads one explicitly selected owner-local
Forever snapshot using the existing `Store.read` validation path. It searches
only `profession_recipe_materials`: no catalogue, network, implicit character
selection, refresh, write, generated game import or gameplay action.

Optional positive u32 recipe/output/reagent IDs and the four captured statuses
combine with AND on the same row. Output and reagent filters match only observed
fields; missing fields never supply negative or zero facts. Status retains its
existing meaning: fixed item materials, unavailable, unsupported required slots,
or budget exhausted. A raw fixed row is not a resolved learning, consistency or
craftability claim; the planner performs its own observation-conflict checks.

Numeric recipe-ID order precedes pagination. Default page size is 20, accepted
range 1–50. `matched_row_count` is counted before paging; `next_offset` is null
after the final page or beyond the end. The search is bounded by the existing
4,096-row snapshot schema and clones only the requested page.

The reply includes the original snapshot ID, observed coverage/profession ID,
captured spell count, matched row count, filters and captured rows with numeric
`recipe_id`. Missing section: `section_observed: false`, nullable counts and
empty records. Observed-empty section: true and zero counts. Both retain
`complete_recipe_knowledge_verified: false`, `craftability_verified: false`,
`live: false` and `game_execution: false`.

The DTO rejects unknown fields, invalid selectors and statuses. Runtime limits
bound pagination. Store failures and invalid requests use a fixed safe error
without snapshot contents or filesystem paths. Route annotations, the offline
policy registry, standalone gateway profile and smoke tool route expectations
include the reviewed read-only route. The gateway mount remains read-only.

## Author evidence

The author rebuilt the stdio executable from current source with the locked
offline build. With `PYTHONPATH=.;tests`, direct command
`python -m unittest test_recipe_search test_bulk_recipe_planning -q`
passed two tests in 2.423 seconds.

The search integration uses actual stdio MCP requests and 131 synthetic rows.
It checks all four statuses through multiple pages, numeric rather than lexical
ordering, exact same-row output/reagent intersection, conflicting filters,
metadata counts, missing versus empty, offsets past the end, malformed inputs,
non-Forever captures and missing/traversal snapshot IDs. An actual search result
is passed into target-output planning. Unrelated selected-schematic private text
is absent from search results/errors. The synthetic owner store remains
byte-for-byte unchanged. Gateway rendering includes the new route and keeps its
snapshot volume read-only.

The real `tools.smoke_mcp` module entry point ran against the built executable
with an isolated empty temporary store. It asserted 11 read-only routes, all
three families, eight invalid-snapshot rejections, traversal denial and unknown
mutation-tool rejection. It exited successfully. Existing route expectations
were also corrected to include the already implemented character resolver.
The HTTP smoke source was updated consistently but not executed: hosted/live
testing remains deferred and requires its own credentialled endpoint evidence.

The unchanged full real-stdio protocol test and gateway-upgrade contract test
also passed: two tests, 7.772 seconds, using
`python -m unittest test_mcp.MCPTests.test_rust_stdio_server test_gateway_upgrade -q`.
`cargo fmt --check` and `git diff --check` passed. Tests ran directly; no CI or
pipeline was run. No deployment or live-qualification claim follows from these
source/offline results.

Jev plan advisory: `jev-1.14.0`, request
`e73b3f61d5cbeb0e61d10f374066c8fb982ae7f200f06eb33e75f6a09c5b9674`,
507 input / 47 output tokens, 0.293 seconds; proceed probability 0.94. Advisory
probabilities do not replace tests, source inspection or independent review.

## Independent review

One round, independent `gpt-6-astra` high-effort Callers and Failure/security
lenses: both clean. Callers ran
`python -m unittest test_recipe_search test_mcp -v`: five tests passed, 11.462 s.
It traced route enumeration, profiles, store validation and planner handoff,
and independently checked 4,096 rows, maximum-u32 IDs, final-page termination,
maximum-u64 offsets and null defaults through the existing executable.

Failure/security ran `python -m unittest test_recipe_search -v`: one passed,
0.970 s. Its independent bounded stdio probe checked 82 pages covering 4,096
exact rows, complete counts/numeric order, 44 malformed scalar/container filters,
11 corrupt/foreign/path/schema snapshots, maximum IDs/offsets, unknown-output
exclusion and private-marker isolation. No concrete finding. Reviewer execution
does not establish current-source build provenance; the author's build supplies
that separate evidence. Neither reviewer made source or live changes.

Post-verification Jev advisory: `jev-1.14.0`, request
`bfee8d8b24ef26c82d083d97ad96677c8a90717f51521d6f1cdd664d0b257d6b`,
555 input / 49 output tokens, 0.291 s; land-scoped probability 0.99. Advisory
only. Hosted/native qualification and broader system coverage remain open.
