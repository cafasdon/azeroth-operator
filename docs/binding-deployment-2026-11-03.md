# Binding development deployment — 2026-11-03

Merged source `90740e8ce600030a2a2779da92c23871e311858b` built and tested in the Linux release container. Personal pylon CT290 reader and importer now use `sha256:1a8442741b46994c01fc359bb9310c1afdaca9cf31c0bce64b965c2ff582f089`. Importer qualification preserved legacy snapshots and accepted binding state, contexts and new receipt enums in an isolated store. Reader receipt: `/var/tmp/azeroth-reader-upgrade-o7l5v_02`. Loopback authentication/tool checks passed; independent public TLS SDK discovery verified nine tools.

Windows companion `3e28a5f79582549b40a81dd446551dd9537d165c6e3531f5b2305405cce2ace0` installed in its hash directory, selected in sync configuration. Prior configuration retained as `local-data/sync/config-before-bindings.json`.

Retail addon 0.3.0-dev installed from ZIP SHA256 `b679d7a90a9bb1ccbf7ef5a568b5be85913ebebdfb44bf10ec39071abdb85da6`. Packaged Lua verification passed, and all nine installed files independently matched. Previous addon retained at `local-data/addon-backups/addon-update-xqis1sru`. No live keybindings changed. Player reload/export requested for actual context/set observation and a no-change live check. A no-op alone will not qualify SetBinding/SaveBindings; a later bounded reversible change needs its own evidence.

Advisory claims check: Jev `jev-1.14.0`, request `e730ab5b3587802b1166fb1c24dabf795ae631bfe0a7b7d074726c6d7c68e251`, 940 input/135 output tokens, 0.299s. Returned insufficient_evidence (confidence 0.62), not a pass. It does not qualify live operations or completion. Source tests, deployment receipts and public tool discovery support only their respective claims above. Macro creation remains live-verified; its deletion cleanup remains pending a newer player export.
