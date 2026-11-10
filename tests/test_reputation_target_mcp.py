"""Explicit standing goals preserve signed units and supplied resource assumptions."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def sample():
    value = fixture(); value['client'].update(family='forever', build='70862', interface=16001)
    value['sections'].update(reputation={'coverage': 'visible_faction_rows_only', 'entries': {'1': {
        'name': 'PRIVATE_FACTION', 'kind': 'standard', 'account_wide': False, 'reaction': 5,
        'standing': 3500, 'threshold': 3000, 'next_threshold': 9000}}},
        bags={'0': {'1': {'item_id': 10, 'count': 200}, '2': {'item_id': 20, 'count': 5}}},
        character_bank={'6': {'1': {'item_id': 10, 'count': 100000}}}, wallet={'copper': 100})
    return value


class ReputationTargetTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_signed_targets_budgets_unknowns_and_existing_outputs(self):
        root = Path(__file__).resolve().parents[1]; base = sample()
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as foreign:
            store = SnapshotStore(Path(folder)); key = store.ingest(base)
            other = deepcopy(base); other['character']['name'] = 'PRIVATE_OTHER'
            foreign_key = SnapshotStore(Path(foreign)).ingest(other)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}; env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    route = next(t for t in (await session.list_tools()).tools if t.name == 'inspect_snapshot')
                    schema = route.inputSchema['properties']['reputation_target_standing']
                    self.assertEqual(schema['minimum'], -2147483648)
                    self.assertEqual(schema['maximum'], 2147483647)
                    async def call(value=None, snapshot=key, error=False, **extra):
                        if value is not None: snapshot = store.ingest(value)
                        args = {'snapshot_id': snapshot, 'section': 'reputation', 'faction_id': 1,
                            'reputation_points_per_turn_in': 250,
                            'reputation_turn_in_item_cost': {'item_id': 10, 'quantity': 20},
                            'reputation_turn_in_copper_cost': {'copper_per_turn_in': 5, 'wallet_reserve_copper': 20}, **extra}
                        reply = await session.call_tool('inspect_snapshot', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        if error:
                            self.assertNotIn('PRIVATE', str(reply)); self.assertNotIn(folder, str(reply)); self.assertNotIn(foreign, str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    old = await call()
                    self.assertNotIn('reputation_target_plan', old)
                    nullable = await call(reputation_target_standing=None)
                    self.assertEqual({k:v for k,v in old.items() if k!='age_seconds'},
                                     {k:v for k,v in nullable.items() if k!='age_seconds'})
                    answer = await call(reputation_target_standing=10000)
                    target = answer.pop('reputation_target_plan')
                    old.pop('age_seconds'); answer.pop('age_seconds')
                    self.assertEqual(answer, old)
                    self.assertNotIn('PRIVATE', json.dumps(target))
                    self.assertEqual(target['standing_shortfall'], 6500)
                    self.assertEqual(target['turn_ins_if_constant_gain'], 26)
                    self.assertEqual(target['projected_standing_before_caps_if_completed'], 10000)
                    self.assertEqual(answer['reputation_turn_in_estimate']['turn_ins_to_next_rank'], 22)
                    resources = target['resource_plan']
                    self.assertEqual(resources['item_costs'][0]['required_quantity'], 520)
                    self.assertEqual(resources['item_costs'][0]['shortfall_vs_carried'], 320)
                    self.assertEqual(resources['wallet']['required_copper'], 130)
                    self.assertEqual(resources['wallet']['wallet_shortfall_copper'], 50)
                    self.assertNotIn('turn_ins_shortfall_to_next_rank', resources['wallet'])
                    self.assertEqual(resources['supported_turn_ins_toward_target'], 10)
                    self.assertEqual(resources['remaining_standing_shortfall_after_supported_turn_ins'], 4000)
                    for flag in ['caps_applied', 'actual_costs_verified', 'target_achievability_verified',
                                 'turn_in_eligibility_verified', 'reward_repeatability_verified', 'bank_or_mail_credit',
                                 'current_inventory_verified', 'active_character_verified', 'game_execution', 'live']:
                        self.assertFalse(target[flag])
                    multiple = (await call(reputation_target_standing=10000, reputation_turn_in_item_cost=None,
                        reputation_turn_in_item_costs=[{'item_id': 10, 'quantity': 20}, {'item_id': 20, 'quantity': 1}]))['reputation_target_plan']
                    self.assertEqual(multiple['resource_plan']['supported_turn_ins_toward_target'], 5)
                    no_costs = (await call(reputation_target_standing=3500, reputation_turn_in_item_cost=None,
                        reputation_turn_in_copper_cost=None))['reputation_target_plan']
                    self.assertTrue(no_costs['saved_target_already_met']); self.assertIsNone(no_costs['resource_plan'])
                    unknown = deepcopy(base); unknown['sections'].pop('wallet')
                    report = (await call(unknown, reputation_target_standing=10000))['reputation_target_plan']['resource_plan']
                    self.assertEqual(report['unknown_resource_count'], 1); self.assertIsNone(report['supported_turn_ins_toward_target'])
                    unknown = deepcopy(base); unknown['sections'].pop('bags')
                    report = (await call(unknown, reputation_target_standing=10000))['reputation_target_plan']['resource_plan']
                    self.assertIsNone(report['supported_turn_ins_toward_target'])
                    unknown['sections']['wallet']['copper'] = 0
                    report = (await call(unknown, reputation_target_standing=10000))['reputation_target_plan']['resource_plan']
                    self.assertEqual(report['supported_turn_ins_toward_target'], 0)
                    for kind in ['renown', 'friendship', 'paragon', 'unknown']:
                        special = deepcopy(base); special['sections']['reputation']['entries']['1']['kind'] = kind
                        report = (await call(special, reputation_target_standing=10000))['reputation_target_plan']
                        self.assertEqual(report['status'], 'standard_progress_unavailable')
                        self.assertIsNone(report['turn_ins_if_constant_gain']); self.assertIsNone(report['resource_plan'])
                    self.assertIsNone((await call(faction_id=999, reputation_target_standing=10000))['reputation_target_plan']['captured_standing'])
                    capped = deepcopy(base); capped['sections']['reputation']['entries']['1'].update(
                        reaction=8, standing=42000, threshold=42000, next_threshold=43000)
                    report = (await call(capped, reputation_target_standing=43000))['reputation_target_plan']
                    self.assertTrue(report['captured_capped_reaction']); self.assertFalse(report['caps_applied'])
                    self.assertFalse(report['target_achievability_verified'])
                    huge = (await call(reputation_target_standing=10000,
                        reputation_turn_in_item_cost={'item_id': 10, 'quantity': 18446744073709551615},
                        reputation_turn_in_copper_cost={'copper_per_turn_in': 18446744073709551615}))['reputation_target_plan']['resource_plan']
                    self.assertTrue(huge['item_costs'][0]['required_quantity_overflow'])
                    self.assertTrue(huge['wallet']['required_copper_overflow'])
                    self.assertIsNone(huge['item_costs'][0]['required_quantity'])
                    cases = 0
                    for current in [-50, -1, 0, 50, 99]:
                        value = deepcopy(base); value['sections']['reputation']['entries']['1'].update(
                            standing=current, threshold=-100, next_threshold=100)
                        for desired in [-10, 0, 1, 5, 99, 101, 10000]:
                            for gain in [1, 3, 250, 4294967295]:
                                plan = (await call(value, reputation_target_standing=desired,
                                    reputation_points_per_turn_in=gain))['reputation_target_plan']
                                shortfall = max(desired-current, 0); needed = (shortfall+gain-1)//gain
                                self.assertEqual(plan['turn_ins_if_constant_gain'], needed)
                                self.assertEqual(plan['resource_plan']['item_costs'][0]['required_quantity'], needed*20)
                                self.assertEqual(plan['resource_plan']['supported_turn_ins_toward_target'], min(needed, 10, 16))
                                cases += 1
                    self.assertEqual(cases, 140)
                    for current, desired, threshold, next_threshold in [
                        (-2147483648, 2147483647, -2147483648, -2147483548),
                        (2147483647, -2147483648, 2147483547, 2147483647)]:
                        value = deepcopy(base); value['sections']['reputation']['entries']['1'].update(
                            standing=current, threshold=threshold, next_threshold=next_threshold)
                        plan = (await call(value, reputation_target_standing=desired,
                            reputation_points_per_turn_in=1))['reputation_target_plan']
                        self.assertEqual(plan['standing_shortfall'], max(desired-current, 0))
                        self.assertEqual(plan['turn_ins_if_constant_gain'], max(desired-current, 0))
                    for bad in [True, 1.0, [], {}, 'PRIVATE_MARKER', -2147483649, 2147483648]:
                        await call(reputation_target_standing=bad, error=True)
                    for extra in [{'faction_id': None}, {'reputation_points_per_turn_in': None},
                                  {'section': 'bags'}, {'coverage_sections': ['reputation']},
                                  {'snapshot': foreign_key}]:
                        await call(reputation_target_standing=10000, error=True, **extra)
                    files = {p: p.read_bytes() for p in Path(folder).glob('*.json')}
                    await call(reputation_target_standing=10000)
                    self.assertEqual(files, {p: p.read_bytes() for p in files})
