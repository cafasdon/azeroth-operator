# Linux MCP container

The Dockerfile runs Rust/Lua/CLI tests during its build, then copies only the
release executable into a Debian runtime. Official base manifests are pinned.
Cargo.lock is enforced. The runtime uses uid/gid 10006 and stores snapshots in
/data. No listening port or network access is needed for stdio operation.

Build: `docker build -t azeroth-operator:qualification .`

Run with `docker run --rm -i --network none --read-only --cap-drop ALL
--security-opt no-new-privileges azeroth-operator:qualification` for empty-store
qualification. Production discovery should use an immutable image ID and a
read-only /data bind mount populated by the separately authorised importer.
The importer requires a separate writable invocation; do not grant write access
to the read-only discovery service merely to simplify ingestion.

`python tools/smoke_mcp.py -- <command> [arguments]` exercises real stdio MCP
initialisation, exact tool discovery, read-only annotations, all three family
listings and traversal refusal. It requires an empty snapshot store and the
development Python MCP dependency. It can launch SSH plus a remote container.

This container is a gateway backend, not a public HTTP server. Scope routing,
authentication, owner-isolated storage and authenticated Windows uploads still
need integration and qualification. No public endpoint is claimed by this build.

## Upload primitive

`azeroth-operator import-stdin` reads one bounded JSON or SavedVariables export,
validates it and atomically publishes it to the configured data directory. It
prints the content hash only after successful publication. Repeating identical
input returns the same hash without creating another snapshot. Invalid input
returns a non-zero status and no success receipt.

This is a CLI primitive, not a remotely exposed MCP tool or an authenticated
endpoint by itself. A deployment must authenticate the sender, assign its store
outside the payload, impose a wall-clock deadline, and provide a writable mount
only to the importer. The producer must close stdin after sending one export.
The read-only MCP worker must see that same owner's data through a read-only
mount. Do not use a shared writable store across principals.

The standalone profile generator is tools/gateway_profile.py. Its version-one
output was accepted by CT290 Docker MCP. It selects only Azeroth tools and a
fixed read-only Rodrigo store; it neither reads nor modifies the homelab profile.
Acceptance does not verify container mounts, authentication or public routing.
