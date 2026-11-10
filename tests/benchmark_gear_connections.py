"""Offline synthetic nine-window benchmark; run with PYTHONPATH=.;tests.

The fresh mode models the previous per-window connection lifetime. Its outer
placeholder never supplies data; every window opens a real initialized MCP.
No simulator, real character store, deployment or network is used.
"""
import asyncio
from contextlib import asynccontextmanager
import json
import os
from pathlib import Path
import statistics
import tempfile
from time import perf_counter
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import collect_forever_gear_windows as controller


def main():
    root=Path(__file__).resolve().parents[1]
    binary=(root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
    catalogue=(root/'local-data/forever/catalogue.json').resolve(strict=True)
    trials=[]
    for mode in ['fresh','reused']*3:
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder:
            base=Path(folder);value=fixture()
            value['client'].update(family='forever',build='70862',interface=16001)
            value['sections']['bags']={'0':{str(i):{'item_id':item,'count':1,'link':f'item:{item}:0:0:0:0:0:0'}
                for i,item in enumerate([4549,4550],1)}}
            key=SnapshotStore(base).ingest(value);template,_,_=inputs()
            (base/'template.json').write_text(json.dumps(template))
            args=controller.parser().parse_args(['--mcp',str(binary),'--snapshot-id',key,
                '--slot','finger1','--slot','finger2','--template',str(base/'template.json'),
                '--journal',str(base/'plans.sqlite'),'--collapse-identical-items',
                '--window-size','1','--max-windows','16','--time-budget-seconds','60'])
            starts=[0];native_stdio=controller.ranker.stdio_client
            native_connection=controller.ranker.collection_connection
            placeholder=object()
            @asynccontextmanager
            async def counted(*a,**kw):
                starts[0]+=1
                async with native_stdio(*a,**kw) as streams:yield streams
            @asynccontextmanager
            async def fresh_window(binary,errlog,existing=None):
                if existing is None:
                    yield placeholder
                else:
                    assert existing is placeholder
                    async with native_connection(binary,errlog) as session:yield session
            connection=fresh_window if mode=='fresh' else native_connection
            with patch.dict(os.environ,{'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue)}), \
                    patch.object(controller.ranker,'stdio_client',counted), \
                    patch.object(controller.ranker,'collection_connection',connection):
                begin=perf_counter();report=asyncio.run(controller.collect_windows(args));elapsed=perf_counter()-begin
            assert report['progress']['reported_coordinate_coverage_complete']
            assert report['windows_inserted']==9
            assert starts[0]==(9 if mode=='fresh' else 1)
            trials.append({'mode':mode,'seconds':elapsed,'mcp_starts':starts[0],'windows':9})
    medians={mode:statistics.median(row['seconds'] for row in trials if row['mode']==mode)
        for mode in ['fresh','reused']}
    print(json.dumps({'trials':trials,'median_seconds':medians,
        'fresh_over_reused_ratio':medians['fresh']/medians['reused']}))


if __name__=='__main__':main()
