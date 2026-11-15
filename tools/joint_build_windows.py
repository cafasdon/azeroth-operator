"""Native experiment pages and a bounded, unauthenticated reported coverage ledger."""
import argparse
import asyncio
from copy import deepcopy
import json
import math
from tools import joint_build_optimizer as joint
from tools import build_search as wire
from tools.sync_exports import absolute, ordinary

evaluate_joint_build_window = joint.evaluate_joint_build_window
ERROR = 'Invalid or failed bounded joint build window operation.'
PROOFS = ('candidate_legality_verified','model_compatibility_verified','source_data_complete_verified','constraints_verified',
          'snapshot_reference_verified','active_character_verified','all_owned_items_covered','global_optimum_verified',
          'statistical_superiority_verified','game_execution')
TRUST = 'User-supplied receipts are unauthenticated. Reported coverage and scores are not proof of native execution.'
COVERAGE = 'Only this explicit window was evaluated. No earlier coordinate coverage, native legality, ownership or global optimum is proved.'
ROW_KEYS = ('ordinal_decimal','selection','candidate_sha256','request_sha256','mean_dps','dps_standard_deviation','iterations','seed','cache_status')
WINDOW_KEYS = ('kind','search_id','planner_search_id','case','results','declared_coordinates','resolved_coordinates','evaluated_coordinates',
               'unique_requests','equivalent_coordinates','unique_native_iterations','simulation_performed','objective_evaluated',
               'declared_matrix_complete','coverage','total_decimal','start_decimal','end_decimal','next_offset','window_complete',*PROOFS)
SUMMARY_KEYS = ('kind','search_id','planner_search_id','case','total_decimal','top_k','coverage_ranges','reported_evaluated_decimal',
                'reported_matrix_complete','next_offset','top_results','top_k_truncated','simulation_performed','objective_evaluated','receipt_trust',*PROOFS)


def bounded(value,maximum):
    size = len(wire.encoded(value).encode('ascii'))
    if size>maximum:
        raise ValueError('Input or output exceeds transport budget')
    return size


def output(value):
    bounded(dict(content=[dict(type='text',text=wire.encoded(value))],isError=False),wire.MAX_BYTES)
    return value


def decimal(value):
    return int(wire.text(value,wire.OFFSET_PATTERN,80))


def false_flags(value):
    if any(value[k] is not False for k in PROOFS):
        raise ValueError('Unsupported proof claim')


def row_check(row,coordinate,requests,ranked=False):
    wire.exact(row,(*ROW_KEYS,'rank','tied_observed_mean') if ranked else ROW_KEYS)
    for key in ('ordinal_decimal','selection','candidate_sha256','request_sha256'):
        if wire.encoded(row[key])!=wire.encoded(coordinate[key]):
            raise ValueError('Coordinate or request binding changed')
    request = requests[coordinate['request_sha256']]
    if (any(type(row[k]) not in (int,float) or not math.isfinite(row[k]) or row[k]<0 for k in ('mean_dps','dps_standard_deviation'))
            or type(row['iterations']) is not int or row['iterations']!=request['simOptions']['iterations']
            or type(row['seed']) is not str or row['seed']!=request['simOptions']['randomSeed']
            or row['cache_status'] not in ('hit','miss','disabled')):
        raise ValueError('Invalid reported numeric row')
    return {k:deepcopy(row[k]) for k in ROW_KEYS}


def ordered(rows):
    return sorted(rows,key=lambda r:(-r['mean_dps'],decimal(r['ordinal_decimal'])))


def consistent(rows):
    scores = {}
    for row in rows:
        metrics = tuple(row[k] for k in ('mean_dps','dps_standard_deviation','iterations','seed'))
        if scores.setdefault(row['request_sha256'],metrics)!=metrics:
            raise ValueError('Equivalent requests report different numeric results')


def check_window(manifest,window,search_id):
    bounded(window,wire.MAX_BYTES); wire.exact(window,WINDOW_KEYS); false_flags(window)
    start,end = decimal(window['start_decimal']),decimal(window['end_decimal'])
    if not 1<=end-start<=64:
        raise ValueError('Expected a nonempty complete window')
    plan,requests,coordinates,iterations = joint.preflight(manifest,search_id,window_mode=True,offset=str(start),limit=end-start)
    expected = dict(kind='joint_build_window_v1',search_id=plan['search_id'],planner_search_id=plan['planner_search_id'],case=plan['case'],
                    total_decimal=plan['total_decimal'],end_decimal=plan['end_decimal'],next_offset=plan['next_offset'],coverage=COVERAGE,
                    window_complete=True,simulation_performed=True,objective_evaluated=True,
                    declared_matrix_complete=start==0 and str(end)==plan['total_decimal'])
    if any(wire.encoded(window[k])!=wire.encoded(v) for k,v in expected.items()):
        raise ValueError('Window receipt binding changed')
    for key,count in dict(declared_coordinates=len(coordinates),resolved_coordinates=len(coordinates),evaluated_coordinates=len(coordinates),
                          unique_requests=len(requests),equivalent_coordinates=len(coordinates)-len(requests),unique_native_iterations=iterations).items():
        if type(window[key]) is not int or window[key]!=count:
            raise ValueError('Incomplete window accounting')
    rows = window['results']
    if type(rows) is not list or len(rows)!=len(coordinates):
        raise ValueError('Incomplete coordinate rows')
    by_ordinal = {c['ordinal_decimal']:c for c in coordinates}; seen = set(); clean = []
    for row in rows:
        if type(row) is not dict or type(row.get('ordinal_decimal')) is not str or row['ordinal_decimal'] not in by_ordinal or row['ordinal_decimal'] in seen:
            raise ValueError('Duplicate or foreign row')
        clean.append(row_check(row,by_ordinal[row['ordinal_decimal']],requests,True)); seen.add(row['ordinal_decimal'])
    if clean!=ordered(clean):
        raise ValueError('Window rank order changed')
    for row in rows:
        rank = 1+sum(other['mean_dps']>row['mean_dps'] for other in rows)
        tied = sum(other['mean_dps']==row['mean_dps'] for other in rows)>1
        if type(row['rank']) is not int or row['rank']!=rank or row['tied_observed_mean'] is not tied:
            raise ValueError('Window ranks changed')
    return (start,end),clean


def ranges_check(values,total):
    if type(values) is not list or not 1<=len(values)<=512:
        raise ValueError('Invalid coverage intervals')
    ranges = []
    for value in values:
        wire.exact(value,('start_decimal','end_decimal')); start,end = decimal(value['start_decimal']),decimal(value['end_decimal'])
        if not 0<=start<end<=total or ranges and start<=ranges[-1][1]:
            raise ValueError('Noncanonical or overlapping coverage')
        ranges.append((start,end))
    return ranges


def check_summary(manifest,summary,plan,top_k):
    bounded(summary,wire.MAX_BYTES); wire.exact(summary,SUMMARY_KEYS); false_flags(summary)
    expected = dict(kind='joint_build_summary_v1',search_id=plan['search_id'],planner_search_id=plan['planner_search_id'],case=plan['case'],
                    total_decimal=plan['total_decimal'],top_k=top_k,simulation_performed=False,objective_evaluated=False,receipt_trust=TRUST)
    if any(wire.encoded(summary[k])!=wire.encoded(v) for k,v in expected.items()):
        raise ValueError('Previous summary binding changed')
    total = decimal(plan['total_decimal']); ranges = ranges_check(summary['coverage_ranges'],total); count = sum(b-a for a,b in ranges)
    complete = ranges==[(0,total)]; next_offset = None if complete else str(ranges[0][1] if ranges[0][0]==0 else 0)
    if (decimal(summary['reported_evaluated_decimal'])!=count or summary['reported_matrix_complete'] is not complete
            or summary['next_offset']!=next_offset or summary['top_k_truncated'] is not (count>top_k)):
        raise ValueError('Previous summary accounting changed')
    rows = summary['top_results']
    if type(rows) is not list or len(rows)!=min(top_k,count):
        raise ValueError('Previous top results are incomplete')
    clean,seen = [],set()
    for row in rows:
        if type(row) is not dict:
            raise ValueError('Invalid top result')
        ordinal = decimal(row['ordinal_decimal'])
        if ordinal in seen or not any(a<=ordinal<b for a,b in ranges):
            raise ValueError('Top result outside reported coverage')
        _,requests,coordinates,_ = joint.preflight(manifest,plan['search_id'],window_mode=True,offset=str(ordinal),limit=1)
        clean.append(row_check(row,coordinates[0],requests)); seen.add(ordinal)
    if clean!=ordered(clean):
        raise ValueError('Previous top results are not ordered')
    return ranges,clean


def merge_joint_build_windows(manifest,windows,summary=None,top_k=16):
    wire.integer(top_k,64)
    if type(windows) is not list or not 1<=len(windows)<=32:
        raise ValueError('Expected one to 32 window receipts')
    sizes = bounded(manifest,wire.MAX_INPUT_BYTES)+sum(bounded(w,wire.MAX_BYTES) for w in windows)+(bounded(summary,wire.MAX_BYTES) if summary is not None else 0)
    if sizes>16*1024*1024:
        raise ValueError('Merge input budget exceeded')
    plan,_,_,_ = joint.preflight(manifest,window_mode=True,limit=1)
    ranges,rows = check_summary(manifest,summary,plan,top_k) if summary is not None else ([],[])
    for window in windows:
        interval,results = check_window(manifest,window,plan['search_id']); ranges.append(interval); rows.extend(results)
    consistent(rows)
    combined = []
    for start,end in sorted(ranges):
        if combined and start<combined[-1][1]:
            raise ValueError('Overlapping or duplicate coverage')
        if combined and start==combined[-1][1]:
            combined[-1] = (combined[-1][0],end)
        else:
            combined.append((start,end))
    if len(combined)>512:
        raise ValueError('Coverage interval budget exceeded')
    count = sum(b-a for a,b in combined); total = decimal(plan['total_decimal']); complete = combined==[(0,total)]
    result = dict(kind='joint_build_summary_v1',search_id=plan['search_id'],planner_search_id=plan['planner_search_id'],case=plan['case'],
                  total_decimal=str(total),top_k=top_k,coverage_ranges=[dict(start_decimal=str(a),end_decimal=str(b)) for a,b in combined],
                  reported_evaluated_decimal=str(count),reported_matrix_complete=complete,
                  next_offset=None if complete else str(combined[0][1] if combined[0][0]==0 else 0),top_results=ordered(rows)[:top_k],
                  top_k_truncated=count>top_k,simulation_performed=False,objective_evaluated=False,receipt_trust=TRUST,**dict.fromkeys(PROOFS,False))
    return output(result)


def read_json(value,maximum):
    path = ordinary(absolute(value))
    if not path.is_file() or path.stat().st_nlink!=1:
        raise ValueError('Expected an ordinary single-link input')
    with path.open('rb') as stream:
        data = stream.read(maximum+1)
    if len(data)>maximum:
        raise ValueError('Input budget exceeded')
    return json.loads(data,object_pairs_hook=wire.unique_object)


def schemas():
    sha = dict(type='string',pattern='^[0-9a-f]{64}$',maxLength=64)
    props = dict(expected_search_id=sha,offset=dict(type='string',pattern='^(?:0|[1-9][0-9]{0,79})$',maxLength=80),limit=dict(type='integer',minimum=1,maximum=64))
    receipt = dict(type='object',additionalProperties=False,required=list(WINDOW_KEYS),properties={k:{} for k in WINDOW_KEYS})
    receipt['properties']['results'] = dict(type='array',maxItems=64,items=dict(type='object',additionalProperties=False,required=[*ROW_KEYS,'rank','tied_observed_mean'],properties={k:{} for k in (*ROW_KEYS,'rank','tied_observed_mean')}))
    summary = dict(type='object',additionalProperties=False,required=list(SUMMARY_KEYS),properties={k:{} for k in SUMMARY_KEYS})
    summary['properties']['coverage_ranges'] = dict(type='array',maxItems=512)
    summary['properties']['top_results'] = dict(type='array',maxItems=64)
    return dict(evaluate_joint_build_window=dict(type='object',additionalProperties=False,required=['expected_search_id'],properties=props),
                merge_joint_build_windows=dict(type='object',additionalProperties=False,required=['expected_search_id','windows'],properties=dict(expected_search_id=sha,
                    windows=dict(type='array',minItems=1,maxItems=32,items=receipt),summary=summary,top_k=dict(type='integer',minimum=1,maximum=64))))


def serve(manifest,engine,cache):
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import TextContent,Tool,ToolAnnotations
    prepared,_,_,_ = joint.preflight(manifest,window_mode=True,limit=1)
    server = Server('Azeroth Bounded Native Joint Windows'); busy = asyncio.Lock()
    def release(completed):
        busy.release()
        if not completed.cancelled():
            completed.exception()
    @server.list_tools()
    async def list_tools():
        label = prepared['case']['client']['variant']+', '+prepared['case']['specialization']+', level '+str(prepared['case']['character_level'])
        return [Tool(name=name,inputSchema=schema,description='Configured model case: '+label+'. Current character is unbound. Fixed search_id: '+prepared['search_id']+'. Offline fixed-manifest windows. Supplied receipts prove no execution, native legality, ownership or global optimum.',
                     annotations=ToolAnnotations(readOnlyHint=True,destructiveHint=False,openWorldHint=False)) for name,schema in schemas().items()]
    @server.call_tool(validate_input=False)
    async def call_tool(name,arguments):
        try:
            required = {'expected_search_id'} if name=='evaluate_joint_build_window' else {'expected_search_id','windows'}
            allowed = required|({'offset','limit'} if name=='evaluate_joint_build_window' else {'summary','top_k'})
            if name not in schemas() or type(arguments) is not dict or not required<=arguments.keys() or set(arguments)-allowed:
                raise ValueError(ERROR)
            if wire.text(arguments['expected_search_id'],wire.HASH_PATTERN,64)!=prepared['search_id']:
                raise ValueError(ERROR)
            if name=='merge_joint_build_windows':
                result = merge_joint_build_windows(manifest,**{k:v for k,v in arguments.items() if k!='expected_search_id'})
            else:
                if busy.locked():
                    raise ValueError(ERROR)
                await busy.acquire()
                task = asyncio.create_task(asyncio.to_thread(evaluate_joint_build_window,manifest,engine,cache=cache,**arguments))
                task.add_done_callback(release); result = await asyncio.shield(task)
            return [TextContent(type='text',text=wire.encoded(output(result)))]
        except joint.FAILURES:
            raise ValueError(ERROR) from None
    async def run():
        async with stdio_server() as (read,write):
            await server.run(read,write,server.create_initialization_options())
    asyncio.run(run())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('prepare','page','merge','serve')); parser.add_argument('--input',required=True)
    parser.add_argument('--engine'); parser.add_argument('--cache-root'); parser.add_argument('--offset',default='0'); parser.add_argument('--limit',type=int,default=64)
    parser.add_argument('--expected-search-id'); parser.add_argument('--windows',nargs='+'); parser.add_argument('--summary'); parser.add_argument('--top-k',type=int,default=16)
    args = parser.parse_args()
    try:
        manifest = read_json(args.input,wire.MAX_INPUT_BYTES)
        if args.action=='prepare':
            plan,_,_,_ = joint.preflight(manifest,args.expected_search_id,window_mode=True,offset=args.offset,limit=args.limit)
            result = dict(search_id=plan['search_id'],planner_search_id=plan['planner_search_id'],total_decimal=plan['total_decimal'],simulation_performed=False)
        elif args.action=='merge':
            if not args.windows or len(args.windows)>32:
                raise ValueError(ERROR)
            result = merge_joint_build_windows(manifest,[read_json(p,wire.MAX_BYTES) for p in args.windows],read_json(args.summary,wire.MAX_BYTES) if args.summary else None,args.top_k)
            if args.expected_search_id is not None and args.expected_search_id!=result['search_id']:
                raise ValueError(ERROR)
        else:
            engine = ordinary(absolute(args.engine)); cache = joint.SimulationCache(ordinary(absolute(args.cache_root))) if args.cache_root else None
            if args.action=='serve':
                serve(manifest,engine,cache); return
            result = evaluate_joint_build_window(manifest,engine,args.offset,args.limit,args.expected_search_id,cache)
        print(wire.encoded(output(result)))
    except joint.FAILURES:
        parser.exit(1,ERROR+'\n')


if __name__=='__main__':
    main()
