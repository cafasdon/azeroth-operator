# Mailbox numeric funds and COD quotes: offline verification

10 November 2026. Source/offline only. ADR084 defines capture and selected quote
arithmetic. No native game, install, deployment or public-index change occurred.

## Author evidence

- `cargo test --lib mail_funds --offline`: five tests passed. Exact fields,
  statuses, numeric limits, contiguous slots, 100 maximum-value rows, unknown/
  empty distinctions, wallet-only budgets, reserves, unrecorded selected slots,
  paging and invalid/missing/wrong-family context are covered. A later fresh
  rebuild resolved the reviewers' initially older unit-executable timestamp.
- `cargo test --test mail_funds --offline`: two Lua5.1/Core export/strict Rust
  decoder tests passed, latest0.37 seconds. Normal capture reads six headers
  for three loaded rows and excludes private first-four/later returns and
  errors. Inherited visibility methods, maximum exact integers, zero loaded,
  unavailable rows, changed amounts, nil/secret/fraction/negative/string/error,
  count/context/getter/frame/guard/family changes, busy/hidden frames and combat
  refusal are covered. Forbidden mail actions, correspondence reads and wallet
  reads stay at zero. Two callback regressions below now refuse the section.
- `cargo test --test mail_attachments --offline`: existing attachment collector
  regression passed,0.03 seconds. Its source and private-content limits remain
  unchanged; numeric header observations are a separate collector.
- `cargo build --offline`: fresh main executable passed in26.27 seconds. Only
  Lua/tests/prose changed afterward; Rust production logic remains identical.
- `$env:PYTHONPATH='.;tests'; python -m unittest test_mail_funds_validation test_mail_funds_mcp`:
  eight tests passed in1.661 seconds. Seven strict reader/storage tests and
  one actual stdio MCP workflow cover numeric privacy, summaries, missing vs
  known zero, selected unrecorded slots, pages, owner isolation and unchanged
  owner/foreign bytes. An independent arithmetic oracle covers144 combinations
  of nine slot selections, four wallet states and four reserves. Mail money
  never funds COD. The fixture has100 pending copper, wallet5 and known COD20:
  the selected quote budget does not fit and its shortfall is15, not zero.
  Unknown amounts prevent a positive aggregate fit; reserve deficits also fail.
- `git diff --check` passed.

Implementation used two workers (requested gpt-6.1-sol medium, fresh context)
for the Rust model/wiring and Python reader/tests. Root owned capture/Core,
Lua/MCP tests, callback corrections and docs. Resolved models/usage were not
exposed. Workers did not review, commit, build the main binary, deploy, use the
network or operate a game. The cached pinned upstream UI source was inspected,
not executed; its exact SHA-256 and raw line links are in ADR084.

## Three review rounds

Failure and callers/security-data lenses requested gpt-6-astra high with fresh
context. Resolved models/usage were not exposed. The initial tests/probes passed,
but the failure lens reasoned that a final pending-command getter could hide
the frame after visibility was already checked. No standalone Lua interpreter
was available to that reviewer. Root added `pending_hides_frame` to the existing
mlua harness and reproduced a failing refusal assertion. Checking pending before
visibility fixed this case; both Lua tests passed in0.36 seconds.

Round two's callers lens identified the complementary case: a visibility getter
could start a command after the pending check. Root added
`visibility_starts_command`; its refusal assertion failed. Rather than reverse
the earlier order, root retained it and sampled the complete idle/visible
context twice at each boundary. Both regressions now refuse. This is a sampled
observation contract, not continuous UI stability or native-state proof.

Jev1.14.0 advisory triage selected reproduce for the first case, confidence0.96,
request `759e1e6dc93a1c693173e435b9ecf3601a241a75c0fd0f4b866bb039c621d7bd`,
516 input/50 output tokens,0.277 seconds; and the complementary case,
confidence0.91, request
`7298e123c2ad514ac799f65a1fe96e3b3049e1e4c5a9fae3fc632fff369badd1`,
579 input/50 output tokens,0.299 seconds.

Initial callers/security probes passed eight Python/MCP tests in1.149 seconds,
both existing Lua tests, source hash/API-position checks, and independent actual
MCP refusals for correctly hashed sender injection, wrong family, hash tamper,
traversal and absent data without private sentinel/path leakage. Mixed selected/
paged unknowns and immutable bytes passed; public knowledge ingestion rejected
the private snapshot. The five Rust unit tests initially had a timestamp caveat;
root rebuilt them and both reviewers passed that fresh executable in follow-up.

Round three was clean in both lenses. Each read the exact two-sample fix and
both retained regression cases, then ran the fresh Lua executable: two passed
in0.31/0.34 seconds. No further concrete blocker was found within the documented
snapshot contract. No reviewer source writes, builds, network, deployment or
native game test occurred. Native header semantics, current availability,
identity, payment eligibility and complete mailbox coverage remain unproven.

Jev1.14.0 advisory shipping judgement selected ready, confidence0.50, request
`08f40e36203e1570f0e6f4adbc9ff2e8de91271ddbbbae7c81ed41edff1f1f42`,
729 input/44 output tokens,0.292 seconds. Author accepted only the bounded
source/offline feature on fresh exporter/reader tests, exact wallet arithmetic
and clean third-round lenses. The advisory is not approval or native evidence.
Installation/deployment/live qualification remain deferred; no payment,
retrieval, send, ownership credit or claim of native continuity is established.
