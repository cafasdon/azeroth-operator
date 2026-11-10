# Selected bag sale-value quote: offline evidence

Scope: ADR-096. Synthetic owner-local snapshots and the actual stdio executable.
No sale, game action, installed change, external query or deployment occurred.

Author commands:

- `cargo build --offline`: passed in 30.20 seconds. Final nested-object guard
  rebuild passed in 7.55 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_bag_sale_quote_mcp`:
  passed in 0.671 seconds before review; final regression passed in 0.649 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_bag_sale_quote_mcp test_buyback_cost_review test_merchant_buyback`:
  3 passed, 2.089 seconds. TEMP/TMP used C:/Temp/codex for legacy fixtures.
- `git diff --check`: passed with no output.

The actual SDK test includes 50 independent integer-reference quantity cases,
location-specific prices for equal item IDs, partial/missing prices, zero prices,
missing wallet, per-line/subtotal/wallet-addition overflow, reserve arithmetic,
strict schema/bounds/duplicates/quantity guards, missing/mismatched locations,
owner-store/path/family refusal, private text exclusion from quote/errors and
unchanged saved capture bytes. Omission/null comparison excludes only the
time-dependent age_seconds field.

Author corrections: the first missing-count fixture failed Python ingestion
before MCP, so the test now asserts that refusal and uses a hash-valid malformed
file to test Rust separately. The Retail fixture initially retained the Forever-
only wallet section; the valid Retail fixture now tests the family guard.
Neither change relaxed validation. An additional author probe reproduced
Serde's acceptance of `[0,1,10,2]` as a Selection despite an object schema.
The request adapter now rejects every non-object selection before deserialisation,
and a regression confirms the same input fails through the real MCP.

Initial independent callers review identified a reasoned test flake: exact
plain/null response equality could fail across a second boundary because of
age_seconds. Root accepted the finding and changed only the test's comparison
of that field. The caller's initial binary probes rejected 880 incompatible
option combinations, checked a valid bag-restock composition, 20 selections,
container 64/slot 1024, and seven malformed quantity types. Initial independent
failure/privacy probes matched 120 seeded integer-reference cases (seed 96),
eight malformed fields, seven malformed stored rows and three invalid IDs.
Both reviewers then received the rebuilt binary for the final guard review.

Final independent round: callers and failure/privacy, each requested on
gpt-6-astra at high effort, both clean. The actual SDK unittest passed in
0.661 seconds for callers and 0.648 seconds for failure/privacy. Callers rejected
nine nested-selection cases without marker echo, confirmed a subsequent valid
quote of 6 copper, and confirmed the original bag-restock review stayed unchanged
when combined with the quote. Failure/privacy rejected 12 nested/mixed-array,
type/empty/duplicate/unknown-field requests while valid object selections passed;
errors exposed no private markers or store paths. No arithmetic code changed
between the initial integer-oracle probes and the final guard review.

Root judgement: ready for source integration after the initial pass and final
clean round. The object-shape fault and test age comparison are fixed. Paid Jev
did not run under the user's spending stop. Native acceptance and deployment
remain deferred, not passed.
