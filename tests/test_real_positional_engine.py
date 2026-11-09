"""Opt-in qualification of the isolated positional engine; no MCP/game deployment."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest

from test_simulator_level import inputs
from tools import build_forever_positional_engine as builder
from tools import run_forever_sim as adapter


class RealPositionalEngineTests(unittest.TestCase):
    def setup_artifacts(self):
        if not os.environ.get('AZEROTH_POSITIONAL_ARTIFACTS'):
            self.skipTest('Explicit local positional build required; no downloads/builds')
        root=Path(os.environ['AZEROTH_POSITIONAL_ARTIFACTS']).resolve(strict=True)
        manifest=json.loads((root/'build-manifest.json').read_text())
        engine=root/'wowsimcli-positional.exe'; probe=root/'equipment-probe.exe'
        for path,key in [(engine,'engine_sha256'),(probe,'probe_sha256')]:
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),manifest[key])
        self.assertEqual(manifest['contract'],builder.CONTRACT)
        self.assertEqual(manifest['source_archive_sha256'],builder.SOURCE_SHA)
        for path,key in [(Path(builder.__file__),'builder_sha256'),
            (builder.FRAGMENTS/'constructor.go.txt','constructor_sha256'),
            (builder.FRAGMENTS/'probe.go.txt','probe_source_sha256'),
            (builder.FRAGMENTS/'capability.go.txt','capability_source_sha256')]:
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),manifest[key])
        return engine,probe,manifest

    def test_probe_slot_fidelity_modifiers_rejections_and_capability(self):
        engine,probe,manifest=self.setup_artifacts()
        capability=subprocess.run([str(engine),'azeroth-equipment-contract'],capture_output=True,text=True,timeout=10)
        self.assertEqual(capability.returncode,0,capability.stderr)
        self.assertEqual(json.loads(capability.stdout),{'contract':builder.CONTRACT,'equipment_field':'preserveSlots',
            'required_item_count':17,'legacy_default':'auto_place','native_qualification_performed':False})
        def run(items,preserve=True,success=True):
            reply=subprocess.run([str(probe)],input=json.dumps({'items':items,'preserveSlots':preserve}),
                capture_output=True,text=True,timeout=10)
            self.assertEqual(reply.returncode==0,success,reply.stderr)
            if success: return json.loads(reply.stdout)
            self.assertEqual(reply.stdout,''); self.assertEqual(reply.stderr.strip(),'Invalid equipment input')
        for slot,item in [(10,4549),(11,4549),(12,744),(13,744),(15,754)]:
            items=[{} for _ in range(17)]; items[slot]={'id':item}
            result=run(items)
            self.assertEqual([r['id'] for r in result],[row.get('id',0) for row in items])
            legacy=run(items,preserve=False)
            if slot in (11,13,15): self.assertNotEqual(legacy,result)
        items=[{} for _ in range(17)]
        items[11]={'id':9461,'randomSuffix':13012}; items[14]={'id':2042,'enchant':1900}
        result=run(items)
        self.assertEqual(result[11],{'id':9461,'enchant':0,'randomSuffix':13012})
        self.assertEqual(result[14],{'id':2042,'enchant':1900,'randomSuffix':0})
        run([{} for _ in range(16)],success=False)
        run([{} for _ in range(18)],success=False)
        wrong=[{} for _ in range(17)]; wrong[0]={'id':4549}; run(wrong,success=False)
        empty=[{} for _ in range(17)]; empty[11]={'enchant':1900}; run(empty,success=False)
        conflict=[{} for _ in range(17)]; conflict[14]={'id':2042}; conflict[15]={'id':754}; run(conflict,success=False)
        unknown=[{} for _ in range(17)]; unknown[11]={'id':4294967295}; run(unknown,success=False)
        for enchant in [2147483647,-1,1900]:
            invalid=[{} for _ in range(17)]; invalid[11]={'id':4549,'enchant':enchant}
            run(invalid,success=False)

    def test_actual_canonical_simulation_equivalence_and_exact_sparse_hole(self):
        engine,probe,manifest=self.setup_artifacts()
        if not all(os.environ.get(k) for k in ['AZEROTH_QUALIFY_ENGINE','AZEROTH_QUALIFY_TEMPLATE']):
            self.skipTest('Explicit incumbent engine and public template required')
        incumbent=Path(os.environ['AZEROTH_QUALIFY_ENGINE']).resolve(strict=True)
        template_path=Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        incumbent_sha='0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5'
        self.assertEqual(hashlib.sha256(incumbent.read_bytes()).hexdigest(),incumbent_sha)
        self.assertEqual(hashlib.sha256(template_path.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        template=json.loads(template_path.read_text()); _,review,encounter=inputs()
        review['checks']['simulator_equipment_spec']=deepcopy(adapter.single_player(template['raid'])['equipment'])
        review['captured_active_pet']={'present':True,'level':60}
        encounter['simulation_input']['simOptions']={'iterations':1000,'randomSeed':'20261109'}
        request=adapter.request_from(template,review,encounter)
        old=adapter.run(incumbent,incumbent_sha,request)
        legacy=adapter.run(engine,manifest['engine_sha256'],request)
        metrics=lambda row:{k:v for k,v in row.items() if k!='engine_sha256'}
        self.assertEqual(legacy['engine_sha256'],manifest['engine_sha256'])
        self.assertEqual(metrics(old),metrics(legacy))
        positional=deepcopy(request); adapter.single_player(positional['raid'])['equipment']['preserveSlots']=True
        exact=adapter.run(engine,manifest['engine_sha256'],positional)
        self.assertEqual(metrics(old),metrics(exact))
        # The incumbent discards unknown protobuf fields: the wire flag alone cannot gate this feature.
        old_flagged=adapter.run(incumbent,incumbent_sha,positional)
        self.assertEqual(old_flagged,old)
        denied=subprocess.run([str(incumbent),'azeroth-equipment-contract'],capture_output=True,text=True,timeout=10)
        self.assertNotEqual(denied.returncode,0)
        items=[{} for _ in range(17)]; items[11]={'id':4549}; items[14]={'id':2042}; items[16]={'id':22347}
        sparse=deepcopy(positional); adapter.single_player(sparse['raid'])['equipment']['items']=items
        measured=adapter.run(engine,manifest['engine_sha256'],sparse)
        self.assertGreater(measured['mean_dps'],0)
        print(json.dumps({'qualification':'isolated_positional_engine','engine_sha256':manifest['engine_sha256'],
            'canonical_legacy_equal':True,'canonical_positional_equal':True,'old_flag_ignored':True,
            'sparse_mean_dps':measured['mean_dps'],'game_execution':False}))


if __name__=='__main__': unittest.main()
