# Recommendation methods and the local optimiser

Research date: 15 November 2026. Scope: published product descriptions, official
documentation and public simulator code. No login, private character query,
premium service, bulk dataset scrape or live game test was used.

## What the services actually do

### Ask Mr. Robot: separate search from performance scoring

AMR's implementation changed over time. Its developer describes earlier stat
weights, then simulator-generated training data and learned scoring functions,
followed by Dragonflight-era analytical class models. That is historical
evidence, not a description of every current version.
[Developer explanation, October 2023/March 2025](https://forums.askmrrobot.com/t/where-did-the-old-optimizer-go/15021).

For Retail, the August 2026 update explicitly reports a return to simulation-based
optimisation. Available beta report pages and actual optimisation coverage are
different features.
[Retail update, 10 August 2026](https://forums.askmrrobot.com/t/midnight-12-1-patch-updates/18054).

For Forever, the November developer explanation describes a planner that examines
sensible ability-use alternatives, chooses a rotation and scores gear with it.
The interface exposes that rotation. The launch notes distinguish low-level
owned-gear optimisation from maximum-level BiS, and disclose incomplete items,
limited tests and server-side coefficients that need logs. This supports joint
gear/rotation analysis, not a claim about their undisclosed search algorithm.
[Forever roadmap, 29 October–5 November 2026](https://forums.askmrrobot.com/t/world-of-warcraft-forever-launch-roadmap/18162).

Forever's per-slot upgrade list compares alternatives in its selected BiS set.
That is a different baseline from the player's current equipment. Filters and
known item coverage constrain the list.
[Forever upgrade explanation, 12 November 2026](https://forums.askmrrobot.com/t/can-you-not-use-the-find-upgrade-option-anymore-in-wow-forever/18172).

No public source examined exposes AMR's complete optimiser implementation or a
contract that we can treat as a public optimisation API. The developer reported
no immediate native MCP plan on 27 October. The API proposal in that thread is
the requester's proposal, not an AMR specification.
[MCP discussion](https://forums.askmrrobot.com/t/feature-request-native-mcp-integration/18159).

### Raidbots and SimulationCraft: repeated event simulation

Raidbots' published Smart Sim design screens combinations cheaply, then spends
more iterations on competitive candidates. The support article, last updated
4 July 2023, describes three precision stages. It also explains why small gem and
enchant differences need more computation. Those documented settings are not a
verified promise that its private 2026 service still uses identical thresholds.
[Smart Sim](https://support.raidbots.com/article/55-smart-sim),
[gem/enchant costs](https://support.raidbots.com/article/64-why-do-gem-enchant-sims-take-so-long).

SimulationCraft separates the combat model from its action-priority lists.
The engine selects an available action according to conditions, cooldowns,
resources and target state. A rotation is therefore part of the tested policy,
not just an item score multiplier.
[Action lists](https://github.com/simulationcraft/simc/wiki/ActionLists).

Encounter events can include movement, adds, target availability and phase-like
timing. A stationary single-target result is not an all-encounter result.
[Raid events](https://github.com/simulationcraft/simc/wiki/RaidEvents).

Repeated iterations estimate a distribution, rather than reveal an exact expected
value. The documentation explicitly warns that a shared seed does not guarantee
comparable random effects after actions diverge. Selection and final confirmation
need separate statistical treatment.
[Statistical behaviour](https://github.com/simulationcraft/simc/wiki/StatisticalBehaviour).

### Wowhead: authored advice, structured data and planning tools

Wowhead includes authored class guides as well as separate calculators and
planners. Its November Forever development log describes character/talent
imports, talent purchase order, stat-weight gear planning and encounter advice.
It also labels Classic Era drop-data fallbacks until Forever observations replace
them. These are distinct evidence types: an authored levelling order, a weighted
gear score and a personal combat simulation are not interchangeable.
[Development log through 10 November 2026](https://www.wowhead.com/latest-dev-updates).

An older Wowhead guide explicitly directs advanced personal gear questions to
simulation tools. Its current site patch badge does not update the guide's
15 June 2020 publication date.
[SimulationCraft/Pawn guide](https://www.wowhead.com/guide/how-to-use-simulationcraft-and-pawn-4885).

### Blizzard Armory/profile APIs: character evidence

Blizzard's Classic API announcement describes profile and equipment endpoints.
It does not describe a build optimiser. This historical announcement does not
verify current Forever endpoint coverage or complete bag/bank access. Raidbots'
2023 documentation prefers addon exports for richer, fresher character input.
Treat profile data as an input source, with explicit edition and freshness checks.
[Blizzard announcement, 4 August 2023](https://eu.forums.blizzard.com/en/wow/t/new-apis-now-available-for-testing/461426),
[Raidbots input comparison](https://support.raidbots.com/article/54-installing-and-using-the-simulationcraft-addon).

The current developer documentation could not be fully retrieved in this check.
No numeric Blizzard API rate limit or Forever support claim was established.
The full API terms page also returned 403. This report is not a licence review.

### Archon/log aggregation: observed choices

Archon's FAQ distinguishes popular logged builds from personal optimisation and
recommends personal simulation. Encounter and player-population filters matter.
Inference: selection of successful logged runs can introduce survivorship and
selection bias; a popular build does not establish causation or an individual
optimum. The FAQ was available through indexed primary text; direct retrieval
returned 403 in this check.
[Archon methodology and disclaimers](https://www.archon.gg/wow/articles/help/archon-disclaimers-and-faq).

### Open-source engine reuse

WoWSims Forever publicly supplies a community-maintained simulation framework
under MIT, with a request for visible attribution. Reusing its code is different
from proving every mechanic or level accurate. The repository's pinned model
and data need their own validation and compatibility checks.
[WoWSims Forever README](https://github.com/wowsims/forever).

## Design inference for Azeroth Operator

The machine needs separate, versioned components:

1. **Case:** exact character capture, edition/build, level, objective, encounter,
   constraints, model and data hashes. Survival, utility, cost and allowed item
   sources must be explicit rather than hidden behind a universal score.
2. **Rules and data:** spell ranks, talents, prerequisites, level budgets, item
   variants, unique/equip constraints, physical copies, pets and professions.
   Missing mechanics and old-edition fallbacks remain explicit.
3. **Whole-build candidates:** gear, talents, rotation, consumables, enhancement
   choices and cooldown timing must vary together. Independent slot/stat ranking
   can miss set bonuses, resource thresholds and effect interactions.
4. **Legality and resolution:** reject impossible candidates before scoring.
   Unknown legality is not legal. A hash reference alone is not a resolved item
   or a validated talent build.
5. **Scoring model:** event timing, GCD/casts/cooldowns, resources, procs, periodic
   effects, stacking/snapshot rules, target state, movement and encounter phases.
   Every supported mechanic needs a known source and regression evidence.
6. **Search:** exact windows for tractable spaces; candidate heuristics and
   low-precision simulation for larger spaces. A heuristic exclusion is not a
   proof that a candidate cannot win. Branch-and-bound needs valid bounds.
7. **Confirmation:** evaluate finalists on independent trials and quantify noise,
   practical differences and selection effects. A tie or unresolved comparison
   is a valid result. More iterations do not repair an incorrect model.
8. **Explanation:** show the baseline, selected rotation/timings, interactions,
   gains, costs, uncertainty, coverage and remaining assumptions.

Expert build records can seed candidates and explain utility. They must retain
source, patch, class/spec, level range and activity. A talent endpoint is different
from its point-by-point levelling path: every purchase needs rank, budget,
prerequisite and tree-gate validation at that level. A final level-60 winner does
not automatically define the best level-22 path.

Local caching should key all relevant inputs and invalidate changed model/data
dependencies. Numeric summaries can remain private and bounded; no private
character data needs to be sent to recommendation websites. This is a proposal
for our architecture, not a claim about competitors' private implementation.

## Meaning of “absolute best”

For a finite declared space and an exact deterministic score, complete legal
enumeration or valid bounding can establish a model optimum. That proof still
depends on the model and case assumptions. For random combat performance, finite
samples establish estimates and confidence, not certainty about the true mean.

Keep separate result states for a best measured candidate, statistically
unresolved alternatives, complete declared-space accounting and a justified
model-optimality claim. Never turn a beam-search shortlist, stat-weight winner,
popular build or incomplete inventory into an unconditional best-in-slot claim.
The system should preserve a candidate's reason for exclusion or unresolved
status, and should retain resume points when the time or resource budget ends.

## Current repository and first implementation slice

Existing source already includes owned-gear windows, resolution accounting,
adaptive bulk screening, independent-seed confirmation, numeric caches, journals,
role metrics and multi-encounter Pareto/minimax views. These are useful components,
not a complete joint character optimiser. `run_forever_sim.py` still requires an
explicit captured level of 60; this work does not relax it.

The new bounded slice is a joint experiment-window planner. It binds a case and
ordered candidate axes, then emits exact mixed-radix windows without allocating
the entire cross-product. It can combine slot/loadout, talent, rotation, timing
and consumable references. It does not resolve references, enforce talent/equip
legality, run a simulator or produce a winner. Its complete-matrix counts describe
only the supplied references. Resolver/model qualification remains the next gate.
Its contract and evidence are in [joint build search](joint-build-search.md).

No competitor database, authored guide collection, private endpoint or closed
algorithm is copied. New data acquisition still needs source-specific rights.
Live tests remain deferred.
