# ADR-072: Personal group context without roster capture

Date: 2026-11-09. Status: offline source integration.

Add Forever-only group_context to passive manual exports. Coverage is
reported_player_group_context with one or two named categories, home and instance.
Explicit category constants are required; missing/unreadable constants do not
fall back to default-group calls. Colliding category IDs are ambiguous and cannot
establish either category. No native availability is assumed from a constant.

Each independently readable category contains in_group, in_raid and
reported_member_count, plus optional player_is_leader. Two full projections must
match. Optional leader failure does not discard otherwise readable group data.
Required global API, restriction-guard or family changes invalidate capture;
unsafe/missing category observations remain unavailable, not solo or zero.
Counts are raw u32 values, not a roster-size guarantee; no max-40 or count-to-flag
relationship is invented. Flags do not prove group identity continuity.

Pinned UI source at 9dc84f0d3bbae1768ac3405b3716000bd93eda06 uses explicit home
category calls in Blizzard_GroupFinder/Mainline/LFGList.lua and LFDFrame.lua.
Blizzard_APIDocumentationGenerated/UnitDocumentation.lua declares the optional
partyCategory argument and restricted bool return of UnitIsGroupLeader. These
inform API usage; instance-category native qualification remains open.

Strict owner-local Rust/Python readers reject unknown categories, names, GUIDs,
roster fields and malformed values. MCP inspection and numeric/boolean summary
keep categories separate. Current group, group identity, complete roster and
social-action eligibility verification stay false. Generic historical comparison
can show changed fields without inferring joins, departures or invitations.

Do not enumerate members, inspect other units or collect chat/invites. Do not
invite, kick, leave, convert, set roles, ready-check, vote or queue. Player-only
leadership is a saved observation, not authority to perform any social action.
Live testing, installation and deployment remain deferred.

Set `group_context_changes` to `true` on `compare_snapshots` to see both saved
observations and a signed member-count delta for each category. The delta is
available when both captures report that category. Missing observations remain
null. The same-character, same-client and chronological checks apply. Group
identity and membership events remain unverified.

Offline MCP tests exercised missing categories, the full signed difference of
two u32 member counts, reversed timestamps, different characters, mixed clients
and non-Forever requests. The default comparison retains its existing shape.
Jev 1.14.0 advisory `43f478ae404239e9a15f5e10ff421cfe39654032bec77adc22cee4d03067a25b`
favoured revision at .25 confidence before the additional boundary cases ran.
It used 418 input and 48 output tokens in .343 seconds.
Independent review also verified both signed delta extremes, absent sections,
and rejection on name, realm, build and variant mismatches.
