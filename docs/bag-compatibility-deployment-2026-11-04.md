# Bag compatibility deployment — 2026-11-04

Source `5272ffb996cbc0a03c02b91f2c2b2ed6e3dbe8d6` (PR79). Personal pylon/CT290 identities, old importer pin/active reader and installed0.19.0-dev35file hashes verified before changes. Linux release tests/build passed.

Importer/reader image `sha256:87935f7bb3f71d40c547197fd829c14a774377c4d792bc5d3e748af1b50ac814`; prior image `sha256:15fc58e33abe25ea3fcc659e43d020f9363a0851f35a4bae7d77c3dd7de8a73d`. Isolated legacy/extended importer fixtures and guarded reader profile/read-only mount/authenticated loopback checks passed. Post-check confirmed active service/pin. Reader rollback `/var/tmp/azeroth-reader-upgrade-w6axjbw1`; source `/var/tmp/azeroth-source-5272ffb`; image ID `/var/tmp/azeroth-import-5272ffb.iid`. No unrelated profile changed.

Public TLS SDK verified14tools using existing synthetic snapshot `2c6a7903e7ee68327d6cee50b393b1346be38f5e92239a852f3da5f96c533c61`. Moves/splits/swaps advertise0.20.0-dev; swap invalid-request refusals, legacy formats, action-copy/move, receipt readback and unchanged observations passed. Evidence `local-data/bag-compatibility-public-check.json`; helper `local-data/check-bag-compatibility-release.py`. These checks do not execute native preflight or item movement.

Companion SHA256 `495eecf1b2fdaf6bc339bf4c323269a75f1b2c11922545fc506aa8b9706da9cf` hash-verified and atomically selected; config backup `local-data/sync/config-before-bag-compatibility.json`. Sync2026-11-04T02:27:48.377571Z: Retail unchanged at `184cd8a9ffd714eb7a5f33f4b661ba15e542c117fd483acef5b65e041b15f3c8`, remote_rechecked=false; Forever waiting_for_export. Cached acknowledgement is not fresh remote character verification. Scheduled task unchanged.

Retail0.20.0-dev installed; all36files independently match manifest. ZIP `dist/bag-compatibility-release/AzerothOperator-0.20.0-dev-retail-live.zip`, SHA256 `5f84162696f78e44469d5cdebcd5cfad3c5a851175d258532eb40b91b7dd339a`; backup `local-data/addon-backups/addon-update-krr1hv_q`. Running game may retain an older addon until player reload/restart. No game input or test prompt occurred; uncertain Cutlass move not replayed.

Native metadata availability/eligibility semantics, taint, timing and actual placement remain unqualified. Same-ID stack handling and broader operation-matrix coverage remain incomplete; Classic/Forever mutations disabled.
