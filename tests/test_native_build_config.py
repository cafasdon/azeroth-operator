"""Opt-in native Codex discovery of the exact saved offline MCP entries."""
import asyncio
import json
import os
from pathlib import Path
import subprocess
import tomllib
import unittest
from tools.install_build_mcp import plan_configuration,desired_entries,private_directory

ROOT=Path(__file__).resolve().parents[1]


class NativeBuildConfigTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('AZEROTH_MCP_CONFIG') and os.environ.get('AZEROTH_NATIVE_CODEX_HOME'),
                         'Explicit saved configuration and private native check home needed')
    def test_native_client_discovers_all_six_tools(self):
        source=tomllib.loads(Path(os.environ['AZEROTH_MCP_CONFIG']).read_bytes().decode())
        entries=desired_entries(ROOT)
        self.assertTrue(all(source['mcp_servers'][name]==value for name,value in entries.items()))
        home=Path(os.environ['AZEROTH_NATIVE_CODEX_HOME']);private_directory(home)
        base=b'[apps._default]\nenabled=false\n[analytics]\nenabled=false\n'
        config,_=plan_configuration(base,{name:source['mcp_servers'][name] for name in entries})
        (home/'config.toml').write_bytes(config)
        prefix=json.loads(os.environ.get('AZEROTH_NATIVE_CODEX_COMMAND','["node","C:/Users/cafas/AppData/Roaming/npm/node_modules/@openai/codex/bin/codex.js"]'))
        evidence=[]
        async def caller():
            process=await asyncio.create_subprocess_exec(*prefix,'app-server','--strict-config','--listen','stdio://',
                cwd=ROOT,env={**os.environ,'CODEX_HOME':str(home)},stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.DEVNULL,limit=2*1024*1024)
            async def request(identity,method,params):
                process.stdin.write((json.dumps(dict(id=identity,method=method,params=params))+'\n').encode());await process.stdin.drain()
                for _ in range(200):
                    line=await asyncio.wait_for(process.stdout.readline(),40)
                    self.assertTrue(line,'Native app server closed before response')
                    message=json.loads(line)
                    if message.get('id')==identity:
                        self.assertNotIn('error',message,'Native RPC rejected the documented request')
                        return message['result']
                self.fail('Native response budget exceeded')
            try:
                await request(0,'initialize',dict(clientInfo=dict(name='azeroth-offline-config-check',version='1'),
                                                  capabilities=dict(experimentalApi=True)))
                process.stdin.write(b'{"method":"initialized"}\n');await process.stdin.drain()
                for index,name in enumerate(entries,1):
                    result=await request(index,'mcpServerStatus/list',dict(serverName=name,limit=10,detail='full'))
                    self.assertEqual(len(result['data']),1)
                    server=result['data'][0]
                    self.assertEqual(server['name'],name);self.assertIsNone(server.get('toolsError'))
                    self.assertEqual({tool['name'] for tool in server['tools'].values()},set(entries[name]['enabled_tools']))
                    evidence.append(dict(server=name,tools=sorted(t['name'] for t in server['tools'].values()),discovered=True))
            finally:
                # The owned native process tree must not outlive the check.
                if process.returncode is None:
                    subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'],stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=15,check=False)
                    await asyncio.wait_for(process.wait(),15)
        asyncio.run(asyncio.wait_for(caller(),150))
        if os.environ.get('AZEROTH_NATIVE_MCP_EVIDENCE'):
            Path(os.environ['AZEROTH_NATIVE_MCP_EVIDENCE']).write_bytes((json.dumps(dict(servers=evidence,
                strict_config_accepted=True,model_request_sent=False,game_execution=False),indent=2)+'\n').encode())


if __name__=='__main__':
    unittest.main()
