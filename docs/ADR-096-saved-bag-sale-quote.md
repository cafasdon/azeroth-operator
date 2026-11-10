# Saved bag sale-value quote

Date: 10 November 2026. Status: source/offline only.

The Forever collector already retains stable per-item vendor prices from linked
inventory rows. `inspect_snapshot` can now request `bag_sale_quote` on the
`bags` section. This is arithmetic for explicit saved locations and quantities,
not a sale recommendation or a game action. No new capture API is used.

The closed request object contains 1..20 distinct `selections`, each with
container (0..64), slot (1..1024), positive item ID and positive quantity.
`reserve_copper` defaults to 0, bounded to 9,999,999,999,999. Every saved stack
must exist, match the supplied base item ID and contain the requested quantity.
Missing bags/counts, mismatches, duplicates and invalid bounds fail generically.
The option is Forever-only and cannot accompany a coverage-section query.
Omission/null preserves the existing raw inspection response.

Each line multiplies that exact location's captured unit price by its selected
quantity. Equal item IDs at different locations keep separate prices and are
never treated as equivalent variants. Lines sort by container/slot. The report
projects only numeric quote fields; links, GUIDs and arbitrary item text are
excluded. The parent inspection retains its existing owner-authorised metadata.

Missing prices remain unknown. A recorded zero is a known zero quote, not proof
of sale eligibility. Checked per-line multiplication and subtotal addition
distinguish overflow from missing prices. Known subtotal excludes unpriced or
overflow lines, but a complete total exists only when every line is priced and
all proceeds arithmetic succeeds. No partial total funds a wallet projection.

Only the same capture's wallet can join a complete proceeds total. Checked
addition produces a hypothetical post-sale balance, reserve shortfall and
amount above reserve. Missing wallet and arithmetic overflow keep projections
null. Pending mail, bank items, rewards and outside funds receive no credit.
These hypothetical figures are not written back into wallet or inventory.
Large mathematical balances do not establish a native currency cap or ability
to complete a transaction. Current locations and sale eligibility stay unverified.

Direct validation: `cargo build --offline` and
`PYTHONPATH=.;tests python -B -m unittest test_bag_sale_quote_mcp`.
Native testing, installation and deployment remain deferred.
