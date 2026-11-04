# Equipment-set association deployment — 2026-11-04

Source `51de2a77a4260fee49976986e37f2bdf5d09a02b` (PR35). Personal pylon/CT290 `mcp-gateway` identities, previous image and installed Retail 0.11.0-dev twenty-file manifest were checked before changes; CT290 had 31 GiB free. No unrelated profile or scheduled task changed.

Linux release tests/build passed. Hosted importer and reader now use `sha256:32a4039ed36090abcc35eae7a7cedf05082274a8dd29b1ed9b79b408f1eef942`. Legacy/extended isolated importer fixtures passed, including the spec map and assign/unassign receipts. Reader profile/read-only mount and loopback authentication/protocol checks passed. Rollback evidence `/var/tmp/azeroth-reader-upgrade-aaoab7b0`; source staging `/var/tmp/azeroth-source-51de2a7`.

Public TLS SDK checks discovered fourteen tools. Synthetic snapshot `1359d9e6288a0d4433ae48307c452488cb54d1c9e829ee087a5b8f920bc9a589` (`EquipmentSetSpecReleaseFixture`) was uploaded through the restricted channel. Assign/unassign preparation correctly bound spec IDs, reported displaced ownership, required addon 0.12.0-dev, refused an invalid spec index and left captured set data unchanged. The synthetic unknown unassignment receipt was readable. Local evidence `local-data/equipment-set-spec-public-check.json`; no prepared code was applied in game.

Windows companion SHA256 `4ca3502e851ae2d2f0516b221bc0bd0e5cb58fe34f1c92197d68805ca2beaec8` is installed in its content-addressed runtime directory and selected in sync config. Previous config: `local-data/sync/config-before-equipment-set-spec.json`. Sync at `2026-11-04T00:00:28.902059Z` reported Retail unchanged at `184cd8a9ffd714eb7a5f33f4b661ba15e542c117fd483acef5b65e041b15f3c8` with `remote_rechecked:false`; Forever waits for export. Cached acknowledgement is not a fresh remote verification.

Retail addon 0.12.0-dev installed after importer/reader/companion qualification. All twenty-two files independently match its manifest. ZIP SHA256 `90822ea609d68efcf591778b09bd2b4498e3fe892199dbb503045bfc46c00c6a`; backup `local-data/addon-backups/addon-update-3e_4am8j`. A running client may retain the earlier addon until the player reloads/restarts; no game input was triggered.

Spec identity capture and association mutations remain unqualified in the native client, as do paste/render limits and displacement semantics. Set creation/save and unresolved empty-slot semantics remain incomplete, alongside broader roadmap work. No in-game test occurred and the previous unknown Cutlass move was not replayed.
