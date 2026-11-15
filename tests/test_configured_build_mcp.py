"""Opt-in offline checks using the actual saved MCP launch commands."""
import asyncio
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tomllib
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools.install_build_mcp import desired_entries
from tools import joint_build_optimizer as joint

ROOT=Path(__file__).resolve().parents[1]
PUBLIC_SHA='99a6f97159121160cea7a1a119c34336432f11aea540cef736790bc6b24552d4'


class ConfiguredBuildMcpTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('AZEROTH_MCP_CONFIG'),'Explicit installed configuration needed')
    def test_saved_launches_and_all_six_tools(self):
        config_path=Path(os.environ['AZEROTH_MCP_CONFIG'])
        self.assertTrue(config_path.is_absolute())
        raw=config_path.read_bytes(); config=tomllib.loads(raw.decode('utf-8'))
        expected=desired_entries(ROOT)
        for name,entry in expected.items():
            self.assertEqual(config['mcp_servers'][name],entry)
        case_path=ROOT/'clients/forever/fixtures/public-hunter-256.json'
        self.assertEqual(hashlib.sha256(case_path.read_bytes()).hexdigest(),PUBLIC_SHA)
        manifest=json.loads(case_path.read_bytes()); profile=manifest['base_request']['raid']['parties'][0]['players'][0]
        self.assertEqual(manifest['case']['client']['variant'],'public-model-fixture')
        context=dict(class_id=3,specialization='hunter',race=profile['race'],level=60,
                     professions=[profile['profession1'],profile['profession2']])
        equipment=deepcopy(profile['equipment'])
        inventory=[dict(item=i,count=1) for i in equipment['items'] if i]
        identity=joint.preflight(manifest,window_mode=True,limit=1)[0]['search_id']
        evidence=[]

        async def exercise(name):
            entry=config['mcp_servers'][name]
            params=StdioServerParameters(command=entry['command'],args=entry['args'],cwd=entry['cwd'],
                                          env={**os.environ,**entry['env']})
            async with stdio_client(params) as (read,write),ClientSession(read,write) as session:
                await session.initialize(); listing=await session.list_tools()
                self.assertEqual({t.name for t in listing.tools},set(entry['enabled_tools']))
                async def call(tool,arguments):
                    reply=await session.call_tool(tool,arguments)
                    self.assertFalse(reply.isError,reply)
                    self.assertLess(len(reply.model_dump_json().encode()),256*1024)
                    return json.loads(reply.content[0].text)
                if name=='azeroth-talents':
                    validated=await call('validate_talent_build',dict(class_name='hunter',level=60,talents_string=profile['talentsString']))
                    self.assertTrue(validated['valid']); self.assertFalse(validated['native_legality_verified'])
                    arguments=dict(class_name='hunter',level=60,base_talents=profile['talentsString'],choices=[
                        dict(definition_id=134422,ranks=[4,5]),dict(definition_id=134438,ranks=[3,4])])
                    generated=await call('generate_talent_builds',arguments)
                    self.assertEqual(generated['accepted_count'],2)
                    bad=await session.call_tool('generate_talent_builds',arguments|dict(path='DO_NOT_ECHO'))
                elif name=='azeroth-gear':
                    validated=await call('validate_gear_build',dict(context=context,equipment=equipment,inventory=inventory))
                    self.assertTrue(validated['valid']); self.assertFalse(validated['ownership_verified'])
                    arguments=dict(context=context,base_equipment=equipment,inventory=inventory,
                                   choices=[dict(slot=13,items=[equipment['items'][13],{}])])
                    generated=await call('generate_gear_builds',arguments)
                    self.assertEqual(generated['accepted_count'],2); self.assertEqual(generated['unresolved_count'],0)
                    bad=await session.call_tool('generate_gear_builds',arguments|dict(probe_path='DO_NOT_ECHO'))
                else:
                    self.assertTrue(all('public-model-fixture' in t.description and 'Current character is unbound' in t.description for t in listing.tools))
                    arguments=dict(expected_search_id=identity,offset=os.environ.get('AZEROTH_MCP_WINDOW_OFFSET','63'),limit=2)
                    page=await call('evaluate_joint_build_window',arguments)
                    self.assertEqual(page['total_decimal'],'256'); self.assertEqual(page['evaluated_coordinates'],2)
                    self.assertTrue(all(r['mean_dps']>0 for r in page['results']))
                    first_statuses=[r['cache_status'] for r in page['results']]
                    if os.environ.get('AZEROTH_MCP_EXPECT_COLD')=='1':
                        self.assertEqual(first_statuses,['miss','miss'])
                    warm=await call('evaluate_joint_build_window',arguments)
                    self.assertTrue(all(r['cache_status']=='hit' for r in warm['results']))
                    self.assertEqual([r['mean_dps'] for r in page['results']],[r['mean_dps'] for r in warm['results']])
                    merged=await call('merge_joint_build_windows',dict(expected_search_id=identity,windows=[page],top_k=2))
                    self.assertEqual(merged['reported_evaluated_decimal'],'2'); self.assertEqual(merged['next_offset'],'0')
                    self.assertFalse(merged['simulation_performed']); self.assertFalse(merged['global_optimum_verified'])
                    bad=await session.call_tool('evaluate_joint_build_window',arguments|dict(engine='DO_NOT_ECHO'))
                self.assertTrue(bad.isError); self.assertNotIn('DO_NOT_ECHO',bad.content[0].text)
                result=dict(server=name,tools=sorted(entry['enabled_tools']),all_routes_passed=True,override_rejected=True)
                if name=='azeroth-joint-public256':
                    result.update(offset=arguments['offset'],first_cache_statuses=first_statuses,repeat_cache_statuses=['hit','hit'])
                evidence.append(result)

        async def run():
            for name in expected:
                await exercise(name)
        asyncio.run(asyncio.wait_for(run(),180))
        if os.environ.get('AZEROTH_MCP_EVIDENCE'):
            output=Path(os.environ['AZEROTH_MCP_EVIDENCE']); self.assertTrue(output.is_absolute())
            output.write_bytes((json.dumps(dict(servers=evidence,public_case_sha256=PUBLIC_SHA,public_search_id=identity,
                config_sha256=hashlib.sha256(raw).hexdigest(),current_character_bound=False,game_execution=False),indent=2)+'\n').encode())


if __name__=='__main__':
    unittest.main()
