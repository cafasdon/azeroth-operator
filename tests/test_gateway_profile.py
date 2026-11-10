import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from tools.gateway_profile import profile, snapshot_store, TOOLS


class GatewayProfileTests(unittest.TestCase):
    def test_dedicated_profile_has_only_read_tools_and_store(self):
        image = "sha256:" + "a" * 64
        store = '/srv/fixture/snapshots'
        result = profile(image, store=store)
        self.assertEqual(result["id"], "azeroth-operator")
        self.assertEqual(len(result["servers"]), 1)
        entry = result["servers"][0]
        self.assertEqual(entry["tools"], TOOLS)
        self.assertNotIn("import-stdin", entry["tools"])
        server = entry["snapshot"]["server"]
        self.assertEqual(server["image"], image)
        self.assertEqual(server["volumes"], [store + ":/data:ro"])
        self.assertEqual(server['env'],[{'name':'AZEROTH_DATA_DIR','value':'/data'}])

    def test_explicit_owner_paths_never_fall_back_to_another_user(self):
        image='sha256:'+'a'*64
        with self.assertRaises(TypeError):profile(image)
        first=profile(image,store='/srv/alice/snapshots')
        second=profile(image,store='/srv/bob/snapshots')
        self.assertNotIn('alice',json.dumps(second));self.assertNotIn('bob',json.dumps(first))
        self.assertNotIn('rodrigo',json.dumps(first)+json.dumps(second))
        first['servers'][0]['tools'].append('MUTATION')
        first['servers'][0]['snapshot']['server']['volumes'].append('OTHER')
        fresh=profile(image,store='/srv/alice/snapshots')
        self.assertEqual(fresh['servers'][0]['tools'],TOOLS)
        self.assertEqual(fresh['servers'][0]['snapshot']['server']['volumes'],['/srv/alice/snapshots:/data:ro'])

    def test_ambiguous_or_nonliteral_store_paths_refuse(self):
        for store in [None,False,1,'','/','/srv','relative/snapshots','//srv/alice/snapshots',
                '/srv/alice//snapshots','/srv/alice/./snapshots','/srv/alice/../bob/snapshots',
                '/srv/alice/snapshots/','C:/PRIVATE/snapshots','/srv/PRIVATE:/data:rw',
                '/srv/PRIVATE\\snapshots','/srv/PRIVATE\n/snapshots','/srv/PRIVATE\0/snapshots',
                '/srv/$USER/snapshots','/srv/${USER}/snapshots','/srv/~/snapshots',
                '/srv/*/snapshots','/srv/a b/snapshots','/srv/é/snapshots','/srv/'+('a'*4096)]:
            with self.subTest(store=store),self.assertRaises(ValueError):snapshot_store(store)

    def test_real_module_requires_store_and_emits_only_the_chosen_read_mount(self):
        root=Path(__file__).resolve().parents[1]
        command=[sys.executable,'-B','-m','tools.gateway_profile','--image','sha256:'+'a'*64]
        env={**os.environ,'AZEROTH_DATA_DIR':'/srv/PRIVATE_OTHER/snapshots','USER':'PRIVATE_OTHER'}
        missing=subprocess.run(command,cwd=root,env=env,capture_output=True,text=True,timeout=15)
        self.assertEqual(missing.returncode,2);self.assertEqual(missing.stdout,'')
        self.assertNotIn('PRIVATE_OTHER',missing.stderr)
        valid=subprocess.run([*command,'--store','/srv/fixture/snapshots'],cwd=root,env=env,
            capture_output=True,text=True,timeout=15)
        self.assertEqual(valid.returncode,0,valid.stderr)
        result=json.loads(valid.stdout)
        self.assertEqual(result['servers'][0]['snapshot']['server']['volumes'],['/srv/fixture/snapshots:/data:ro'])
        self.assertNotIn('PRIVATE_OTHER',valid.stdout+valid.stderr)
        refused=subprocess.run([*command,'--store','/srv/PRIVATE_BAD:/data:rw'],cwd=root,env=env,
            capture_output=True,text=True,timeout=15)
        self.assertEqual(refused.returncode,2);self.assertEqual(refused.stdout,'')
        self.assertNotIn('PRIVATE_BAD',refused.stderr);self.assertNotIn('Traceback',refused.stderr)

    def test_tags_and_untrusted_image_syntax_rejected(self):
        for image in ["latest", "azeroth:latest", "sha256:" + "A" * 64,
                      "sha256:" + "a" * 63, "sha256:" + "a" * 64 + "\n"]:
            with self.assertRaises(ValueError):
                profile(image, store='/srv/fixture/snapshots')
