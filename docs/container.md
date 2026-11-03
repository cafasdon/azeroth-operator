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

This container is a stdio gateway backend. The separately deployed Docker MCP
Gateway supplies authenticated public HTTP access and the fixed owner store.
Container-build success alone does not qualify that deployment.

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

Public route verified 2026-11-03: https://mcp.cafas.com.br/scopes/azeroth/mcp.
Dedicated bearer authentication is required; no credential is stored in source.
Six read-only snapshot/analysis tools are exposed, including equipment review.
The service override and network policy are owned by cafasdon/Proxmox (PR197/199).
CT150 isolation covers 8823 alongside 8765–8821, with 8822/8824 excluded.

Reader image after the equipment-review release:
`sha256:823a4a9111ab8814febf8b3646768398f110dfe004ae5bfe9f58937e4a87a163`,
Rust source `b008768febf79ece1f1917b7a18a1fae0d6959e8`.
The restricted upload receiver retains its separately pinned importer image.

`deploy/upgrade_gateway.py --expected-image <current-id> --image <candidate-id>`
checks the dedicated profile identity, exact old image, fixed read-only store and
environment before replacing it. It retains before/after profiles under a private
temporary directory, checks the imported tool allowlist, restarts only the dedicated
service, waits for its socket and probes authentication/discovery. Failure attempts
to restore the previous profile; rollback health must still be inspected.
Run independent public readback before claiming release qualification.
