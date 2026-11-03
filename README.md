# Azeroth Operator

Local character analysis and player-operated configuration tools for World of Warcraft.

## Client subdivisions

- `clients/retail/`: Retail adapter, addon and capability evidence.
- `clients/classic/`: Classic adapter, addon and capability evidence. Classic variants must be identified explicitly; Era and progression clients are not interchangeable.
- `clients/forever/`: Forever adapter, addon and capability evidence.
- `shared/`: client-independent snapshot contracts, policy enforcement and planning.
- `mcp/`: shared MCP interface, requiring an explicit client identity.
- `tests/`: shared tests and separate client qualification suites.

Status: read-only stdio MCP foundation implemented and integration-tested. No live game client qualified yet.

Hosted read-only MCP: `https://mcp.cafas.com.br/scopes/azeroth/mcp`, using a dedicated
bearer token. A separate restricted SSH credential supports the Rust CLI
`azeroth-operator upload <export-file>`. Configure the absolute
`AZEROTH_UPLOAD_SSH_CONFIG` path to its dedicated SSH config; normal administrator
SSH configuration is never used as a fallback. Uploaded bytes are validated and
confirmed by matching content hashes. Automated file monitoring is not installed.

The companion is Rust; in-game adapters use Lua. Build with `cargo build --locked`. Run `target/debug/azeroth-operator.exe` for stdio MCP. Set `AZEROTH_DATA_DIR` to an absolute snapshot directory; the default is `local-data/snapshots` relative to the process working directory. Tools: `capabilities`, `inspect_snapshot`, `list_snapshots` (explicit family, optional offset/limit).

Import a JSON snapshot or the addon's narrow SavedVariables export with `target/debug/azeroth-operator.exe import <export-file>`. Import is a local CLI operation, not an arbitrary-path MCP tool. It reads data only; never executes Lua. See `addon/README.md` for the export workflow and outstanding live-client qualification.

Analysis tools: `character_summary` reports observed counts and unknowns; `compare_snapshots` requires the same character and exact client/build, rejects reversed capture order, and returns at most 100 changed JSON paths per section with truncation metadata. Missing sections are not treated as empty inventories. These are snapshot observations, not recommendations about optimal equipment.

Synthetic performance baseline: `cargo run --release --locked --example benchmark` measures warm-cache listing and inspection over 200 generated snapshots. It uses no account data.

The Python prototype is retained temporarily as a reference and independent protocol test harness. Install its test dependencies with `python -m pip install -r requirements.txt`. Set `AZEROTH_TEST_BINARY` to the Rust executable to test that server instead of the prototype.

For a local development addon ZIP: `python tools/package_addon.py --family retail --variant live --interface 120105 --version 0.1.0-dev`. Explicit client metadata is required. Generated `dist/` files are ignored; manifests include per-file and archive hashes. These packages are development artifacts, not certified compatibility claims.

## Boundaries

Read ordinary addon exports and official API data; analyse and prepare player-reviewed imports. No autonomous gameplay, computer use, memory access, injection, packet interception, protected-action bypasses or arbitrary Lua execution. In-game writes remain disabled pending operation-specific policy review and client verification. A callable API is not proof of Blizzard authorisation.

No character data, account identifiers, credentials or local exports belong in Git.

## Verification

Run `cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, and `python -m unittest discover -s tests -v` from the project root. Set `AZEROTH_TEST_BINARY` as above for Rust protocol verification. These checks do not establish runtime game compatibility.
