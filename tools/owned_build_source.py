"""Read one owner's saved active export and account-scoped character snapshots."""
from contextlib import asynccontextmanager
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import tempfile

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, MAX_BYTES, validate_snapshot
from tools import build_search as wire
from tools.account_inventory import AccountInventory, account, text
from tools.sync_exports import absolute, ordinary

ERROR = 'Owned character source unavailable, changed or invalid'


def read_json(path, maximum):
    path = ordinary(absolute(str(path)))
    if not path.is_file() or path.stat().st_nlink != 1:
        raise ValueError(ERROR)
    with path.open('rb') as stream:
        raw = stream.read(maximum + 1)
    if len(raw) > maximum:
        raise ValueError(ERROR)
    return json.loads(raw, object_pairs_hook=wire.unique_object)


class OwnedBuildSource:
    def __init__(self, config):
        wire.exact(config, ('reader','snapshots','active_export','inventory_database','account_key','client'))
        self.config = config
        self.reader, self.root, self.export, self.database = (
            ordinary(absolute(config[key])) for key in ('reader','snapshots','active_export','inventory_database'))
        account(config['account_key'])
        wire.exact(config['client'], ('family','variant','build','interface'))
        if config['client']['family'] != 'forever' or type(config['client']['interface']) is not int or config['client']['interface'] < 1:
            raise ValueError(ERROR)
        for key in ('variant','build'):
            text(config['client'][key])

    def list_characters(self):
        inventory, rows, offset = AccountInventory(self.database), [], 0
        while True:
            page = inventory.query(self.config['account_key'], offset=offset, limit=100)
            rows.extend(row for row in page['characters'] if row['client'] == self.config['client'])
            if not page['has_more']:
                break
            offset += 100
            if offset > 1000:
                raise ValueError(ERROR)
        return dict(characters=rows, live=False, active_character_verified=False)

    def _export_snapshot(self):
        ordinary(self.reader); ordinary(self.export)
        if not self.reader.is_file() or not self.export.is_file() or self.export.stat().st_nlink != 1:
            raise ValueError(ERROR)
        with tempfile.TemporaryFile() as output:
            try:
                subprocess.run([str(self.reader),'decode-export',str(self.export)], stdout=output,
                               stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL, check=True, timeout=15)
            except (OSError, subprocess.SubprocessError):
                raise ValueError(ERROR) from None
            if output.tell() > MAX_BYTES:
                raise ValueError(ERROR)
            output.seek(0)
            snapshot = validate_snapshot(json.loads(output.read(MAX_BYTES + 1), object_pairs_hook=wire.unique_object))
        return snapshot

    def select(self, character=None):
        exported = None
        if character is None:
            exported = self._export_snapshot()
            character = exported['character']
            if exported['client'] != self.config['client']:
                raise ValueError(ERROR)
        wire.exact(character, ('name','realm'))
        identity = tuple(text(character[key]).casefold() for key in ('name','realm'))
        rows = [row for row in self.list_characters()['characters']
                if tuple(row['character'][key].casefold() for key in ('name','realm')) == identity]
        if len(rows) != 1:
            raise ValueError(ERROR)
        row = rows[0]
        path = ordinary(self.root / (wire.text(row['snapshot_id'],wire.HASH_PATTERN,64)+'.json'))
        if not path.is_file() or path.stat().st_nlink != 1:
            raise ValueError(ERROR)
        snapshot = SnapshotStore(self.root).read(row['snapshot_id'])
        stamp = datetime.fromisoformat(snapshot['captured_at'].replace('Z','+00:00'))
        if (snapshot['source'] != 'addon' or snapshot['client'] != self.config['client']
                or snapshot['character'] != row['character']
                or stamp != datetime.fromisoformat(row['captured_at'])):
            raise ValueError(ERROR)
        if exported is not None and (stamp != datetime.fromisoformat(exported['captured_at'].replace('Z','+00:00'))
                or dict(snapshot,captured_at=None,unavailable=snapshot.get('unavailable',{}))
                    != dict(exported,captured_at=None,unavailable=exported.get('unavailable',{}))):
            raise ValueError(ERROR)
        age = (datetime.now(timezone.utc)-stamp).total_seconds()
        if age < 0:
            raise ValueError(ERROR)
        return dict(snapshot_id=row['snapshot_id'],snapshot=snapshot,
                    selection_basis='saved_active_export' if exported is not None else 'explicit_saved_character',
                    age_seconds=round(age),live=False,active_character_verified=False)

    @asynccontextmanager
    async def session(self, catalogue_path=None):
        ordinary(self.reader); ordinary(self.root); ordinary(self.database)
        environment = dict(os.environ, AZEROTH_DATA_DIR=str(self.root))
        environment.pop('AZEROTH_FOREVER_CATALOGUE', None)
        if catalogue_path is not None:
            environment['AZEROTH_FOREVER_CATALOGUE'] = str(ordinary(absolute(str(catalogue_path))))
        with tempfile.TemporaryFile(mode='w+') as errors:
            async with stdio_client(StdioServerParameters(command=str(self.reader),env=environment),errlog=errors) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    yield session


async def saved_call(session, name, arguments):
    reply = await session.call_tool(name, arguments)
    if reply.isError or len(reply.content) != 1 or reply.content[0].type != 'text' or len(reply.content[0].text.encode()) > 256*1024:
        raise ValueError(ERROR)
    return json.loads(reply.content[0].text, object_pairs_hook=wire.unique_object)
