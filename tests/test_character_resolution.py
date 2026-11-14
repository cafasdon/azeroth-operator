"""Character selection never substitutes newest saved data for live identity."""
from copy import deepcopy
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class CharacterResolutionTests(unittest.IsolatedAsyncioTestCase):
    async def test_exact_fresh_session_reference_never_falls_back(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as other:
            store = SnapshotStore(Path(folder))
            now = datetime.now(timezone.utc).replace(microsecond=0)
            base = fixture()
            base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            base['captured_at'] = (now - timedelta(minutes=2)).isoformat()
            reference = store.ingest(base)
            newer = deepcopy(base)
            newer['captured_at'] = (now - timedelta(minutes=1)).isoformat()
            newer['character']['name'] = 'Another'
            newest = store.ingest(newer)
            stale = deepcopy(base)
            stale['captured_at'] = (now - timedelta(minutes=16)).isoformat()
            stale_id = store.ingest(stale)
            foreign = deepcopy(base)
            foreign['character']['name'] = 'PrivateOtherOwner'
            foreign_id = SnapshotStore(Path(other)).ingest(foreign)
            # Bypass the producer validator to check the Rust consumer against future data.
            future = deepcopy(base)
            future['captured_at'] = (now + timedelta(minutes=2)).isoformat()
            raw = json.dumps(future).encode()
            future_id = hashlib.sha256(raw).hexdigest()
            (Path(folder) / f'{future_id}.json').write_bytes(raw)
            corrupt_id = 'b' * 64
            (Path(folder) / f'{corrupt_id}.json').write_text('{}')
            env = {**os.environ, 'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    args = {'family':'forever','current_snapshot_id':reference}
                    async def resolve(arguments):
                        reply = await session.call_tool('resolve_character', arguments)
                        self.assertFalse(reply.isError, reply)
                        return json.loads(reply.content[0].text)
                    result = await resolve(args)
                    self.assertEqual(result['status'],'current_session_snapshot_reference')
                    self.assertEqual(result['selection_basis'],'caller_current_session_snapshot_reference')
                    self.assertEqual(result['selection']['snapshot_id'],reference)
                    self.assertFalse(result['live'])
                    self.assertFalse(result['active_character_verified'])
                    expiry = datetime.fromisoformat(result['expires_at'].replace('Z','+00:00'))
                    self.assertEqual(expiry, datetime.fromisoformat(base['captured_at']) + timedelta(minutes=15))
                    self.assertEqual((await resolve(args))['expires_at'],result['expires_at'])
                    covered = await resolve({**args,'required_sections':['equipment'],
                                             'exact_client':base['client'],
                                             'captured_from':base['captured_at'],
                                             'captured_through':base['captured_at']})
                    self.assertEqual(covered['selection']['snapshot_id'],reference)
                    override = await resolve({**args,'current_snapshot_id':stale_id,
                                              'character':newer['character']})
                    self.assertEqual(override['status'],'explicit_saved_character')
                    self.assertEqual(override['selection']['snapshot_id'],newest)
                    for invalid in [
                        {**args,'current_snapshot_id':value}
                        for value in [stale_id, future_id, foreign_id, corrupt_id, 'a' * 64, '../escape', '']
                    ] + [
                        {**args,'family':'retail'},
                        {**args,'exact_client':{**base['client'],'build':'different'}},
                        {**args,'exact_client':{**base['client'],'interface':0}},
                        {**args,'exact_client':{**base['client'],'family':'retail'}},
                        {**args,'required_sections':['bags']},
                        {**args,'required_sections':['invalid']},
                        {**args,'required_sections':[]},
                        {**args,'captured_from':newer['captured_at']},
                        {**args,'captured_through':stale['captured_at']},
                        {**args,'captured_from':'bad'},
                        {**args,'captured_from':newer['captured_at'],'captured_through':stale['captured_at']},
                        {**args,'require_unique_latest':True},
                        {**args,'require_latest_coverage':True,'required_sections':['equipment']},
                    ]:
                        reply = await session.call_tool('resolve_character',invalid)
                        self.assertTrue(reply.isError, (invalid, reply))
                        self.assertNotIn('PrivateOtherOwner', str(reply))
                    self.assertEqual((await resolve({'family':'forever'}))['status'],
                                     'active_identity_unavailable')

    async def test_explicit_selection_unknown_default_and_owner_boundary(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as other:
            store = SnapshotStore(Path(folder))
            base = fixture()
            base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            base['sections']['weapon_enchants'] = {'coverage':'public_weapon_enchant_lists','hands':{}}
            first = store.ingest(base)
            later = deepcopy(base)
            later['captured_at'] = '2025-01-02T00:00:00Z'
            later['sections'].pop('weapon_enchants')
            expected = store.ingest(later)
            changed_build = deepcopy(later)
            changed_build['captured_at'] = '2025-01-02T12:00:00Z'
            changed_build['client']['build'] = '70863'
            changed_build_key = store.ingest(changed_build)
            newest = deepcopy(later)
            newest['captured_at'] = '2025-01-03T00:00:00Z'
            newest['character']['name'] = 'Another'
            newest['sections']['weapon_enchants'] = base['sections']['weapon_enchants']
            store.ingest(newest)
            foreign = deepcopy(newest)
            foreign['character']['name'] = 'PrivateOtherOwner'
            SnapshotStore(Path(other)).ingest(foreign)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def resolve(args):
                        reply = await session.call_tool('resolve_character', args)
                        self.assertFalse(reply.isError, reply)
                        return json.loads(reply.content[0].text)
                    default = await resolve({'family': 'forever'})
                    self.assertEqual(default, {'status': 'active_identity_unavailable', 'selection': None,
                                               'live': False, 'active_character_verified': False})
                    result = await resolve({'family': 'forever', 'character': {'name': 'example', 'realm': 'test'}})
                    self.assertEqual(result['status'], 'explicit_saved_character')
                    self.assertEqual(result['selection']['snapshot_id'], changed_build_key)
                    self.assertNotEqual(first, expected)
                    self.assertFalse(result['active_character_verified'])
                    self.assertFalse(result['live'])
                    target = {'family':'forever','character':{'name':'Example','realm':'Test'}}
                    covered = await resolve({**target,'required_sections':['weapon_enchants','equipment']})
                    self.assertEqual(covered['selection']['snapshot_id'],first)
                    self.assertFalse(covered['active_character_verified'])
                    for bounds in [{'exact_client':changed_build['client']},
                                   {'captured_from':'2025-01-02T00:00:00Z'}]:
                        no_coverage = await resolve({**target,**bounds,'required_sections':['weapon_enchants']})
                        self.assertIsNone(no_coverage['selection'])
                    exact = await resolve({**target,'exact_client':base['client']})
                    self.assertEqual(exact['selection']['snapshot_id'],expected)
                    async def list_exact(client, **filters):
                        response = await session.call_tool('list_snapshots', {
                            **target, 'exact_client':client, **filters})
                        self.assertFalse(response.isError,response)
                        return json.loads(response.content[0].text)
                    listed = await list_exact(base['client'], coverage_summary_sections=['weapon_enchants'])
                    self.assertEqual([row['snapshot_id'] for row in listed['snapshots']],[expected,first])
                    self.assertEqual(listed['section_coverage_summary']['sections']['weapon_enchants'],
                        {'observed':1,'reported_unavailable':0,'not_recorded':1})
                    latest = await list_exact(base['client'],latest_per_character=True)
                    self.assertEqual([row['snapshot_id'] for row in latest['snapshots']],[expected])
                    self.assertEqual((await list_exact(base['client'],required_sections=['weapon_enchants']))
                        ['snapshots'][0]['snapshot_id'],first)
                    for client in [{**base['client'],'family':'retail'},{**base['client'],'interface':0}]:
                        self.assertTrue((await session.call_tool('list_snapshots',{
                            **target,'exact_client':client})).isError)
                    absent_client = {**base['client'],'variant':'different'}
                    self.assertIsNone((await resolve({**target,'exact_client':absent_client}))['selection'])
                    bounded = await resolve({**target,'captured_through':'2025-01-01T00:00:00Z'})
                    self.assertEqual(bounded['selection']['snapshot_id'], first)
                    stale = await resolve({**target,'captured_from':'2025-01-03T00:00:00Z'})
                    self.assertEqual(stale['status'], 'no_matching_snapshot')
                    self.assertIsNone(stale['selection'])
                    for args in [
                        {'family':'forever','captured_from':'2025-01-01T00:00:00Z'},
                        {'family':'forever','exact_client':base['client']},
                        {'family':'forever','required_sections':['weapon_enchants']},
                        {**target,'required_sections':['unknown']},
                        {**target,'exact_client':{**base['client'],'family':'retail'}},
                        {**target,'captured_from':'2025-01-03T00:00:00Z',
                         'captured_through':'2025-01-01T00:00:00Z'},
                    ]:
                        self.assertTrue((await session.call_tool('resolve_character', args)).isError)
                    for family, name in [('retail', 'Example'), ('classic', 'Example'),
                                         ('forever', 'Missing'), ('forever', 'PrivateOtherOwner')]:
                        result = await resolve({'family': family, 'character': {'name': name, 'realm': 'Test'}})
                        self.assertEqual(result['status'], 'no_matching_snapshot')
                        self.assertIsNone(result['selection'])
                        self.assertNotIn('PrivateOtherOwner', json.dumps(result))
                    for args in [ {'family': 'invalid'}, {'family': 'forever', 'active_character': newest['character']},
                                  {'family': 'forever', 'character': {'name': '', 'realm': 'Test'}},
                                  {'family': 'forever', 'character': {'name': 'Example'}},
                                  {'family': 'forever', 'character': {'name': 'Example', 'realm': 'Test', 'path': other}}]:
                        reply = await session.call_tool('resolve_character', args)
                        self.assertTrue(reply.isError)
                        self.assertNotIn(other, str(reply))
                    strict_coverage = {**target, 'required_sections':['weapon_enchants','equipment'],
                                       'require_latest_coverage':True}
                    uncovered = await resolve(strict_coverage)
                    self.assertEqual(uncovered['status'], 'latest_snapshot_missing_required_sections')
                    self.assertIsNone(uncovered['selection'])
                    self.assertEqual(uncovered['latest_section_coverage'],
                                     {'weapon_enchants':'not_recorded','equipment':'observed'})
                    covered_latest = await resolve({**strict_coverage,'captured_through':base['captured_at']})
                    self.assertEqual(covered_latest['selection']['snapshot_id'], first)
                    self.assertEqual(covered_latest['latest_section_coverage'],
                                     {'weapon_enchants':'observed','equipment':'observed'})
                    for invalid in [{**target,'require_latest_coverage':True},
                                    {**strict_coverage,'required_sections':[]},
                                    {**strict_coverage,'required_sections':['invalid']}]:
                        self.assertTrue((await session.call_tool('resolve_character',invalid)).isError)
                    tied_capture = deepcopy(changed_build)
                    tied_capture['client']['build'] = '70864'
                    tied_capture['sections']['weapon_enchants'] = base['sections']['weapon_enchants']
                    tied_key = store.ingest(tied_capture)
                    strict = {**target,'require_unique_latest':True}
                    tied = await resolve(strict)
                    self.assertEqual(tied, {'status':'ambiguous_latest_snapshot','selection':None,
                                            'live':False,'active_character_verified':False})
                    for filters, selected in [
                        ({'exact_client':changed_build['client']}, changed_build_key),
                        ({'required_sections':['weapon_enchants']}, tied_key),
                        ({'captured_through':later['captured_at']}, expected),
                    ]:
                        self.assertEqual((await resolve({**strict, **filters}))['selection']['snapshot_id'],
                                         selected)
                    tied_coverage = await resolve({**strict_coverage,'require_unique_latest':True})
                    self.assertEqual(tied_coverage['status'],'ambiguous_latest_snapshot')
                    self.assertNotIn('latest_section_coverage',tied_coverage)
                    self.assertEqual((await resolve(target))['selection']['snapshot_id'],
                                     min(tied_key,changed_build_key))
                    self.assertTrue((await session.call_tool('resolve_character', {
                        'family':'forever','require_unique_latest':True})).isError)
                    tools = {tool.name for tool in (await session.list_tools()).tools}
                    self.assertIn('resolve_character', tools)


if __name__ == '__main__':
    unittest.main()
