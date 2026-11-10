"""Actual offline public-knowledge MCP traversal against the pinned local index."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools import forever_knowledge as knowledge


class KnowledgeWalkMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_pinned_source_chain_schema_bounds_and_no_network(self):
        root = Path(__file__).resolve().parents[1]
        python = root/'.venv/Scripts/python.exe'
        catalogue, index, model = root/'local-data/forever/catalogue.json', root/'local-data/knowledge/index.sqlite', root/'local-data/knowledge-model'
        self.assertTrue(all(path.exists() for path in [python, catalogue, index, model]), 'Local pinned fixtures required; no downloads')
        source = knowledge.read_catalogue(catalogue)
        npcs = {row['id']: row for row in source['npcs']}
        zones = {row['id']: row for row in source['zones']}
        selected = None
        for item in source['items']:
            if len(item.get('sources', [])) > 4 or len(item.get('randomSuffixOptions', [])) > 8 or len(item.get('effectIds', [])) > 8:
                continue
            for origin in item.get('sources', []):
                for value in origin.values():
                    if isinstance(value, dict) and value.get('npcId') in npcs:
                        npc = npcs[value['npcId']]
                        if npc.get('zoneId') in zones:
                            selected = (item, npc, zones[npc['zoneId']]); break
                if selected: break
            if selected: break
        self.assertIsNotNone(selected, 'Pinned item/NPC/zone chain required')
        item, npc, zone = selected
        key = f"items:{item['id']}"
        before = hashlib.sha256(index.read_bytes()).hexdigest()
        script = (
            'import socket,runpy,sys;'
            'original=socket.socket.connect;'
            "socket.socket.connect=lambda sock,address:original(sock,address) if isinstance(address,tuple) "
            "and address[0] in ('127.0.0.1','::1') else (_ for _ in ()).throw(RuntimeError('External network forbidden'));"
            "sys.argv=['forever_knowledge','serve',*sys.argv[1:]];"
            "runpy.run_module('tools.forever_knowledge',run_name='__main__')"
        )
        with tempfile.TemporaryFile(mode='w+', dir=root/'local-data') as errors:
            async with stdio_client(StdioServerParameters(command=str(python),
                    args=['-B', '-c', script, '--catalogue', str(catalogue), '--index', str(index), '--model', str(model)],
                    cwd=str(root), env={**os.environ, 'HF_HUB_OFFLINE': '1', 'TRANSFORMERS_OFFLINE': '1'}), errlog=errors) as (read, write):
                async with ClientSession(read, write, read_timeout_seconds=None) as session:
                    await asyncio.wait_for(session.initialize(), 60)
                    tools = await session.list_tools()
                    self.assertEqual({tool.name for tool in tools.tools}, {'search_forever_knowledge', 'get_forever_knowledge_record'})
                    route = next(tool for tool in tools.tools if tool.name == 'get_forever_knowledge_record')
                    self.assertTrue(route.annotations.readOnlyHint); self.assertFalse(route.annotations.openWorldHint)
                    for field, low, high, default in [('reference_depth', 1, 3, 1),
                            ('reference_node_budget', 1, 256, 64), ('reference_edge_budget', 1, 1024, 256)]:
                        schema = route.inputSchema['properties'][field]
                        self.assertEqual((schema['minimum'], schema['maximum'], schema['default']), (low, high, default))
                    async def call(error=False, **extra):
                        reply = await asyncio.wait_for(session.call_tool('get_forever_knowledge_record', {'record_key': key, **extra}), 45)
                        self.assertEqual(bool(reply.isError), error, reply)
                        return None if error else json.loads(reply.content[0].text)
                    old = await call(include_references=True)
                    self.assertNotIn('incoming_catalogue_references', old['record'])
                    reverse = await call(record_key=f"npcs:{npc['id']}", include_incoming_references=True)
                    incoming = reverse['record']['incoming_catalogue_references']
                    all_edges = list(incoming['references'])
                    offset = 32
                    while incoming['has_more_references']:
                        incoming = (await call(record_key=f"npcs:{npc['id']}", include_incoming_references=True,
                                              incoming_reference_offset=offset))['record']['incoming_catalogue_references']
                        all_edges.extend(incoming['references']); offset += 32
                    self.assertEqual(len(all_edges), incoming['reference_count'])
                    self.assertTrue(any(edge['source_record_key'] == key for edge in all_edges))
                    self.assertTrue(all(edge['target_record_key'] == f"npcs:{npc['id']}" for edge in all_edges))
                    await call(error=True, incoming_reference_offset=1)
                    await call(error=True, include_incoming_references=1)
                    await call(error=True, include_incoming_references=True, incoming_reference_offset=-1)
                    self.assertNotIn('catalogue_reference_walk', old['record'])
                    self.assertEqual(old, await call(include_references=True, reference_depth=1))
                    result = await call(include_references=True, reference_depth=3)
                    walk = result['record']['catalogue_reference_walk']
                    self.assertNotIn('catalogue_references', result['record'])
                    self.assertTrue(walk['traversal_complete_within_requested_depth'])
                    nodes = {row['record_key']: row for row in walk['nodes']}
                    for expected, section in [(item, 'items'), (npc, 'npcs'), (zone, 'zones')]:
                        node = nodes[f"{section}:{expected['id']}"]
                        self.assertEqual(node['catalogue_record_sha256'], knowledge.digest(knowledge.encoded(expected).encode()))
                    self.assertTrue(any(edge['source_record_key'] == f"npcs:{npc['id']}"
                        and edge['target_record_key'] == f"zones:{zone['id']}" and edge['source_path'] == '/zoneId' for edge in walk['edges']))
                    self.assertEqual(result, await call(include_references=True, reference_depth=3))
                    partial = (await call(include_references=True, reference_depth=3, reference_node_budget=1))['record']['catalogue_reference_walk']
                    self.assertEqual(partial['status'], 'node_budget_exhausted')
                    self.assertFalse(partial['traversal_complete_within_requested_depth']); self.assertEqual(partial['node_count'], 1)
                    for bad in [{'reference_depth': 2}, {'include_references': True, 'reference_depth': 4},
                                {'include_references': True, 'reference_depth': True},
                                {'include_references': True, 'reference_depth': 2, 'reference_offset': 1},
                                {'include_references': True, 'reference_depth': 2, 'reference_node_budget': 0},
                                {'include_references': True, 'reference_depth': 2, 'reference_edge_budget': 1025}]:
                        await call(error=True, **bad)
                    unknown = await call(record_key='../../PRIVATE', include_references=True, reference_depth=3)
                    self.assertFalse(unknown['found']); self.assertIsNone(unknown['record'])
                    self.assertNotIn('PRIVATE', json.dumps(unknown))
        self.assertEqual(hashlib.sha256(index.read_bytes()).hexdigest(), before)
