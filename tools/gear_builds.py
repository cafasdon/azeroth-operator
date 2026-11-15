"""Bounded, offline gear combinations under pinned Forever reference constraints."""
import argparse
import asyncio
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import time
from tools import build_search as wire, run_forever_sim as adapter
from tools.sync_exports import absolute, ordinary
from tools.sync_forever_catalogue import verify, MAX_BYTES, PIN

RULES_PATH = Path(__file__).resolve().parents[1]/'clients/forever/gear-rules.json'
RULES_SHA256 = 'd91d4abd941cd31a584fd1d5aa98b0e6ab7288727fba7d08ffebcc12c749609a'
PROBE_SHA256 = '209297881300e029cc726b1734c56c0d84efdfdf205e56e27682eeea6378300f'
CATALOGUE_SHA256 = '5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e'
ERROR = 'Invalid or oversized offline gear build request'
FLAGS = dict(native_legality_verified=False,simulation_model_verified=False,ownership_verified=False,
             active_character_verified=False,game_execution=False,skill_ranks_verified=False,weapon_training_verified=False)


def item_spec(value):
    if type(value) is not dict or set(value)-{'id','enchant','randomSuffix'} or value and 'id' not in value:
        raise ValueError('Invalid item specification')
    for key, number in value.items():
        if type(number) is not int or not (1 if key=='id' else 0)<=number<=2**31-1:
            raise ValueError('Invalid item identifier')
    return {k:v for k,v in value.items() if v}


def equipment_spec(value):
    wire.exact(value,('items','preserveSlots'))
    if value['preserveSlots'] is not True or type(value['items']) is not list or len(value['items'])!=17:
        raise ValueError('Expected 17 explicit equipment slots')
    return dict(items=[item_spec(v) for v in value['items']],preserveSlots=True)


def prepare(context,inventory,catalogue_path):
    raw = RULES_PATH.read_bytes()
    if len(raw)>128*1024 or hashlib.sha256(raw).hexdigest()!=RULES_SHA256:
        raise ValueError('Gear rules changed')
    rules = json.loads(raw); enums = rules['enums']
    wire.exact(context,('class_id','specialization','race','level','professions'))
    actor,spec,race,professions = (context[k] for k in ('class_id','specialization','race','professions'))
    if (type(actor) is not int or str(actor) not in rules['classes'] or type(spec) is not str
            or spec not in adapter.SPEC_CLASSES or adapter.CLASS_IDS[adapter.SPEC_CLASSES[spec]]!=actor
            or type(race) is not str or race not in enums['Race']
            or enums['Race'][race] not in rules['classes'][str(actor)]['races']
            or str(enums['Race'][race]) not in rules['race_factions'] or type(context['level']) is not int or context['level']!=60
            or type(professions) is not list or len(professions)>2
            or any(type(p) is not str or p not in enums['Profession'] or not enums['Profession'][p] for p in professions)
            or len(set(professions))!=len(professions)):
        raise ValueError('Unsupported character context')
    context = dict(context,professions=sorted(professions))
    with ordinary(absolute(str(catalogue_path))).open('rb') as stream:
        raw = stream.read(MAX_BYTES+1)
    verify(raw)
    if hashlib.sha256(raw).hexdigest()!=CATALOGUE_SHA256:
        raise ValueError('Pinned gear catalogue changed')
    catalogue = json.loads(raw)
    items = {v['id']:v for v in catalogue['items']}; copies = Counter()
    if type(inventory) is not list or len(inventory)>2048:
        raise ValueError('Expected bounded declared inventory')
    for entry in inventory:
        wire.exact(entry,('item','count')); item = item_spec(entry['item']); wire.integer(entry['count'],2**31-1)
        key = wire.digest(item)
        if not item or item['id'] not in items or key in copies:
            raise ValueError('Unknown or duplicate inventory variant')
        copies[key] = entry['count']
    return context,rules,items,{v['effectId']:v for v in catalogue['enchants']},copies


def conversion(equipment,probe_path,deadline):
    try:
        path = ordinary(absolute(str(probe_path)))
        with path.open('rb') as stream:
            if hashlib.file_digest(stream,'sha256').hexdigest()!=PROBE_SHA256:
                return 'probe_untrusted'
        remaining = deadline-time.monotonic()
        if remaining<=0:
            return 'deadline'
        with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
            process = subprocess.run([str(path)],input=wire.encoded(equipment).encode(),stdout=output,stderr=errors,
                                     timeout=min(10,remaining),check=False)
            if process.returncode or output.tell()>64*1024:
                return 'probe_failed'
            output.seek(0); result = json.loads(output.read(64*1024+1),object_pairs_hook=wire.unique_object)
        with path.open('rb') as stream:
            if hashlib.file_digest(stream,'sha256').hexdigest()!=PROBE_SHA256:
                return 'probe_untrusted'
        expected = [dict(id=i.get('id',0),enchant=i.get('enchant',0),randomSuffix=i.get('randomSuffix',0)) for i in equipment['items']]
        if (type(result) is not list or len(result)!=17 or any(type(v) is not dict or set(v)!=set(expected[0])
                or any(type(n) is not int for n in v.values()) for v in result)):
            return 'probe_failed'
        return None if result==expected else 'modifier_not_preserved'
    except (OSError,ValueError,subprocess.SubprocessError):
        return 'probe_failed'


def evaluate(context,equipment,rules,items,enchants,copies,probe_path,deadline):
    violations,unresolved,used,unique,categories = [],[],Counter(),Counter(),Counter()
    actor = rules['classes'][str(context['class_id'])]; enums = rules['enums']
    professions = {enums['Profession'][p] for p in context['professions']}
    faction = rules['race_factions'][str(enums['Race'][context['race']])]
    def reason(code,slot):
        violations.append(dict(code=code,slot=slot))
    def requirements(item,slot):
        allowed = item.get('classAllowlist',[]); profession = item.get('requiredProfession',0)
        if any(type(c) is not int or c not in enums['Class'].values() or not c for c in allowed) or profession not in enums['Profession'].values():
            reason('unknown_reference_enum',slot)
        if allowed and context['class_id'] not in allowed:
            reason('class_restriction',slot)
        if profession and profession not in professions:
            reason('profession_required',slot)
    for slot,spec in enumerate(equipment['items']):
        if not spec:
            continue
        used[wire.digest(spec)]+=1; item = items.get(spec['id'])
        if item is None:
            reason('unknown_item',slot); continue
        requirements(item,slot)
        if item.get('reqLvl',0)>context['level']:
            reason('level_required',slot)
        restriction = item.get('factionRestriction',0)
        if restriction not in (0,1,2) or restriction and restriction!=faction:
            reason('faction_restriction',slot)
        kind,hand,weapon,ranged,armor = (item.get(k,0) for k in ('type','handType','weaponType','rangedWeaponType','armorType'))
        if any(type(value) is not int or value not in enums[name].values() for name,value in
               (('ItemType',kind),('HandType',hand),('WeaponType',weapon),('RangedWeaponType',ranged),('ArmorType',armor))):
            reason('unknown_reference_enum',slot); continue
        slots = rules['hand_slots'].get(str(hand),[]) if kind==13 else rules['slots'].get(str(kind),[])
        if slot not in slots:
            reason('slot_fit',slot)
        if kind==13:
            if str(weapon) not in actor['weapons'] or hand==4 and not actor['weapons'].get(str(weapon),False):
                reason('weapon_type',slot)
            if slot==15 and weapon not in (5,7) and context['specialization'] not in rules['dual_wield']:
                reason('dual_wield',slot)
        elif kind==14 and ranged not in actor['ranged']:
            reason('ranged_type',slot)
        elif kind not in (11,12,14) and armor>max(actor['armor']):
            reason('armor_type',slot)
        if item.get('unique'):
            unique[item['id']]+=1
        if item.get('limitCategory',0):
            categories[item['limitCategory']]+=1
        if spec.get('randomSuffix',0) not in [0]+item.get('randomSuffixOptions',[]):
            reason('suffix_not_available',slot)
        if spec.get('enchant'):
            enchant = enchants.get(spec['enchant'])
            if enchant is None:
                reason('unknown_enchant',slot)
            else:
                requirements(enchant,slot)
    if any(count>copies[key] for key,count in used.items()):
        reason('declared_copies_exceeded',-1)
    if any(count>1 for count in unique.values()):
        reason('unique_item',-1)
    if any(count>1 for count in categories.values()):
        reason('limit_category',-1)
    main,off = (equipment['items'][s] for s in (14,15))
    if off and main and items.get(main['id'],{}).get('handType')==4:
        reason('two_hand_conflict',15)
    if not violations and any(i.get('enchant') or i.get('randomSuffix') for i in equipment['items']):
        failure = conversion(equipment,probe_path,deadline)
        if failure:
            unresolved.append(dict(code=failure))
    return dict(valid=not violations and not unresolved,status='rejected' if violations else 'unresolved' if unresolved else 'accepted',
                context=context,equipment=equipment,violations=violations,unresolved=unresolved,rules_sha256=RULES_SHA256,
                catalogue_sha256=PIN['sha256'],probe_sha256=PROBE_SHA256,reference_constraints_verified=not violations and not unresolved,**FLAGS)


def validate_gear_build(context,equipment,inventory,*,catalogue_path,probe_path):
    deadline = time.monotonic()+45
    context,rules,items,enchants,copies = prepare(context,inventory,catalogue_path)
    return evaluate(context,equipment_spec(equipment),rules,items,enchants,copies,probe_path,deadline)


def generate_gear_builds(context,base_equipment,inventory,choices,*,catalogue_path,probe_path,offset='0',limit=16,scan_limit=4096,expected_search_id=None):
    deadline = time.monotonic()+45
    context,rules,items,enchants,copies = prepare(context,inventory,catalogue_path); base = equipment_spec(base_equipment)
    if type(choices) is not list or len(choices)>17:
        raise ValueError('Expected bounded equipment domains')
    domains,seen,total = [],set(),1
    for choice in choices:
        wire.exact(choice,('slot','items')); slot,values = choice['slot'],choice['items']
        if type(slot) is not int or not 0<=slot<17 or slot in seen or type(values) is not list or not 1<=len(values)<=128:
            raise ValueError('Invalid slot domain')
        values = [item_spec(v) for v in values]
        if len({wire.digest(v) for v in values})!=len(values):
            raise ValueError('Duplicate item domain')
        seen.add(slot); domains.append(dict(slot=slot,items=values)); total*=len(values)
    search_id = wire.digest(dict(version=1,context=context,base=base,inventory=dict(sorted(copies.items())),choices=domains,
                                 rules_sha256=RULES_SHA256,catalogue_sha256=PIN['sha256'],probe_sha256=PROBE_SHA256))
    if expected_search_id is not None and wire.text(expected_search_id,wire.HASH_PATTERN,64)!=search_id:
        raise ValueError('Gear search changed')
    start = int(wire.text(offset,wire.OFFSET_PATTERN,80)); wire.integer(limit,128); wire.integer(scan_limit,4096)
    if start>total:
        raise ValueError('Offset outside equipment matrix')
    end,accepted,references,rejected,unknown = start,[],{},Counter(),Counter()
    counts = Counter()
    while end<min(start+scan_limit,total) and len(accepted)<limit and time.monotonic()<deadline:
        coordinate,selection = end,list(base['items'])
        for domain in reversed(domains):
            coordinate,digit = divmod(coordinate,len(domain['items'])); selection[domain['slot']] = domain['items'][digit]
        equipment = dict(items=selection,preserveSlots=True)
        result = evaluate(context,equipment,rules,items,enchants,copies,probe_path,deadline); counts[result['status']]+=1
        if result['valid']:
            entry = dict(category='gear',value=equipment); ref = wire.digest(entry); references[ref] = entry
            accepted.append(dict(ordinal_decimal=str(end),reference_sha256=ref))
        else:
            rejected.update({v['code'] for v in result['violations']}); unknown.update({v['code'] for v in result['unresolved']})
        end+=1
    return dict(search_id=search_id,context=context,rules_sha256=RULES_SHA256,catalogue_sha256=PIN['sha256'],probe_sha256=PROBE_SHA256,
                total_decimal=str(total),start_decimal=str(start),end_decimal=str(end),next_offset=str(end) if end<total else None,
                examined_count=end-start,accepted_count=counts['accepted'],rejected_count=counts['rejected'],unresolved_count=counts['unresolved'],
                rejection_reasons=dict(rejected),unresolved_reasons=dict(unknown),accepted=accepted,references=references,
                axis=dict(name='gear',category='gear',candidates=list(references)) if references else None,
                declared_matrix_complete=start==0 and end==total and not counts['unresolved'],full_gear_space_covered=False,
                accepted_reference_constraints_verified=True,**FLAGS)


def dispatch(name,arguments,catalogue_path,probe_path):
    required = ('context','equipment','inventory') if name=='validate_gear_build' else ('context','base_equipment','inventory','choices')
    optional = () if name=='validate_gear_build' else ('offset','limit','scan_limit','expected_search_id')
    if name not in ('validate_gear_build','generate_gear_builds') or type(arguments) is not dict or not set(required)<=arguments.keys() or set(arguments)-set(required)-set(optional):
        raise ValueError(ERROR)
    result = (validate_gear_build if name=='validate_gear_build' else generate_gear_builds)(**arguments,catalogue_path=catalogue_path,probe_path=probe_path)
    if len(wire.encoded(dict(content=[dict(type='text',text=wire.encoded(result))],isError=False)))>wire.MAX_BYTES-1024:
        raise ValueError(ERROR)
    return result


def schemas():
    def obj(properties,required):
        return dict(type='object',properties=properties,required=required,additionalProperties=False)
    spec = obj({k:dict(type='integer',minimum=1 if k=='id' else 0,maximum=2**31-1) for k in ('id','enchant','randomSuffix')},[])
    gear = obj(dict(items=dict(type='array',minItems=17,maxItems=17,items=spec),preserveSlots=dict(const=True)),['items','preserveSlots'])
    context = obj(dict(class_id=dict(type='integer'),specialization=dict(type='string'),race=dict(type='string'),level=dict(const=60),
                       professions=dict(type='array',maxItems=2,uniqueItems=True,items=dict(type='string'))),['class_id','specialization','race','level','professions'])
    common = dict(context=context,inventory=dict(type='array',maxItems=2048,items=obj(dict(item=spec,count=dict(type='integer',minimum=1,maximum=2**31-1)),['item','count'])))
    domain = obj(dict(slot=dict(type='integer',minimum=0,maximum=16),items=dict(type='array',minItems=1,maxItems=128,items=spec)),['slot','items'])
    return dict(validate_gear_build=obj(common|dict(equipment=gear),['context','equipment','inventory']),
                generate_gear_builds=obj(common|dict(base_equipment=gear,choices=dict(type='array',maxItems=17,items=domain),offset=dict(type='string',maxLength=80),
                    limit=dict(type='integer',minimum=1,maximum=128),scan_limit=dict(type='integer',minimum=1,maximum=4096),expected_search_id=dict(type='string',maxLength=64)),['context','base_equipment','inventory','choices']))


def serve(catalogue_path,probe_path):
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import Tool,ToolAnnotations,TextContent
    server = Server('Azeroth Offline Gear Build Generator')
    @server.list_tools()
    async def list_tools():
        return [Tool(name=name,inputSchema=schema,description='Offline reference constraints over declared inventory. No game action or native legality claim.',
                     annotations=ToolAnnotations(readOnlyHint=True,destructiveHint=False,idempotentHint=True,openWorldHint=False)) for name,schema in schemas().items()]
    @server.call_tool(validate_input=False)
    async def call_tool(name,arguments):
        try:
            return [TextContent(type='text',text=wire.encoded(dispatch(name,arguments,catalogue_path,probe_path)))]
        except (OSError,ValueError,TypeError,KeyError,OverflowError,RecursionError):
            raise ValueError(ERROR) from None
    async def run():
        async with stdio_server() as (read,write):
            await server.run(read,write,server.create_initialization_options())
    asyncio.run(run())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('validate','generate','serve')); parser.add_argument('--input')
    parser.add_argument('--catalogue',required=True); parser.add_argument('--probe',required=True); args = parser.parse_args()
    try:
        catalogue,probe = ordinary(absolute(args.catalogue)),ordinary(absolute(args.probe))
        if args.action=='serve':
            serve(catalogue,probe); return
        path = ordinary(absolute(args.input))
        if not path.is_file() or path.stat().st_nlink!=1:
            raise ValueError(ERROR)
        with path.open('rb') as stream:
            data = stream.read(wire.MAX_INPUT_BYTES+1)
        if len(data)>wire.MAX_INPUT_BYTES:
            raise ValueError(ERROR)
        print(wire.encoded(dispatch('validate_gear_build' if args.action=='validate' else 'generate_gear_builds',
                                   json.loads(data,object_pairs_hook=wire.unique_object),catalogue,probe)))
    except (OSError,ValueError,TypeError,KeyError,OverflowError,RecursionError):
        parser.exit(1,ERROR+'\n')


if __name__=='__main__':
    main()
