# Incoming public reference evidence

Source-only companion feature, ADR-102. No installed-profile update, deployment,
private capture or live game test occurred.

Author commands with `PYTHONPATH=.;tests`:

```
.venv/Scripts/python.exe -B -m unittest test_knowledge_incoming test_knowledge_reference_walk
Ran 7 tests in 0.019s
OK

.venv/Scripts/python.exe -B -m unittest test_knowledge_reference_walk_mcp test_forever_knowledge.KnowledgeTests.test_direct_reference_bound_and_mutation_isolation test_forever_knowledge.KnowledgeTests.test_effect_references_exact_namespace_pages_and_unknowns
Ran 3 tests in 21.883s
OK
```

The unit cases cover exact namespaces, repeated references, pages beyond 32,
beyond-end offsets, hashes, detached results, malformed inputs before index
access, deadline refusal and CLI forwarding/early rejection. The actual MCP
test uses the pinned local public catalogue/model/index with external socket
connections denied. It pages all incoming references for a real selected NPC,
checks the known referring item and target keys, rejects malformed requests,
retains default response shape and checks an unchanged index hash. Existing
forward-reference tests pass after the shared extractor refactor.

`git diff --check` passed.

## Independent review

One round, two gpt-6-astra/high agents, callers/failure and security/data. Both
returned no concrete fault. The callers reviewer compared all 8,324 forward
pages with the committed implementation: identical. Its reverse oracle used
that old forward implementation and matched complete pages, counts and order
for all 597 resolved targets and 33,566 edges (41.488 seconds across the probes).
The actual offline CLI returned two incoming links for `spell_effects:679187`,
including `consumables:118`. Four invalid CLI cases failed before model/input
access. Scoped incoming tests passed (3 tests, 0.007 seconds), as did updated
walk/MCP tests (5 tests, 21.135 seconds).

The security reviewer independently extracted raw pinned fields without the
shared helper. Maximum fan-in target `random_suffixes:12718` had 951 edges;
offsets 0/32/950/951/uint32-max returned 32/32/1/0/0 with exact counts and hashes.
Sixteen malformed requests failed before index access. A path-like unknown key
returned neither a record nor a key echo. Deadline expiry after the scan still
raised an error, without a partial count. Its scoped incoming and walk/MCP tests
passed (3 tests in 0.006 seconds; 5 tests in 17.039 seconds).

Reviewers changed no files and denied network connections during direct probes.
Jev did not run because its tool declares external paid inference and no separate
spending approval exists. Author judgement accepted the direct deterministic
evidence and both independent clean reviews. Agent usage was not exposed by the
tool. Native qualification, installed availability and complete world coverage
remain unproven.
