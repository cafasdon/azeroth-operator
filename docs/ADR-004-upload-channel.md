# ADR 004: separate SSH upload credential

Date: 2026-11-03. Status: implementation candidate; not provisioned or live-tested.

The read-only MCP bearer must not acquire upload authority. Use a dedicated SSH
key whose server-side authorised-key entry has `restrict` and a fixed forced
command: `/usr/bin/timeout 70 /usr/bin/python3 /opt/azeroth-upload/receive_upload.py`.
No shell, forwarded ports, agent forwarding, PTY or user rc is permitted. The
receiver also rejects SSH_ORIGINAL_COMMAND and SSH_TTY. Provisioning must verify
these restrictions by attempting prohibited operations, not merely inspecting text.

The Windows Rust command `azeroth-operator upload <export-file>` decodes and
validates locally, serialises canonical JSON, and sends it to the fixed
`azeroth-upload` SSH alias with strict host-key checking and batch mode. A 60-second
deadline owns the SSH process; errors/timeouts are unknown outcomes, never success.
Only an exact matching SHA256 receipt plus successful process exit confirms upload.
Identical retries deduplicate via the existing content-addressed store.

The alias lives in a dedicated config supplied by the absolute local
AZEROTH_UPLOAD_SSH_CONFIG path. The client uses `ssh -F` so ordinary administrator
identities cannot become a fallback. Configure only the upload IdentityFile,
IdentitiesOnly=yes and IdentityAgent=none, with the existing verified known-hosts
file. Missing configuration fails before launching SSH.

On personal pylon, the forced receiver reads at most 4 MiB with a read deadline,
then invokes one pinned CT290 container with no network and a fixed owner store.
No client path, command, account or image selection is accepted. Image configuration
must be root-owned, non-writable by group/others and not a symlink. The reader MCP
keeps its read-only mount. This single-owner channel does not claim multi-user support.

Before activation: refresh the Linux image to include import-stdin, provision the
restricted key and pinned alias without exposing private material, verify denied
commands/forwarding, upload/retry a fixture, verify via public MCP and remove it.
No HTTP upload endpoint, game input or in-game command channel is introduced.
