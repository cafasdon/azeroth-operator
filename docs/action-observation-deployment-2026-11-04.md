# Action observation deployment — 2026-11-04

Source96c72f104652d149304f69e345625e8058cc0f1a (PR12). Personal pylon/CT290 mcp-gateway identity reverified;96GiB filesystem,31GiB free at precheck.

Hosted reader and importer now pin sha256:32d4768980f8fe2d63bfa770c8aedfd87a9f9df893e3979af8263be2266c4df0. Linux release tests/build passed. Importer legacy/extended fixtures passed including action_bar_observation. Dedicated reader profile upgrade and loopback auth/protocol probe passed; rollback evidence /var/tmp/azeroth-reader-upgrade-s88ng8jz. Independent public TLS SDK discovers13tools. Existing real snapshot184cd8a9ffd714eb7a5f33f4b661ba15e542c117fd483acef5b65e041b15f3c8 still reads correctly, preserving unresolved bag receipt and source identity.

Windows release companion863492ae34d882682fb5d16c12e9b229900b1880301e32fb464a6935336e72f0 installed in content-addressed runtime; config backed up as local-data/sync/config-before-action-observation.json. Scheduled task unchanged.

Retail addon0.8.0-dev installed;15files independently match manifest. ZIP4bda579eac15d3eec124c9fdb9278160f55402f92d8e02a980c1aff96fdcab8c passed packaged Lua loading/export and changed-interface rejection. Previous files backed up at local-data/addon-backups/addon-update-hv3uvuog. The package includes observation only, not ActionOperations.lua or ActionImport.lua. MCP preparation explicitly reports addon_execution_enabled=false.

No game input or retry. Running-client reload, real action observation and live action operations remain unverified. User deferred in-game testing; no new prompt was issued.
