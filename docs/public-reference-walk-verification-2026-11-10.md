# Exact public reference walk: offline evidence

Scope: source-only companion MCP extension described in ADR-101. No installed
profile, private snapshot, native client or live game operation was touched.

Author commands, with `PYTHONPATH=.;tests` and the project virtual environment:

```
.venv/Scripts/python.exe -B -m unittest test_knowledge_reference_walk test_knowledge_reference_walk_mcp
Ran 4 tests in 13.331s
OK
```

This run covered cycles, explicit missing endpoints, source hashes, depth limits,
node/edge/time/byte bounds, more than 32 edges and actual stdio MCP requests against
the pinned public index. The child process denied external socket connections.
The actual test checked schema bounds, repeated output, old direct-preview shape,
invalid requests, unknown exact keys and an unchanged index hash.

After a CLI forwarding test was added:

```
.venv/Scripts/python.exe -B -m unittest test_knowledge_reference_walk
Ran 4 tests in 0.010s
OK
```

The new CLI test checks all three walk options and rejects a deep request without
its parent reference option before model construction.

Existing compatibility tests:

```
.venv/Scripts/python.exe -B -m unittest test_forever_knowledge.KnowledgeTests.test_exact_record_lookup_bounds_copy_and_no_encoding test_forever_knowledge.KnowledgeTests.test_direct_reference_bound_and_mutation_isolation test_forever_knowledge.KnowledgeTests.test_effect_references_exact_namespace_pages_and_unknowns
Ran 3 tests in 0.029s
OK
```

`git diff --check` passed. Offline evidence does not qualify native world identity,
current acquisition, owned variants, routes or complete world coverage.

## Independent review

One round, two independent gpt-6-astra/high agents: callers/failure and
security/data. Both returned no concrete finding. The callers reviewer ran the
actual offline MCP suite (4 tests, 14.237 seconds), then the updated unit suite
(4 tests, 0.009 seconds). Its independent breadth-first projection matched all
3,181 pinned records with outgoing references, from 7,749 total records, including
exact edges, shortest node depths and source pointers. It also checked 270
synthetic depth/node/edge budget combinations, 30 invalid inputs and actual CLI
forwarding plus pre-model rejection for invalid get/search/serve requests.

The security reviewer ran the offline MCP suite (4 tests, 13.399 seconds), checked
20 malformed inputs before index access, and confirmed a path-like unknown key
returned no private path or echoed key. On maximum-degree root `items:14270`,
budgets 1/1 returned node exhaustion at 1 node/1 edge; 2/1 returned edge exhaustion
at 2/1; 256/1,024 completed at 44/44. Independent pointers and row hashes matched.
Both reviewers blocked network access for their direct probes and changed no files.

Jev did not run: its tool declares external paid inference, and no separate
spending approval exists for that service. Author judgement accepted the concrete
offline evidence and both clean independent reviews. No probabilistic benchmark
claim follows from this deterministic feature. Review usage was not exposed by
the agent tool. Installation and live qualification remain deferred.
