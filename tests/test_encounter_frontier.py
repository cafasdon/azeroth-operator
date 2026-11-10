"""An independent exhaustive dominance reference checks saved point-estimate frontiers."""
from copy import deepcopy
import random
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from tools.encounter_frontier import Frontier, options
from tools import role_metrics as roles
from tools import encounter_frontier


def row(index, gains):
    return {'simulator_gear_sha256': f'{index:064x}', 'gains': gains}


class EncounterFrontierTests(unittest.TestCase):
    def test_adjacent_large_integer_gains_keep_strict_dominance(self):
        frontier = Frontier(1, 'gains')
        frontier.add(row(1, [2**53])); frontier.add(row(2, [2**53+1]))
        result = frontier.report(complete=True, offset=0, limit=20)
        self.assertEqual([r['simulator_gear_sha256'] for r in result['gear_sets']], [f'{2:064x}'])
        first = Frontier(1, 'gains'); second = Frontier(1, 'gains')
        first.add(row(1, [1])); second.add(row(1, [1.0]))
        self.assertEqual(first.report(complete=True, offset=0, limit=1)['frontier_point_estimates_sha256'],
                         second.report(complete=True, offset=0, limit=1)['frontier_point_estimates_sha256'])

    def test_finalisation_respects_original_analysis_deadline(self):
        clock = [1]
        with patch('tools.encounter_frontier.monotonic', side_effect=lambda: clock[0]):
            frontier = Frontier(1, 'gains', deadline=2); frontier.add(row(1, [1]))
            clock[0] = 3
            result = frontier.report(complete=True, offset=0, limit=1)
            self.assertEqual(result['status'], 'analysis_time_budget_exhausted')
            self.assertIsNone(result['frontier_point_estimates_sha256'])
            self.assertEqual(result['gear_sets'], [])
            empty = Frontier(1, 'gains', deadline=2).report(complete=True, offset=0, limit=1)
            self.assertEqual(empty['status'], 'analysis_time_budget_exhausted')
            clock[0] = 1
            frontier = Frontier(1, 'gains', deadline=2); frontier.add(row(1, [1]))
            original_hash = encounter_frontier.hashlib.sha256
            def late_hash(data):
                clock[0] = 3
                return original_hash(data)
            with patch('tools.encounter_frontier.hashlib.sha256', side_effect=late_hash):
                result = frontier.report(complete=True, offset=0, limit=1)
            self.assertEqual(result['status'], 'analysis_time_budget_exhausted')
            self.assertIsNone(result['frontier_gear_set_count']); self.assertEqual(result['gear_sets'], [])

    def test_independent_dominance_oracle_pages_ties_and_order(self):
        rng = random.Random(98)
        for dimensions in range(1, 9):
            for _ in range(20):
                rows = [row(i, [rng.randrange(-4, 5) for _ in range(dimensions)]) for i in range(1, 21)]
                expected = sorted(r['simulator_gear_sha256'] for r in rows if not any(
                    all(a >= b for a,b in zip(other['gains'], r['gains']))
                    and any(a > b for a,b in zip(other['gains'], r['gains'])) for other in rows))
                before = deepcopy(rows); fingerprints = []
                for _ in range(3):
                    rng.shuffle(rows); frontier = Frontier(dimensions, 'gains')
                    for candidate in rows: frontier.add(candidate)
                    actual = frontier.report(complete=True, offset=0, limit=20)
                    self.assertEqual([r['simulator_gear_sha256'] for r in actual['gear_sets']], expected)
                    fingerprints.append(actual['frontier_point_estimates_sha256'])
                    combined = []
                    for offset in range(0, len(expected)+1, 3):
                        page = frontier.report(complete=True, offset=offset, limit=3)
                        combined.extend(r['simulator_gear_sha256'] for r in page['gear_sets'])
                        self.assertEqual(page['frontier_gear_set_count'], len(expected))
                        self.assertEqual(page['frontier_point_estimates_sha256'], fingerprints[-1])
                    self.assertEqual(combined, expected)
                    unknown = frontier.report(complete=False, offset=0, limit=20)
                    self.assertIsNone(unknown['frontier_gear_set_count']); self.assertEqual(unknown['gear_sets'], [])
                self.assertEqual(len(set(fingerprints)), 1)
                self.assertEqual(sorted(rows,key=lambda r:r['simulator_gear_sha256']), before)

    def test_bounds_unknown_budget_conflicting_duplicate_and_zero_normalisation(self):
        frontier = Frontier(2, 'gains', candidate_limit=2)
        frontier.add(row(1, [1, 2])); frontier.add(row(2, [1.0, 2.0]))
        self.assertEqual(frontier.report(complete=True, offset=0, limit=20)['frontier_gear_set_count'], 2)
        frontier.add(row(3, [2, 3]))
        result = frontier.report(complete=True, offset=0, limit=20)
        self.assertEqual(result['status'], 'candidate_budget_exhausted')
        self.assertIsNone(result['frontier_gear_set_count']); self.assertEqual(result['gear_sets'], [])
        timeout = Frontier(1, 'gains', deadline=0); timeout.add(row(1, [1]))
        self.assertEqual(timeout.report(complete=True, offset=0, limit=1)['status'], 'analysis_time_budget_exhausted')
        duplicate = Frontier(1, 'gains'); duplicate.add(row(1, [1]))
        with self.assertRaises(ValueError): duplicate.add(row(1, [2]))
        for gains in [[True], [float('nan')], [float('inf')], [10**400], [], [1, 2]]:
            with self.assertRaises(ValueError): Frontier(1, 'gains').add(row(1, gains))
        for bad in [None, 0, 1, 'true', []]:
            with self.assertRaises(ValueError): options(SimpleNamespace(include_encounter_frontier=bad))
        for bad in [-1, True, 4097, None]:
            with self.assertRaises(ValueError): options(SimpleNamespace(include_encounter_frontier=True, encounter_frontier_offset=bad))
        with self.assertRaises(ValueError): options(SimpleNamespace(encounter_frontier_offset=1))
        empty = Frontier(1, 'gains').report(complete=True, offset=4096, limit=1)
        self.assertEqual(empty['frontier_gear_set_count'], 0); self.assertFalse(empty['has_more'])

    def test_maximum_tied_pool_and_role_improvement_direction(self):
        tied = Frontier(8, 'gains')
        for index in range(1, 4097): tied.add(row(index, [1]*8))
        page = tied.report(complete=True, offset=4090, limit=20)
        self.assertEqual(page['frontier_gear_set_count'], 4096)
        self.assertEqual(len(page['gear_sets']), 6); self.assertFalse(page['has_more'])
        tied.add(row(4097, [2]*8))
        self.assertIsNone(tied.report(complete=True, offset=0, limit=20)['frontier_gear_set_count'])
        for metric in ['dtps', 'tps', 'death', 'hps', 'oom']:
            base = .5 if metric == 'death' else 100
            better = .4 if metric == 'death' else 90 if metric in ['dtps', 'oom'] else 110
            profiles = [{'baseline_review': 1, 'results': [
                {'review': 1, roles.FIELDS[metric]: base},
                {'review': 2, roles.FIELDS[metric]: better if encounter == 0 else base},
                {'review': 3, roles.FIELDS[metric]: base if encounter == 0 else better}]} for encounter in range(2)]
            rankings = roles.weighted_ranking(profiles, [.5, .5], metric)
            frontier = Frontier(2, 'gains')
            for candidate in rankings['results']:
                frontier.add(row(candidate['review'], candidate['improvement_by_encounter']))
            result = frontier.report(complete=True, offset=0, limit=20)
            self.assertEqual([r['simulator_gear_sha256'] for r in result['gear_sets']], [f'{i:064x}' for i in [2,3]])
