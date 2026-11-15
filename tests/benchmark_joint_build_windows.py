"""Offline 256-build native benchmark from validated gear and talent pools."""
import argparse
from copy import deepcopy
import itertools
import json
import math
import os
from pathlib import Path
import time

from tests.benchmark_joint_build_optimizer import fixture
from tools import gear_builds, talent_builds, joint_build_optimizer as joint
from tools import joint_build_windows as windows, run_forever_sim as adapter
from tools.simulation_cache import SimulationCache

ROOT=Path(__file__).resolve().parents[1]


def manifest_from_validated_pools(catalogue,probe):
    profile=json.loads((ROOT/'tests/fixtures/forever-hunter-bm.json').read_bytes())['player']
    equipment=deepcopy(profile['equipment']);equipment['preserveSlots']=True
    context=dict(class_id=3,specialization='hunter',race=profile['race'],level=60,
                 professions=[profile['profession1'],profile['profession2']])
    inventory=[dict(item=item,count=1) for item in equipment['items'] if item]
    choices=[dict(slot=slot,items=[equipment['items'][slot],{}]) for slot in range(7)]
    offset,identity,references='0',None,{}
    while offset is not None:
        page=gear_builds.generate_gear_builds(context,equipment,inventory,choices,catalogue_path=catalogue,
            probe_path=probe,offset=offset,limit=16,expected_search_id=identity)
        assert page['unresolved_count']==page['rejected_count']==0
        assert int(page['end_decimal'])>int(offset), 'Generation made no progress'
        identity=page['search_id'];references.update(page['references']);offset=page['next_offset']
    assert len(references)==128
    talents=talent_builds.generate_talent_builds('hunter',60,profile['talentsString'],
        [dict(definition_id=134422,ranks=[4,5]),dict(definition_id=134438,ranks=[3,4])])
    assert talents['accepted_count']==2
    manifest=fixture(20261115,2)
    manifest.update(axes=[dict(name='gear',category='gear',candidates=list(references)),talents['axis']],
                    references=references|talents['references'])
    manifest['case']['constraints_sha256']=joint.planner.digest(dict(gear_search=identity,talent_search=talents['search_id']))
    return manifest,identity,talents['search_id']


def independent_requests(manifest):
    result=[]
    for selection in itertools.product(*(axis['candidates'] for axis in manifest['axes'])):
        request=deepcopy(manifest['base_request']);player=request['raid']['parties'][0]['players'][0]
        for axis,ref in zip(manifest['axes'],selection,strict=True):
            player[dict(gear='equipment',talents='talentsString')[axis['category']]]=deepcopy(manifest['references'][ref]['value'])
        result.append(request)
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for flag in ('engine','probe','catalogue','cache-root','output'):
        parser.add_argument('--'+flag,type=Path,required=True)
    args=parser.parse_args()
    if not all(value.is_absolute() for value in vars(args).values()):
        parser.error('Explicit absolute paths needed')
    os.environ['GOMAXPROCS']='2'
    started=time.perf_counter()
    manifest,gear_id,talent_id=manifest_from_validated_pools(args.catalogue,args.probe)
    generation_seconds=time.perf_counter()-started
    cache=SimulationCache(args.cache_root)
    offset,summary,pages='0',None,[]
    while offset is not None:
        start=time.perf_counter()
        page=windows.evaluate_joint_build_window(manifest,args.engine,offset=offset,limit=64,cache=cache)
        assert int(page['end_decimal'])>int(offset), 'Evaluation made no progress'
        elapsed=time.perf_counter()-start
        summary=windows.merge_joint_build_windows(manifest,[page],summary=summary)
        pages.append((page,elapsed));offset=page['next_offset']
    assert len(pages)==4
    assert summary['reported_matrix_complete'] and summary['reported_evaluated_decimal']=='256'
    start=time.perf_counter()
    oracle=adapter.compare(args.engine,joint.layout.ENGINE_SHA256,independent_requests(manifest))['results']
    oracle_seconds=time.perf_counter()-start
    actual={int(row['ordinal_decimal']):row for page,_ in pages for row in page['results']}
    assert len(actual)==len(oracle)==256
    assert all(math.isclose(actual[i]['mean_dps'],row['mean_dps'],rel_tol=1e-12,abs_tol=1e-10) for i,row in enumerate(oracle))
    expected_top=sorted(range(256),key=lambda i:(-oracle[i]['mean_dps'],i))[:16]
    assert [int(row['ordinal_decimal']) for row in summary['top_results']]==expected_top
    last=pages[-1][0]
    warm=windows.evaluate_joint_build_window(manifest,args.engine,offset=last['start_decimal'],limit=64,cache=cache)
    assert all(row['cache_status']=='hit' for row in warm['results'])
    assert {row['ordinal_decimal']:row['mean_dps'] for row in warm['results']}=={row['ordinal_decimal']:row['mean_dps'] for row in last['results']}
    record=dict(gear_generation_search_id=gear_id,talent_generation_search_id=talent_id,
        generation_seconds=generation_seconds,search_id=summary['search_id'],coordinates=256,gear_variants=128,talent_variants=2,
        native_trials_per_request=2,independent_oracle_match=True,top16_match=True,oracle_seconds=oracle_seconds,
        page_seconds=[seconds for _,seconds in pages],warm_last_page_hits=64,
        engine_sha256=joint.layout.ENGINE_SHA256,probe_sha256=gear_builds.PROBE_SHA256,
        gear_rules_sha256=gear_builds.RULES_SHA256,talent_rules_sha256=talent_builds.RULES_SHA256,
        summary=summary,game_execution=False,
        notes='Public model fixture, GOMAXPROCS2. Two trials test execution/accounting, not statistical build superiority. '
              'Independent Cartesian native comparison uses the same engine, so agreement does not establish game accuracy.')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
    args.output.with_name('manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(coordinates=256,oracle_match=True,top16_match=True,output=str(args.output))))


if __name__=='__main__':
    main()
