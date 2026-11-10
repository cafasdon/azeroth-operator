# Explicit snapshot store for standalone profile generation

Date: 10 November 2026. Status: source/offline only.

The generic profile renderer previously defaulted to Rodrigo's snapshot store.
A new installation could therefore render a different user's path without an
explicit choice. Remove that default: profile(image, *, store) and the module
CLI now require a store. Do not infer it from usernames, environment variables,
home directories, current working directory or the shared Docker profile.

Accept a literal, normalised absolute Linux host path with at least two
components, bounded at 4,096 characters. Components use ASCII letters, digits,
dots, underscores and hyphens. Reject root/top-level paths, dot/parent traversal,
duplicate separators, trailing slash, controls, colons, backslashes, shell/config
placeholders and other unsupported syntax. This is a renderer contract, not
proof of filesystem existence, canonical symlink target, ownership or permissions
on the remote Docker host. Space/Unicode paths are unsupported rather than
silently rewritten. Errors do not echo a supplied path.

Emit exactly one selected source mount at fixed /data:ro and fixed
AZEROTH_DATA_DIR=/data, retaining immutable image validation and read tools.
The renderer neither opens the store nor reads/writes an installed profile.
Each profile result has independent tool and volume lists. One dedicated
standalone profile remains one owner boundary; this adds no multi-tenant routing
or identity authentication and does not authorise access to another user's path.

The existing personal CT290 upgrade script keeps its exact deployment owner
pin as a local constant and passes it explicitly to the renderer. Its current
mount/environment/image guards remain unchanged. A generated profile for another
owner is still refused by that upgrade check. Importer/authentication mappings
remain separately deployment-specific; no live path, token or data moves here.

This intentionally breaks renderer callers that omit store. Repository callers,
tests and current container guidance now supply it explicitly. Prior dated
deployment evidence describes the old generator and is not rewritten.
Installation, upgrade execution and live qualification remain deferred.
