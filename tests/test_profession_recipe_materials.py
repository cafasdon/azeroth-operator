"""Bulk schematics remain owner-local observations, with strict reader parity."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


class ProfessionRecipeMaterialsTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_and_strict_reader_parity(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        fixed = {'status': 'fixed_items', 'reagents': {'2840': 5}, 'output_item_id': 2870,
                 'base_output_quantity': {'minimum': 1, 'maximum': 2}}
        section = {'coverage': 'reported_known_spell_schematics', 'profession_id': 164,
                   'entries': {'2675': fixed, '3508': {'status': 'unavailable'},
                               '3509': {'status': 'unsupported_required_slots'},
                               '3510': {'status': 'budget_exhausted'}}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for observed in [None, section, {**section, 'entries': {}},
                             {**section,'entries':{'1':{**fixed,'reported_craftable_count':0}}},
                             {**section,'entries':{'1':{**fixed,'reported_craftable_count':4294967295}}},
                             {**section, 'entries': {'1': {'status': 'fixed_items', 'reagents': {}}}}]:
                value = deepcopy(base)
                if observed is not None:
                    value['sections']['profession_recipe_materials'] = observed
                valid.append((store.ingest(value), observed))
            bad_sections = [None, {}, {**section, 'coverage': 'all_recipes'},
                            {**section, 'profession_id': True}, {**section, 'profession_id': 0},
                            {**section, 'entries': {str(i): {'status': 'unavailable'} for i in range(1, 4098)}}]
            for key in ['0', '01', '4294967296', '-1', '1.0']:
                bad_sections.append({**section, 'entries': {key: fixed}})
            for row in [None, [], {}, {'status': []}, {'status': 'learned'},
                        {'status': 'unavailable', 'reagents': {}},
                        {'status': 'fixed_items'}, {**fixed, 'reagents': None},
                        {**fixed, 'reagents': {'0': 1}}, {**fixed, 'reagents': {'1': True}},
                        {**fixed, 'reagents': {'1': 0}}, {**fixed, 'reagents': {'1': 4294967296}},
                        {**fixed, 'reagents': {str(i): 1 for i in range(1, 66)}},
                        {**fixed, 'output_item_id': None}, {**fixed, 'output_item_id': True},
                        {**fixed, 'base_output_quantity': None},
                        {**fixed, 'base_output_quantity': {'minimum': 2, 'maximum': 1}},
                        {'status': 'fixed_items', 'reagents': {},
                         'base_output_quantity': {'minimum': 1, 'maximum': 1}},
                        {**fixed, 'name': 'PRIVATE_SCHEMATIC'}]:
                bad_sections.append({**section, 'entries': {'2675': row}})
            for invalid in [None,True,-1,1.5,'PRIVATE',{},4294967296]:
                bad_sections.append({**section,'entries':{'2675':{**fixed,'reported_craftable_count':invalid}}})
            bad_sections.append({**section,'entries':{'2675':{'status':'unavailable','reported_craftable_count':0}}})
            reagents = {str(i): 1 for i in range(1, 65)}
            bad_sections.append({**section, 'entries': {str(i): {'status': 'fixed_items', 'reagents': reagents}
                                                        for i in range(1, 258)}})
            bad_ids = []
            for bad in bad_sections:
                value = deepcopy(base)
                value['sections']['profession_recipe_materials'] = bad
                with self.assertRaises(ValueError):
                    validate_snapshot(value)
                raw = json.dumps(value, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            for family in ['retail', 'classic']:
                value = deepcopy(base)
                value['client']['family'] = family
                value['sections']['profession_recipe_materials'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(value)
                raw = json.dumps(value, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    for key, observed in valid:
                        reply = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertFalse(reply.isError, reply)
                        summary = json.loads(reply.content[0].text)['profession_recipe_material_observations']
                        if observed is None:
                            self.assertIsNone(summary)
                            continue
                        self.assertEqual(summary['reported_spell_count'], len(observed['entries']))
                        for field, status in [('fixed_item_recipe_count', 'fixed_items'),
                                              ('unavailable_recipe_count', 'unavailable'),
                                              ('unsupported_recipe_count', 'unsupported_required_slots'),
                                              ('budget_exhausted_recipe_count', 'budget_exhausted')]:
                            self.assertEqual(summary[field], sum(row['status'] == status for row in observed['entries'].values()))
                        self.assertFalse(summary['complete_recipe_knowledge_verified'])
                        self.assertFalse(summary['craftability_verified'])
                        reply = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'profession_recipe_materials'})
                        self.assertFalse(reply.isError, reply)
                        self.assertEqual(json.loads(reply.content[0].text)['data'], observed)
                    for key in bad_ids:
                        reply = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'profession_recipe_materials'})
                        self.assertTrue(reply.isError, key)
                        self.assertNotIn('PRIVATE_SCHEMATIC', str(reply))
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
