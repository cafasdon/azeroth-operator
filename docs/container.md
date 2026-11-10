# Linux MCP container

The Dockerfile builds the release executable and copies it into a Debian runtime. Run scoped tests directly before deployment. Official base manifests are pinned.
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
initialisation, exact tool discovery and capability inventory, read-only/non-destructive
annotations, all three family listings, traversal refusal, missing-snapshot
rejection by all five snapshot-consuming tools and unknown mutation-tool refusal. It requires an empty snapshot store and the
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

The standalone profile generator is tools/gateway_profile.py. Earlier version-one
output was accepted by CT290 Docker MCP. Current generation requires both an
immutable image and an explicit dedicated snapshot directory on the Linux Docker
host, for example:

```text
python -m tools.gateway_profile --image sha256:<64-lowercase-hex-digits> --store /srv/alice/snapshots
```

It emits only Azeroth read tools and that one read-only store at /data. It has no
user/environment fallback and neither reads nor modifies the homelab profile.
Paths must use normalised absolute Linux syntax with ASCII letter/digit/dot/
underscore/hyphen components; root, traversal, placeholders and volume options
are rejected. Syntax validation does not prove host filesystem ownership or
permissions. Assign the installing user's private store outside any incoming
payload and check its authentication/mount boundary before deployment. Current
source changes do not prove Docker mount acceptance, authentication or routing.

Public route verified 2026-11-03: https://mcp.cafas.com.br/scopes/azeroth/mcp.
Dedicated bearer authentication is required; no credential is stored in source.
Seven read-only snapshot/analysis tools are exposed, including equipment review and captured talent loadouts.
The service override and network policy are owned by cafasdon/Proxmox (PR197/199).
CT150 isolation covers 8823 alongside 8765–8821, with 8822/8824 excluded.

Reader image after the carried-item review release:
`sha256:5f434aa2aede0bcd774d9f726c19435c1add1e804bdad5c8c417204012bca196`,
Rust source `cf24f3844e5dec97556aeaf2a15bf7dbb29cf5c1`.
The restricted importer retains sha256:5a3467d27b7c238a00d9113e0273881106bcfab0d6e8c81e68f27767c02ce02c; schema is unchanged.

`deploy/upgrade_gateway.py --expected-image <current-id> --image <candidate-id>`
checks the dedicated profile identity, exact old image, fixed read-only store and
environment before replacing it. It retains before/after profiles under a private
temporary directory, checks the imported tool allowlist, restarts only the dedicated
service, waits for its socket and probes authentication and discovery. The
probe also checks that the gateway advertises the current optional snapshot
inspection fields. A failed verification stops and disables the dedicated
reader and retains the previous profile for diagnosis. Restoring the old
write-code routes would bypass the policy gate.
Run independent public readback with
`deploy/probe_gateway.py --public --require-current-inspection` before
claiming release qualification.

## Offline policy and owner-scope cutover, 7 November 2026

The dedicated personal pylon CT290 reader was built from `89a7ce0` as image
`sha256:5b8ad4e538bf6493eadb490531895cf8f7a0eaa4b7f1e42498f9ad216cba4d4c`.
The network-disabled candidate passed stdio checks. The guarded upgrade and
public TLS probe passed with seven offline tools, seven direct preparation-route
denials, and HTTP 401 for missing and incorrect credentials. The authenticated
reader listed its configured owner store. Its mount remained read-only, the
snapshot directory was mode 0750, and the credential file was mode 0600.
The service was active and enabled after cutover. No importer image or game
client was changed. The prior profile retained under
`/var/tmp/azeroth-reader-upgrade-vulz9ffy` is evidence only and would reintroduce
the retired routes if restored.

Adversarial review exercised callers and failure/security paths. The fixes
covered reboot-safe denial after failed deployment, explicit unknown-tool
verification and suppression of filesystem paths in Python reference errors.
Jev advisory triage for the cutover findings used `jev-1.14.0`, request
`65ecad06b92d09613492f36afda365541681235ba660a3b4437c39696ac49713`, 450 input
and 46 output tokens, 0.281 seconds.

## Forever snapshot rollout, 7 November 2026

Personal pylon CT290 now runs the Forever-compatible reader and importer image
`sha256:73dafe7bb0c0e8c905d9d5b5639ad4a3ef4ee2fb122e876f795c233c7103d738`.
Image source `13961cb`; reader deployment script `77d7f17`. Reader changed first
and passed its authenticated loopback probe. Importer qualification round-tripped
legacy Retail, extended Retail and the nine-section Forever fixture in an isolated
store, then verified the updated pin. A local real MCP readback matched every
Forever fixture section and returned the expected one-item recipe shortfall.
No synthetic snapshot was added to the owner store.

The independent public TLS probe verified eight read-only tools, direct denial
of seven legacy preparation routes, and HTTP 401 for absent and incorrect
credentials. The connected MCP also reported `review_forever_recipe` and the
configured-owner policy. Local clients with cached tool discovery need to reconnect.
The reader remains enabled. Previous profile evidence is under
`/var/tmp/azeroth-reader-upgrade-w6v38_z1`.

Failure review caught a retry that could leave a healthy reader disabled at
boot. The updater now enables the reader after the authenticated probe and
checks enabled state. Follow-up failure review verified the ordering and
fail-closed path. Jev triage classified the finding as material
(`jev-1.14.0`, request `58ada5890fc6e9830fc800772b90f35bdb45d2e0c687b7bf98e85a29f3ec746a`,
426 input/47 output tokens, 0.305 seconds).

Forever build `1.60.1.70862` was identified by the local metadata probe.
Azeroth Operator was absent from the Forever addon directory. No game client,
addon installation or Windows sync companion was changed.
