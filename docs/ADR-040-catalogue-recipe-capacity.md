# ADR 040: retain selected recipe capacity semantics in acquisition lookup

Date: 9 November 2026. Status: source/offline; no deployment.

Crafted acquisition sources project selected-recipe finite material capacity,
output and raw cooldown observations. After ADR 039, null finite capacity alone
does not distinguish explicit no-item requirements from unavailable material
evidence. The selected acquisition projection also omits combined charge limits
and shared captured-state classification, while bulk nested reviews retain them.

Project exactly three additional fields from the existing matching selected
review: `material_capacity_basis`, `material_and_charge_capacity` and
`captured_cooldown_state`. Reuse the existing one-craft review and recipe-ID /
known-output mismatch guards. No selected match produces nulls. Unknown output
identity remains unknown under the existing ID match contract; it is not promoted
to a confirmed catalogue output. Do not copy bulk data into these selected fields.

Preserve learning conflict handling: these are historical selected observations,
not an eligibility verdict. Keep bulk review qualification and evidence separate.
Do not add raw recipe/name/character fields, requests, permissions or actions.
Current craftability, full acquisition coverage and native qualification remain
outside this source/offline projection.
