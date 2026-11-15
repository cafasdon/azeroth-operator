# Local character optimisation machine

Design and research: 15 November 2026. Scope: WoW Forever, offline analysis.
See [the service research](recommendation-methods-2026-11-15.md) for the product
comparison and [ADR 108](ADR-108-native-joint-build-evaluation.md) for the first
executable slice. This is a development specification, not a claim that every
component below exists or that the result beats a commercial service.

## What “best” means

A question must specify a character snapshot, client build, data/model revision,
legal candidate space, encounter distribution, player assumptions, objective and
resource budget. Gear, talents, action priorities, cooldown timing and consumes
are joint variables. Changing one variable can change the value of all the
others. Per-slot scores and coordinate ascent cannot certify the joint answer.

We distinguish four statements:

1. **Coverage:** every coordinate in the supplied finite matrix was evaluated.
2. **Sample leader:** a build has the highest observed score on the recorded
   simulation trials. Exact numerical ties remain ties.
3. **Statistical selection:** under explicit assumptions, a selected build is
   within an explicit tolerance with a controlled error probability.
4. **Game recommendation:** data, mechanics, native legality, ownership,
   encounter assumptions and character identity are sufficiently qualified.

None implies the next. “Absolute best” is available only for a complete legal
space and an exact deterministic objective, or with a clearly stated statistical
qualification for a sampled objective. Neither establishes the best real-world
play. A huge search over the wrong mechanics is a precisely wrong answer.

## Why this design

### Conventions isolated

Do not choose a vector database, an LLM search agent, a fashionable optimiser or
a larger simulator merely because it is common. Keep established privacy,
ownership, execution and native-input boundaries. Use the existing local
reference stores and pinned simulator before adding infrastructure.

### Domain axioms

- Talents, item effects and action timing interact. Some improvements require
  simultaneous changes through a worse intermediate build.
- Simulations are expensive, noisy model observations. A selected sample winner
  can be a noise winner. Mechanics and hidden coefficients can also be wrong.
- Unknown items, unresolved modifiers and unobserved bank contents are missing
  evidence, never evidence that no better build exists.
- A case is reproducible only when its complete inputs and evaluator are bound.
- Cheap exact constraints can remove impossible candidates. An inaccurate score
  prediction cannot safely prove that a candidate is worse.
- Repeated identical work has no value. A changed patch or assumption invalidates
  affected results, even when the character's visible gear looks unchanged.

### Synthesis

Build a trustworthy exhaustive reference first. Add speed methods only when
they demonstrate lower cost at equal measured regret and acceptable uncertainty.
Separate proposal generation, legality, simulation and evidence accounting.
Reuse the existing owner-private numeric cache and bounded public data stores.

## Research conclusions

AMR already describes joint gear/rotation reasoning for Forever. Its November
2026 notes also disclose uncertain mechanics and incomplete items. A rotation
planner alone is not a differentiator. The opportunity is an inspectable local
case, explicit gaps, complete combination accounting and a reproducible answer.
[AMR developer account](https://forums.askmrrobot.com/t/world-of-warcraft-forever-launch-roadmap/18162).

Raidbots documents cheap screening followed by higher precision. Its public
article dates to July 2023. Staged simulations are an established baseline,
not our novel algorithm or a verified description of every 2026 setting.
[Smart Sim](https://support.raidbots.com/article/55-smart-sim).

The 2016 Track-and-Stop result separates sample allocation from confidence
stopping, with asymptotic optimality for specified one-parameter bandit models.
It supplies a design lesson, not a theorem about this simulator.
[Garivier and Kaufmann](https://proceedings.mlr.press/v49/garivier16a.html).

The April 2021 confidence-sequence paper gives time-uniform intervals under
stated assumptions. A repeatedly inspected pointwise 95% interval is not that
guarantee. We must justify a sample range or tail model, account for candidate
and encounter multiplicity, and measure calibration. Clipping DPS changes the
objective and must never silently manufacture a bounded-sample guarantee.
[Howard et al.](https://arxiv.org/abs/1810.08240).

SMAC is a concrete implementation reference for model-guided configuration
search. Use predictions to order work, with exploration and exact interaction
blocks. Do not let a prediction authorise a proof-bearing exclusion.
[Official SMAC implementation](https://github.com/automl/SMAC3).

Distributionally robust optimisation needs an explicit uncertainty set for
encounter weights. It is a different objective from a weighted mean or a
minimum over named encounters. The 2020 DRBO paper studies one such formulation,
not a universal definition of a good build.
[Kirschner et al.](https://proceedings.mlr.press/v108/kirschner20a.html).

## Architecture and acceptance gates

| Component | Concrete behaviour | Evidence before adoption |
| --- | --- | --- |
| Case compiler | Bind full snapshot, model, data, encounter, objective and constraints. Default to the active character only through the existing selector. An explicit alternative stays explicit. | Identity drift rejects resume; no implicit cross-character merge. |
| Legal candidate generator | Enforce physical copies, storage access, faction/class/level/profession restrictions, unique groups, weapons, sockets, talent prerequisites and point budgets. Track unknowns separately. | Illegal best-looking candidates reject; unresolved candidates prevent a coverage certificate. |
| Joint materialiser | Replace complete disjoint player subtrees using content-addressed fragments. Strictly parse the actual pinned native request schema. | Independent Cartesian enumeration, stable hashes, no silent unknown fields. |
| Reference evaluator | Evaluate every declared small-case combination and retain equivalent coordinates. Rank estimates with complete accounting. | Native public fixture agrees with an independent exhaustive traversal. |
| Statistical allocator | Spend trials on unresolved close alternatives; use an explicit error/tolerance contract and independent final trials. | Fixed-seed noisy benchmarks report error, regret and unresolved outcomes, including near ties and bad covariance. |
| Large-space proposer | Conditional categorical surrogate, broad exploration, simultaneous interaction moves, warm starts. | Compare against exhaustive small cases, equal-budget random search, coordinate ascent and staged screening. Predictions never become proof. |
| Encounter evaluator | Evaluate movement, adds, phases, downtime, durations and player execution assumptions. Show weighted, worst-case and Pareto answers separately. | Specialists and compromise fixtures produce the intended distinct answers. |
| Model qualification | Compare predicted spells, effects and timings with voluntarily supplied logs and known examples, then qualify by patch/spec/mechanic. | Error report and provenance; no blanket all-class qualification. No live test in the current task. |
| Explanation | Report the actual selected fragments, changed interactions, numeric uncertainty, costs, alternatives and missing evidence. | Reproduction resolves the same content hashes and request digests. |

An embedding index can retrieve descriptions and explanations. It is not the
authority for item IDs, legality, talent prerequisites, exact effects or scores.
Use typed relational/graph records for those facts. SQLite remains suitable for
the current bounded local stores; a measured bottleneck must justify a change.

## Efficient execution without false certainty

1. Resolve and validate the whole bounded case before executing any candidate.
2. Remove only provably impossible combinations. Record each reason.
3. Canonicalise native requests, then simulate each distinct request once.
4. Cache numeric summaries by complete request, engine and metric mode. Never
   use only a gear hash for a joint build. Preserve per-owner storage limits.
5. For larger spaces, screen and propose with an explicit cost cap. Keep a
   challenge set and exploration budget to find interaction failures.
6. Confirm selected alternatives with trials that search did not consume.
7. Return unresolved alternatives at budget exhaustion. Do not force a winner.

Shared seeds do not prove beneficial common randomness after action order
changes. Compare paired and independent variance before adopting that shortcut.
Existing paired intervals remain approximate pointwise diagnostics.
[SimulationCraft's statistical documentation](https://github.com/simulationcraft/simc/wiki/StatisticalBehaviour).

Cache invalidation binds all supplied inputs. Model qualification must eventually
also track the mechanics and records a result used. Keep one active source
version and bounded result retention. Do not build an unbounded archive of
profiles, requests, combat logs or previous patch variants.

## How we earn a competitive claim

No public source exposes every competitor's private algorithm. There is no
evidence today that we beat AMR, Raidbots or every optimiser. Publish a claim only
for matched cases, objectives, model revisions, hardware and budgets that we
can reproduce and independently assess.

The benchmark ledger must contain:

- Exact interaction traps, valleys, ties, legal exclusions and unknown states.
- Public native class cases across distinct encounters and fixed trial seeds.
- Optimum recovery, regret, false certificates and unresolved results.
- Evaluations, simulated iterations, cold/warm wall time, CPU and peak memory.
- Search, independent confirmation and final reference results separately.
- Failures and excluded cases as well as successes.

Release a speed method only after it reduces cost at a declared error/regret
target on held-out cases. A single fast run is not an accuracy or superiority
claim. Public profiles test execution, not the private pilot's character.

## Current scope and next executable work

The original native slice supports a complete matrix of at most 64 combinations.
Separate [talent](talent-builds.md) and [gear](gear-builds.md) generators now enforce
the represented pinned constraints and export joint references. The
[window evaluator](joint-build-windows.md) covers larger declared matrices, with
bounded incremental reported coverage and top-K results. Its 256-build native
benchmark matches an independent exhaustive traversal. None of these results
establishes complete owned inventory, native legality or every item/action effect.

The native evaluator still supports level 60, DPS and one fixed encounter.
Statistically justified racing and a measured proposer remain next dependencies.
Native model qualification and personal character recommendations follow their
own evidence gates. Live game tests stay deferred.
