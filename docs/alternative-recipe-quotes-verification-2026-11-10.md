# Alternative recipe quote offline evidence

Scope: ADR-104. Existing Rust MCP tool with an optional comparison; no game action,
installation, deployment, external data request or real character capture.

Author commands, with `PYTHONPATH=.;tests` and TEMP/TMP `C:/Temp/codex`:

```
cargo build --offline --bin azeroth-operator
Finished dev profile

.venv/Scripts/python.exe -B -m unittest test_recipe_restock_alternatives_mcp test_recipe_restock_quote_mcp
Ran 2 tests in 0.776s
OK
```

The new actual stdio MCP test independently calculates 36 target/stock cases
with different recipe yields, reagent costs and bundle sizes. It checks both
candidate costs and the requested minimum. It also checks wallet affordability,
missing offers, missing recipe data, no-purchase zero costs without merchant or
wallet data, ties, duplicate/empty/oversized IDs, malformed private markers,
conflicting joint/schedule modes, wrong outputs, foreign stores and path-like
snapshot IDs. Existing saved snapshot bytes remain unchanged. The legacy saved
restock merchant-quote test passes. No simulator model or live game was used.

Only the new Rust module was formatted. Later changes to the tool description
and error text do not change the calculation. The final offline binary build
passed in 12.72 seconds; both tests passed again in 0.602 seconds.
`git diff --check` passed.

## Independent review

One round, two independent gpt-6-astra/high agents: callers/failure and
security/data. Neither found a concrete fault. Both read the owning code, MCP
boundary, dependencies and ADR-104. The callers reviewer ran the actual new MCP
test (one test, 0.601 seconds, OK). Extra probes covered null/default compatibility,
seven malformed ID cases, all seven alternatives, deterministic order, implicit
selected-primary duplicate rejection, zero-cost methods without merchant/wallet,
over-1,000-craft uncertainty and independent shared-material ties.

The security reviewer ran the same actual test (one test, 0.496 seconds, OK).
Its independent probes rejected 14 malformed/conflicting requests and four
path/digest requests without submitted sentinel text in errors. Eight candidate
rows remained deterministic. An over-limit alternative stayed unknown; a known
zero-cost method beside an unknown method retained a null requested minimum.
The final byte cap, after optional views and snapshot metadata, was checked in
source; an oversized response was not reproduced. Owner-isolation and unchanged
snapshot bytes were checked by the actual caller test.

Reviewers used only temporary synthetic captures and changed no source. No
network, build, installation, private capture or game access occurred in review.
Jev did not run because its tool declares external paid inference and no separate
spending approval exists. Author judgement accepted the direct deterministic
tests and both clean reviews. Agent usage was not exposed by the tool. Native
output equivalence, complete recipe coverage and current craftability remain
unproven.
