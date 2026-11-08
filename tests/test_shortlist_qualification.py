"""Qualification reports preserve misses, deduplication and fresh seed inputs."""
from copy import deepcopy
from pathlib import Path
import io
import subprocess
import unittest
from unittest.mock import patch
from tools.qualify_forever_shortlist import audit_pool, main


class QualificationTests(unittest.TestCase):
    def pool(self):
        return [{'raid': {'parties': [{'players': [{'equipment': {'items': [{'id': item}]}}]}]},
                 'simOptions': {'iterations': 1000, 'randomSeed': '99'}} for item in (1, 2, 2, 3)]

    def test_distinct_aliases_misses_and_unmodified_input(self):
        pool = self.pool()
        original = deepcopy(pool)
        fixed = {'unique_runs': 3, 'results': [
            {'review':i, 'mean_dps':dps} for i,dps in [(1,100),(2,110),(3,110),(4,120)]]}
        staged = {'results':[{'review':1,'mean_dps':100},{'review':2,'mean_dps':110}],
                  'stage_metrics':[{'stage':'BulkSimStageLow','survivors':1}]}
        with patch('tools.qualify_forever_shortlist.verify_engine'), \
             patch('tools.qualify_forever_shortlist.compare', return_value=fixed) as run, \
             patch('tools.qualify_forever_shortlist.compare_bulk', return_value=staged) as bulk:
            result = audit_pool(pool,[10,20],2,Path('fixed'),'a'*64,Path('bulk'),'b'*64)
        self.assertEqual(pool,original)
        self.assertEqual(result['runs_with_missed_fixed_top'],2)
        self.assertEqual(result['runs'][0]['fixed_top_missing_from_shortlist'],[4])
        self.assertEqual(result['runs'][0]['fixed_top_reviews'],[4,2])
        self.assertEqual(result['runs'][0]['fixed_unique_gear_sets'],3)
        self.assertNotIn('raid', str(result))
        for calls in (run.call_args_list,bulk.call_args_list):
            self.assertEqual([call.args[2][0]['simOptions']['randomSeed'] for call in calls],['10','20'])
            self.assertTrue(all('cache' not in call.kwargs for call in calls))

    def test_invalid_budgets_rejected_before_launch(self):
        for seeds, top in [([],1),([1,1],1),([True],1),([0],1),([2**32],1),([1,2,3,4],1),([1],0),([1],11)]:
            with self.subTest(seeds=seeds,top=top), patch('tools.qualify_forever_shortlist.verify_engine') as verify:
                with self.assertRaises(ValueError):
                    audit_pool(self.pool(),seeds,top,Path('x'),'a'*64,Path('y'),'b'*64)
                verify.assert_not_called()

    def test_duplicate_only_pool_and_aggregate_budget(self):
        duplicate = [self.pool()[0]] * 2
        fixed = {'unique_runs':1,'results':[{'review':1,'mean_dps':100},{'review':2,'mean_dps':100}]}
        with patch('tools.qualify_forever_shortlist.verify_engine'), \
             patch('tools.qualify_forever_shortlist.compare_bulk',return_value=None), \
             patch('tools.qualify_forever_shortlist.compare',return_value=fixed):
            result = audit_pool(duplicate,[1],1,Path('x'),'a'*64,Path('y'),'b'*64)
        self.assertEqual(result['runs'][0]['fixed_top_reviews'],[])
        self.assertEqual(result['runs'][0]['stage_metrics'],[])
        pool = [deepcopy(self.pool()[0]) for _ in range(101)]
        for i,row in enumerate(pool): row['raid']['parties'][0]['players'][0]['equipment']['items'][0]['id']=i
        with patch('tools.qualify_forever_shortlist.verify_engine') as verify:
            with self.assertRaisesRegex(ValueError,'100000'):
                audit_pool(pool,[1],1,Path('x'),'a'*64,Path('y'),'b'*64)
            verify.assert_not_called()

    def test_budget_counts_after_seed_normalisation(self):
        pool = [deepcopy(self.pool()[0]) for _ in range(101)]
        for i,row in enumerate(pool): row['simOptions']['randomSeed']=str(i+1)
        fixed = {'unique_runs':1,'results':[{'review':i,'mean_dps':100} for i in range(1,102)]}
        with patch('tools.qualify_forever_shortlist.verify_engine'), \
             patch('tools.qualify_forever_shortlist.compare_bulk',return_value=None), \
             patch('tools.qualify_forever_shortlist.compare',return_value=fixed):
            result = audit_pool(pool,[999],1,Path('x'),'a'*64,Path('y'),'b'*64)
        self.assertEqual(result['runs'][0]['fixed_unique_gear_sets'],1)

    def test_cli_handles_malformed_pool_and_engine_failure(self):
        argv = ['audit','--pool','pool.json','--engine','x','--engine-sha256','a'*64,
                '--bulk-engine','y','--bulk-engine-sha256','b'*64,'--seed','1']
        for error in (IndexError('list index out of range'), subprocess.TimeoutExpired('engine',120)):
            with patch('sys.argv',argv), patch('tools.qualify_forever_shortlist.read_object',return_value={'requests':self.pool()}), \
                 patch('tools.qualify_forever_shortlist.verify_engine'), \
                 patch('tools.qualify_forever_shortlist.compare_bulk',side_effect=error), \
                 patch('sys.stderr',new_callable=io.StringIO) as stderr:
                with self.assertRaises(SystemExit) as stopped: main()
                self.assertEqual(stopped.exception.code,1)
                self.assertIn('Shortlist audit failed:',stderr.getvalue())


if __name__ == '__main__':
    unittest.main()
