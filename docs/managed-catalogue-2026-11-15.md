# Managed catalogue acceptance — 15 November 2026

The local knowledge reader now accepts either its original catalogue JSON file
or the managed catalogue cache root. This connects the second pinned provider
to the incremental cache, without another persistent copy for knowledge queries.
Existing simulator callers can still use their standalone catalogue file.

An existing ownership marker is mandatory before cache construction. Exact
provider, revision and file hashes must match the repository catalogue pin. The
cache lock covers the bounded JSON read. The existing 8 MiB size limit and
catalogue checksum verifier apply to both inputs. Parsing occurs after the
reader owns a private byte copy, so a later sync cannot change that input.

No source download, index rebuild, model rebuild or tool schema change occurs.
The managed source cache's 16 MiB quota excludes the knowledge index and model.
A new catalogue revision still needs a reviewed lock and an explicitly rebuilt
matching index. This does not establish complete item/effect data or best-in-slot
results, and does not change private account inventory.

## Actual local MCP acceptance

The real `local-data/reference-cache/catalogue` supplied the current catalogue.
An independent-cwd SDK client launched the existing knowledge MCP with this
input, the existing index and the local embedding model. Both tools appeared.
Exact lookup returned item 4253 as Toughened Leather Gloves. Keyword/vector
search restricted to that item returned the same exact record key. The cache
status and index SHA-256 before and after matched. No index/model build ran.

The installed `azeroth-knowledge` registration changed only its `--catalogue`
argument. Its two enabled tools, offline environment, timeouts, model path and
index path remained unchanged. Configuration writes used explicit UTF-8 bytes;
both staged and saved files passed the native Codex parser. A private backup
stays under ignored `_secrets`, outside source control.
An additional SDK launch used that saved configuration and returned both tools
and the expected exact item record. Existing app connections may need a refresh.

## Tests and review

Eleven new focused tests passed. They cover original JSON/document parity,
unchanged source files, identical content at a wrong revision, wrong provider,
wrong hash, corrupt data, invalid/missing marker, the actual size cap, lock
contention and lock ownership during the bounded read.

One independent review round used two Astra/high reviewers for callers/failures
and security/data. Both ran the tests and separate probes. The real build CLI
reported the existing index unchanged. Managed and standalone searches returned
identical results. Rejected inputs left source and index hashes unchanged; failed
builds created no output directory. There were no medium, high or low findings.
No service deployment changed, so no infrastructure drift lens was necessary.

The planning advisory used Jev `jev-1.14.0`, request hash
`ebac7b627b6d8f36fdfe20af45e080964af25d96019017cb24033c49048f1799`,
486 input and 47 output tokens, 0.299 seconds. It selected proceed. This was
advice, not test evidence or authority to publish.

The release advisory used the same Jev version, request hash
`2da6e2a6f6bbe0bc3b5f137f96de195b89266513ec0154d3fa31e652b8d9f7e9`,
556 input and 44 output tokens, 0.294 seconds. It selected ship. The main session
accepted the direct checks and independent review as the release basis.
