# Equipment development deployment — 2026-11-03

PR #1 merged as 288c6c469d677d3f6175d27688f55947463f1948. Build source 939b28c has an identical tree (git diff passed). Linux release tests and build passed on personal pylon CT290 (mcp-gateway).

Reader and importer image: sha256:9090d8d0f758ef46c4ecc0b648f9f0dd706038d1caa806e01783235d64f3fbe7. Importer isolated legacy and extended fixtures passed, including item GUIDs and equipment receipts. Reader rollback evidence: /var/tmp/azeroth-reader-upgrade-d4fv7o9g. Loopback auth checks and independent public TLS SDK discovery passed: ten tools. This verifies service availability, not live equipment execution.

Windows companion SHA256: 5cd286f2348dd407dc1e00cc648839f371a8f428720dd517582287d0a35bc997. Installed in its hash directory and selected in sync configuration; prior configuration retained at local-data/sync/config-before-equipment.json. Sync reports the prior snapshot unchanged; that is not a fresh remote readback.

Retail addon 0.4.0-dev installed after hosted schema upgrades. ZIP SHA256 1f2f078810df5745507c6626d8257d59cbd49e23a07f36da73ec73eb5ea41b2f. All eleven installed files independently match the manifest. Backup: local-data/addon-backups/addon-update-hzts13q6.

Player reload/export/reload requested to capture actual item GUIDs. No gear change performed. Live swap qualification remains outstanding; Classic/Forever mutation support and multi-slot operations remain incomplete.

Live GUID export verified through the public MCP on 2026-11-03: snapshot d4bd0612e55e1c0954c3d3d2b117e8e3e254b08372534b3df09da1e85685c76c contains GUIDs for all 17 equipped items and all 77 occupied carried slots. Private readback evidence: local-data/live-qualification/item-identities.json. This qualifies identity capture, upload and hosted schema readback; no gear swap was performed.
