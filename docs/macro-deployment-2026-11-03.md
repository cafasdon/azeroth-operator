# Macro operation development deployment — 2026-11-03

Source `4d191042013cab861048ff8e013af665dce69ae9` is merged into main and deployed on personal pylon CT290 (`mcp-gateway`). Reader and importer use image `sha256:163b7fde17b5c27a6ab5d8a22c7ed13304e6469baa542e97da44bce4a4a9a323`. Linux release tests passed during the image build. Importer qualification verified legacy and receipt-bearing snapshots in an isolated store. Reader upgrade receipt: `/var/tmp/azeroth-reader-upgrade-qyt_gee5`.

Loopback and public authentication probes passed: missing/wrong tokens returned 401. Actual public MCP discovery returned eight tools. `prepare_macro_operation` compiled a blank character-scoped `AO MCP Test` operation from the existing Retail snapshot. This generated data only; no game input or macro mutation occurred. The deployed probe's printed count was a stale literal seven even though its assertion checked all eight names; the source diagnostic is corrected in the follow-up commit.

Windows companion SHA256 `9edc8a1aa6b011df96a7e8c65e3856035cc4d2179f98c72b6c989baa75c3da27` is installed under the hash-named local runtime directory and selected in the sync configuration. Previous configuration is retained as `local-data/sync/config-before-macros.json`.

Retail addon `0.2.0-dev` ZIP SHA256 `bae98cfc6aeb113885bc04c6cb06c2acabecb7cc03d2e8346a34b8a4eb51a0a7` passed packaged Lua5.1 loading and was installed over the hash-matched `0.1.5-dev` release. Seven installed files independently matched the manifest. Backup: `local-data/addon-backups/addon-update-yla3warm`. An initial verification invocation used a relative manifest path and was rejected; repeating with the required absolute path passed.

Live qualification remains pending: the player must reload, paste the prepared code into `/aomacros`, review and apply, then export/reload. No computer-use automation was used. The local test import code is in ignored `local-data/macro-test-import.txt`. Macro edits/deletion/scope moves require their own bounded live qualification; Classic and Forever execution remain unsupported. This deployment does not complete the broader operations goal.
