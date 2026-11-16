"""Real stdio owner optimiser routes, strict arguments and saved launcher settings."""
import asyncio
from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tests import test_owned_build_optimizer as fixtures
from tools import install_build_mcp as installer
from tools.simulation_cache import private_windows_path

ROOT=Path(__file__).resolve().parents[1]
TOOLS={'list_owned_build_characters','prepare_owned_build_search','evaluate_owned_build_window'}


def private_file(path):
    account=os.environ['USERDOMAIN']+'\\'+os.environ['USERNAME']
    subprocess.run(['icacls',str(path),'/inheritance:r','/grant:r',account+':(F)'],check=True,timeout=10,
                   stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    if not private_windows_path(path):raise ValueError('Private fixture config needed')


class OwnedBuildMcpTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.fixture=fixtures.OwnedBuildOptimizerTests();self.fixture.setUp();self.addCleanup(self.fixture.temp.cleanup)
        self.config=self.fixture.root/'source.json'
        self.config.write_bytes((json.dumps(self.fixture.config)+'\n').encode());private_file(self.config)
        self.entry=deepcopy(installer.desired_owned_entry(ROOT)['azeroth-owned-builds'])
        self.entry['args'][self.entry['args'].index('--source')+1]=str(self.config)

    async def test_all_routes_owner_binding_native_score_and_private_errors(self):
        entry=self.entry
        async with stdio_client(StdioServerParameters(command=entry['command'],args=entry['args'],cwd=entry['cwd'],
                                 env={**os.environ,**entry['env']})) as (read,write),ClientSession(read,write) as session:
            await session.initialize();listed=await session.list_tools();self.assertEqual({t.name for t in listed.tools},TOOLS)
            async def call(name,args):
                result=await session.call_tool(name,args);self.assertFalse(result.isError,result)
                self.assertLess(len(result.model_dump_json().encode()),256*1024)
                return json.loads(result.content[0].text)
            characters=await call('list_owned_build_characters',{})
            self.assertEqual(len(characters['characters']),1)
            self.assertNotIn('account_key',json.dumps(characters));self.assertNotIn(str(self.fixture.root),json.dumps(characters))
            blocked=await call('prepare_owned_build_search',{})
            self.assertEqual(blocked['status'],'plan_only');self.assertEqual(blocked['snapshot_id'],self.fixture.identifier)
            gear=self.fixture.model['raid']['parties'][0]['players'][0]['equipment']
            request=dict(model=self.fixture.model,gear_choices=[dict(slot=0,items=[gear['items'][0],{}])])
            prepared=await call('prepare_owned_build_search',request)
            self.assertEqual(prepared['status'],'ready');self.assertEqual(prepared['total_decimal'],'2')
            args=dict(expected_search_id=prepared['search_id'],limit=2)
            result=await call('evaluate_owned_build_window',args)
            self.assertTrue(all(r['mean_dps']>0 for r in result['results']))
            warm=await call('evaluate_owned_build_window',args)
            self.assertTrue(all(r['cache_status']=='hit' for r in warm['results']))
            self.assertEqual([r['mean_dps'] for r in result['results']],[r['mean_dps'] for r in warm['results']])
            self.assertFalse(result['active_character_verified']);self.assertFalse(result['all_owned_items_covered'])
            for name,args in [('list_owned_build_characters',dict(account_key='DO_NOT_ECHO')),
                ('prepare_owned_build_search',dict(source='DO_NOT_ECHO')),
                ('prepare_owned_build_search',dict(character=dict(name='DO_NOT_ECHO',realm='Absent'))),
                ('evaluate_owned_build_window',dict(expected_search_id='0'*64,engine='DO_NOT_ECHO'))]:
                response=await session.call_tool(name,args)
                self.assertTrue(response.isError);self.assertNotIn('DO_NOT_ECHO',response.content[0].text)
                self.assertNotIn(str(self.fixture.root),response.content[0].text)

    def test_owned_registration_is_additive_and_rejects_arbitrary_servers(self):
        old=b'[mcp_servers.azeroth]\nenabled=false\ndisabled_tools=["game_action"]\n'
        entry=installer.desired_owned_entry(ROOT)
        saved,names=installer.plan_configuration(old,entry)
        self.assertEqual(names,('azeroth-owned-builds',));self.assertTrue(saved.startswith(old))
        parsed=tomllib.loads(saved.decode());self.assertEqual(parsed['mcp_servers']['azeroth'],tomllib.loads(old.decode())['mcp_servers']['azeroth'])
        self.assertEqual(set(entry['azeroth-owned-builds']['enabled_tools']),TOOLS)
        self.assertEqual(installer.plan_configuration(saved,entry),(saved,()))
        with self.assertRaises(ValueError):installer.plan_configuration(old,{'unrequested-server':entry['azeroth-owned-builds']})

    def test_unsafe_source_permissions_stop_before_tool_discovery(self):
        subprocess.run(['icacls',str(self.config),'/grant','*S-1-1-0:(R)'],check=True,timeout=10,
                       stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        self.assertFalse(private_windows_path(self.config))
        result=subprocess.run([self.entry['command'],*self.entry['args']],cwd=ROOT,env={**os.environ,**self.entry['env']},
            input=b'',capture_output=True,timeout=10)
        self.assertEqual(result.returncode,1);self.assertEqual(result.stdout,b'')
        self.assertIn(b'Owned build tool unavailable',result.stderr)
        self.assertNotIn(str(self.fixture.root).encode(),result.stderr)

    @unittest.skipUnless(os.environ.get('AZEROTH_MCP_CONFIG'),'Explicit installed configuration needed')
    async def test_installed_saved_character_routes(self):
        raw=Path(os.environ['AZEROTH_MCP_CONFIG']).read_bytes();config=tomllib.loads(raw.decode())
        entry=config['mcp_servers']['azeroth-owned-builds']
        self.assertEqual(entry,installer.desired_owned_entry(ROOT)['azeroth-owned-builds'])
        async with stdio_client(StdioServerParameters(command=entry['command'],args=entry['args'],cwd=entry['cwd'],
                                 env={**os.environ,**entry['env']})) as (read,write),ClientSession(read,write) as session:
            await session.initialize();listing=await session.list_tools();self.assertEqual({t.name for t in listing.tools},TOOLS)
            reply=await session.call_tool('prepare_owned_build_search',{})
            self.assertFalse(reply.isError);default=json.loads(reply.content[0].text)
            reply=await session.call_tool('prepare_owned_build_search',dict(character=default['selection']['character']))
            self.assertFalse(reply.isError);explicit=json.loads(reply.content[0].text)
            self.assertEqual(default['snapshot_id'],explicit['snapshot_id'])
            self.assertEqual(default['selection']['selection_basis'],'saved_active_export')
            self.assertEqual(explicit['selection']['selection_basis'],'explicit_saved_character')
            self.assertFalse(default['game_execution']);self.assertFalse(default['active_character_verified'])
            self.assertEqual(default['status'],'plan_only');self.assertIn('unsupported_character_level',default['blocked_reasons'])
            if os.environ.get('AZEROTH_OWNED_MCP_EVIDENCE'):
                Path(os.environ['AZEROTH_OWNED_MCP_EVIDENCE']).write_bytes((json.dumps(dict(all_three_tools_discovered=True,
                    actual_saved_default_and_explicit_same_capture=True,unsupported_level_preserved=True,
                    live_game_test=False,private_payload_published=False))+'\n').encode())


if __name__=='__main__':unittest.main()
