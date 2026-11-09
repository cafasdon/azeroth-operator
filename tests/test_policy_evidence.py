"""Real MCP policy claims and direct-call denials remain conservative."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters, McpError
from mcp.client.stdio import stdio_client


class PolicyEvidenceTests(unittest.IsolatedAsyncioTestCase):
    async def test_offline_evidence_is_not_authorization(self):
        root = Path(__file__).resolve().parents[1]
        binary = root / 'target/debug' / ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder:
            async with stdio_client(StdioServerParameters(command=str(binary),
                    env={**os.environ, 'AZEROTH_DATA_DIR':folder})) as (reader, writer):
                async with ClientSession(reader, writer) as session:
                    await session.initialize()
                    reply = await session.call_tool('capabilities', {})
                    self.assertFalse(reply.isError)
                    capabilities = json.loads(reply.content[0].text)
                    policy = capabilities['policy']
                    self.assertEqual(policy['mode'], 'offline_analysis')
                    self.assertEqual(policy['default'], 'deny_unreviewed_tools')
                    self.assertEqual(policy['snapshot_scope'], 'configured_owner_store')
                    for flag in ['blizzard_authorization_verified',
                                 'complete_applicable_terms_review_verified',
                                 'absolute_compliance_guaranteed', 'generated_game_imports']:
                        self.assertIs(policy[flag], False)
                    self.assertIs(capabilities['game_execution'], False)
                    self.assertEqual(capabilities['live_client_qualified'], [])
                    self.assertEqual(policy['review_scope'], 'published_source_checks_and_project_controls_only')
                    evidence = policy['source_evidence']
                    self.assertEqual(len({row['url'] for row in evidence}), 5)
                    self.assertEqual(sum(row['retrieval'] == 'direct_access_refused' for row in evidence), 2)
                    self.assertEqual(sum(row['retrieval'] == 'official_opening_post_read' for row in evidence), 3)
                    self.assertTrue(all(row['checked_on'] == policy['reviewed_on'] for row in evidence))
                    listed = {tool.name for tool in (await session.list_tools()).tools}
                    self.assertEqual(set(capabilities['implemented']), listed)
                    for blocked in ['prepare_macro_operation', 'prepare_binding_operation',
                                    'prepare_equipment_operation', 'prepare_bag_operation',
                                    'prepare_talent_operation', 'prepare_action_operation',
                                    'prepare_equipment_set_operation', 'equip_item', 'cast_spell',
                                    'run_script', 'send_chat_message']:
                        self.assertNotIn(blocked, listed)
                        with self.assertRaisesRegex(McpError, 'tool not found'):
                            await session.call_tool(blocked, {})
            self.assertEqual(list(Path(folder).iterdir()), [])


if __name__ == '__main__':
    unittest.main()
