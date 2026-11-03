# Action-operation deployment — 2026-11-04

Source7c197557322c129b55e241a7b12d5dbdad3e821d (PR15). Verified personal pylon/CT290 mcp-gateway and prior image before changes. Linux release tests/build passed.

Importer/reader image sha256:c78c503f495c8f9b00cd89db8b8add0437fd8ff22bee590a505492824411ce01. Legacy/extended importer fixtures passed, including move_action_slot receipt and action observations. Dedicated reader profile/loopback probe passed; rollback evidence /var/tmp/azeroth-reader-upgrade-qu7yuevs. Independent public TLS SDK discovers13tools.

Windows companion SHA256404d55d6c3fa076bc3bea21f00f1d9e16ee6f13b5bd115ae965b381f341d131b installed in content-addressed runtime. Sync configuration backup local-data/sync/config-before-action-operations.json. Scheduled task unchanged.

Retail0.9.0-dev installed:17files independently match manifest. ZIP SHA256307c3d3e7baf0f00fd62abdea83009e7084542d2f1b9475a69ea747c0284785a. Backup local-data/addon-backups/addon-update-u5ok_4w6. Action executor/decoder now included. Requires client reload/restart to load; no automatic input generated.

Player-applied action-slot moves/swaps are development-supported, not live-qualified. No action attempted, no pending bag operation replayed. Actual action observation export, cursor/protection behaviour and real slot changes remain unverified; user deferred testing. Existing macros/bindings/gear/bag/talent qualification limits remain unchanged.
