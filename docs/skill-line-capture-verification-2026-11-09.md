# Skill-line capture guards and stability — 2026-11-09

Baseline 03ef909. Previous goal turn made progress by integrating saved skill
queries. This unit improves their underlying passive capture. The broad MCP
coverage/owned-BiS goal remains active. No live game, account data, installation
or deployment used; no network/API expansion or training action introduced.

## Concrete failures and correction

The old collector indexed a restricted namespace before checking access and
only re-read count. New focused Lua 5.1 tests first failed: restricted namespace
access counter was 1 instead of 0; a same-count rank change was accepted. Initial
red build 20.64s; both tests failed. Source now guards namespace/getters, captures
function references, checks their identity and namespace after each projection,
and compares two <=512-row projections including header/ID order and every
emitted value. Required errors remain unavailable through existing Core isolation.
Optional unreadable fields remain omitted, with transitions rejected if emitted
projections differ. Two equal observations do not prove atomicity or completeness.
Collapsed headers stay collapsed; no UI action is taken.

## Direct author verification

```powershell
cargo test --locked --offline --test skill_line_capture
cargo test --locked --offline --test addon forever_bank_export_distinguishes_observed_empty_and_unavailable
python -m tools.package_addon --family forever --variant live --interface 16001 --version 0.51.0-dev --output dist/forever-skill-capture-0.51.0-dev
python -m tools.verify_addon_packages --directory dist/forever-skill-capture-0.51.0-dev --verifier target/debug/examples/verify_addon.exe
```

After the initial correction, 2 tests passed (compile .63s, runtime .01s).
Expanded boundary tests: 3 passed (compile .64s, runtime .02s), and real Core
capture/export fixture: 1 passed, 1 unrelated filtered out (compile 1.87s,
runtime .25s). Cases include restricted API/getters, rank/cap/ID/category/bonus
changes, optional omission, name/header drift, namespace/function replacement,
512/513-row boundaries, explicit zero caps and header-only empty observations.
Existing duplicate-skill section isolation and export decoding pass unchanged.
No CI or unrelated full-suite execution.

Package verifier checked exact source rendering, checksums, generated adapter,
Lua 5.1 roundtrip, Forever capture and changed-interface rejection. It reported
live_qualified=false. Archive remains uninstalled:
`dist/forever-skill-capture-0.51.0-dev/AzerothOperator-0.51.0-dev-forever-live.zip`.
SHA-256 `0be6dc0fe039ad087a6018212670fbbcf229232896e917df1d5a54a87cc85229`.
Packaged LF Collector SHA-256 `f48700b3b1362613ca48ed3befd7730ddeed984fca14db7015275b33cb91062c`.
Packaged LF Core unchanged: `28f8eded6884753ce25ff2aa4ee08db3e803424cd2df093faf15d27eb401b03f`.
No schema, MCP binary or stored-data migration required.

## Advisory plan

Jev resolved jev-1.14.0, request
`5ddc9ed9ebecbee375daac15a8e023a4de070ec374124de9dd026adff2f9d4da`:
proceed .92 / revise .07 / insufficient .01, confidence .88; 468 input /47 output
tokens, .319s. Advisory only; actual failures and independent reviews decide.

## Independent adversarial review

Two read-only reviewers were assigned gpt-6-astra high, Callers and
Failure/privacy (runtime model IDs and token usage not exposed to the children).
Both ran the prebuilt three capture tests and one filtered Core export test:
3 passed; 1 passed /1 filtered. Callers traced Core pcall isolation and the
unchanged caller/schema shape. Failure/privacy additionally ran 91 independent
synthetic Lua 5.3 ctypes cases with zero restricted metamethod accesses. These
supplement, rather than replace, primary Lua 5.1 tests. Cases covered restricted
and invalid required/optional fields, nil/secret transitions, counts, same-count
reordering, duplicates, API replacement, header fields and u32/i32 boundaries.
Both documentation readbacks were clean; Failure verified archive and packaged
LF source hashes. No confirmed review fault or source correction was required.
No reviewer builds, source writes, live access or deployment. Live drift omitted
because installation/testing remains deferred. Final fmt and diff checks passed.

Final advisory Jev resolved jev-1.14.0, request
`22f34892de628cfb19d6b9329e27bbcc4b20e8d914b28afcc9415f84f53c6a65`:
integrate .98 / revise .01 / insufficient .01, confidence .97; 490 input /52
output tokens, .297s. Author integrates on reproduced red/green evidence and
clean independent reviews; this does not qualify native operation.
