# Profession capture isolation — 8 November 2026

Scope: source and offline qualification only. No installation, deployment,
game interaction or native capture was performed.

An unloaded `GetProfessionInfo` row previously exported an empty object.
The real Core exporter succeeded, but the Rust importer rejected the entire
snapshot with `Missing required field: name`, discarding valid equipment too.
The new regression reproduced that failure before the collector correction.

The shared collector now checks secret status before operations, validates
the existing reader contract (text up to 16,384 bytes; unsigned 32-bit ranks;
positive skill-line IDs), validates UTF-8 names without requiring a Lua UTF-8
library, and compares two reads including slot indices.
Sparse slots and a genuinely empty profession list remain valid. Invalid,
unloaded or inconsistent data throws within Core's section-level protection;
only a fixed unavailable reason is exported, never the API exception text.
Unused icon and other return values remain excluded. No gameplay actions
or API cache-load requests were introduced.

The public [Forever profession book source](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_ProfessionsBook/Blizzard_ProfessionsBook.lua)
confirms the `GetProfessions` positions and the name/rank/max-rank/skill-line
return positions used here. The six-slot shared contract is unchanged.

Verification:

- `cargo test --locked --offline --test profession_capture`: passed. Uses the
  actual Lua collectors and Core exporter followed by the Rust importer;
  checks sparse and empty captures, Retail equivalence, unloaded rows,
  invalid and restricted fields, API exceptions, changed indices/ranks and
  added/removed slots. Valid Unicode boundaries and invalid/truncated,
  overlong, surrogate and out-of-range encodings are covered. Invalid observations preserve equipment item 123 and
  never include the synthetic private marker.
- `cargo test --locked --offline --test profession_recipe_view`: passed.
- Development-only package built with `python -m tools.package_addon
  --family forever --variant live --interface 16001
  --version 0.0.0-profession-capture-test
  --output local-data/profession-capture-qualification`.
  Archive SHA-256:
  `94f311c4abf077770b9160ed24b5834dfaee6684ee77309067adce9df616da89`.

Independent caller and failure/security reviews identified malformed UTF-8
as another whole-export failure. The author reproduced `invalid unicode
code point`, added the narrow validator and boundary regressions, and
completed a second caller and failure/security review round with no further
concrete findings. The caller probe re-ran actual Core/import against five
invalid encodings: all now omitted professions, retained equipment and
imported successfully; valid Unicode and embedded NUL remained accepted.
No raw API exception or invalid name is
exported. An ad hoc reviewer probe was blocked by automatic approval review;
that action was not retried or bypassed. Ordinary repository regression
tests provided the author and failure-review evidence instead.

Jev advisory triage agreed to fix the reproduced encoding failure narrowly:
`jev-1.14.0`, request
`d08e7f49f87825c2d8a6782b01fc9d1f7936ad097db054f70d106f5644defb7b`,
512 input / 54 output tokens, 0.318 seconds. The independent reproduction
and tests, rather than the advisory probability, establish the defect/fix.

This fixes export isolation; it does not bind captured professions to a
simulation template or qualify native behaviour.
