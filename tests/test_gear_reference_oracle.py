"""Check extracted class capabilities against exact upstream TypeScript evaluation."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import unittest
import zipfile
from tools.joint_build_optimizer import request_type

ORACLE = r"""
const fs=require('node:fs'),vm=require('node:vm');
const {stripTypeScriptTypes}=require('node:module');
const p=JSON.parse(fs.readFileSync(0,'utf8'));
let source=p.source.replace(/^import[\s\S]*?from ['"][^'"]+['"];\s*/m,'');
source=stripTypeScriptTypes(source).replaceAll('export const ','const ');
source+=`\nglobalThis.result={classes:Object.fromEntries(Object.keys(CLASS_ARMOR_TYPES).map(id=>[id,{
armor:CLASS_ARMOR_TYPES[id],races:CLASS_RACES[id],ranged:CLASS_RANGED_WEAPON_TYPES[id],
weapons:Object.fromEntries(CLASS_WEAPON_TYPES[id].map(v=>[v.weaponType,v.canUseTwoHand]))
}])),dual_wield:[...SPEC_CAN_DUAL_WIELD]};`;
const context=p.enums;vm.runInNewContext(source,context,{timeout:1000});
process.stdout.write(JSON.stringify(context.result));
"""


class GearReferenceOracleTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('AZEROTH_TALENT_SOURCE_ZIP') and shutil.which('node'),
                         'Explicit pinned source and Node needed')
    def test_exact_capability_tables(self):
        archive=Path(os.environ['AZEROTH_TALENT_SOURCE_ZIP'])
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(),
                         'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712')
        with zipfile.ZipFile(archive) as bundle:
            raw=bundle.read('forever-9dc32ac8ca0d8d1b76337861d38b93c0f884af11/ui/sim/player/classes/capabilities_auto_gen.ts')
        pool=request_type().DESCRIPTOR.file.pool
        enums={kind:{v.name:v.number for v in pool.FindEnumTypeByName('proto.'+kind).values}
               for kind in ('ArmorType','Class','Race','RangedWeaponType','Spec','WeaponSkill','WeaponType')}
        reply=subprocess.run(['node','-e',ORACLE],input=json.dumps(dict(source=raw.decode(),enums=enums)),
                             capture_output=True,text=True,check=True,timeout=15)
        expected=json.loads(reply.stdout)
        actual=json.loads((Path(__file__).resolve().parents[1]/'clients/forever/gear-rules.json').read_bytes())
        self.assertEqual(actual['classes'],expected['classes'])
        spec=enums['Spec']
        self.assertEqual({spec['Spec'+name[0].upper()+name[1:]] for name in actual['dual_wield']},set(expected['dual_wield']))
        self.assertEqual(actual['source_hashes']['ui/sim/player/classes/capabilities_auto_gen.ts'],hashlib.sha256(raw).hexdigest())


if __name__=='__main__':
    unittest.main()
