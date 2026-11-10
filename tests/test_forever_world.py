"""Offline public world build and actual stdio MCP readback."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools import forever_world as world

ROOT = Path(__file__).resolve().parents[1]
LOCAL = ROOT/'local-data/world-reference'
CACHE = LOCAL/'forever-world.jsonl'
SOURCE = LOCAL/'pinned-source'


class WorldBuildTests(unittest.TestCase):
    def test_actual_pinned_build_reproducible_and_no_overwrite(self):
        self.assertTrue(SOURCE.is_dir(), 'Explicit pinned source sync required; tests never download')
        with patch.object(world.subprocess,'run') as launch:
            self.assertEqual(world.sync_source(SOURCE)['status'],'unchanged')
            launch.assert_not_called()
        with tempfile.TemporaryDirectory(dir=LOCAL) as folder:
            path = Path(folder)/'world.jsonl'
            with patch.dict(os.environ, {'LUA_INIT': 'error("unexpected ambient Lua init")'}):
                self.assertEqual(world.build(SOURCE,path)['status'],'built')
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),world.PIN['cache_sha256'])
            self.assertEqual(world.build(Path(folder)/'not-read',path)['status'],'unchanged')
            bad = Path(folder)/'keep.jsonl'; bad.write_bytes(b'unchanged unrelated input')
            with self.assertRaisesRegex(ValueError,'checksum'): world.build(SOURCE,bad)
            self.assertEqual(bad.read_bytes(),b'unchanged unrelated input')
            with self.assertRaisesRegex(ValueError,'checksum'): world.World(bad)
            tampered = Path(folder)/'tampered'; tampered.mkdir()
            name = next(iter(world.PIN['source_files']))
            record = tampered/name; record.parent.mkdir(parents=True,exist_ok=True); record.write_bytes(b'wrong')
            with patch.object(world.subprocess,'run') as launch:
                with self.assertRaisesRegex(ValueError,'checksum'):
                    world.build(tampered,Path(folder)/'must-not-exist.jsonl')
                launch.assert_not_called()
            self.assertFalse((Path(folder)/'must-not-exist.jsonl').exists())


class WorldMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_offline_public_records_and_paging(self):
        self.assertTrue(CACHE.is_file(), 'Explicit local world build required; tests never download')
        python = ROOT/'.venv/Scripts/python.exe'
        source = CACHE.read_bytes()
        script = (
            'import socket,runpy,sys,subprocess;'
            'original=socket.socket.connect;'
            "socket.socket.connect=lambda sock,address:original(sock,address) if isinstance(address,tuple) "
            "and address[0] in ('127.0.0.1','::1') else (_ for _ in ()).throw(RuntimeError('External network forbidden'));"
            "subprocess.run=lambda *a,**k:(_ for _ in ()).throw(RuntimeError('No build at query time'));"
            "sys.argv=['forever_world','serve',*sys.argv[1:]];runpy.run_module('tools.forever_world',run_name='__main__')"
        )
        with tempfile.TemporaryFile(mode='w+',dir=LOCAL) as errors:
            params = StdioServerParameters(command=str(python),args=['-B','-c',script,'--cache',str(CACHE)],cwd=str(ROOT))
            async with stdio_client(params,errlog=errors) as (read,write):
                async with ClientSession(read,write) as session:
                    try:
                        await asyncio.wait_for(session.initialize(),15)
                    except Exception as error:
                        errors.seek(0)
                        raise AssertionError(errors.read(4096)) from error
                    tools = (await session.list_tools()).tools
                    self.assertEqual({tool.name for tool in tools},{'get_forever_world_record','search_forever_world_records'})
                    self.assertTrue(all(tool.annotations.readOnlyHint and not tool.annotations.openWorldHint for tool in tools))
                    async def call(tool, error=False, **args):
                        reply = await asyncio.wait_for(session.call_tool(tool,args),10)
                        self.assertEqual(bool(reply.isError),error,reply)
                        return None if error else json.loads(reply.content[0].text)
                    get, search = 'get_forever_world_record', 'search_forever_world_records'
                    local = await call(search,query='Hogger',kind='npc',spawn_area_id=12)
                    self.assertEqual(local['results'][0]['id'],448)
                    area = await call(search,spawn_area_id=12,limit=1)
                    self.assertGreater(area['match_count'],1)
                    self.assertTrue(area['has_more'])
                    await call(search,error=True,kind='item',spawn_area_id=12)
                    chain = await call(get,kind='quest',entity_id=5,include_quest_links=True)
                    next_link = chain['quest_links']['nextQuestInChain']['1']
                    self.assertEqual(next_link['quest_id'],chain['record']['fields']['nextQuestInChain'])
                    self.assertEqual(next_link['record']['kind'],'quest')
                    self.assertTrue(chain['quest_links']['breadcrumbs'])
                    container = await call(get,kind='item',entity_id=16882,include_item_sources=True)
                    self.assertIn(422,[item['id'] for item in container['sourced_items']['itemDrops']['items']])
                    rewards = await call(get,kind='quest',entity_id=6,include_item_sources=True)
                    self.assertIn(60,[item['id'] for item in rewards['sourced_items']['questRewards']['items']])
                    item = await call(get,kind='item',entity_id=1931,include_item_sources=True)
                    drops = item['item_sources']['npcDrops']
                    self.assertEqual(drops['kind'],'npc')
                    self.assertTrue(drops['references'])
                    hogger = await call(get,kind='npc',entity_id=448,include_item_sources=True)
                    self.assertIn(1931,[row['id'] for row in hogger['sourced_items']['npcDrops']['items']])
                    past = await call(get,kind='npc',entity_id=448,include_item_sources=True,
                                      item_source_offset=hogger['sourced_items']['npcDrops']['count'])
                    self.assertEqual(past['sourced_items']['npcDrops']['items'],[])
                    end = await call(get,kind='item',entity_id=1931,include_item_sources=True,item_source_offset=drops['count'])
                    self.assertEqual(end['item_sources']['npcDrops']['references'],{})
                    await call(get,error=True,kind='item',entity_id=1931,item_source_offset=1)
                    linked = await call(get,kind='quest',entity_id=2,include_quest_links=True)
                    supplied = linked['quest_objective_targets']['sourceItemId']['references']['1']
                    self.assertEqual(supplied['entity_id'],linked['record']['fields']['sourceItemId'])
                    self.assertEqual(supplied['record']['kind'],'item')
                    supplies = await call(get,kind='quest',entity_id=28,include_quest_links=True)
                    required = supplies['quest_objective_targets']['requiredSourceItems']['references']
                    self.assertEqual({key: val['entity_id'] for key, val in required.items()},
                                     supplies['record']['fields']['requiredSourceItems'])
                    prerequisite = linked['quest_links']['preQuestSingle']['1']
                    exact = await call(get,kind='quest',entity_id=6383)
                    self.assertEqual(prerequisite['quest_id'],6383)
                    self.assertEqual(prerequisite['record']['record_sha256'],exact['record']['record_sha256'])
                    self.assertNotIn('quest_links',exact)
                    npc = await call(get,kind='npc',entity_id=240,include_quest_links=True)
                    self.assertTrue(any(link.get('quest_id') == 176 for link in npc['quest_links']['questEnds'].values()))
                    obj = await call(get,kind='object',entity_id=31,include_quest_links=True)
                    self.assertTrue(obj['quest_links']['questStarts'])
                    starter = await call(get,kind='item',entity_id=1307,include_quest_links=True)
                    link = starter['quest_links']['startQuest']['1']
                    self.assertEqual(link['quest_id'],starter['record']['fields']['startQuest'])
                    self.assertEqual(link['record']['kind'],'quest')
                    related = await call(get,kind='item',entity_id=159,include_quest_links=True)
                    self.assertTrue(related['quest_links']['relatedQuests'])
                    result = await call(get,kind='quest',entity_id=176)
                    record = result['record']
                    targets = (await call(get,kind='quest',entity_id=176,include_quest_links=True))['quest_objective_targets']
                    self.assertEqual(targets['3']['kind'],'item')
                    self.assertEqual(targets['3']['references']['1']['record']['record_key'],'questiedb:item:1931')
                    contacts = (await call(get,kind='quest',entity_id=176,include_quest_links=True))['quest_contacts']
                    self.assertEqual(contacts['startedBy']['2']['kind'],'object')
                    finisher = contacts['finishedBy']['1']['references']['1']
                    self.assertEqual(finisher['entity_id'],240)
                    self.assertEqual(finisher['record'],{
                        key: val for key, val in (await call(get,kind='npc',entity_id=240))['record'].items()
                        if key != 'fields'
                    })
                    self.assertEqual(record['name'],'WANTED: "Hogger"')
                    self.assertEqual(record['fields']['finishedBy'],{'1':{'1':240}})
                    self.assertEqual(result['record_counts'],{'quest':5682,'npc':15559,'object':7258,'item':23858})
                    self.assertEqual(result['generation'],{'contract':4,'derived_passes':3,'static_corrections':104905})
                    self.assertFalse(result['dynamic_character_corrections_applied'])
                    self.assertFalse(result['native_availability_verified'])
                    self.assertFalse(result['complete_world_data_verified'])
                    for kind, identity in [('npc',240),('object',31),('item',25),('quest',176)]:
                        item = (await call(get,kind=kind,entity_id=identity))['record']
                        line = next(line for line in source.splitlines()[1:]
                            if (row:=json.loads(line))['kind']==kind and row['id']==identity)
                        self.assertEqual(item['record_sha256'],hashlib.sha256(line).hexdigest())
                        self.assertEqual(item['fields'],json.loads(line)['fields'])
                        self.assertEqual(item['record_key'],f'questiedb:{kind}:{identity}')
                    missing = await call(get,kind='quest',entity_id=2**32-1)
                    self.assertFalse(missing['found']); self.assertIsNone(missing['record'])
                    first = await call(search,kind='npc',query=' WOLF ',limit=2)
                    second = await call(search,kind='npc',query='wolf',limit=2,offset=2)
                    combined = await call(search,kind='npc',query='wolf',limit=4)
                    self.assertEqual(first['results']+second['results'],combined['results'])
                    self.assertEqual(first['match_count'],second['match_count']); self.assertTrue(first['has_more'])
                    self.assertEqual((await call(search,kind='npc',query='wolf',offset=2**32-1))['results'],[])
                    self.assertEqual((await call(search,kind='quest',query='hogger'))['results'][0]['record_key'],record['record_key'])
                    for args in [{'kind':'quest','entity_id':True},{'kind':'quest','entity_id':0},
                                 {'kind':'quest','entity_id':2**32},{'kind':'unknown','entity_id':176}]:
                        await call(get,error=True,**args)
                    for args in [{'query':' '},{'query':'é'*65},{'query':'wolf','limit':21},
                                 {'query':'wolf','offset':True},{'query':'wolf','offset':-1}]:
                        await call(search,error=True,**args)
        self.assertEqual(CACHE.read_bytes(),source)
