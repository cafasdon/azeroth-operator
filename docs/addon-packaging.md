# Pinned addon packaging

Run from the repository root in Linux/WSL, with Python3, Bash4.3+, git, zip, curl,
awk, grep, sed and util-linux `unshare` available. The build must be able to create
an isolated user/network namespace; failure is an error, with no online fallback.

```sh
python3 -m tools.package_bigwigs --family retail --variant live --interface 120105 --version 0.1.0-dev
python3 -m tools.package_bigwigs --family forever --variant beta --interface 16001 --version 0.1.0-dev
```

These interface/variant values describe the development packages tested on
2026-11-03. Recheck installation/runtime metadata before building for a changed
client. Classic requires an explicit variant and interface; `era-fixture/11500`
was used only for a synthetic test, with no Classic installation claimed.

`tools/packager.lock.json` pins upstream BigWigs commit and release.sh SHA256.
The first run downloads that exact script over HTTPS and verifies its checksum;
later runs verify the cached copy. The build stages only the four addon files,
a manual development changelog and `.pkgmeta` in a new temporary Git repository.
No existing repository config, hooks, credentials or `.env` enter that staging.

The upstream invocation uses `-d -e -l -u`: no upload, external checkout or
localisation download, with Unix line endings. It runs with a cleared environment,
empty HOME, disabled global/system Git configuration and a network-disabled Linux
namespace. Subprocess timeouts kill their process group. Staging is removed after
the build. No WoW installation, SavedVariables or remote hosting is modified.

Every upstream ZIP entry must match the exact expected path and source bytes.
Unexpected paths/files, duplicate names, symlinks and oversized entries fail.
ZIP timestamps/order are normalised only after this verification; source payloads
are not replaced. Outputs under `dist/bigwigs` include SHA256 manifests with the
upstream tool pin, original ZIP hash, final ZIP hash and per-file hashes. Two
independent Retail runs produced identical normalised ZIP bytes on the tested WSL
environment. Cross-platform byte identity is not established.

To execute the actual packaged files in the Lua5.1 fixture (Windows shown):

```powershell
cargo build --locked --example verify_addon
python -m tools.verify_addon_packages --directory dist/bigwigs --verifier target/debug/examples/verify_addon.exe
```

The verifier confirms exported family/variant/interface and an equipment
observation, then changes the mock interface and proves capture fails while the
prior export survives. `live_qualified` remains false. This supplements the
eleven-section producer tests; it does not test the real game APIs.

For Forever packages, pass `--mcp target/debug/azeroth-operator.exe` to
`tools.verify_addon_packages` to import the packaged fixture's actual
synthetic export into a temporary owner store and read it back through
local MCP stdio. This optional check requires the Python MCP client.
It checks exact capture metadata, section coverage, bank item counts,
recipe quantities and a quest inventory checklist from that same export.
The temporary store is removed with the staged package files.

Packages remain development-only. Actual client exports and manager flavour
detection are still required before advertising a qualified release. No GitHub,
CurseForge, Wago or WoWInterface release is published by these tools.

Upstream references: [packager documentation](https://github.com/BigWigsMods/packager)
and [pinned source](https://github.com/BigWigsMods/packager/blob/0f24d2169de3ceb3b79a981311b2ebab4c6160c7/release.sh).
