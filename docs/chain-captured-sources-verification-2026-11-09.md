# Captured acquisition references for chain needs — 9 November 2026

Base source: `ba2fcc8`. ADR 026 adds an opt-in join from complete already-net
chain external requirements to saved owner-local `recipe_item_sources`, without
requiring a catalogue or active provider. Explicit reference selection retains
identity/client guards and historical provenance. No addon-capture, game-API or
snapshot-schema changes or public data sync are introduced.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 17.02s
```

An initial compilation identified an incorrect local enum module path; it was
corrected to existing `crate::Family` before verification/review. A later build
after tool description/error updates completed in 4.99s.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_chain_captured_sources test_chain_catalogue_sources test_reagent_chain_stock test_reagent_chain_merchant -v
```

Four tests passed in 1.839s through the actual local Rust stdio MCP. After adding
explicit producer-choice and public-catalogue composition assertions, the new
module alone passed in 0.632s. Its main session has no configured catalogue and
uses synthetic owner-store captures; a second session uses the existing pinned
public cache for simultaneous independent annotation verification.

The new test asserts:

- A two-step producer chain leaves five external items. Four have captured rows,
  two are provider-reported and one has retained source IDs. Produced item 20 is
  absent. Four required raw units minus one observed unit remain three, without
  a second subtraction during source lookup.
- All five category types, explicit empty maps, missing categories, eight-of-ten
  truncated IDs, provider miss, unavailable row and uncaptured item preserve
  their meanings. Snapshot/time and provider coverage are projected exactly.
- Missing capture yields unavailable statuses and null coverage; complete empty
  demand yields empty records; blocked chains yield null.
- Newer inventory reduces demand but cannot replace original reference IDs or
  timestamps. Explicit older, different-profession references use their own
  source-mode metadata and vendor ID while leaving the recipe model unchanged.
- Wrong name/realm/build/interface/variant, missing/path-like selectors and
  incompatible modes reject, including explicit selectors on blocked chains.
  Selector-without-opt-in, wrong flag type, selected-only recipe, batch and
  dependency-order mode reject. Omitted/false output is unchanged.
- Producer choices, stock preparation and merchant quotes compose without
  changing captured annotations. With the pinned cache configured, catalogue
  and captured annotations coexist separately. Source annotations omit private
  link markers and all verification/execution flags remain false.
- Original snapshot bytes remain unchanged. No provider, HTTP or game client is
  needed at query time.

These are saved reference hints, not resolved source locations, currently
available acquisitions, complete world data or a verified route. Native capture
acceptance and actual provider revision/accuracy remain unqualified. No install,
deployment, live test or CI ran.

## Planning advisory

Jev `jev-1.14.0`, request
`fb2ebf4d833a39e15a3e33c2e4d7ad1fecbda60ad27e3a3a799c78959c77b379`,
683 input / 47 output tokens, 0.263s: proceed 0.88, revise 0.08,
insufficient-evidence 0.04, confidence 0.82. Advisory only, not proof or approval.

## Independent review

One round with two independent `gpt-6-astra` agents at high effort: Callers and
Failure/security. Neither found a material defect; no behavioural correction
round was required. Product source stayed frozen. Callers suggested clarifying
that snapshot/game-API contracts are unchanged while MCP input schema expands;
that wording precision was applied above.

Both directly ran the actual-MCP test:

```text
python -m unittest discover -s tests -p test_chain_captured_sources.py -v
Callers: Ran 1 test in 0.696s — OK
Failure/security: Ran 1 test in 0.628s — OK
```

Callers confirmed the installed pinned-cache composition branch executed. Its
independent inline actual-Rust probe asserted known-empty versus absent
categories, net three minus two observed units remaining one, explicit missing
reference captures without recipe/inventory/latest fallback, missing default
recipe references despite later inventory data, covered-leaf exclusion and
fully covered empty record lists. Hash-valid malformed source categories,
false reference entries and unknown statuses rejected. All synthetic capture
bytes remained unchanged.

Failure/security's independent actual-Rust probe used 18 content-addressed
synthetic captures. A newer explicit reference capture with inflated stock and
wallet could not influence inventory accounting: net demand two, quote seven
and wallet shortfall one remained unchanged. Explicit missing/empty references
did not fall back. Step budget, craft budget and partially processed cycles
yielded null annotations; fully stocked chains joined no records. Nine malformed
source payloads bypassing Python validation and nine selector/client/type
attacks rejected without private-marker leakage. Repeated calls were
deterministic and all 18 files stayed byte-identical. Its initial Retail fixture
incorrectly retained a Forever-only wallet and was corrected before MCP launch;
that was a probe-fixture error, not a product defect.

Both traced the real guarded owner-store route and checked documentation against
projection semantics. No edits, builds, Git mutations, HTTP, database use, game
operations, installation or deployment ran in review. Native behaviour remains
unqualified.

Final scoped Jev advisory: `jev-1.14.0`, request
`91342928531112b9243c44952a9c316d30179fa3b66785d19a1a02f80fe56408`,
913 input / 49 output tokens, 0.268s: land-scoped 0.94, fix-first 0.03,
insufficient-evidence 0.03, confidence 0.91. Advisory only, not proof or approval.
