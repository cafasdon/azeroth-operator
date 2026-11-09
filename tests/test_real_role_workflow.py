"""Opt-in actual offline tank metrics from pinned public source inputs, not game data."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import evaluate_forever_owned_gear as owned
from tools import run_forever_saved_journal as journal
from tools import positional_equipment as layout


class RealRoleWorkflowTests(unittest.TestCase):
    def test_opt_in_pinned_tank_metrics_and_shared_family_checkpoints(self):
        if not all(os.environ.get(key) for key in ['AZEROTH_POSITIONAL_ARTIFACTS','AZEROTH_QUALIFY_SOURCE_ARCHIVE']):
            self.skipTest('Explicit local qualified engine/source archive required; no downloads/builds')
        archive=Path(os.environ['AZEROTH_QUALIFY_SOURCE_ARCHIVE']).resolve(strict=True)
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(),
            'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712')
        prefix='forever-9dc32ac8ca0d8d1b76337861d38b93c0f884af11/ui/specs/warrior/protection/'
        with zipfile.ZipFile(archive) as source:
            apl=source.read(prefix+'apls/default.apl.json')
            gear=source.read(prefix+'gear_sets/prebis.gear.json')
        self.assertEqual(hashlib.sha256(apl).hexdigest(),'befce19883766e21937c3c904a996dae2f7c4f56efe1b1f97ea41fb615f3399d')
        self.assertEqual(hashlib.sha256(gear).hexdigest(),'21ba1f3f145900d4e32500fa9bfd7a16252bea120c50751933e9d3e412788402')
        root=Path(__file__).resolve().parents[1]
        engine=(Path(os.environ['AZEROTH_POSITIONAL_ARTIFACTS'])/'wowsimcli-positional.exe').resolve(strict=True)
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),layout.ENGINE_SHA256)
        value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['stats']={'level':60,'class':'WARRIOR','race':'Orc','client_version':'synthetic'}
        value['sections']['professions']={str(i):{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':skill}
            for i,skill in enumerate([202,164],1)}
        # Public preset base IDs only; this synthetic capture intentionally has no enchants/suffixes.
        slots=[1,2,3,15,5,9,10,6,7,8,11,12,13,14,16,17,18]
        value['sections']['equipment']={str(slot):{'item_id':item['id'],'link':f"item:{item['id']}:0:0:0:0:0:0"}
            for slot,item in zip(slots,json.loads(gear)['items'],strict=True) if item.get('id')}
        player={'class':'ClassWarrior','race':'RaceOrc','protectionWarrior':{'options':{'classOptions':{
            'defaultStance':2,'startingRage':0,'queueDelay':250,'useBattleShout':True}}},
            'talentsString':'05-05050005-150531120300001351','profession1':'Engineering','profession2':'Blacksmithing',
            'rotation':json.loads(apl),'healingModel':{'hps':815,'cadenceSeconds':.3,'cadenceVariation':.64,
                'absorbFrac':.015,'burstWindow':6,'inspirationUptime':.25}}
        template={'raid':{'parties':[{'players':[player]}],'tanks':[{'type':1,'index':0}]}}
        _,_,encounter=inputs(); encounter['simulation_input']['simOptions']={'iterations':1000,'randomSeed':'20261109'}
        encounter['simulation_input']['encounter']['targets']=[{'level':63,'minBaseDamage':1500,'swingSpeed':2,'tankIndex':0}]
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); key=SnapshotStore(base).ingest(value); snapshot=(base/f'{key}.json').read_bytes()
            (base/'template.json').write_text(json.dumps(template))
            (base/'first.json').write_text(json.dumps(encounter))
            encounter['simulation_input']['encounter']['duration']=90
            (base/'second.json').write_text(json.dumps(encounter))
            argv=['--mcp',str((root/'target/debug/azeroth-operator.exe').resolve(strict=True)),
                '--snapshot-id',key,'--template',str(base/'template.json'),'--journal',str(base/'plans.sqlite'),
                '--engine',str(engine),'--engine-sha256',layout.ENGINE_SHA256,'--simulation-journal',str(base/'results'),
                '--encounter-lookup',str(base/'first.json'),'--encounter-lookup',str(base/'second.json'),
                '--rank-by','dtps','--window-size','1','--max-windows','1','--positional-equipment']
            with patch.dict(os.environ,{'AZEROTH_DATA_DIR':folder,
                'AZEROTH_FOREVER_CATALOGUE':str((root/'local-data/forever/catalogue.json').resolve(strict=True))}):
                result=asyncio.run(owned.execute(owned.parser().parse_args(argv)))
                self.assertEqual(result['status'],'collection_incomplete')
                self.assertEqual(result['simulation']['new_runs'],2)
                self.assertTrue(all(value>0 for value in result['simulation']['ranked_gear_sets'][0]['metric_values_by_encounter']))
                options=owned.parser().parse_args([*argv,'--checkpoint-only'])
                for metric in ['tps','death']:
                    options.rank_by=metric
                    ranked=asyncio.run(journal.execute(options))
                    self.assertEqual(ranked['new_runs'],0)
                    self.assertTrue(ranked['all_saved_resolved_results_complete'])
                    values=ranked['ranked_gear_sets'][0]['metric_values_by_encounter']
                    self.assertTrue(all(value>0 for value in values) if metric=='tps' else all(0<=value<=1 for value in values))
                self.assertFalse(result['complete_ownership_verified']); self.assertFalse(result['best_in_slot_verified'])
                self.assertEqual((base/f'{key}.json').read_bytes(),snapshot)


if __name__=='__main__': unittest.main()
