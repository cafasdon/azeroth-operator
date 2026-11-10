"""Independent default profile contract checked against the actual offline MCP."""
from copy import deepcopy
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import patch
import urllib.error
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from deploy.probe_gateway import verify_default_reader_tools, verify_current_inspection_schema
from tools.gateway_profile import profile


class GatewayProbeTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_catalogue_matches_profile_and_refuses_drift(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder:
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = [tool.model_dump(by_alias=True, exclude_none=True)
                             for tool in (await session.list_tools()).tools]
                    self.assertEqual(len(tools), 12)
                    verify_default_reader_tools(tools)
                    verify_current_inspection_schema(tools)
                    rendered = profile('sha256:'+'a'*64, store='/srv/test-owner/snapshots')
                    self.assertEqual(set(rendered['servers'][0]['tools']), {tool['name'] for tool in tools})
                    # Reproduce the old nine-route contract without deriving expectations from the verifier.
                    old = [t for t in tools if t['name'] not in {
                        'resolve_character', 'search_forever_recipes', 'plan_forever_recipe_restock'}]
                    self.assertEqual(len(old), 9)
                    for bad in [None, {}, old, tools[:-1], tools+[tools[0]],
                                tools[:-1]+[tools[0]], tools[:-1]+[{'name': 'prepare_bag_operation'}],
                                tools[:-1]+[{'name': 'lookup_forever_catalogue'}],
                                tools[:-1]+[{'name': []}], tools[:-1]+[None]]:
                        with self.assertRaises(ValueError): verify_default_reader_tools(bad)
                    for key, values in [('readOnlyHint', [False, None, 1, 'true']),
                                        ('destructiveHint', [True, None, 0, 'false'])]:
                        for value in values:
                            bad = deepcopy(tools); bad[0]['annotations'][key] = value
                            with self.assertRaises(ValueError): verify_default_reader_tools(bad)
                    bad = deepcopy(tools); bad[0].pop('annotations')
                    with self.assertRaises(ValueError): verify_default_reader_tools(bad)
                    # Exercise the complete script with an offline HTTP boundary.
                    # Discovery and policy data come from the real stdio server;
                    # this fixture is not evidence of a deployed gateway's auth.
                    capability = await session.call_tool('capabilities', {})
                    snapshots = await session.call_tool('list_snapshots', {'family': 'retail'})
                    token = 'x'*64
                    catalogue = tools
                    requests = []
                    accept_bad_auth = False
                    fault = None
                    class Reply(io.BytesIO):
                        status = 200
                        headers = {'Mcp-Session-Id': 'offline-fixture'}
                    def urlopen(req, timeout):
                        self.assertEqual(timeout, 20)
                        self.assertEqual(req.full_url, 'http://127.0.0.1:8823/mcp')
                        requests.append(req)
                        auth = req.get_header('Authorization')
                        if auth != 'Bearer '+token and not accept_bad_auth and not (
                                fault == 'wrong_auth' and auth == 'Bearer incorrect-token'):
                            raise urllib.error.HTTPError(req.full_url, 401, 'Unauthorised', {}, None)
                        payload = json.loads(req.data)
                        method = payload['method']
                        if method == 'tools/list': body = {'result': {'tools': catalogue}}
                        elif method == 'tools/call':
                            name = payload['params']['name']
                            if name == 'capabilities': body = {'result': capability.model_dump(by_alias=True, exclude_none=True)}
                            elif name == 'list_snapshots': body = {'result': snapshots.model_dump(by_alias=True, exclude_none=True)}
                            else:
                                self.assertIn(name, {f'prepare_{op}_operation' for op in
                                    ['macro', 'binding', 'equipment', 'bag', 'talent', 'action', 'equipment_set']})
                                body = {'error': {'message': 'Tool not found'}}
                        else:
                            self.assertIn(method, ['initialize', 'notifications/initialized'])
                            body = {'result': {}}
                        if method == 'tools/call':
                            name = payload['params']['name']
                            if name == 'capabilities':
                                contents = json.loads(body['result']['content'][0]['text'])
                                if fault == 'policy_mode': contents['policy']['mode'] = 'online'
                                if fault == 'game_imports': contents['policy']['generated_game_imports'] = True
                                if fault == 'snapshot_scope': contents['policy']['snapshot_scope'] = 'all_users'
                                body['result']['content'][0]['text'] = json.dumps(contents)
                                if fault == 'capabilities_error': body['result']['isError'] = True
                            elif name == 'list_snapshots':
                                if fault == 'list_error': body['result']['isError'] = True
                                if fault == 'non_list_snapshots':
                                    body['result']['content'][0]['text'] = json.dumps({'snapshots': {}})
                            elif fault == 'prepare_success': body = {'result': {'content': []}}
                            elif fault == 'prepare_argument_error': body = {'error': {'message': 'Invalid arguments'}}
                        response = Reply(json.dumps(body).encode())
                        if (fault == 'discovery_status' and method == 'tools/list'
                                or fault == 'notify_status' and method == 'notifications/initialized'):
                            response.status = 503
                        return response
                    def env_file(path, *args, **kwargs):
                        self.assertEqual(path.as_posix(), '/etc/azeroth-operator/gateway.env')
                        return 'MCP_GATEWAY_AUTH_TOKEN='+token+'\n'
                    command = [str(root/'deploy/probe_gateway.py'), '--require-current-inspection']
                    source = Path(command[0]).read_text()
                    output = io.StringIO()
                    with patch('sys.argv', command), patch('pathlib.Path.read_text', env_file), \
                         patch('urllib.request.urlopen', urlopen), redirect_stdout(output):
                        runpy.run_path(command[0], run_name='__main__')
                    result = json.loads(output.getvalue())
                    self.assertEqual(result['tools'], 12)
                    self.assertEqual(result['prepare_routes_blocked'], 7)
                    self.assertEqual(len(requests), 14)
                    self.assertNotIn(token, output.getvalue())
                    # Security checks and the requests within them must survive -O/-OO.
                    for optimisation in [1, 2, 0]:
                        requests.clear(); output = io.StringIO()
                        with patch('sys.argv', command), patch('pathlib.Path.read_text', env_file), \
                             patch('urllib.request.urlopen', urlopen), redirect_stdout(output):
                            exec(compile(source, command[0], 'exec', optimize=optimisation),
                                 {'__name__': '__main__', '__file__': command[0]})
                        self.assertEqual(len(requests), 14, f'optimisation={optimisation}')
                        accept_bad_auth = True
                        with patch('sys.argv', command), patch('pathlib.Path.read_text', env_file), \
                             patch('urllib.request.urlopen', urlopen), self.assertRaises(ValueError):
                            exec(compile(source, command[0], 'exec', optimize=optimisation),
                                 {'__name__': '__main__', '__file__': command[0]})
                        accept_bad_auth = False
                        for fault in ['wrong_auth', 'policy_mode', 'game_imports', 'snapshot_scope',
                                      'capabilities_error', 'list_error', 'non_list_snapshots',
                                      'prepare_success', 'prepare_argument_error', 'discovery_status', 'notify_status']:
                            with patch('sys.argv', command), patch('pathlib.Path.read_text', env_file), \
                                 patch('urllib.request.urlopen', urlopen), redirect_stdout(io.StringIO()), \
                                 self.assertRaises(ValueError, msg=f'{fault}, optimisation={optimisation}'):
                                exec(compile(source, command[0], 'exec', optimize=optimisation),
                                     {'__name__': '__main__', '__file__': command[0]})
                        fault = None
                    catalogue = old
                    with patch('sys.argv', command), patch('pathlib.Path.read_text', env_file), \
                         patch('urllib.request.urlopen', urlopen), self.assertRaises(ValueError):
                        runpy.run_path(command[0], run_name='__main__')
