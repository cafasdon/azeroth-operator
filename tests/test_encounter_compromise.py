"""Independent exhaustive minimax reference for saved encounter compromise gear."""
from copy import deepcopy
import asyncio
from fractions import Fraction
import random
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from tools.encounter_compromise import Compromise, options


def row(index, gains):
    return {'simulator_gear_sha256': f'{index:064x}', 'gains': gains}


def fraction(value):
    return Fraction(int(value['numerator']), int(value['denominator']))


class CompromiseTests(unittest.TestCase):
    def test_exhaustive_reference_pages_order_and_ties(self):
        rng = random.Random(103)
        for dimensions in range(1, 9):
            for _ in range(15):
                rows = [row(i, [rng.randint(-10, 20) for _ in range(dimensions)]) for i in range(1, 22)]
                before = deepcopy(rows)
                best = [max(r['gains'][j] for r in rows) for j in range(dimensions)]
                expected = sorted(rows, key=lambda r: (max(a-b for a,b in zip(best,r['gains'])), r['simulator_gear_sha256']))
                scores = [max(a-b for a,b in zip(best,r['gains'])) for r in expected]
                fingerprints = []
                for _ in range(2):
                    rng.shuffle(rows)
                    pool = Compromise(dimensions, 'gains')
                    for candidate in rows: pool.add(candidate)
                    pages = [pool.report(complete=True, offset=offset, limit=4) for offset in range(0, 24, 4)]
                    actual = [entry for page in pages for entry in page['gear_sets']]
                    self.assertEqual([entry['gear_set'] for entry in actual], expected)
                    self.assertEqual([fraction(entry['worst_regret_exact']) for entry in actual], scores)
                    self.assertEqual(pages[0]['optimal_tied_gear_set_count'], scores.count(scores[0]))
                    fingerprints.extend(page['point_estimates_sha256'] for page in pages)
                    self.assertTrue(all(page['eligible_distinct_candidate_count'] == 21 for page in pages))
                    self.assertFalse(pages[-1]['has_more'])
                self.assertEqual(len(set(fingerprints)), 1)
                self.assertEqual(sorted(rows, key=lambda r:r['simulator_gear_sha256']), before)

    def test_exact_fraction_extremes_dominated_ties_and_no_input_mutation(self):
        pool = Compromise(2, 'gains')
        # Dominated candidates can tie on minimax regret and must remain present.
        for candidate in [row(1,[10,0]), row(2,[0,10]), row(3,[5,5]), row(4,[5,6])]: pool.add(candidate)
        report = pool.report(complete=True, offset=0, limit=20)
        self.assertEqual(report['optimal_tied_gear_set_count'], 2)
        self.assertEqual([e['gear_set']['simulator_gear_sha256'] for e in report['gear_sets'][:2]], [f'{i:064x}' for i in [3,4]])
        report['gear_sets'][0]['gear_set']['gains'].clear()
        self.assertEqual(len(pool.report(complete=True, offset=0, limit=1)['gear_sets'][0]['gear_set']['gains']),2)
        exact = Compromise(1,'gains')
        exact.add(row(1,[2**53])); exact.add(row(2,[2**53+1]))
        output = exact.report(complete=True,offset=0,limit=20)
        self.assertEqual(fraction(output['gear_sets'][1]['worst_regret_exact']),1)
        huge = Compromise(1,'gains'); huge.add(row(1,[-1.7e308])); huge.add(row(2,[1.7e308]))
        self.assertEqual(fraction(huge.report(complete=True,offset=0,limit=20)['gear_sets'][1]['worst_regret_exact']),
                         Fraction(1.7e308)-Fraction(-1.7e308))

    def test_unknowns_caps_deadlines_and_options(self):
        pool = Compromise(1,'gains',candidate_limit=1); pool.add(row(1,[1])); pool.add(row(2,[2]))
        capped = pool.report(complete=True,offset=0,limit=1)
        self.assertEqual(capped['status'],'candidate_budget_exhausted')
        self.assertIsNone(capped['point_estimates_sha256']); self.assertEqual(capped['gear_sets'],[])
        partial = Compromise(1,'gains'); partial.add(row(1,[1]))
        result = partial.report(complete=False,offset=0,limit=1)
        self.assertEqual(result['status'],'incomplete_saved_results'); self.assertIsNone(result['minimum_worst_regret_exact'])
        late = Compromise(1,'gains',deadline=0)
        self.assertEqual(late.report(complete=True,offset=0,limit=1)['status'],'analysis_time_budget_exhausted')
        with patch('tools.encounter_compromise.monotonic',side_effect=[0,0,6]):
            after = Compromise(1,'gains',deadline=5)
            self.assertEqual(after.report(complete=True,offset=0,limit=1)['gear_sets'],[])
        for bad in [True,'1',float('nan'),float('inf'),10**400]:
            with self.assertRaises(ValueError): Compromise(1,'gains').add(row(1,[bad]))
        for args in [SimpleNamespace(include_encounter_compromise=1),SimpleNamespace(encounter_compromise_offset=1),
                     SimpleNamespace(include_encounter_compromise=True,encounter_compromise_offset=4097)]:
            with self.assertRaises(ValueError): options(args)

    def test_final_hash_deadline_and_entrypoint_validation(self):
        from tools import encounter_compromise as module
        from tools import run_forever_saved_journal as runner
        from tools import evaluate_forever_owned_gear as owned
        clock = [0]
        with patch.object(module,'monotonic',side_effect=lambda:clock[0]):
            pool = Compromise(1,'gains',deadline=5); pool.add(row(1,[1]))
            original = module.hashlib.sha256
            def late_hash(data):
                clock[0] = 6
                return original(data)
            with patch.object(module.hashlib,'sha256',side_effect=late_hash):
                result = pool.report(complete=True,offset=0,limit=1)
            self.assertEqual(result['status'],'analysis_time_budget_exhausted')
            self.assertIsNone(result['point_estimates_sha256']); self.assertEqual(result['gear_sets'],[])
        for entrypoint in [runner,owned]:
            for args in [SimpleNamespace(include_encounter_compromise=1),SimpleNamespace(encounter_compromise_offset=1)]:
                with patch.object(runner.journal,'journal_status') as read:
                    with self.assertRaises(ValueError): asyncio.run(entrypoint.execute(args))
                    read.assert_not_called()
