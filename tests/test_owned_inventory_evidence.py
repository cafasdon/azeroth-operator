"""Owned review coverage describes the unfiltered capture, never complete BiS."""
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


class OwnedInventoryEvidenceTests(unittest.IsolatedAsyncioTestCase):
    async def test_unfiltered_coverage_missing_empty_and_bounded_ids(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            base = fixture()
            base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            base['sections']['equipment'] = {'11': {'item_id': 4549}}
            base['sections']['bags'] = {'0': {'1': {'item_id': 4294967295, 'count': 7},
                                               '2': {'item_id': 4294967295, 'count': 1}}}
            partial = store.ingest(base)
            complete_sections = deepcopy(base)
            complete_sections['sections'].update(character_bank={'6': {}}, account_bank={})
            all_sections = store.ingest(complete_sections)
            stocked_bank = deepcopy(complete_sections)
            stocked_bank['sections']['account_bank'] = {'64': {'1': {'item_id': 4549, 'count': 1}}}
            stocked_bank_id = store.ingest(stocked_bank)
            empty = deepcopy(complete_sections)
            empty['sections'].update(equipment={}, bags={})
            empty_id = store.ingest(empty)
            many = deepcopy(complete_sections)
            many['sections']['bags'] = {'0': {str(i + 1): {'item_id': 4294967200 + i, 'count': 1}
                                             for i in range(52)}}
            many_id = store.ingest(many)
            mixed = deepcopy(base)
            mixed['sections']['bags']['0']['11'] = {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}
            mixed['sections']['equipment_durability'] = {
                'coverage': 'readable_equipped_slots_only',
                'entries': {'11': {'current': 0, 'maximum': 40}},
            }
            mixed_id = store.ingest(mixed)
            enchanted = deepcopy(mixed)
            enchanted['sections']['equipment']['16'] = {'item_id': 4549}
            enchant_rows = {'1': {'has_enchant': True, 'enchant_type': 'temporary', 'remaining_milliseconds': 0}}
            enchanted['sections']['weapon_enchants'] = {
                'coverage': 'public_weapon_enchant_lists',
                'hands': {'main_hand': {'entries': enchant_rows}},
            }
            enchanted_id = store.ingest(enchanted)
            set_capture = deepcopy(base)
            set_capture['sections']['equipment'] = {'16': {'item_id': 7950}}
            set_capture['sections']['bags'] = {'0': {
                '1': {'item_id': 7950, 'count': 1},
                '2': {'item_id': 7953, 'count': 1},
                '3': {'item_id': 7953, 'count': 1},
            }}
            set_capture['sections']['character_bank'] = {'6': {}}
            set_capture_id = store.ingest(set_capture)
            missing_equipment = deepcopy(set_capture)
            del missing_equipment['sections']['equipment']
            missing_equipment_id = store.ingest(missing_equipment)
            merchant_capture = deepcopy(base)
            merchant_capture['sections']['wallet'] = {'copper': 5}
            merchant_capture['sections']['stats'] = {
                'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'fixture', 'map_id': 1411,
            }
            merchant_capture['sections']['merchant_stock'] = {'1': {
                'item_id': 7950, 'bundle_size': 1, 'price_copper': 7,
                'available': -1, 'extended_cost': False,
            }}
            merchant_capture_id = store.ingest(merchant_capture)
            second_visit = deepcopy(merchant_capture)
            second_visit['captured_at'] = '2025-01-02T00:00:00Z'
            del second_visit['sections']['stats']['map_id']
            second_visit['sections']['merchant_stock']['1'].update(item_id=7953, price_copper=3)
            second_visit_id = store.ingest(second_visit)
            full_visit = deepcopy(merchant_capture)
            full_visit['sections']['merchant_stock']['2'] = {
                **second_visit['sections']['merchant_stock']['1'], 'price_copper': 5,
            }
            full_visit_id = store.ingest(full_visit)
            separate_wallet = deepcopy(base)
            separate_wallet['sections']['wallet'] = {'copper': 9}
            separate_wallet_id = store.ingest(separate_wallet)
            wrong_merchant = deepcopy(merchant_capture)
            wrong_merchant['character']['name'] = 'Other'
            wrong_merchant_id = store.ingest(wrong_merchant)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder,
                   'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(**args):
                        reply = await session.call_tool('lookup_forever_catalogue', args)
                        return reply
                    match = await lookup(section='items', set_name='  gArB OF THERO-SHAN  ')
                    self.assertFalse(match.isError)
                    records = json.loads(match.content[0].text)['records']
                    self.assertEqual({row['id'] for row in records}, {7950, 7953})
                    mismatch = await lookup(section='items', set_name='thero-shan', set_id=162)
                    self.assertFalse(mismatch.isError)
                    self.assertEqual(json.loads(mismatch.content[0].text)['records'], [])
                    for section, name in [('items', ' '), ('items', 'x' * 161), ('consumables', 'thero-shan')]:
                        self.assertTrue((await lookup(section=section, set_name=name)).isError)
                    async def review(key, **filters):
                        response = await session.call_tool('review_forever_owned_gear', {'snapshot_id': key, **filters})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    result = await review(partial)
                    sourced = await review(partial, set_id=162, include_unobserved_set_sources=True, offset=99)
                    acquisition = sourced['set_observation']['unobserved_piece_acquisition']
                    self.assertEqual([row['item_id'] for row in acquisition['records']],
                                     sourced['set_observation']['unobserved_catalogue_item_ids'])
                    self.assertTrue(any(row['item_id'] == 6473 for row in acquisition['source_references']))
                    self.assertFalse(acquisition['truncated'])
                    self.assertIsNone(acquisition['captured_merchant_quote'])
                    merchant = await review(merchant_capture_id, set_id=221, include_unobserved_set_sources=True)
                    quote = merchant['set_observation']['unobserved_piece_acquisition']['captured_merchant_quote']
                    self.assertEqual(quote['copper_only_bundle_mix_quoted_subtotal'], 7)
                    self.assertEqual(quote['copper_only_bundle_mix_quoted_subtotal_wallet_shortfall'], 2)
                    self.assertEqual(quote['copper_only_bundle_mix_unquoted_item_ids'], [7953])
                    self.assertIsNone(quote['copper_only_bundle_mix_total'])
                    self.assertFalse(quote['current_offers_verified'])
                    self.assertEqual(quote['merchant_snapshot_id'], merchant_capture_id)
                    separate = await review(separate_wallet_id, set_id=221,
                        include_unobserved_set_sources=True, set_merchant_snapshot_id=merchant_capture_id)
                    separate_quote = separate['set_observation']['unobserved_piece_acquisition']['captured_merchant_quote']
                    self.assertEqual(separate_quote['merchant_snapshot_id'], merchant_capture_id)
                    self.assertEqual(separate_quote['copper_only_bundle_mix_quoted_subtotal'], 7)
                    self.assertEqual(separate_quote['copper_only_bundle_mix_quoted_subtotal_wallet_shortfall'], 0)
                    visits = await review(separate_wallet_id, set_id=221, include_unobserved_set_sources=True,
                                          set_merchant_snapshot_ids=[partial, merchant_capture_id])
                    visit_acquisition = visits['set_observation']['unobserved_piece_acquisition']
                    self.assertNotIn('captured_merchant_quote', visit_acquisition)
                    self.assertEqual(visit_acquisition['captured_merchant_quotes'], [
                        {'merchant_snapshot_id': partial, 'quote': None},
                        {'merchant_snapshot_id': merchant_capture_id, 'quote': separate_quote},
                    ])
                    self.assertIsNone(visit_acquisition['captured_merchant_combined_plan'])
                    partial_plan = visit_acquisition['captured_merchant_partial_plan']
                    self.assertEqual(partial_plan['excluded_unquoted_item_ids'], [7953])
                    self.assertEqual(partial_plan['plan']['complete_quote_copper'], 7)
                    self.assertEqual([row['item_id'] for row in partial_plan['plan']['items']], [7950])
                    self.assertEqual(visit_acquisition['captured_merchant_combined_plan_status'], {
                        'status': 'unquoted_pieces', 'unquoted_item_ids': [7953], 'maximum_visits': 4,
                    })
                    split = await review(separate_wallet_id, set_id=221, include_unobserved_set_sources=True,
                        set_merchant_snapshot_ids=[merchant_capture_id, second_visit_id],
                        set_merchant_wallet_reserve_copper=1)
                    plan = split['set_observation']['unobserved_piece_acquisition']['captured_merchant_combined_plan']
                    self.assertEqual(plan['complete_quote_copper'], 10)
                    self.assertEqual(plan['visit_count'], 2)
                    self.assertEqual(plan['after_wallet_reserve']['complete_quote_shortfall_copper'], 2)
                    self.assertEqual([stop['after_wallet_reserve'] for stop in plan['merchant_visit_checklist']], [
                        {'remaining_spendable_copper': 1, 'shortfall_copper': 0, 'affordable': True},
                        {'remaining_spendable_copper': None, 'shortfall_copper': 2, 'affordable': False},
                    ])
                    self.assertEqual([(row['item_id'], row['merchant_snapshot_id']) for row in plan['items']],
                                     [(7950, merchant_capture_id), (7953, second_visit_id)])
                    self.assertEqual([(stop['captured_player_map_id'], stop['merchant_captured_at'])
                                      for stop in plan['merchant_visit_checklist']],
                                     [(1411, merchant_capture['captured_at']), (None, second_visit['captured_at'])])
                    self.assertEqual(partial_plan['plan']['merchant_visit_checklist'][0]['captured_player_map_id'], 1411)
                    travel = await review(separate_wallet_id, set_id=221, include_unobserved_set_sources=True,
                        set_merchant_snapshot_ids=[merchant_capture_id, second_visit_id, full_visit_id],
                        set_merchant_visit_costs_copper=[3, 3, 1])
                    travel_plan = travel['set_observation']['unobserved_piece_acquisition']['captured_merchant_combined_plan']
                    self.assertEqual((travel_plan['complete_quote_copper'], travel_plan['assumed_visit_cost_copper']), (13, 1))
                    self.assertEqual(travel_plan['merchant_visit_checklist'][0]['merchant_snapshot_id'], full_visit_id)
                    capped = await review(separate_wallet_id, set_id=221, include_unobserved_set_sources=True,
                        set_merchant_snapshot_ids=[merchant_capture_id, second_visit_id], set_merchant_max_visits=1)
                    self.assertIsNone(capped['set_observation']['unobserved_piece_acquisition']['captured_merchant_combined_plan'])
                    self.assertEqual(capped['set_observation']['unobserved_piece_acquisition']
                                     ['captured_merchant_combined_plan_status']['status'], 'visit_limit')
                    overflow = await review(separate_wallet_id, set_id=221, include_unobserved_set_sources=True,
                        set_merchant_snapshot_ids=[merchant_capture_id, second_visit_id],
                        set_merchant_visit_costs_copper=[2**64 - 1, 0])
                    self.assertEqual(overflow['set_observation']['unobserved_piece_acquisition']
                                     ['captured_merchant_combined_plan_status']['status'], 'unrepresentable_total')
                    reserved = await review(separate_wallet_id, set_id=221,
                        include_unobserved_set_sources=True, set_merchant_snapshot_id=merchant_capture_id,
                        set_merchant_wallet_reserve_copper=4)
                    self.assertEqual(reserved['set_observation']['unobserved_piece_acquisition']
                                     ['captured_merchant_quote']['after_wallet_reserve'], {
                        'wallet_reserve_copper': 4, 'spendable_copper': 5, 'reserve_shortfall_copper': 0,
                        'quoted_subtotal_shortfall_copper': 2, 'complete_quote_shortfall_copper': None,
                        'complete_quote_affordable': None,
                    })
                    for options in [
                        {'set_merchant_snapshot_id': merchant_capture_id},
                        {'set_merchant_wallet_reserve_copper': 0},
                        {'set_merchant_snapshot_id': wrong_merchant_id, 'include_unobserved_set_sources': True},
                        {'set_merchant_snapshot_ids': [merchant_capture_id, merchant_capture_id],
                         'include_unobserved_set_sources': True},
                        {'set_merchant_snapshot_ids': [merchant_capture_id, wrong_merchant_id],
                         'include_unobserved_set_sources': True},
                        {'set_merchant_visit_costs_copper': [0], 'include_unobserved_set_sources': True},
                        {'set_merchant_snapshot_ids': [merchant_capture_id], 'set_merchant_max_visits': 0,
                         'include_unobserved_set_sources': True},
                    ]:
                        rejected = await session.call_tool('review_forever_owned_gear',
                            {'snapshot_id': separate_wallet_id, 'set_id': 221, **options})
                        self.assertTrue(rejected.isError)
                    complete_set = await review(set_capture_id, set_name='thero-shan',
                                                include_unobserved_set_sources=True)
                    self.assertEqual(complete_set['set_observation']['unobserved_piece_acquisition']['records'], [])
                    ambiguous_sources = await session.call_tool('review_forever_owned_gear', {
                        'snapshot_id': partial, 'set_name': 'of', 'include_unobserved_set_sources': True,
                    })
                    self.assertTrue(ambiguous_sources.isError)
                    set_review = await review(set_capture_id, set_id=221, storage='account_bank', offset=99)
                    self.assertEqual(set_review['items'], [])
                    named_owned = await review(set_capture_id, set_name='  THERO-shan ', storage='bags', limit=1)
                    self.assertEqual(named_owned['filtered_item_locations'], 3)
                    self.assertTrue(named_owned['has_more'])
                    self.assertEqual(named_owned['items'][0]['observed']['item_id'], 7950)
                    self.assertEqual(named_owned['set_observation'], set_review['set_observation'])
                    self.assertEqual(named_owned['set_name_resolution'], {
                        'matched_set_count': 1, 'matched_set_ids': [221], 'matched_set_ids_truncated': False,
                        'matched_sets': [{'set_id': 221, 'set_name': 'Garb of Thero-shan',
                                          'catalogue_distinct_pieces': 2, 'unobserved_catalogue_distinct_pieces': 0,
                                          'observed_distinct_pieces': 2, 'observed_equipped_distinct_pieces': 1}],
                    })
                    ambiguous = await review(set_capture_id, set_name='of', limit=1)
                    self.assertNotIn('set_observation', ambiguous)
                    self.assertGreater(ambiguous['set_name_resolution']['matched_set_count'], 1)
                    resolution = ambiguous['set_name_resolution']
                    self.assertEqual([row['set_id'] for row in resolution['matched_sets']],
                                     resolution['matched_set_ids'])
                    self.assertTrue(all('of' in row['set_name'].lower() for row in resolution['matched_sets']))
                    self.assertEqual(next(row for row in resolution['matched_sets'] if row['set_id'] == 221),
                                     named_owned['set_name_resolution']['matched_sets'][0])
                    unknown = await review(missing_equipment_id, set_name='thero-shan')
                    self.assertIsNone(unknown['set_name_resolution']['matched_sets'][0]
                                      ['observed_equipped_distinct_pieces'])
                    no_match = await review(set_capture_id, set_name='no-matching-set-zzzz')
                    missing_pieces = (await review(partial, set_name='thero-shan'))['set_name_resolution']['matched_sets'][0]
                    self.assertEqual(missing_pieces['unobserved_catalogue_distinct_pieces'], 2)
                    self.assertNotIn('set_observation', no_match)
                    self.assertEqual(no_match['set_name_resolution']['matched_set_count'], 0)
                    self.assertEqual((await review(set_capture_id, set_name='thero-shan', set_id=162))['items'], [])
                    invalid_name = await session.call_tool('review_forever_owned_gear',
                        {'snapshot_id': set_capture_id, 'set_name': ' '})
                    self.assertTrue(invalid_name.isError)
                    self.assertEqual(set_review['set_observation']['observed_distinct_pieces'], 2)
                    self.assertEqual(set_review['set_observation']['observed_distinct_pieces_by_storage'],
                                     {'equipment': 1, 'bags': 2, 'character_bank': 0, 'account_bank': None})
                    self.assertEqual(set_review['set_observation']['observed_stored_not_equipped_item_ids'], [7953])
                    self.assertEqual(set_review['set_observation']['unobserved_catalogue_item_ids'], [])
                    unknown_equipment = (await review(missing_equipment_id, set_id=221))['set_observation']
                    self.assertIsNone(unknown_equipment['observed_stored_not_equipped_item_ids'])
                    self.assertEqual(unknown_equipment['unobserved_catalogue_item_ids'], [])
                    unobserved_set = (await review(partial, set_id=221))['set_observation']
                    self.assertEqual(unobserved_set['observed_stored_not_equipped_item_ids'], [])
                    self.assertEqual(unobserved_set['unobserved_catalogue_item_ids'], [7950, 7953])
                    self.assertNotIn('observed_equipped_durability', result['items'][0])
                    same_item = (await review(mixed_id, item_id=4549))['items']
                    self.assertEqual([(row['storage'], row['slot']) for row in same_item],
                                     [('equipment', '11'), ('bags', '11')])
                    self.assertEqual(same_item[0]['observed_equipped_durability'], {'current': 0, 'maximum': 40})
                    self.assertNotIn('observed_equipped_durability', same_item[1])
                    enchant_matches = (await review(enchanted_id, item_id=4549))['items']
                    self.assertEqual([
                        (row['storage'], row['slot'], row['observed_weapon_hand_enchants'])
                        for row in enchant_matches if 'observed_weapon_hand_enchants' in row
                    ], [('equipment', '16', {'hand': 'main_hand', 'entries': enchant_rows})])
                    container_field = 'observed_container_occupied_slots'
                    self.assertEqual(result['coverage']['bags'][container_field], {'0': 2})
                    self.assertNotIn(container_field, result['coverage']['equipment'])
                    self.assertNotIn(container_field, result['coverage']['character_bank'])
                    observed = (await review(all_sections, storage='equipment', offset=99))['coverage']
                    self.assertEqual(observed['bags'][container_field], {'0': 2})
                    self.assertEqual(observed['character_bank'][container_field], {'6': 0})
                    self.assertEqual(observed['account_bank'][container_field], {})
                    backpack = await review(mixed_id, container_id=0, offset=1, limit=1)
                    self.assertEqual(backpack['filtered_item_locations'], 3)
                    self.assertTrue(backpack['has_more'])
                    self.assertEqual(backpack['items'][0]['container'], '0')
                    self.assertEqual(backpack['coverage']['bags'][container_field], {'0': 3})
                    empty_tab = await review(all_sections, container_id=6, storage='character_bank')
                    self.assertEqual(empty_tab['filtered_item_locations'], 0)
                    self.assertEqual(empty_tab['coverage']['character_bank'][container_field], {'6': 0})
                    other_storage = await review(all_sections, container_id=0, storage='character_bank')
                    self.assertEqual(other_storage['filtered_item_locations'], 0)
                    bank_tab = await review(stocked_bank_id, container_id=64, item_id=4549)
                    self.assertEqual(bank_tab['filtered_item_locations'], 1)
                    self.assertEqual(bank_tab['items'][0]['storage'], 'account_bank')
                    self.assertEqual(bank_tab['items'][0]['container'], '64')
                    for invalid in [{'container_id':65}, {'container_id':0,'storage':'equipment'}]:
                        rejected = await session.call_tool('review_forever_owned_gear', {
                            'snapshot_id': mixed_id, **invalid})
                        self.assertTrue(rejected.isError)
                    mixed_evidence = (await review(mixed_id))['inventory_evidence']
                    resolved = await review(mixed_id, simulator_input_resolved=True, limit=1)
                    self.assertEqual(resolved['filtered_item_locations'], 1)
                    self.assertEqual(resolved['items'][0]['simulator_item_resolution'], 'resolved')
                    self.assertFalse(resolved['has_more'])
                    unresolved = await review(mixed_id, simulator_input_resolved=False, offset=1, limit=1)
                    self.assertEqual(unresolved['filtered_item_locations'], 3)
                    self.assertEqual(unresolved['items'][0]['simulator_item_resolution'], 'catalogue_item_unavailable')
                    self.assertTrue(unresolved['has_more'])
                    past_end = await review(mixed_id, simulator_input_resolved=False, offset=3, limit=1)
                    self.assertEqual(past_end['filtered_item_locations'], 3)
                    self.assertEqual(past_end['items'], [])
                    self.assertFalse(past_end['has_more'])
                    rings = await review(mixed_id, simulator_input_resolved=False, simulator_slot='finger1')
                    self.assertEqual(rings['filtered_item_locations'], 1)
                    self.assertEqual(rings['items'][0]['simulator_item_resolution'], 'item_link_unresolved')
                    for filtered in [resolved, unresolved, past_end, rings]:
                        self.assertEqual(filtered['inventory_evidence'], mixed_evidence)
                    unmatched = await review(mixed_id, catalogue_matched=False, limit=1)
                    self.assertEqual(unmatched['filtered_item_locations'], 2)
                    self.assertTrue(unmatched['has_more'])
                    self.assertEqual(unmatched['items'][0]['simulator_item_resolution'], 'catalogue_item_unavailable')
                    tail = await review(mixed_id, catalogue_matched=False, offset=1, limit=1, storage='bags')
                    self.assertEqual(tail['filtered_item_locations'], 2)
                    self.assertFalse(tail['has_more'])
                    self.assertNotEqual(unmatched['items'],tail['items'])
                    matched = await review(mixed_id, catalogue_matched=True, simulator_input_resolved=False)
                    self.assertEqual(matched['filtered_item_locations'], 1)
                    self.assertEqual(matched['items'][0]['simulator_item_resolution'], 'item_link_unresolved')
                    conflict = await review(mixed_id, catalogue_matched=False, simulator_slot='finger1')
                    self.assertEqual(conflict['items'], [])
                    for page in [unmatched, tail, matched, conflict]:
                        self.assertEqual(page['inventory_evidence'], mixed_evidence)
                    by_status = {
                        'catalogue_item_unavailable':2, 'item_link_unresolved':1, 'resolved':1,
                        'modifier_values_unavailable':0, 'suffix_reference_unresolved':0,
                        'enchant_reference_unresolved':0, 'simulator_item_unresolved':0,
                    }
                    for status, count in by_status.items():
                        page = await review(mixed_id, simulator_item_resolution=status)
                        self.assertEqual(page['filtered_item_locations'], count)
                        self.assertEqual([row['simulator_item_resolution'] for row in page['items']],
                                         [status]*count)
                        self.assertEqual(page['inventory_evidence'], mixed_evidence)
                    last = await review(mixed_id, simulator_item_resolution='catalogue_item_unavailable',
                                        catalogue_matched=False, storage='bags', offset=1, limit=1)
                    self.assertEqual(last['items'],tail['items'])
                    self.assertFalse(last['has_more'])
                    incompatible = await review(mixed_id, simulator_item_resolution='resolved',
                                                simulator_input_resolved=False)
                    self.assertEqual(incompatible['filtered_item_locations'],0)
                    self.assertTrue((await session.call_tool('review_forever_owned_gear', {
                        'snapshot_id':mixed_id,'simulator_item_resolution':'unknown'})).isError)
                    counts_field = 'simulator_item_resolution_counts'
                    storage_counts_field = 'simulator_item_resolution_counts_by_storage'
                    self.assertNotIn(counts_field, await review(mixed_id))
                    self.assertNotIn(storage_counts_field, await review(mixed_id))
                    expected_counts = {status:count for status,count in by_status.items() if count}
                    for offset in [0,1,4,99]:
                        counted = await review(mixed_id, include_simulator_item_resolution_counts=True,
                                               offset=offset, limit=1)
                        self.assertEqual(counted.pop(counts_field), expected_counts)
                        self.assertEqual(counted.pop(storage_counts_field), {
                            'equipment':{'item_link_unresolved':1},
                            'bags':{'catalogue_item_unavailable':2,'resolved':1},
                        })
                        self.assertEqual(counted, await review(mixed_id, offset=offset, limit=1))
                    filtered_counts = await review(mixed_id, include_simulator_item_resolution_counts=True,
                                                   simulator_input_resolved=False, catalogue_matched=True)
                    self.assertEqual(filtered_counts[counts_field], {'item_link_unresolved':1})
                    self.assertEqual(filtered_counts[storage_counts_field], {'equipment':{'item_link_unresolved':1}})
                    self.assertEqual((await review(empty_id,
                        include_simulator_item_resolution_counts=True))[counts_field], {})
                    evidence = result['inventory_evidence']
                    self.assertEqual(evidence['observed_storage_sections'], ['equipment', 'bags'])
                    self.assertEqual(evidence['unavailable_storage_sections'], ['character_bank', 'account_bank'])
                    self.assertFalse(evidence['all_storage_sections_observed'])
                    self.assertEqual(evidence['catalogue_unmatched_item_locations'], 2)
                    self.assertEqual(evidence['catalogue_unmatched_distinct_item_ids'], 1)
                    self.assertEqual(evidence['catalogue_unmatched_item_ids'], [4294967295])
                    self.assertFalse(evidence['catalogue_unmatched_item_ids_truncated'])
                    for filters in [{'item_id': 4549}, {'storage': 'equipment'}, {'simulator_slot': 'finger1'},
                                    {'offset': 99, 'limit': 1}]:
                        self.assertEqual((await review(partial, **filters))['inventory_evidence'], evidence)
                    for key in [all_sections, empty_id]:
                        data = (await review(key))['inventory_evidence']
                        self.assertTrue(data['all_storage_sections_observed'])
                        self.assertEqual(data['unavailable_storage_sections'], [])
                        for flag in ['complete_ownership_verified', 'complete_simulator_resolution_verified', 'best_in_slot_verified']:
                            self.assertFalse(data[flag])
                    self.assertEqual((await review(empty_id))['inventory_evidence']['catalogue_unmatched_item_ids'], [])
                    bounded = (await review(many_id, item_id=4549))['inventory_evidence']
                    self.assertEqual(bounded['catalogue_unmatched_distinct_item_ids'], 52)
                    self.assertEqual(bounded['catalogue_unmatched_item_locations'], 52)
                    self.assertEqual(bounded['catalogue_unmatched_item_ids'], list(range(4294967200, 4294967250)))
                    self.assertTrue(bounded['catalogue_unmatched_item_ids_truncated'])
                    beyond_sample = await review(many_id, catalogue_matched=False, offset=50, limit=2)
                    self.assertEqual(beyond_sample['filtered_item_locations'],52)
                    self.assertEqual(len(beyond_sample['items']),2)
                    self.assertFalse(beyond_sample['has_more'])



if __name__ == '__main__':
    unittest.main()
