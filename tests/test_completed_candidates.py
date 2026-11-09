"""Interrupted searches rank no candidate missing a requested encounter."""
from copy import deepcopy
import json
import unittest
from tools.run_forever_encounter_windows import completed_candidate_ranking
from tools import role_metrics as roles


class CompletedCandidateTests(unittest.TestCase):
    def test_intersection_not_union_or_reweighted_missing_scores(self):
        def row(review, value):
            return {'review':review, 'mean_dps':value, 'mean_dps_delta_from_baseline':value-100,
                    '_dps_samples':['PRIVATE'], 'name':'PRIVATE'}
        results = [{'results':[row(1,100),row(2,120),row(3,200)]},
                   {'results':[row(1,100),row(2,90),row(4,500)]}]
        before = deepcopy(results)
        gear = {i:format(i,'064x') for i in range(1,5)}
        result = completed_candidate_ranking(results,[.25,.75],'dps',4,gear)
        self.assertEqual([r['review'] for r in result['results']], [1,2])
        self.assertAlmostEqual(result['results'][1]['weighted_dps_gain_pct'], -2.5)
        self.assertEqual(result['results'][1]['gain_pct_by_encounter'], [20,-10])
        self.assertEqual((result['completed_candidate_count'],result['omitted_candidate_count']), (2,2))
        self.assertIs(result['requested_results_complete'],False)
        self.assertNotIn('PRIVATE',json.dumps(result))
        self.assertEqual(results,before)
        self.assertIsNone(completed_candidate_ranking(results[:1],[.25,.75],'dps',4,gear))
        absent = deepcopy(results);absent[1]['results'].pop(0)
        self.assertIsNone(completed_candidate_ranking(absent,[.25,.75],'dps',4,gear))
        zero = deepcopy(results);zero[1]['results'][0]['mean_dps']=0
        self.assertIsNone(completed_candidate_ranking(zero,[.25,.75],'dps',4,gear))

    def test_objectives_ties_and_native_zero_baselines(self):
        gear = {i:format(i,'064x') for i in range(1,5)}
        results = [{'results':[{'review':i,'mean_dps':v,'mean_dps_delta_from_baseline':v-100}
                               for i,v in [(1,100),(2,200),(3,110)]]},
                   {'results':[{'review':i,'mean_dps':v,'mean_dps_delta_from_baseline':v-100}
                               for i,v in [(1,100),(2,90),(3,110)]]}]
        average = completed_candidate_ranking(results,[.5,.5],'dps',4,gear,top=1)
        robust = completed_candidate_ranking(results,[.5,.5],'dps',4,gear,'worst_encounter_gain',1)
        self.assertEqual(average['results'][0]['review'],2)
        self.assertEqual(robust['results'][0]['review'],3)
        self.assertIs(robust['results_truncated'],True)
        self.assertEqual(robust['completed_candidate_count'],3)
        for metric,field in roles.FIELDS.items():
            if metric == 'dps': continue
            profiles = [{'results':[{'review':1,field:0},{'review':2,field:.1}]} for _ in range(2)]
            result = completed_candidate_ranking(profiles,[.5,.5],metric,3,gear)
            values = {row['review']:row['weighted_improvement'] for row in result['results']}
            self.assertAlmostEqual(values[2], .1 if metric in ('tps','hps') else -10 if metric=='death' else -.1)
            self.assertEqual(values[1],0)
        tied = [{'results':[{'review':i,'mean_dps':100,'mean_dps_delta_from_baseline':0} for i in [3,1,2]]}]*2
        self.assertEqual([r['review'] for r in completed_candidate_ranking(tied,[.5,.5],'dps',4,gear,top=2)['results']], [1,2])
        for bad in [0,21,True]:
            with self.assertRaises(ValueError):
                completed_candidate_ranking(results,[.5,.5],'dps',4,gear,top=bad)


if __name__ == '__main__':
    unittest.main()
