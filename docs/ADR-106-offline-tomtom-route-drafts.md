# Offline TomTom waypoint drafts

Date: 15 November 2026. Decision: separate route preparation from game execution.

The user wants addon-assisted quest and item navigation with multiple ordered
steps. TomTom can store multiple waypoints and accept bulk `/way` text, but its
closest-waypoint arrow selection is not an ordered quest controller. Arrival
also does not prove objective completion, item acquisition or quest turn-in.

Use a small data-only local MCP companion with a neutral numbered manifest and
TomTom manual-paste adapter. It has no game connection, addon callback, input
driver, SavedVariables writer or generated Lua. The user reviews coordinates
and pastes the generated text through TomTom. This does not enable any of the
seven blocked legacy Apply routes or create a general addon-control interface.

The safe default emits only the explicitly selected step. An optional all-pins
draft preserves input order and numbered titles, but never claims that TomTom
will follow that order. A future ordered arrow controller needs its own source
review, explicit progression controls and native acceptance; it is not enabled
by this draft feature. No distance callback marks a quest step complete.

Coordinates must be explicit native UIMapIDs with percent x/y values. Provider
area IDs and spawn coordinates are not silently translated. Metadata declares
native coordinate validity, accessibility, runtime addon availability and arrow
sequence enforcement unverified. Five fractional percent digits are rendered;
both requested and rendered values and a rounding flag remain visible.
The manifest also separates paste labels from predicted parser-normalised labels
for both numeric locales. Actual game display remains unverified.

Input limits and strict manual validation prevent additional commands and
unbounded responses. Titles/labels reject controls, format characters, line
separators and WoW hyperlink markup. Whitespace is normalised before hashing.
Fixed numeric command prefixes, route hashes and step numbers keep each pin
distinct, including a return to the same coordinates. No reset-all, sharing,
movement, objective action or automatic advance is generated.

The installed Forever TomTom v4.4.1 declares interface 16001 and documents this
format. Its exact file hashes are recorded in
`clients/forever/tomtom-format-evidence.json`. No TomTom source is bundled or
redistributed. An optional isolated Lua test reads the hash-qualified installed
slash module and replaces native services with fail-closed mocks. That test
qualifies command parsing only, not native placement or arrow control.

See [usage and evidence](tomtom-route-drafts.md). This source review admits only
offline preparation; it is not Blizzard approval or a guarantee of compliance.
