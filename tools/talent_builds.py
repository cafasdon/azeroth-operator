"""Generate bounded talent combinations valid under pinned Forever reference rules."""
import argparse
import asyncio
from collections import Counter
import hashlib
import json
from pathlib import Path
from tools import build_search as wire
from tools.sync_exports import absolute, ordinary

RULES_PATH = Path(__file__).resolve().parents[1]/'clients/forever/talent-rules.json'
RULES_SHA256 = 'f864f3f2fc197c9844004ee56de4b69818fdd6c123c886e9ce5daf5cf9245d34'
CLASSES = ('druid','hunter','mage','paladin','priest','rogue','shaman','warlock','warrior')
ERROR = 'Invalid or oversized offline talent build request'


def rules():
    data = RULES_PATH.read_bytes()
    if len(data) > 128*1024 or hashlib.sha256(data).hexdigest() != RULES_SHA256:
        raise ValueError('Talent rules changed')
    return json.loads(data)


def parse(name, level, value, source):
    if type(name) is not str or name not in CLASSES:
        raise ValueError('Unknown class')
    wire.integer(level, 60)
    if type(value) is not str or len(value) > 256 or any(c not in '0123456789-' for c in value):
        raise ValueError('Invalid talent string')
    parts = value.split('-')
    trees = source['classes'][name]
    if len(parts) > 3 or any(len(p) > len(t) for p,t in zip(parts,trees)):
        raise ValueError('Invalid talent string size')
    parts += ['']*(3-len(parts))
    return trees, [[int(c) for c in p.ljust(len(t),'0')] for p,t in zip(parts,trees)]


def validate(name, level, value, source):
    trees, points = parse(name,level,value,source)
    errors, total = [], sum(map(sum,points))
    if total > max(0,level-9):
        errors.append(dict(code='point_budget'))
    for tree, ranks in zip(trees,points):
        for index, (node, rank) in enumerate(zip(tree,ranks)):
            if rank > node['max_rank']:
                errors.append(dict(code='rank_limit', definition_id=node['definition_id']))
            if not rank:
                continue
            if sum(p for n,p in zip(tree,ranks) if n['row'] < node['row']) < 5*node['row']:
                errors.append(dict(code='row_gate', definition_id=node['definition_id']))
            prerequisite = node['prerequisite']
            if prerequisite is not None and ranks[prerequisite] != tree[prerequisite]['max_rank']:
                errors.append(dict(code='prerequisite', definition_id=node['definition_id']))
    canonical = '-'.join(''.join(map(str,tree)).rstrip('0') for tree in points)
    return dict(valid=not errors, class_name=name, level=level, talents_string=canonical, points_spent=total, point_budget=max(0,level-9),
                violations=errors, rules_sha256=RULES_SHA256, native_legality_verified=False,
                simulation_model_verified=False, game_execution=False)


def validate_talent_build(class_name, level, talents_string):
    return validate(class_name,level,talents_string,rules())


def generate_talent_builds(class_name,level,base_talents,choices,offset='0',limit=32,scan_limit=4096,expected_search_id=None):
    source = rules()
    trees, base = parse(class_name,level,base_talents,source)
    if type(choices) is not list or len(choices) > 16:
        raise ValueError('Expected up to 16 rank domains')
    locations = {n['definition_id']:(t,i,n['max_rank']) for t,tree in enumerate(trees) for i,n in enumerate(tree)}
    domains, seen, total = [], set(), 1
    for choice in choices:
        wire.exact(choice,('definition_id','ranks'))
        definition, ranks = choice['definition_id'], choice['ranks']
        if type(definition) is not int or definition not in locations or definition in seen:
            raise ValueError('Unknown or duplicate talent definition')
        if type(ranks) is not list or not 1 <= len(ranks) <= 10:
            raise ValueError('Invalid rank domain')
        if any(type(rank) is not int or not 0 <= rank <= locations[definition][2] for rank in ranks) or len(set(ranks)) != len(ranks):
            raise ValueError('Invalid rank choices')
        seen.add(definition); domains.append((definition,ranks)); total *= len(ranks)
    baseline = '-'.join(''.join(map(str,t)).rstrip('0') for t in base)
    search_id = wire.digest(dict(version=1,rules_sha256=RULES_SHA256,class_name=class_name,level=level,base=baseline,choices=choices))
    if expected_search_id is not None and wire.text(expected_search_id,wire.HASH_PATTERN,64) != search_id:
        raise ValueError('Talent search changed')
    start = int(wire.text(offset,wire.OFFSET_PATTERN,80))
    wire.integer(limit,128); wire.integer(scan_limit,4096)
    if start > total:
        raise ValueError('Offset outside talent matrix')
    end, accepted, references, rejected = start, [], {}, Counter()
    while end < min(start+scan_limit,total) and len(accepted) < limit:
        index, selected = end, [list(tree) for tree in base]
        for definition, ranks in reversed(domains):
            index, digit = divmod(index,len(ranks))
            tree, node, _ = locations[definition]
            selected[tree][node] = ranks[digit]
        string = '-'.join(''.join(map(str,t)).rstrip('0') for t in selected)
        result = validate(class_name,level,string,source)
        if result['valid']:
            entry = dict(category='talents',value=result['talents_string'])
            ref = wire.digest(entry); references[ref] = entry
            accepted.append(dict(ordinal_decimal=str(end),talents_string=result['talents_string'],reference_sha256=ref))
        else:
            rejected.update({v['code'] for v in result['violations']})
        end += 1
    return dict(search_id=search_id,class_name=class_name,level=level,rules_sha256=RULES_SHA256,total_decimal=str(total),start_decimal=str(start),end_decimal=str(end),
                next_offset=str(end) if end < total else None, examined_count=end-start,accepted_count=len(accepted),
                rejected_count=end-start-len(accepted),rejection_reasons=dict(rejected),accepted=accepted,references=references,
                axis=dict(name='talents',category='talents',candidates=list(references)) if references else None,
                declared_matrix_complete=start==0 and end==total, full_talent_space_covered=False,
                native_legality_verified=False, simulation_model_verified=False,active_character_verified=False,game_execution=False)


def dispatch(name, arguments):
    required = ('class_name','level','talents_string') if name=='validate_talent_build' else ('class_name','level','base_talents','choices')
    optional = () if name=='validate_talent_build' else ('offset','limit','scan_limit','expected_search_id')
    if name not in ('validate_talent_build','generate_talent_builds') or type(arguments) is not dict or not set(required)<=arguments.keys() or set(arguments)-set(required)-set(optional):
        raise ValueError(ERROR)
    result = (validate_talent_build if name=='validate_talent_build' else generate_talent_builds)(**arguments)
    envelope = dict(content=[dict(type='text',text=wire.encoded(result))],isError=False)
    if len(wire.encoded(envelope)) > wire.MAX_BYTES-1024:
        raise ValueError(ERROR)
    return result


def schemas():
    string = dict(type='string',maxLength=256)
    common = dict(class_name=dict(type='string',enum=list(CLASSES)),level=dict(type='integer',minimum=1,maximum=60))
    def obj(properties, required):
        return dict(type='object',properties=properties,required=required,additionalProperties=False)
    definition = obj(dict(definition_id=dict(type='integer',minimum=1,maximum=2**32-1),
                          ranks=dict(type='array',minItems=1,maxItems=10,uniqueItems=True,items=dict(type='integer',minimum=0,maximum=9))),['definition_id','ranks'])
    return dict(validate_talent_build=obj(common|dict(talents_string=string),['class_name','level','talents_string']),
                generate_talent_builds=obj(common|dict(base_talents=string,choices=dict(type='array',maxItems=16,items=definition),
                    offset=dict(type='string',pattern='^(?:0|[1-9][0-9]{0,79})(?![\\s\\S])'),limit=dict(type='integer',minimum=1,maximum=128),
                    scan_limit=dict(type='integer',minimum=1,maximum=4096),expected_search_id=dict(type='string',pattern='^[0-9a-f]{64}(?![\\s\\S])')),
                    ['class_name','level','base_talents','choices']))


def serve():
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import Tool,ToolAnnotations,TextContent
    server = Server('Azeroth Offline Talent Build Generator')
    @server.list_tools()
    async def list_tools():
        return [Tool(name=name,inputSchema=schema,description='Offline pinned-reference talent constraints. No native legality, active character selection or game action.',
                     annotations=ToolAnnotations(readOnlyHint=True,destructiveHint=False,idempotentHint=True,openWorldHint=False)) for name,schema in schemas().items()]
    @server.call_tool(validate_input=False)
    async def call_tool(name,arguments):
        try:
            return [TextContent(type='text',text=wire.encoded(dispatch(name,arguments)))]
        except (OSError,ValueError,TypeError,KeyError,OverflowError,RecursionError):
            raise ValueError(ERROR) from None
    async def run():
        async with stdio_server() as (read,write):
            await server.run(read,write,server.create_initialization_options())
    asyncio.run(run())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('validate','generate','serve'));parser.add_argument('--input')
    args = parser.parse_args()
    if args.action=='serve':
        serve();return
    try:
        path = ordinary(absolute(args.input))
        if not path.is_file() or path.stat().st_nlink!=1:
            raise ValueError(ERROR)
        with path.open('rb') as stream:
            data = stream.read(wire.MAX_INPUT_BYTES+1)
        if len(data)>wire.MAX_INPUT_BYTES:
            raise ValueError(ERROR)
        print(wire.encoded(dispatch('validate_talent_build' if args.action=='validate' else 'generate_talent_builds',json.loads(data,object_pairs_hook=wire.unique_object))))
    except (OSError,ValueError,TypeError,KeyError,OverflowError,RecursionError):
        parser.exit(1,ERROR+'\n')


if __name__=='__main__':
    main()
