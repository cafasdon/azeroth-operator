# Local public catalogue search

Install optional dependencies in a project environment.

```powershell
python -m venv --system-site-packages .venv
./.venv/Scripts/python.exe -m pip install -r requirements-knowledge.txt
./.venv/Scripts/python.exe -m tools.sync_forever_embedding --output local-data/knowledge-model
./.venv/Scripts/python.exe -m tools.forever_knowledge build --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model
```

The existing catalogue sync command supplies the pinned catalogue.
Model sync is explicit. Build, search and MCP serving use local data.

```powershell
./.venv/Scripts/python.exe -m tools.forever_knowledge search --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model --query "Garb of Thero-shan" --section items --limit 5
./.venv/Scripts/python.exe -m tools.forever_knowledge serve --catalogue local-data/forever/catalogue.json --index local-data/knowledge/index.sqlite --model local-data/knowledge-model
```

For an MCP client, launch the second command from this repository's root,
using absolute paths for the interpreter and all three configured inputs.
Its tool is `search_forever_knowledge`. Allowed section filters are
`items`, `npcs`, `zones`, `factions` and `consumables`. Limit is 1–20.
Use returned section and record IDs with the main catalogue tools for
exact facts. Similarity scores describe retrieval.

The index contains public reference records. Its builder accepts the pinned
catalogue and verifies its checksum. Search makes no source refresh or game
call. Personal snapshot data stays in the existing owner-scoped store.
