"""Add three offline MCP entries with private backups and native Codex parsing."""
import argparse
from contextlib import contextmanager
from copy import deepcopy
import json
import os
from pathlib import Path
import stat
import subprocess
import tomllib
import uuid

from tools.positional_equipment import ENGINE_SHA256
from tools.simulation_cache import private_windows_path
from tools.sync_exports import ordinary

ERROR = 'Offline MCP configuration update failed'


def desired_entries(project_root):
    root = ordinary(Path(project_root))
    if not root.is_absolute():
        raise ValueError(ERROR)
    root = root.resolve()
    runtime = root / 'local-data' / 'forever-build-runtime' / ENGINE_SHA256
    def entry(module, names, timeout, extra=()):
        return dict(command=str(root / '.venv' / 'Scripts' / 'python.exe'),
                    args=['-B', '-m', 'tools.' + module, 'serve', *map(str, extra)],
                    cwd=str(root), env=dict(PYTHONPATH=str(root), GOMAXPROCS='2'),
                    enabled=True, enabled_tools=list(names),
                    startup_timeout_sec=30, tool_timeout_sec=timeout)
    return {
        'azeroth-talents': entry('talent_builds', ('validate_talent_build', 'generate_talent_builds'), 30),
        'azeroth-gear': entry('gear_builds', ('validate_gear_build', 'generate_gear_builds'), 60,
            ('--catalogue', root / 'local-data' / 'forever' / 'catalogue.json',
             '--probe', runtime / 'equipment-probe.exe')),
        'azeroth-joint-public256': entry('joint_build_windows',
            ('evaluate_joint_build_window', 'merge_joint_build_windows'), 150,
            ('--input', root / 'clients' / 'forever' / 'fixtures' / 'public-hunter-256.json',
             '--engine', runtime / 'wowsimcli-positional.exe',
             '--cache-root', runtime.parent / 'numeric-cache'))}


def parsed(data):
    return tomllib.loads(data.decode('utf-8'))


def same_values(left, right):
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(same_values(v, right[k]) for k, v in left.items())
    if isinstance(left, list):
        return len(left) == len(right) and all(same_values(a, b) for a, b in zip(left, right))
    return left == right


def plan_configuration(existing_bytes, entries):
    """Preserve unrelated values; reject an existing target with different values."""
    try:
        before = parsed(existing_bytes)
        if not entries or set(entries) - set(desired_entries(Path(__file__).resolve().parents[1])):
            raise ValueError(ERROR)
        servers = before.get('mcp_servers', {})
        if not isinstance(servers, dict):
            raise ValueError(ERROR)
        for name, entry in entries.items():
            if name in servers and not same_values(servers[name], entry):
                raise ValueError(ERROR)
        added = tuple(name for name in entries if name not in servers)
        if not added:
            return existing_bytes, added
        candidate = existing_bytes.decode('utf-8').replace('\r\n', '\n')
        def value(item):
            if isinstance(item, dict):
                return '{' + ', '.join(json.dumps(k) + ' = ' + value(v) for k, v in item.items()) + '}'
            return json.dumps(item, ensure_ascii=False, allow_nan=False)
        for name in added:
            candidate += '\n[mcp_servers.' + json.dumps(name) + ']\n'
            candidate += ''.join(json.dumps(k) + ' = ' + value(v) + '\n' for k, v in entries[name].items())
        candidate = candidate.encode('utf-8')
        expected = deepcopy(before)
        expected.setdefault('mcp_servers', {}).update({name: entries[name] for name in added})
        if not same_values(parsed(candidate), expected):
            raise ValueError(ERROR)
        return candidate, added
    except (ValueError, TypeError, UnicodeError, KeyError, RecursionError):
        raise ValueError(ERROR) from None


def private_directory(path):
    ordinary(path)
    info = path.stat()
    if (not stat.S_ISDIR(info.st_mode) or
            not (private_windows_path(path) if os.name == 'nt' else
                 info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700)):
        raise ValueError(ERROR)


def config_bytes(path):
    ordinary(path)
    info = path.stat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise ValueError(ERROR)
    return path.read_bytes()


@contextmanager
def _exclusive_config(path):
    """Open a Windows config with no read, write or rename sharing; fail fast."""
    if os.name != 'nt':
        raise ValueError(ERROR)
    import ctypes
    from ctypes import wintypes
    import msvcrt
    ordinary(path)
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
        ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    handle = kernel.CreateFileW(str(path), 0xC0000000, 0, None, 3, 0x00200080, None)
    if handle == ctypes.c_void_p(-1).value:
        raise ValueError(ERROR)
    try:
        fd = msvcrt.open_osfhandle(handle, os.O_RDWR | os.O_BINARY)
        handle = None  # The CRT descriptor now owns the Windows handle.
        try:
            stream = os.fdopen(fd, 'r+b', buffering=0)
        except BaseException:
            os.close(fd)
            raise
        with stream:
            info = os.fstat(stream.fileno())
            if (not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or
                    getattr(info, 'st_file_attributes', 0) & 0x400):
                raise ValueError(ERROR)
            yield stream
    finally:
        if handle is not None:
            kernel.CloseHandle(handle)


def _write_bytes(stream, data):
    stream.seek(0)
    remaining = memoryview(data)
    while remaining:
        count = stream.write(remaining)
        if not count:
            raise ValueError(ERROR)
        remaining = remaining[count:]
    stream.truncate()
    stream.flush()
    os.fsync(stream.fileno())
    stream.seek(0)
    if stream.read() != data:
        raise ValueError(ERROR)


def _guarded_update(path, expected, replacement):
    """Compare and update under one exclusive handle; repair an interrupted write."""
    with _exclusive_config(path) as stream:
        if stream.read() != expected:
            raise ValueError(ERROR)
        try:
            _write_bytes(stream, replacement)
        except (OSError, ValueError):
            # Our handle still excludes all other writers, so this is our partial write.
            try:
                _write_bytes(stream, expected)
            except (OSError, ValueError):
                pass  # The private before.toml backup remains available.
            raise ValueError(ERROR) from None


def native_parse(home, codex_command):
    """Check the saved file with the caller's local Codex executable, without output."""
    if not codex_command:
        raise ValueError(ERROR)
    environment = dict(os.environ, CODEX_HOME=str(home))
    process = subprocess.Popen([*map(str, codex_command), 'features', 'list'],
        env=environment, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, start_new_session=os.name != 'nt')
    try:
        if process.wait(timeout=30) != 0:
            raise ValueError(ERROR)
    except subprocess.TimeoutExpired:
        try:
            if os.name == 'nt':
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                    stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL, timeout=10, check=False)
            else:
                import signal
                os.killpg(process.pid, signal.SIGKILL)
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=10)
        raise ValueError(ERROR) from None


def apply_configuration(config_path, entries, stage_dir, codex_command):
    """Keep a private backup; compare and write through one exclusive Windows handle."""
    installed = False
    config, stage = Path(config_path), Path(stage_dir)
    try:
        if (os.name != 'nt' or not config.is_absolute() or not stage.is_absolute()
                or config.name != 'config.toml'):
            raise ValueError(ERROR)
        private_directory(stage)
        original = config_bytes(config)
        if config.stat().st_dev != stage.stat().st_dev:
            raise ValueError(ERROR)
        candidate, added = plan_configuration(original, entries)
        native_parse(config.parent, codex_command)
        if not added:
            return dict(added_names=added, backup_path=None, config_path=config)
        run = stage / uuid.uuid4().hex
        run.mkdir(mode=0o700)
        private_directory(run)
        backup = run / 'before.toml'
        backup.write_bytes(original)
        home = run / 'candidate'
        home.mkdir(mode=0o700)
        private_directory(home)
        saved = home / 'config.toml'
        saved.write_bytes(candidate)
        if config_bytes(saved) != candidate:
            raise ValueError(ERROR)
        native_parse(home, codex_command)
        if config_bytes(saved) != candidate:
            raise ValueError(ERROR)
        _guarded_update(config, original, candidate)
        installed = True
        if config_bytes(config) != candidate:
            raise ValueError(ERROR)
        native_parse(config.parent, codex_command)
        readback = config_bytes(config)
        if readback != candidate or plan_configuration(readback, entries) != (candidate, ()):
            raise ValueError(ERROR)
        return dict(added_names=added, backup_path=backup, config_path=config)
    except (OSError, ValueError, TypeError, subprocess.SubprocessError):
        if installed:
            # Reacquire exclusion and compare before rollback. Preserve later writers.
            try:
                _guarded_update(config, candidate, original)
            except (OSError, ValueError):
                pass
        raise ValueError(ERROR) from None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', required=True, type=Path)
    parser.add_argument('--project', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--stage-dir', type=Path)
    parser.add_argument('--codex-launcher', nargs='+')
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    try:
        entries = desired_entries(args.project)
        if args.apply:
            if args.stage_dir is None or not args.codex_launcher:
                raise ValueError(ERROR)
            result = apply_configuration(args.config, entries, args.stage_dir, args.codex_launcher)
            added = result['added_names']
            print('Added entries:', len(added))
            if result['backup_path']:
                print('Private backup:', result['backup_path'])
        else:
            _, added = plan_configuration(config_bytes(args.config), entries)
            print('Planned entries:', len(added))
        print('Names:', ', '.join(added))
    except (OSError, ValueError, TypeError):
        parser.exit(1, ERROR + '\n')


if __name__ == '__main__':
    main()
