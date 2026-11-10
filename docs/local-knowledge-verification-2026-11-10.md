# Local hybrid knowledge verification

The initial public index contained 7,271 records from the pinned catalogue and
384-dimensional vectors from the checksum-pinned MiniLM model.
The local environment uses requirements-knowledge.txt. Model and index
files remain in ignored local-data directories.

The real CLI build completed, and a repeat returned unchanged without
changing index bytes. Two scoped tests passed in 21.877 seconds using
`python -m unittest test_forever_knowledge` with `PYTHONPATH=.;tests`.
They exercise the real local model through stdio MCP, block external
socket connections, assert source identities and hashes, reject invalid
inputs and verify unchanged source-index bytes.

Independent failure and security reviews found and corrected derived-index
reuse checks and unlisted optional model assets. Further failure rounds
extended the checksum to all FTS shadow tables and schema, including FTS
content row identities. The final failure round rejected eleven damaged
in-memory index cases and reached rebuild, while healthy CLI and real MCP
retrieval passed. Security checks rejected extra tokenizer assets before
reading them, detected changed schema/data, and exercised the actual SQLite
progress deadline. No reviewer modified the source or live index.

The security seal detects derived-file corruption. Exact public source
records remain checked independently against the pinned catalogue.
At initial verification the companion had not been registered in client
configuration or deployed to the hosted gateway. Native game testing remains deferred.
Retrieval quality across general gameplay questions remains unbenchmarked.

Advisory judgements used Jev 1.14.0.

- Initial finding triage selected fix_both. Request
  `a5613d8422e3088f605aac46dd18c95249975aab9d4185b77ba8845d6d66883a`.
  Usage was 555 input and 58 output tokens in 0.311 seconds.
- Shadow-index triage selected fix. Request
  `5abcfd12721e7c3b66d8a1c7cc2caa8af2e48bdb78fcf761867a672186fd7928`.
  Usage was 468 input and 46 output tokens in 0.288 seconds.
- The scoped shipping judgement selected supported. Request
  `2c8fe39a92abf9be5824158c9cceb06b6beb933e4366c80f6815943a1ef12565`.
  Usage was 671 input and 45 output tokens in 0.302 seconds.

The author accepted the scoped offline result after the independent reviews.

Local registration on ICARUS subsequently added `azeroth-knowledge` through
`codex mcp add`. A targeted edit set startup to 60 seconds, tools to
30 seconds, and the allowlist to the single search tool. Launching the
exact saved command, arguments and environment returned one advertised
tool and catalogue IDs 7950 and 7953 in 10.669 seconds.

The unrelated-config hash after excluding the new entry changed across
the CLI add. The earlier full configuration was not retained, so the cause
and full-file preservation were not established. The later targeted edit
verified that parsed configuration outside the new entry was identical.
The current chat's tool catalogue had not exposed the new server at the
time of this check. Hosted deployment and game testing remain deferred.

An independent registration review launched the saved entry from C:/Temp,
listed the one search tool and retrieved item 19019 for Thunderfury.
Invalid limit, section and empty-query probes returned sanitised errors.
The advisory scoped-registration judgement selected supported_scoped using
Jev 1.14.0, request
`a506eb32d9429f5b6bf45ba503628df43ae9ef0ae2c55a5ba1ef47c00addaf91`.
Usage was 570 input and 50 output tokens in 0.322 seconds.
This judgement does not resolve the unrelated-config hash discrepancy.

The all-category extension rebuilt the local index to 7,749 documents.
It added 247 enchants, 156 random suffixes, 71 spell effects and four
encounter presets. Independent comparison found all original 7,271
document tuples unchanged. Two offline local-model tests passed, including
real stdio searches with external connections blocked.
Independent review checked all nine source mappings against the Rust
catalogue lookup, distinct enchant variant keys and text encounter paths.
No concrete defects remained. Retrieval quality remains unbenchmarked.

The advisory shipping judgement selected supported with confidence 0.49
using Jev 1.14.0, request
`e7117b947b867080508eb87c209716ef264165e21b6bd357cb1aa185c7286c49`.
Usage was 546 input and 45 output tokens in 0.295 seconds.
The author accepted the scoped identity and retrieval integration based
on the exact corpus comparison and independent real-caller evidence.

Optional source-record responses passed two scoped tests in 25.178 seconds.
Independent failure review verified all 7,749 source joins across nine
categories, default compatibility, duplicate enchant variants, return
mutation isolation, bad arguments, the 256 KiB bound and the actual CLI.
Independent data review verified exact rows/hashes across every category,
unpinned input rejection, private sentinel mutation isolation, oversized
response rejection and zero attempted network connections. Index bytes
were unchanged. No concrete review findings remained.

The advisory shipping judgement selected supported with confidence 0.60
using Jev 1.14.0, request
`f3b2d08d9ce23c9eb5a6933111ae0a96d59c8ea1f515cf8217f9aae208cec9c2`.
Usage was 570 input and 45 output tokens in 0.293 seconds.
The author accepted the scoped feature after both independent reviews.

Candidate item allowlists passed two scoped offline tests in 28.903 seconds,
including an actual Rust owned-gear response feeding the local knowledge MCP.
The public index remains unchanged; no private inventory is persisted there.
Empty or unknown-only candidates return no results. Candidates are restricted
before lexical and vector ranking, with a maximum of 10,000 unique uint32 IDs.
Independent failure review recomputed ranking for 3,022 candidates, verified
the CLI and bounds, and confirmed unchanged index bytes. Independent data
review found MCP integer coercion; StrictInt now rejects booleans, strings,
floats and mixed lists at the actual MCP boundary. Both reviewers cleared the
fix, with external connections blocked. Live game testing remains deferred.

Finding triage selected fix with confidence 0.98 using Jev 1.14.0, request
`cda23cd3f6472bbdfe3259a3804455d1a75b4a80d2ab12207374b9a360ccf36f`;
475 input and 46 output tokens, 0.311 seconds.
Shipping judgement selected ready with confidence 0.92 using Jev 1.14.0,
request `c82ba07d10ec158d22ce9621db878508135b2f72e8d2244927f4272281fb6fd9`;
539 input and 44 output tokens, 0.376 seconds. The author accepted the scoped
change based on direct tests and both independent reviews. Retrieval ranking
is not a simulation score or proof of best-in-slot gear.

A source-grounded retrieval sanity baseline is recorded in
local-knowledge-retrieval-baseline-2026-11-10.json at source a958b1a.
Fourteen manually chosen queries cover all nine categories: ten reference
queries and four descriptive queries. All expected source sets were validated
against the pinned catalogue, including duplicate suffix names, item set names
and spellId versus spell-effect record ID. Results: 13/14 first place, 14/14
within three, reciprocal-rank mean 0.95238. Median warm query time including
index validation was 0.407207 seconds; model loading was 10.865586 seconds.
Socket connect was blocked, zero attempts were observed, and index SHA-256 was
unchanged. A separate readback recomputed source labels, ranks and metrics.
This small selected set is not a held-out relevance benchmark or a comparison
with lexical-only search; no simulation, native interaction or privacy boundary
was qualified by these measurements. No runtime changes were made.

## Index validation latency — 2026-11-10

Profiling identified repeated scalar unit-norm tolerance calls as avoidable
lookup overhead. checked_records now collects the unchanged individual float32
norms and compares tolerance in one NumPy array operation. Source records, FTS
visible rows, derived schema/vector/FTS seal, vector dimensions/finiteness,
model pins, size/deadline guards and retrieval calculations remain unchanged.
No query API, index format or persistent query cache changed.

Author: PYTHONPATH=.;tests python -m unittest test_forever_knowledge passed
three tests in 34.117 seconds, including actual offline stdio/model retrieval,
exact public source records, corruption/no-op build and new resealed boundary
vectors. Boundary cases use the fixed predecessor scalar predicate, including
near-threshold neighbours, zero/underflow, finite overflow, NaN/Inf and malformed
lengths. The intentional finite-overflow negative case emits a NumPy warning;
both old and new predicates reject it. Optional NumPy import follows the local
resource skip check, preserving fresh-environment behaviour.

A twelve-pair alternating serial warm-query benchmark shares one CPU model,
excludes startup and uses the unchanged pinned index. All replies were exactly
equal. Before/after medians were 0.6672735 and 0.412818 seconds, a 38.1336%
reduction for the measured workload. No socket connections were attempted;
index bytes remained unchanged. Input/source hashes, NumPy 2.4.4, query order,
repeats and raw timings are in local-knowledge-validation-benchmark-2026-11-10.json.
This is a small host-specific lookup measurement, not a general speed guarantee,
relevance benchmark, simulation speedup or BiS/native qualification.

One independent adversarial round: failure and security/data plus callers,
requested gpt-6-astra high; resolved model/usage not exposed. No concrete findings.
Failure review compared the exact Git predecessor and candidate: 5,042 float32
values including every representable boundary neighbour agreed, all 7,749
pristine rows matched, and sixteen resealed vector/source/FTS/seal corruption
cases had identical acceptance. Data/callers review passed all three tests,
7,558 independent float32 parity values and four real-model baseline/candidate
queries with attached source records; replies matched, no network attempts and
unchanged index bytes. It verified benchmark hashes, medians and scoped claims.
Diff check passed. No game or deployment operations occurred.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.98,
request b72b37a2dd44b40220ad0a46e01c3016ad685fd87f1e6de5e258c982532afaef;
721 input and 44 output tokens, 0.309 seconds. Author accepted the bounded
optimisation on direct measurements and both independent reviews. Live game
testing and native qualification remain deferred.

## Exact direct public references — 2026-11-10

Optional include_references implies the base source row/hash and attaches
recognised one-hop NPC, zone and faction foreign keys with JSONPointer source
paths and exact target rows/hashes or explicit missing/invalid status. Limit
1..5, at most 32 edges per hit, full edge count/truncation, combined 256KiB cap.
No recursive/name-based enrichment, external lookup, private snapshot access,
new index format, query persistence, model change or installation is introduced.
Native world identity, current acquisition availability and graph completeness
remain unverified. See ADR-075/local-knowledge contract.

Author: PYTHONPATH=.;tests python -m unittest test_forever_knowledge passed four
tests in 35.286 seconds. After adding the real faction join and native-identity
boundary assertion, the scoped actual-model stdio test passed in 25.318 seconds.
It covers exact Earthshaker NPC11982/zone2717 joins, missing vendor NPC11555,
no invented quest link for Thunderfury, direct NPC zone links and faction rows;
every target row/hash and base-row pointer is compared with the pinned source.
Strict malformed Boolean flags and source-bearing limits reject. The pure
helper fixture covers forty edges truncated to 32, missing and invalid IDs,
and mutation isolation. Existing default source-free queries stay unchanged.
Index bytes remain unchanged and external sockets stay blocked in the MCP test.

The actual CLI with --include-references --section items --limit1 --item-id17073
returned the exact Earthshaker source, NPC11982 and zone2717 rows and preserved
index SHA256. This uses local model/data only. Diff check passed.

A scan of every pinned source record found 4,124 recognised edges among 7,749
records, maximum22 per record and zero truncated records. Resolved: 2,066 zone,
625 NPC, 299 faction edges. Missing locally: 882 NPC and 252 zone edges. These
are edge counts, not distinct entities or complete world coverage. Inline
vendor names may remain in base rows even when no NPC target row exists; no
replacement target record is fabricated from those labels.

One independent adversarial round: failure and security/data plus callers,
requested gpt-6-astra high; resolved model/usage not exposed. No concrete
finding. Both independently verified every pinned edge and the documented
counts. Failure review passed escaped pointers, nested mutation isolation,
nonrecursive/unknown-field handling, ten invalid ID forms and 66-to-32 edge
truncation. It used a real read-only index plus fixed synthetic query model for
an oversized-target guard probe; default parity, implied base attachment and
combined 256KiB refusal passed. It separately ran two real model/helper tests
in 24.564 seconds. Data/callers review ran all four tests, checked baseline
source-free/source-only response parity with the real model, and independently
confirmed combined output-cap refusal, zero network attempts and unchanged
index bytes. Both confirmed pin/namespace/privacy/unknown boundaries and docs.
No source/index writes, installation, game or deployment operations occurred.

Advisory shipping judgement: Jev 1.14.0 selected ready, confidence 0.91,
request fc331e3518c697beeb42e853836f06baa963da53088c1a353591f521e1c6f964;
708 input and 44 output tokens, 0.386 seconds. Author accepted the bounded
feature based on exact corpus joins, direct CLI/MCP checks and both independent
reviews. Native qualification, installation/deployment and live testing remain
deferred. The earlier latency benchmark applies to its recorded predecessor
source and default query workload, not a new reference-enrichment benchmark.
