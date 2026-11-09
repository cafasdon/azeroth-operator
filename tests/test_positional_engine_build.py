"""Offline patch/extraction/process bounds, without compiling or running a simulator."""
from pathlib import Path
import subprocess
import os
import sys
import tempfile
from time import monotonic
import unittest
from unittest.mock import Mock, patch
import zipfile

from tools import build_forever_positional_engine as builder


class PositionalBuildTests(unittest.TestCase):
    @unittest.skipUnless(os.name=='nt','Windows process-tree ownership check')
    def test_actual_timeout_reaps_only_owned_python_tree(self):
        import ctypes
        from ctypes import wintypes
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder); log=root/'owned.log'
            script="import subprocess,sys,time,os,json; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print(json.dumps([os.getpid(),p.pid]),flush=True); time.sleep(60)"
            with self.assertRaises(subprocess.TimeoutExpired):
                builder.run_bounded([sys.executable,'-c',script],root,os.environ.copy(),log,monotonic()+1.5)
            pids=__import__('json').loads(log.read_text().splitlines()[0])
            kernel=ctypes.WinDLL('kernel32',use_last_error=True)
            kernel.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD]
            kernel.OpenProcess.restype=wintypes.HANDLE
            kernel.WaitForSingleObject.argtypes=[wintypes.HANDLE,wintypes.DWORD]
            kernel.CloseHandle.argtypes=[wintypes.HANDLE]
            for pid in pids:
                handle=kernel.OpenProcess(0x00100000,False,pid)
                if handle:
                    try: self.assertEqual(kernel.WaitForSingleObject(handle,5000),0)
                    finally: kernel.CloseHandle(handle)
                else: self.assertEqual(ctypes.get_last_error(),87)

    def test_ambient_go_settings_cannot_bypass_offline_or_target_contract(self):
        poisoned={key:'PRIVATE_BAD' for key in ['GOENV','GOWORK','GONOPROXY','GOPRIVATE','GOVCS',
            'GOOS','GOARCH','GOFLAGS','GOEXPERIMENT','GOCACHEPROG','GOTELEMETRY','GOAMD64','GODEBUG']}
        with patch.dict(os.environ,poisoned):
            env=builder.build_environment(Path('C:/Temp/owned'),Path('C:/Temp/modules'),None)
        expected={'GOENV':'off','GOWORK':'off','GONOPROXY':'none','GOPRIVATE':'','GOVCS':'*:off',
            'GOOS':'windows','GOARCH':'amd64','GOFLAGS':'','GOEXPERIMENT':'','GOCACHEPROG':'',
            'GOTELEMETRY':'off','GOAMD64':'v1','GODEBUG':'','GOPROXY':'off','GOSUMDB':'off'}
        self.assertTrue(all(env[key]==value for key,value in expected.items()))
        self.assertNotIn('PRIVATE_BAD',str(env))

    def test_archive_containment_duplicate_and_expansion_bounds(self):
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder)
            for index,names in enumerate([['go/../escape'],['/go/escape'],['go/C:/escape'],
                ['go\\escape'],['other/file'],['go/a','go/A']]):
                archive=base/f'{index}.zip'; target=base/f'target-{index}'; target.mkdir()
                with zipfile.ZipFile(archive,'w') as output:
                    for name in names:
                        info=zipfile.ZipInfo('placeholder'); info.filename=name
                        output.writestr(info,b'data')
                with self.assertRaises(ValueError,msg=names): builder.extract(archive,target,'go',100)
            archive=base/'good.zip'; target=base/'good'; target.mkdir()
            with zipfile.ZipFile(archive,'w') as output: output.writestr('go/src/file',b'data')
            with self.assertRaises(ValueError): builder.extract(archive,target,'go',3)
            self.assertEqual(list(target.iterdir()),[])
            builder.extract(archive,target,'go',100)
            self.assertEqual((target/'go/src/file').read_bytes(),b'data')
            self.assertFalse((base/'escape').exists())

    def test_patch_requires_expected_source_and_adds_explicit_contract(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            for directory in ['sim/core','proto','cmd/wowsimcli/cmd']: (root/directory).mkdir(parents=True)
            (root/'sim/core/database.go').write_text('func ProtoToEquipment(es *proto.EquipmentSpec) Equipment {\n\treturn NewEquipmentSet(ProtoToEquipmentSpec(es))\n}')
            (root/'proto/common.proto').write_text('message EquipmentSpec {\n\trepeated ItemSpec items = 1;\n}')
            builder.patch_source(root)
            self.assertIn('bool preserve_slots = 2;', (root/'proto/common.proto').read_text())
            self.assertIn('if !es.GetPreserveSlots()', (root/'sim/core/database.go').read_text())
            self.assertIn('azeroth-equipment-contract', (root/'cmd/wowsimcli/cmd/azeroth_equipment_contract.go').read_text())
            with self.assertRaises(ValueError): builder.patch_source(root)

    def test_timeout_and_interrupt_reap_owned_process_tree(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            for error in [subprocess.TimeoutExpired('owned',1),KeyboardInterrupt()]:
                child=Mock(pid=98765)
                child.wait.side_effect=[error,0]
                child.poll.return_value=None
                with patch.object(builder.subprocess,'Popen',return_value=child), \
                    patch.object(builder.subprocess,'run') as kill, \
                    patch.object(builder.subprocess,'CREATE_NEW_PROCESS_GROUP',512,create=True), \
                    patch.object(builder.subprocess,'CREATE_NO_WINDOW',134217728,create=True):
                    with self.assertRaises(type(error)):
                        builder.run_bounded(['owned'],root,{},root/'log',monotonic()+10)
                self.assertEqual(kill.call_args.args[0],['taskkill','/PID','98765','/T','/F'])
                self.assertEqual(child.wait.call_count,2)


if __name__=='__main__': unittest.main()
