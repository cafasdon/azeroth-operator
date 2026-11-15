"""Opt-in agreement with the exact pinned upstream TypeScript talent checker."""
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import unittest
import zipfile

from tools.forever_talent_identity import talent_map
from tools import talent_builds

NODE_ORACLE = r"""
const fs = require('node:fs');
const vm = require('node:vm');
const {stripTypeScriptTypes} = require('node:module');
const payload = JSON.parse(fs.readFileSync(0, 'utf8'));
let source = fs.readFileSync(process.argv[1], 'utf8');
source = source.replace("import { locationKey } from './config';", 'const locationKey = location => `${location.rowIdx},${location.colIdx}`;');
source = stripTypeScriptTypes(source).replaceAll('export const ', 'const ');
source += '\nglobalThis.check = findBrokenTalents;';
const context = {};
vm.runInNewContext(source, context, {timeout:1000});
process.stdout.write(JSON.stringify(payload.map(v => context.check(v.trees,v.points).every(tree=>tree.every(broken=>!broken)))));
"""


class UpstreamTalentOracleTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('AZEROTH_TALENT_SOURCE_ZIP') and shutil.which('node'),
                         'Explicit pinned archive and Node with TypeScript stripping needed')
    def test_exact_upstream_checker_on_nine_classes(self):
        archive = Path(os.environ['AZEROTH_TALENT_SOURCE_ZIP'])
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(),
                         'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712')
        oracle = Path(__file__).parent/'fixtures/forever-broken-talents.ts'
        self.assertEqual(hashlib.sha256(oracle.read_bytes()).hexdigest(),
                         '8fd04c8e6228b5b4862472235e5cc59965a3f256919e58963817fd09310a15e4')
        cases, expected_context = [], []
        rng = random.Random(20261115)
        with zipfile.ZipFile(archive) as bundle:
            for name, mapping in talent_map()['classes'].items():
                trees = json.loads(bundle.read(f'forever-9dc32ac8ca0d8d1b76337861d38b93c0f884af11/ui/sim/talents/trees/{name}.json'))
                sizes = mapping['tree_sizes']
                for index in range(80):
                    level = (1,9,10,20,40,60)[index % 6]
                    # Sparse states exercise valid roots and impossible deep-only allocations.
                    digits = [rng.randint(0,row['max_rank']) if rng.random()<.15 else 0 for row in mapping['talents']]
                    if index == 0: digits = [0]*len(digits)
                    points, offset = [], 0
                    for size in sizes:
                        points.append(digits[offset:offset+size]); offset += size
                    string = '-'.join(''.join(map(str,tree)).rstrip('0') for tree in points)
                    cases.append(dict(trees=trees, points=points))
                    expected_context.append((name,level,string,sum(digits)<=max(0,level-9)))
        run = subprocess.run(['node','-e',NODE_ORACLE,str(oracle)],input=json.dumps(cases),
                             capture_output=True,text=True,check=True,timeout=15)
        upstream = json.loads(run.stdout)
        self.assertEqual(len(upstream),720)
        for (name,level,string,within_budget), tree_valid in zip(expected_context,upstream,strict=True):
            result = talent_builds.validate_talent_build(name,level,string)
            with self.subTest(name=name,level=level,string=string):
                self.assertEqual(result['valid'],tree_valid and within_budget)


if __name__ == '__main__':
    unittest.main()
