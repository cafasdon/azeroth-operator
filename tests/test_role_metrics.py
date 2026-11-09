"""Contract examples for native role metric comparisons."""
from copy import deepcopy
from types import SimpleNamespace
import unittest

from tools import role_metrics as metrics


def encounter(metric, values, baseline=1):
    return {'baseline_review': baseline, 'results': [
        {'review': review, metrics.FIELDS[metric]: value}
        for review, value in enumerate(values, 1)]}


class RoleMetricTests(unittest.TestCase):
    def test_argument_default_and_supported_metrics(self):
        self.assertEqual(metrics.rank_by(SimpleNamespace()), 'dps')
        for metric in ('dps', 'dtps', 'tps', 'death', 'hps', 'oom'):
            self.assertEqual(metrics.rank_by(SimpleNamespace(rank_by=metric)), metric)
        class String(str):
            pass
        for bad in (None, True, 1, [], {}, 'DPS', 'tank', String('dps')):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                metrics.rank_by(SimpleNamespace(rank_by=bad))

    def test_known_native_improvements_with_unequal_weights(self):
        examples = [
            ('dtps', [100, 80], [200, 240], [20, -40], 5, 'damage_per_second'),
            ('tps', [100, 120], [-20, -60], [20, -40], 5, 'threat_per_second'),
            ('death', [.5, .3], [.1, .5], [20, -40], 5, 'percentage_points'),
            ('hps', [100, 120], [200, 160], [20, -40], 5, 'healing_per_second'),
            ('oom', [30, 10], [0, 40], [20, -40], 5, 'seconds'),
        ]
        for metric, first, second, improvements, score, unit in examples:
            with self.subTest(metric=metric):
                result = metrics.weighted_ranking(
                    [encounter(metric, first), encounter(metric, second)], [.75, .25], metric)
                self.assertEqual(result['rank_by'], metric)
                self.assertEqual(result['score_unit'], unit)
                self.assertEqual([row['review'] for row in result['results']], [2, 1])
                best, baseline = result['results']
                self.assertAlmostEqual(best['weighted_improvement'], score)
                for actual, expected in zip(best['improvement_by_encounter'], improvements):
                    self.assertAlmostEqual(actual, expected)
                self.assertEqual(best['metric_values_by_encounter'], [first[1], second[1]])
                self.assertEqual(baseline['weighted_improvement'], 0)

    def test_zero_baselines_and_ties_are_valid(self):
        for metric in ('dtps', 'tps', 'death', 'hps', 'oom'):
            with self.subTest(metric=metric):
                rows = metrics.weighted_ranking([encounter(metric, [0, 0, 0])], [1], metric)['results']
                self.assertEqual([row['review'] for row in rows], [1, 2, 3])
                self.assertEqual([row['weighted_improvement'] for row in rows], [0, 0, 0])

    def test_positive_improvement_from_zero_and_negative_regression(self):
        for metric in ('tps', 'hps'):
            rows = metrics.weighted_ranking([encounter(metric, [0, 12])], [1], metric)['results']
            self.assertEqual((rows[0]['review'], rows[0]['weighted_improvement']), (2, 12))
        for metric in ('dtps', 'oom'):
            rows = metrics.weighted_ranking([encounter(metric, [0, 12])], [1], metric)['results']
            self.assertEqual((rows[-1]['review'], rows[-1]['weighted_improvement']), (2, -12))

    def test_output_is_numeric_projection_and_input_is_unchanged(self):
        profile = encounter('hps', [0, 5])
        profile['private_source'] = {'path': 'private'}
        profile['results'][1]['simulator_gear_sha256'] = 'private'
        original = deepcopy(profile)
        result = metrics.weighted_ranking([profile, profile], [.5, .5], 'hps')
        self.assertEqual(profile, original)
        self.assertEqual(set(result), {'rank_by', 'score_unit', 'results'})
        self.assertEqual(set(result['results'][0]), {
            'review', 'weighted_improvement', 'improvement_by_encounter', 'metric_values_by_encounter'})
        self.assertEqual(result['results'][0]['weighted_improvement'], 5)

    def test_value_rejects_missing_nonfinite_non_numeric_and_out_of_range(self):
        for metric in metrics.FIELDS:
            field = metrics.FIELDS[metric]
            for bad in (True, False, None, '1', [], {}, float('nan'), float('inf'), -float('inf'), 10**1000):
                with self.subTest(metric=metric, bad=type(bad).__name__), self.assertRaises(ValueError):
                    metrics.value({field: bad}, metric)
            for row in ({}, None, []):
                with self.assertRaises(ValueError):
                    metrics.value(row, metric)
            if metric != 'tps':
                with self.assertRaises(ValueError):
                    metrics.value({field: -1}, metric)
        self.assertEqual(metrics.value({'mean_tps': -5}, 'tps'), -5)
        with self.assertRaises(ValueError):
            metrics.value({'simulated_death_fraction': 1.01}, 'death')

    def test_rejects_dps_and_unknown_metrics(self):
        for metric in ('dps', 'unknown', None, []):
            with self.assertRaises(ValueError):
                metrics.weighted_ranking([encounter('hps', [0, 1])], [1], metric)

    def test_profiles_must_be_nonempty_bounded_and_well_formed(self):
        valid = encounter('hps', [0, 1])
        invalid = [[], None, {}, [valid] * 9, [None], [{}],
                   [{'baseline_review': 1, 'results': []}],
                   [{'baseline_review': 1, 'results': [None]}],
                   [{'baseline_review': 1, 'results': [{'mean_hps': 0}]}],
                   [{'baseline_review': 1, 'results': [{'review': 1}]}]]
        for profiles in invalid:
            with self.subTest(profiles=profiles), self.assertRaises(ValueError):
                metrics.weighted_ranking(profiles, [1], 'hps')
        incomplete = deepcopy(valid)
        incomplete['requested_results_complete'] = False
        with self.assertRaises(ValueError):
            metrics.weighted_ranking([incomplete], [1], 'hps')

    def test_review_ids_baseline_and_complete_sets(self):
        valid = encounter('hps', [0, 1])
        for bad in (True, False, 1.0, '1', 0, -1, None):
            for key in ('review', 'baseline_review'):
                profile = deepcopy(valid)
                (profile if key == 'baseline_review' else profile['results'][0])[key] = bad
                with self.subTest(bad=bad, key=key), self.assertRaises(ValueError):
                    metrics.weighted_ranking([profile], [1], 'hps')
        duplicate = deepcopy(valid)
        duplicate['results'].append(deepcopy(duplicate['results'][0]))
        absent = encounter('hps', [0, 1], baseline=3)
        for profile in (duplicate, absent):
            with self.assertRaises(ValueError):
                metrics.weighted_ranking([profile], [1], 'hps')
        for profile in (encounter('hps', [0]), encounter('hps', [0, 1, 2]),
                        encounter('hps', [0, 1], baseline=2)):
            with self.assertRaises(ValueError):
                metrics.weighted_ranking([valid, profile], [.5, .5], 'hps')

    def test_weights_must_be_finite_positive_and_normalised(self):
        profile = encounter('hps', [0, 1])
        for weights in (None, [], [1, 1], [0], [-1], [True], ['1'], [float('nan')],
                        [float('inf')], [10**1000], [.5], [1.01]):
            with self.subTest(weights=weights), self.assertRaises(ValueError):
                metrics.weighted_ranking([profile], weights, 'hps')
        with self.assertRaises(ValueError):
            metrics.weighted_ranking([profile, profile], [1e308, 1e308], 'hps')
        result = metrics.weighted_ranking([profile] * 3, [1 / 3] * 3, 'hps')
        self.assertEqual(result['results'][0]['weighted_improvement'], 1)

    def test_finite_metrics_cannot_overflow_an_improvement(self):
        with self.assertRaises(ValueError):
            metrics.weighted_ranking([encounter('tps', [-1e308, 1e308])], [1], 'tps')

    def test_normalisation_tolerance_cannot_allow_nonfinite_scores(self):
        largest = float.fromhex('0x1.fffffffffffffp+1023')
        profile = encounter('hps', [0, largest])
        # These sums are within normalisation tolerance, but must still fail
        # if the product or final sum exceeds the finite float range.
        with self.assertRaises(ValueError):
            metrics.weighted_ranking([profile], [1.0000000001], 'hps')
        with self.assertRaises(ValueError):
            metrics.weighted_ranking([profile, profile], [.5000000001] * 2, 'hps')


if __name__ == '__main__':
    unittest.main()
