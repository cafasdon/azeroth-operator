"""Personal rank observations stay historical, numeric and owner local."""
from copy import deepcopy
import asyncio
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


class PlayerPvpProgressionTests(unittest.IsolatedAsyncioTestCase):
    async def test_reader_parity_and_real_mcp_projection(self):
        root=Path(__file__).resolve().parents[1]
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        section={'coverage':'reported_pvp_rank_faction','faction_id':2800,
            'rank':3,'rank_points':10,'next_rank_threshold':100}
        full={**section,'max_rank':14,'current_week_max_rank':5,'previous_week_max_rank':4}
        numeric=['rank','rank_points','next_rank_threshold','max_rank','current_week_max_rank','previous_week_max_rank']
        maximum={**section,**{key:4294967295 for key in numeric}}
        zero={**section,**{key:0 for key in numeric}}
        overflow={**section,'rank_points':101}
        planned={**full,'total_points_by_rank':{'3':100,'4':200,'5':300,'14':1000}}
        planned_zero={**zero,'total_points_by_rank':{'0':0}}
        planned_max={**maximum,'total_points_by_rank':{'4294967295':4294967295}}
        planned_partial={**full,'total_points_by_rank':{'5':300}}
        planned_past={**planned,'rank_points':300}
        invalid=[None,{},[],{**section,'coverage':'complete'}, {**section,'faction_id':2801},
            {**section,'faction_id':True},{**section,'faction_id':2800.0}, {**section,'player':'PRIVATE'}]
        for key in section:
            row=deepcopy(section);row.pop(key);invalid.append(row)
        for key in numeric:
            for value in [None,True,False,-1,1.0,'1',4294967296,[],{}]:invalid.append({**full,key:value})
        for totals in [None,[],{}, {'0':0},{'03':100},{'PRIVATE':1},{'3':True},{'3':1.0},
                       {'3':None},{'3':'100'},{'3':-1},{'3':4294967296},{'3':{}},
                       {'3':1,'4':2,'5':3,'14':4,'6':5}]:invalid.append({**full,'total_points_by_rank':totals})
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));valid=[];bad=[]
            for row in [None,section,full,maximum,zero,overflow,planned,planned_zero,planned_max,planned_partial,planned_past]:
                saved=deepcopy(base)
                if row is not None:saved['sections']['player_pvp_progression']=row
                valid.append((store.ingest(saved),row))
            snapshots=[]
            for row in invalid:
                saved=deepcopy(base);saved['sections']['player_pvp_progression']=row;snapshots.append(saved)
            for family,interface in [('retail',120000),('classic',11500)]:
                saved=deepcopy(base);saved['client']={'family':family,'variant':'live','build':'1','interface':interface}
                saved['sections']['player_pvp_progression']=section;snapshots.append(saved)
            for saved in snapshots:
                with self.assertRaises(ValueError):validate_snapshot(saved)
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/(key+'.json')).write_bytes(raw);bad.append(key)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(root/'target/debug/azeroth-operator.exe'),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        result=await asyncio.wait_for(session.call_tool(tool,args),timeout=10)
                        self.assertEqual(bool(result.isError),error,result);self.assertNotIn('PRIVATE',str(result))
                        return None if error else json.loads(result.content[0].text)
                    summaries = {}
                    for key,row in valid:
                        inspection=await call('inspect_snapshot',snapshot_id=key,section='player_pvp_progression')
                        report=(await call('character_summary',snapshot_id=key))['player_pvp_progression_observation']
                        summaries[key] = report
                        if row is None:self.assertFalse(inspection['available']);self.assertIsNone(report)
                        else:
                            self.assertEqual(inspection['data'],row)
                            for field,value in row.items():self.assertEqual(report[field],value)
                            expected=row['next_rank_threshold']-row['rank_points'] if row['next_rank_threshold']>0 and row['rank_points']<=row['next_rank_threshold'] else None
                            self.assertEqual(report['remaining_points_to_reported_next_threshold'],expected)
                            if 'total_points_by_rank' in row:
                                totals=row['total_points_by_rank'];current=totals.get(str(row['rank']))
                                current=None if current is None else current+row['rank_points']
                                self.assertEqual(report['reported_total_rank_points'],current)
                                for cap in ['max_rank','current_week_max_rank','previous_week_max_rank']:
                                    target=totals.get(str(row.get(cap)))
                                    self.assertEqual(report['remaining_points_to_reported_caps'][cap],
                                        None if current is None or not target else max(0,target-current))
                            else:self.assertNotIn('remaining_points_to_reported_caps',report)
                            for flag in ['current_progression_verified','rank_eligibility_verified','reward_eligibility_verified','game_execution']:
                                self.assertIs(report[flag],False)
                    old, new = valid[6][0], valid[10][0]
                    selector = 'player_pvp_progression_changes'
                    self.assertNotIn(selector, await call('compare_snapshots',before_id=old,after_id=new))
                    for first, second in [(old,new), (valid[0][0],new), (old,valid[0][0])]:
                        compared = (await call('compare_snapshots',before_id=first,after_id=second,**{selector:True}))[selector]
                        self.assertEqual(compared['before'], summaries[first])
                        self.assertEqual(compared['after'], summaries[second])
                        self.assertIsNone(compared['earned_points_delta'])
                        self.assertIs(compared['same_season_verified'], False)
                        self.assertIs(compared['live'], False)
                    for key in bad:await call('inspect_snapshot',error=True,snapshot_id=key,section='player_pvp_progression')
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
