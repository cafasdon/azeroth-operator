"""Isolated config writes; the real child contract reads CODEX_HOME/config.toml."""
from contextlib import contextmanager, redirect_stderr, redirect_stdout
from copy import deepcopy
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

from tools import install_build_mcp as installer

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = b'''# Preserve this prefix and the fake credential.\nmodel = "example"\napi_key = "synthetic-secret-do-not-print"\n[apps.example]\nenabled = false\n[mcp_servers.azeroth]\nenabled = false\ndisabled_tools = ["send_message", "game_action"]\n[mcp_servers.other.env]\nTOKEN = "synthetic-secret-do-not-print"\n'''


class InstallerTests(unittest.TestCase):
    def setUp(self):
        base = ROOT / 'local-data' / 'installer-tests'
        base.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=base)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.config = self.root / 'config.toml'
        self.config.write_bytes(ORIGINAL)
        self.stage = self.root / 'private-stage'
        self.stage.mkdir(mode=0o700)
        if os.name == 'nt':
            account = os.environ['USERDOMAIN'] + '\\' + os.environ['USERNAME']
            subprocess.run(['icacls', str(self.stage), '/inheritance:r', '/grant:r', account + ':(OI)(CI)(F)'],
                check=True, timeout=10, stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.entries = installer.desired_entries(ROOT)
        # This real subprocess has exactly the production argv/env/stdin contract.
        self.parser = self.root / 'parser.py'
        self.parser.write_bytes(b'''import os, pathlib, sys, tomllib\nassert sys.argv[1:] == ['features', 'list']\nassert sys.stdin.read() == ''\np = pathlib.Path(os.environ['CODEX_HOME']) / 'config.toml'\ntomllib.loads(p.read_bytes().decode('utf-8'))\n''')
        self.command = [sys.executable, str(self.parser)]

    def apply(self):
        return installer.apply_configuration(self.config, self.entries, self.stage, self.command)

    def test_entries_match_real_tool_names_and_paths(self):
        from tools import talent_builds, gear_builds, joint_build_windows
        for name, module, timeout in [('azeroth-talents', talent_builds, 30),
                ('azeroth-gear', gear_builds, 60), ('azeroth-joint-public256', joint_build_windows, 150)]:
            entry = self.entries[name]
            self.assertEqual(set(entry['enabled_tools']), set(module.schemas()))
            self.assertIs(entry['enabled'], True)
            self.assertEqual(entry['startup_timeout_sec'], 30)
            self.assertEqual(entry['tool_timeout_sec'], timeout)
            self.assertTrue(Path(entry['command']).is_absolute())
            self.assertEqual(entry['args'][:4], ['-B', '-m', module.__name__, 'serve'])
            self.assertEqual(entry['env'], dict(PYTHONPATH=str(ROOT), GOMAXPROCS='2'))
        joint = self.entries['azeroth-joint-public256']['args']
        self.assertEqual(joint[joint.index('--cache-root') + 1],
                         str(ROOT / 'local-data' / 'forever-build-runtime' / 'numeric-cache'))

    def test_plan_preserves_prefix_and_exact_unrelated_semantics(self):
        output = io.StringIO()
        with redirect_stdout(output), redirect_stderr(output):
            candidate, names = installer.plan_configuration(ORIGINAL, self.entries)
        self.assertEqual(output.getvalue(), '')
        self.assertTrue(candidate.startswith(ORIGINAL))
        self.assertEqual(set(names), set(self.entries))
        expected = tomllib.loads(ORIGINAL.decode())
        expected['mcp_servers'].update(self.entries)
        self.assertEqual(tomllib.loads(candidate.decode()), expected)
        self.assertEqual(installer.plan_configuration(candidate, self.entries), (candidate, ()))

    def test_crlf_becomes_lf_without_double_carriage_returns(self):
        candidate, _ = installer.plan_configuration(ORIGINAL.replace(b'\n', b'\r\n'), self.entries)
        self.assertNotIn(b'\r', candidate)
        self.assertTrue(candidate.startswith(ORIGINAL))

    def test_malformed_crcrlf_and_non_utf8_fail_with_generic_error(self):
        for value in [ORIGINAL.replace(b'\n', b'\r\r\n'), b'key = "\xff"\n', b'model = [secret']:
            with self.subTest(value=value[:20]):
                with self.assertRaises(ValueError) as caught:
                    installer.plan_configuration(value, self.entries)
                self.assertEqual(str(caught.exception), installer.ERROR)

    def test_conflict_and_bool_integer_confusion_fail_without_secret(self):
        candidate, _ = installer.plan_configuration(ORIGINAL, self.entries)
        for changed in [candidate.replace(b'"enabled" = true', b'"enabled" = false', 1),
                        candidate.replace(b'"enabled" = true', b'"enabled" = 1', 1)]:
            with self.assertRaisesRegex(ValueError, '^' + installer.ERROR + '$'):
                installer.plan_configuration(changed, self.entries)

    def test_real_parser_apply_backup_and_idempotency(self):
        result = self.apply()
        self.assertEqual(result['backup_path'].read_bytes(), ORIGINAL)
        self.assertEqual(self.config.read_bytes(), installer.plan_configuration(ORIGINAL, self.entries)[0])
        self.assertEqual(set(result['added_names']), set(self.entries))
        second = self.apply()
        self.assertEqual(second['added_names'], ())
        self.assertIsNone(second['backup_path'])
        self.assertEqual(len(list(self.stage.iterdir())), 1)

    def test_native_parser_failure_leaves_original_and_private_backup(self):
        self.parser.write_bytes(b'import sys\nsys.exit(2)\n')
        with self.assertRaisesRegex(ValueError, '^' + installer.ERROR + '$'):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)
        self.assertEqual(list(self.stage.iterdir()), [])

    def failure_at(self, number, change=None):
        real = installer.native_parse
        calls = []
        def native_parse(home, codex_command):
            calls.append(home)
            real(home, codex_command)
            if len(calls) == number:
                if change is not None:
                    self.config.write_bytes(change)
                raise ValueError(installer.ERROR)
        return patch.object(installer, 'native_parse', side_effect=native_parse)

    def test_candidate_parser_failure_keeps_original(self):
        with self.failure_at(2), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)
        self.assertEqual(next(self.stage.glob('*/before.toml')).read_bytes(), ORIGINAL)

    def test_post_write_parser_failure_restores_exact_original(self):
        with self.failure_at(3), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)
        self.assertEqual(next(self.stage.glob('*/before.toml')).read_bytes(), ORIGINAL)

    def test_saved_candidate_write_corruption_is_rejected_before_live_write(self):
        real = Path.write_bytes
        def write_bytes(path, data):
            if path.name == 'config.toml' and path.parent.name == 'candidate':
                data = data.replace(b'\n', b'\r\r\n')
            return real(path, data)
        with patch.object(Path, 'write_bytes', new=write_bytes), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_stage_change_after_native_parse_is_rejected(self):
        real = installer.native_parse
        def native_parse(home, codex_command):
            real(home, codex_command)
            if home != self.config.parent:
                (home / 'config.toml').write_bytes(b'model = "changed-stage"\n')
        with patch.object(installer, 'native_parse', side_effect=native_parse), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_readback_failure_restores_original_when_disk_still_has_candidate(self):
        real_read, real_write = installer.config_bytes, installer._write_bytes
        state = dict(installed=False, mismatch=False)
        def write_bytes(stream, data):
            real_write(stream, data)
            state['installed'] = True
        def config_bytes(path):
            value = real_read(path)
            if path == self.config and state['installed'] and not state['mismatch']:
                state['mismatch'] = True
                return value + b'# readback mismatch\n'
            return value
        with patch.object(installer, '_write_bytes', side_effect=write_bytes), \
                patch.object(installer, 'config_bytes', side_effect=config_bytes), self.assertRaises(ValueError):
            self.apply()
        self.assertTrue(state['mismatch'])
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_post_write_concurrent_change_is_never_overwritten(self):
        concurrent = b'model = "concurrent-writer"\n'
        with self.failure_at(3, concurrent), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), concurrent)

    def test_pre_write_concurrent_drift_is_rejected(self):
        concurrent = b'model = "concurrent-writer"\n'
        real = installer.native_parse
        def native_parse(home, codex_command):
            real(home, codex_command)
            if home != self.config.parent:
                self.config.write_bytes(concurrent)
        with patch.object(installer, 'native_parse', side_effect=native_parse), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), concurrent)

    def test_real_second_process_cannot_write_or_rename_under_exclusion(self):
        other = self.root / 'other.toml'
        other.write_bytes(b'model = "other"\n')
        script = self.root / 'writer.py'
        script.write_bytes(b'''import os, pathlib, sys
try:
    if sys.argv[1] == 'write':
        pathlib.Path(sys.argv[2]).write_bytes(b'other-writer')
    else:
        os.replace(sys.argv[3], sys.argv[2])
except OSError as error:
    assert error.winerror == 32 or error.errno == 13, error
    print('blocked:' + sys.argv[1])
else:
    sys.exit(3)
''')
        with installer._exclusive_config(self.config) as stream:
            for operation in ['write', 'rename']:
                completed = subprocess.run([sys.executable, '-B', str(script), operation,
                    str(self.config), str(other)], capture_output=True, timeout=10)
                self.assertEqual(completed.returncode, 0, completed.stderr.decode())
                self.assertEqual(completed.stdout.strip(), ('blocked:' + operation).encode())
            stream.seek(0)
            self.assertEqual(stream.read(), ORIGINAL)
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_real_writer_between_prior_read_and_lock_is_preserved(self):
        concurrent = b'model = "writer-before-lock"\n'
        real = installer._exclusive_config
        @contextmanager
        def exclusive_config(path):
            completed = subprocess.run([sys.executable, '-B', '-c',
                'from pathlib import Path; import sys; Path(sys.argv[1]).write_bytes(bytes.fromhex(sys.argv[2]))',
                str(path), concurrent.hex()], capture_output=True, timeout=10)
            self.assertEqual(completed.returncode, 0, completed.stderr.decode())
            with real(path) as stream:
                yield stream
        with patch.object(installer, '_exclusive_config', side_effect=exclusive_config), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), concurrent)

    def test_partial_write_failure_restores_through_the_same_handle(self):
        real = installer._write_bytes
        handles = []
        def write_bytes(stream, data):
            handles.append(stream.fileno())
            if len(handles) == 1:
                stream.seek(0)
                stream.write(b'partial')
                stream.truncate()
                raise OSError('synthetic disk failure')
            real(stream, data)
        with patch.object(installer, '_write_bytes', side_effect=write_bytes), self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(len(handles), 2)
        self.assertEqual(handles[0], handles[1])
        self.assertEqual(self.config.read_bytes(), ORIGINAL)
        self.assertEqual(next(self.stage.glob('*/before.toml')).read_bytes(), ORIGINAL)

    def test_exclusive_handle_busy_fails_without_mutation(self):
        with installer._exclusive_config(self.config):
            with self.assertRaises(ValueError):
                installer._guarded_update(self.config, ORIGINAL, b'model = "change"\n')
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_private_stage_rejection_before_any_backup(self):
        if os.name == 'nt':
            subprocess.run(['icacls', str(self.stage), '/grant', '*S-1-1-0:(OI)(CI)(R)'],
                timeout=10, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        else:
            self.stage.chmod(0o755)
        with self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)
        self.assertEqual(list(self.stage.iterdir()), [])

    def test_hardlinked_config_rejected(self):
        os.link(self.config, self.root / 'alias.toml')
        with self.assertRaises(ValueError):
            self.apply()
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_cli_only_prints_counts_names_and_no_credentials(self):
        completed = subprocess.run([sys.executable, '-B', '-m', 'tools.install_build_mcp',
            '--config', str(self.config)], cwd=ROOT, capture_output=True, timeout=30)
        self.assertEqual(completed.returncode, 0, completed.stderr.decode())
        self.assertIn(b'Planned entries: 3', completed.stdout)
        self.assertNotIn(b'synthetic-secret', completed.stdout + completed.stderr)
        self.assertEqual(self.config.read_bytes(), ORIGINAL)

    def test_cli_apply_suppresses_child_output(self):
        self.parser.write_bytes(self.parser.read_bytes() + b'print("synthetic-secret-child")\n')
        completed = subprocess.run([sys.executable, '-B', '-m', 'tools.install_build_mcp',
            '--config', str(self.config), '--apply', '--stage-dir', str(self.stage),
            '--codex-launcher', *self.command], cwd=ROOT, capture_output=True, timeout=30)
        self.assertEqual(completed.returncode, 0, completed.stderr.decode())
        self.assertIn(b'Added entries: 3', completed.stdout)
        self.assertNotIn(b'synthetic-secret', completed.stdout + completed.stderr)


if __name__ == '__main__':
    unittest.main()
