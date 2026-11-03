# Captured Retail talent strings

`captured_talent_loadout(snapshot_id)` returns the original talent import string
recorded during a player-triggered addon export. It does not generate an optimised
build, alter talent nodes or invoke any game operation. The output remains bound
to the source client/build, character, capture time and snapshot hash.

The Retail collector checks the namespaced specialisation APIs,
`C_Traits.ConfigHasStagedChanges`, `GetLoadoutSerializationVersion` and
`GenerateImportString`. Missing APIs, staged changes, restricted values or failures
produce an explicit unavailable reason while retaining the other talent data.
Classic and Forever return `client_not_supported` for this export path; their
existing observed trait data is not reinterpreted as Retail import strings.

Schema1 gains an optional `talents.loadout_export` object. Existing snapshots
remain valid. Older strict readers reject the new fields, so upgrade reader,
importer and local companion before installing the new addon. Available exports
contain format `blizzard_traits`, original `import_string`, `spec_id`,
`serialization_version` and `staged_changes:false`. Unavailable exports contain
only status/reason. Ingestion checks types, bounded base64-like alphabet and
absence of staged changes. It does not validate the encoded header/tree hash or
prove the string will be accepted by a current client.

Review current character, specialisation and game build before using the game's
talent-import UI manually. The MCP never applies the string, imports an arbitrary
command, changes talents or treats an API's existence as permission to automate
gameplay. Live compatibility and optimisation remain unverified.

Evidence: Blizzard-generated API declarations mirrored in
[SharedTraitsDocumentation](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/SharedTraitsDocumentation.lua)
and [SpecializationInfoDocumentation](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpecializationInfoDocumentation.lua),
consulted2026-11-03. These establish API signatures, not compatibility with every
installed build or general authorisation for game writes. The fixture string
`QUJDRA==` used in tests is synthetic, not a usable talent build.
