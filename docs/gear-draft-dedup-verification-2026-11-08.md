# Full-draft deduplication — 8 November 2026

`--collapse-identical-items` already bounded equivalent physical copies per
slot, retaining enough copies to fill paired slots. Physical permutations could
still produce identical full simulator gear inputs and exhaust `--max-drafts`
before validation. The opt-in path now resolves up to the existing bounded
1,024 combinations and counts only distinct resolved alternative gear inputs
against that limit. The baseline does not consume the alternative budget.

Canonical JSON of the full positional `simulator_equipment_spec` preserves
item IDs and resolved modifiers. Equivalence is not inferred from base IDs or
unordered sets. The first validated physical representation remains selected;
other selections are retained as skipped entries with representative review
numbers, including baseline review one where appropriate. Physical-source
reuse and vacated-slot checks run before review as before.

Review batches remain at most eight. Responses are consumed per batch rather
than accumulated across the whole bounded search. Default proposed-draft limit
semantics remain unchanged when the option is omitted. Search metadata reports
the budget basis, physical proposal count, distinct gear count and equivalent
draft count. This does not widen native legality or storage coverage claims.

The committed real-MCP regression uses the pinned local catalogue, a synthetic
snapshot and two Seafire Band copies in separate bag slots with explicit zero
modifiers. Six source-distinct proposals resolve to two distinct alternatives
plus baseline: two duplicate resolved drafts collapse, while two off-slot-only
drafts remain unresolved. Two additional proposals are rejected for reusing one source.
The non-collapsed path returns five resolved physical drafts including baseline,
covering the exact same three distinct simulator inputs. The two-alternative
budget now succeeds only with collapse enabled; a one-alternative budget still
fails. A single physical copy cannot produce a two-ring equipped draft.

With `PYTHONPATH=.;tests`,
`python -m unittest test_gear_draft_dedup test_encounter_sensitivity test_simulation_comparison`
passed eight tests. The integration test requires a built Rust executable and
the pinned catalogue; it explicitly skips when the catalogue is absent rather
than downloading data. No skip occurred in this verification. The module's
`python -m tools.rank_forever_slot --help` entry point also passed.

No game actions, installation or live game testing occurred. Complete owned
best-in-slot coverage remains unproven, particularly beyond bounded pools and
unresolved modifiers or equip requirements.

Jev 1.14.0 advisory scope assessment returned supported, separately from test
and review evidence (request
`e07e69456aa677af1c2e437c8fdc33fa7336b498a76eb25ec9a491bf827a7923`,
576 input / 46 output tokens, 0.333 seconds).

Independent review ran two lenses (gpt-6.1-sol, high effort), with no concrete
findings:

- Failure: the real-MCP regression passed in 1.880 seconds. Additional temporary
  fixtures checked baseline-only aliases, mixed baseline/new representatives,
  one-copy source restrictions, unresolved drafts and exact budget boundaries.
  128 distinct necklace alternatives retained 129 drafts including baseline;
  the 129th alternative failed. Recorded review batches never exceeded eight.
- Callers: the regression passed in 1.195 seconds. The actual ranker module CLI
  with `--plan-only --collapse-identical-items --max-drafts 2` returned three
  reviews and representatives 2/3. Forced serial and normal batched MCP reviews
  produced identical full collection results across duplicate rings, baseline
  aliases and distinct-enchant glove fixtures. Gloves with enchant zero and
  enchant 15 both remained distinct. Baseline-only aliases all referenced one.

These extra fixtures were transient review evidence, with no native simulation
or game interaction; the committed real-MCP regression is the retained
reproducible test. No simulation speedup is claimed: the existing simulator
adapter already deduplicates full requests. This change improves bounded
search admission, reduces duplicate review retention and preserves provenance.
