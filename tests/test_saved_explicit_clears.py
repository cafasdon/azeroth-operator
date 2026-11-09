"""Saved empty-slot plans preserve explicit intent and reject malformed metadata."""
from copy import deepcopy
import hashlib
import json
import unittest

from test_merge_gear_windows import report
from tools import positional_equipment as layout
from tools.merge_forever_gear_windows import (NATIVE_EQUIPMENT_SLOTS, cleared_for_review,
    merge_reports)


def bind_spec(row, spec):
    row['simulator_equipment_spec'] = spec
    row['simulator_gear_sha256'] = hashlib.sha256(
        json.dumps(spec, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def explicit_report():
    value = report(0, ['a' * 64, 'b' * 64], 64)
    search = value['gear_search']
    search.update(requested_slots=['finger1'], candidate_locations_by_slot={'finger1': 0},
        maximum_simultaneous_swaps=1, combinations_including_baseline=2,
        equipment_layout=layout.CONTRACT, include_empty_slots=True,
        empty_choices_by_slot={'finger1': True})
    search['combination_window'].update(considered=2, total_combinations_decimal='2')
    baseline, alternative = value['resolved_drafts']
    spec = {'items': [{} for _ in range(17)], 'preserveSlots': True}
    bind_spec(alternative, deepcopy(spec))
    alternative.update(selections=[], explicit_clear_equipment_slots=[11], cleared_equipment_slots=[11])
    spec['items'][10] = {'id': 4549, 'enchant': 0, 'randomSuffix': 0}
    bind_spec(baseline, spec)
    return value


class SavedExplicitClearTests(unittest.TestCase):
    def test_declared_empty_choice_requires_occupied_baseline(self):
        value = explicit_report()
        baseline = value['resolved_drafts'][0]
        baseline['simulator_equipment_spec']['items'][10] = {}
        bind_spec(baseline, baseline['simulator_equipment_spec'])
        value['gear_search']['distinct_simulator_gear_sets'] = 1
        with self.assertRaises(ValueError): merge_reports([value])

    def test_empty_only_draft_is_retained_and_counted(self):
        value = explicit_report()
        result = merge_reports([value])
        self.assertEqual(result['total_combinations_decimal'], '2')
        self.assertEqual(result['distinct_simulator_gear_sets'], 2)
        self.assertTrue(result['reported_coordinate_coverage_complete'])
        self.assertFalse(result['game_execution'])
        self.assertEqual(cleared_for_review(value['resolved_drafts'][1], value['gear_search']), [11])

    def test_explicit_lists_and_clear_union_are_strict(self):
        for field in ['explicit_clear_equipment_slots', 'cleared_equipment_slots']:
            for malformed in [None, [], [True], [11.0], [4], [19], [0], [11, 11], [12, 11], '11']:
                with self.subTest(field=field, malformed=malformed):
                    value = explicit_report()
                    value['resolved_drafts'][1][field] = malformed
                    with self.assertRaises(ValueError): merge_reports([value])
        for union in [[12], [11, 12], [11, 17]]:
            value = explicit_report()
            value['resolved_drafts'][1]['cleared_equipment_slots'] = union
            with self.assertRaises(ValueError): merge_reports([value])

    def test_explicit_auto_offhand_union_requires_mainhand_and_hole(self):
        value = explicit_report()
        row = value['resolved_drafts'][1]
        row['selections'] = [{'target_slot': 16, 'source': {'kind': 'bag', 'bag': 0, 'slot': 1},
                              'expected_item_id': 2042}]
        row['cleared_equipment_slots'] = [11, 17]
        self.assertEqual(cleared_for_review(row, value['gear_search']), [11, 17])
        for slot in [11, 17, True, 19]:
            wrong = deepcopy(row)
            wrong['selections'][0]['target_slot'] = slot
            with self.assertRaises(ValueError): cleared_for_review(wrong, value['gear_search'])
        wrong = deepcopy(row)
        spec = wrong['simulator_equipment_spec']
        spec['items'][15] = {'id': 754, 'enchant': 0, 'randomSuffix': 0}
        bind_spec(wrong, spec)
        with self.assertRaises(ValueError): cleared_for_review(wrong, value['gear_search'])

    def test_search_optin_and_metadata_are_required(self):
        for field, invalid in [('include_empty_slots', False), ('include_empty_slots', 1),
                ('include_empty_slots', None), ('equipment_layout', 'auto_place'),
                ('empty_choices_by_slot', None), ('empty_choices_by_slot', {}),
                ('empty_choices_by_slot', {'finger1': 1}),
                ('empty_choices_by_slot', {'finger1': True, 'head': False})]:
            value = explicit_report()
            value['gear_search'][field] = invalid
            with self.subTest(field=field, invalid=invalid), self.assertRaises(ValueError):
                merge_reports([value])
        for field in ['include_empty_slots', 'empty_choices_by_slot']:
            value = explicit_report()
            del value['gear_search'][field]
            with self.assertRaises(ValueError): merge_reports([value])
        value = explicit_report()
        value['resolved_drafts'][1].update(explicit_clear_equipment_slots=[12], cleared_equipment_slots=[12])
        with self.assertRaises(ValueError): merge_reports([value])
        value = explicit_report()
        value['gear_search']['empty_choices_by_slot']['finger1'] = False
        with self.assertRaises(ValueError):
            cleared_for_review(value['resolved_drafts'][1], value['gear_search'])

    def test_retained_positional_holes_and_nonbaseline_are_required(self):
        value = explicit_report()
        value['resolved_drafts'][0].update(explicit_clear_equipment_slots=[12], cleared_equipment_slots=[12])
        with self.assertRaises(ValueError): merge_reports([value])
        value = explicit_report()
        row = value['resolved_drafts'][1]
        row['simulator_equipment_spec']['items'][10] = {'id': 4549, 'enchant': 0, 'randomSuffix': 0}
        bind_spec(row, row['simulator_equipment_spec'])
        with self.assertRaises(ValueError): merge_reports([value])
        for positional in [False, 1, None]:
            value = explicit_report()
            row = value['resolved_drafts'][1]
            row['simulator_equipment_spec']['preserveSlots'] = positional
            bind_spec(row, row['simulator_equipment_spec'])
            with self.assertRaises(ValueError): merge_reports([value])
        value = explicit_report()
        for row in value['resolved_drafts']: del row['simulator_equipment_spec']
        with self.assertRaises(ValueError): merge_reports([value])

    def test_all_native_clear_slots_and_malicious_selections(self):
        value = explicit_report()
        row = value['resolved_drafts'][1]
        row['explicit_clear_equipment_slots'] = sorted(NATIVE_EQUIPMENT_SLOTS)
        row['cleared_equipment_slots'] = sorted(NATIVE_EQUIPMENT_SLOTS)
        self.assertEqual(cleared_for_review(row), sorted(NATIVE_EQUIPMENT_SLOTS))
        selection = {'target_slot': 16, 'source': {'kind': 'bag', 'bag': 0, 'slot': 1},
                     'expected_item_id': 2042}
        for selected in [[{**selection, 'extra': True}],
                         [{**selection, 'source': {'kind': 'bag', 'slot': 1}}],
                         [{**selection, 'expected_item_id': True}],
                         [selection, selection],
                         [selection, {**selection, 'target_slot': 18}]]:
            row = explicit_report()['resolved_drafts'][1]
            row['selections'] = selected
            with self.subTest(selected=selected), self.assertRaises(ValueError): cleared_for_review(row)

    def test_legacy_reports_and_offhand_guard_remain_unchanged(self):
        self.assertEqual(merge_reports([report(0, ['a' * 64, 'b' * 64])])['distinct_simulator_gear_sets'], 2)
        row = explicit_report()['resolved_drafts'][1]
        del row['explicit_clear_equipment_slots']
        with self.assertRaises(ValueError): cleared_for_review(row)
        row['cleared_equipment_slots'] = [17]
        with self.assertRaises(ValueError): cleared_for_review(row)
        row['selections'] = [{'target_slot': 16, 'source': {'kind': 'bag', 'bag': 0, 'slot': 1},
                              'expected_item_id': 2042}]
        self.assertEqual(cleared_for_review(row), [17])
        row['cleared_equipment_slots'] = [17.0]
        with self.assertRaises(ValueError): cleared_for_review(row)
