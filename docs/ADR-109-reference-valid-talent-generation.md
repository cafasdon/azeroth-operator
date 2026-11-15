# ADR 109: Generate talent combinations from complete pinned tree constraints

Date: 15 November 2026. Status: accepted.

The identity map preserves definition IDs and rank limits, but omits rows and
prerequisites. The native simulator deliberately accepts some broken allocations
for experiments. Neither identity agreement nor successful simulation proves a
valid allocation.

Bundle a separate constraint artifact from the authenticated source archive.
Preserve the original identity map and captured-character guards. Generate a
bounded mixed-radix space of explicit rank alternatives, validate each complete
allocation, and export only accepted content-addressed talent references. Bind
resume to class, level, canonical base, ordered domains and the rule hash.

The guard enforces rank maxima, the level-based point budget, five earlier-row
points per row, and fully ranked prerequisites. The builder checks unique
locations and identities, prerequisite targets, earlier/same-row dependencies
and acyclicity. Native eligibility and model accuracy remain unqualified.

Use the exact pinned upstream TypeScript checker as an independent test oracle.
Keep generation, output and source sizes bounded. Do not add simulation, network
fetches, file writes, character selection or game actions to the generator MCP.
