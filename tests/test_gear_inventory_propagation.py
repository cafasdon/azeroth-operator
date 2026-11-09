"""Retained search scope remains visible without promoting it to ownership proof."""
from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from test_merge_gear_windows import report
from tools.merge_forever_gear_windows import checked_inventory_evidence, merge_reports
from tools.journal_forever_gear_windows import append_report, journal_status


def evidence():
    return {'scope':'all_observed_storage_before_filters',
        'observed_storage_sections':['equipment','bags'],
        'unavailable_storage_sections':['character_bank','account_bank'],
        'all_storage_sections_observed':False,'catalogue_unmatched_item_locations':2,
        'catalogue_unmatched_distinct_item_ids':1,'catalogue_unmatched_item_ids':[4294967295],
        'catalogue_unmatched_item_ids_truncated':False,'complete_ownership_verified':False,
        'complete_simulator_resolution_verified':False,'best_in_slot_verified':False}


class GearInventoryPropagationTests(unittest.TestCase):
    def test_journal_restart_preserves_bound_evidence_and_legacy_unknown(self):
        page=report(0,['a'*64],4); page['gear_search']['inventory_evidence']=evidence()
        self.assertEqual(merge_reports([page])['inventory_evidence'],evidence())
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'plans.sqlite'; append_report(path,page)
            before=path.read_bytes()
            for _ in range(2): self.assertEqual(journal_status(path)['inventory_evidence'],evidence())
            self.assertEqual(path.read_bytes(),before)
            changed=report(4,['a'*64],4); changed['gear_search']['inventory_evidence']=evidence()
            changed['gear_search']['inventory_evidence']['catalogue_unmatched_item_locations']=3
            with self.assertRaises(ValueError): append_report(path,changed)
            self.assertEqual(path.read_bytes(),before)
            legacy=report(0,['a'*64],4)
            old=Path(folder)/'legacy.sqlite'; append_report(old,legacy)
            self.assertIsNone(journal_status(old)['inventory_evidence'])
            del legacy['gear_search']['inventory_evidence']
            self.assertIsNone(merge_reports([legacy])['inventory_evidence'])

    def test_strict_shape_counts_storage_partition_flags_and_id_bounds(self):
        valid=evidence()
        self.assertEqual(checked_inventory_evidence(valid),valid)
        cases=[('PRIVATE','PRIVATE'),('scope','PRIVATE'),('observed_storage_sections',['bags','equipment']),
            ('observed_storage_sections',['equipment','equipment','bags']),
            ('observed_storage_sections',['equipment','bags','character_bank']),
            ('unavailable_storage_sections',['account_bank']),('all_storage_sections_observed',1),
            ('all_storage_sections_observed',True),('catalogue_unmatched_item_locations',True),
            ('catalogue_unmatched_item_locations',0),('catalogue_unmatched_distinct_item_ids',1.0),
            ('catalogue_unmatched_item_ids',[0]),('catalogue_unmatched_item_ids',[2**32]),
            ('catalogue_unmatched_item_ids',[True]),('catalogue_unmatched_item_ids',[]),
            ('catalogue_unmatched_item_ids_truncated',True),('complete_ownership_verified',True),
            ('complete_simulator_resolution_verified',0),('best_in_slot_verified',None)]
        for field,bad in cases:
            candidate=deepcopy(valid); candidate[field]=bad
            with self.subTest(field=field,bad=bad),self.assertRaises(ValueError): checked_inventory_evidence(candidate)
        for change in [
            {'catalogue_unmatched_item_locations':2**128},
            {'catalogue_unmatched_item_locations':2**64,'catalogue_unmatched_distinct_item_ids':2**32},
            {'observed_storage_sections':[],'unavailable_storage_sections':['equipment','bags','character_bank','account_bank']},
            {'catalogue_unmatched_distinct_item_ids':0,'catalogue_unmatched_item_ids':[]}]:
            with self.subTest(change=change),self.assertRaises(ValueError): checked_inventory_evidence({**valid,**change})
        truncated={**valid,'catalogue_unmatched_item_locations':51,'catalogue_unmatched_distinct_item_ids':51,
            'catalogue_unmatched_item_ids':list(range(1,51)),'catalogue_unmatched_item_ids_truncated':True}
        self.assertEqual(checked_inventory_evidence(truncated),truncated)
        complete={**valid,'observed_storage_sections':['equipment','bags','character_bank','account_bank'],
            'unavailable_storage_sections':[],'all_storage_sections_observed':True,
            'catalogue_unmatched_item_locations':0,'catalogue_unmatched_distinct_item_ids':0,'catalogue_unmatched_item_ids':[]}
        self.assertFalse(checked_inventory_evidence(complete)['complete_ownership_verified'])


if __name__=='__main__': unittest.main()
