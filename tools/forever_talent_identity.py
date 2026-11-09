"""Compare captured purchased ranks with the pinned local simulator's definitions.

This checks observed ranks only. Missing definitions and native tree completeness
do not become proof of a full allocation, talent legality, or model accuracy.
"""
import hashlib
import json
from pathlib import Path

MAP_SHA256 = '33dc07ad7ba838a4bb6f95d825df44235b0e1a00e92eb3dfdb9c68d8d96c18b3'
MAP_PATH = Path(__file__).resolve().parents[1] / 'clients/forever/talent-map.json'


class ConflictingTalentObservation(ValueError):
    """The saved native observation conflicts with the simulator template."""


def talent_map():
    with MAP_PATH.open('rb') as handle:
        data = handle.read(128 * 1024 + 1)
    if len(data) > 128 * 1024 or hashlib.sha256(data).hexdigest() != MAP_SHA256:
        raise ValueError('Pinned talent map checksum mismatch')
    return json.loads(data)


def _allocation_definitions(allocation, nodes, trees, count, integer):
    """Validate standalone review JSON independently of snapshot-store ingestion."""
    def key(value):
        return (isinstance(value,str) and value.isascii() and value.isdecimal()
                and str(int(value))==value and 1<=int(value)<=2**32-1)
    if (not isinstance(allocation,dict) or set(allocation)!={'coverage','queried_node_count','queried_entry_count','definitions'}
            or allocation['coverage']!='queried_config_tree_entries'
            or not integer(allocation['queried_node_count']) or allocation['queried_node_count']!=count
            or not integer(allocation['queried_entry_count']) or not isinstance(allocation['definitions'],dict)
            or allocation['queried_entry_count']!=len(allocation['definitions']) or len(allocation['definitions'])>4096):
        raise ValueError('Invalid queried talent allocation')
    definitions={};entries=set();node_trees={};node_counts={};locations={}
    for definition,row in allocation['definitions'].items():
        if (not key(definition) or not isinstance(row,dict)
                or set(row)!={'node_id','entry_id','tree_id','purchased_rank','max_rank'}
                or any(not integer(row[field],1) for field in ['node_id','entry_id','tree_id','max_rank'])
                or not integer(row['purchased_rank']) or row['purchased_rank']>row['max_rank']
                or str(row['tree_id']) not in trees or row['entry_id'] in entries
                or row['node_id'] in node_trees and node_trees[row['node_id']]!=row['tree_id']):
            raise ValueError('Invalid queried talent definition')
        entries.add(row['entry_id']);node_trees[row['node_id']]=row['tree_id']
        node_counts[row['node_id']]=node_counts.get(row['node_id'],0)+1
        if node_counts[row['node_id']]>64:raise ValueError('Queried talent node entry budget exceeded')
        if row['purchased_rank']>0:
            active=nodes.get(str(row['node_id']))
            if active is None or active['entry_id']!=row['entry_id'] or active['tree_id']!=row['tree_id']:
                raise ValueError('Queried purchased talent entry mismatch')
        definitions[int(definition)]=row
        locations[(row['node_id'],row['entry_id'])]=(int(definition),row)
    if len(node_trees)>count:raise ValueError('Queried talent node budget exceeded')
    for node,active in nodes.items():
        observed=locations.get((int(node),active['entry_id']))
        if (observed is None or observed[1]['tree_id']!=active['tree_id']
                or 'definition_id' in active and (active['definition_id']!=observed[0]
                    or any(active[field]!=observed[1][field] for field in ['purchased_rank','max_rank']))):
            raise ValueError('Queried active talent allocation mismatch')
    return definitions


def check_talent_identity(player, observed, class_name, require_match=True):
    report = {'status': 'unverified', 'basis': 'captured_purchased_ranks_and_pinned_definitions',
              'talent_map_sha256': MAP_SHA256, 'allocation_match_verified': False,
              'native_tree_coverage_verified': False, 'simulation_model_verified': False,
              'compared_definition_count': 0, 'unknown_node_count': 0,
              'rank_limit_disagreement_count': 0, 'conflicts': []}
    if observed is None:
        return report
    if not isinstance(observed, dict):
        raise ValueError('Invalid captured talent observation')
    # Equipment review projection contains only these non-personal fields.
    if set(observed) - {'config_id', 'nodes', 'observed_tree_ids', 'observed_node_count', 'staged_changes_observed','definition_allocation'}:
        raise ValueError('Invalid captured talent observation')
    def integer(value, lower=0):
        return type(value) is int and lower <= value <= 2**32 - 1
    nodes = observed.get('nodes'); trees = observed.get('observed_tree_ids')
    count = observed.get('observed_node_count')
    if (not integer(observed.get('config_id'), 1) or not isinstance(nodes, dict)
            or not integer(count) or not len(nodes) <= count <= 4096
            or not isinstance(trees, dict) or not 1 <= len(trees) <= 16
            or any(not isinstance(key, str) or not key.isascii() or not key.isdecimal() or str(int(key)) != key
                   or not 1 <= int(key) <= 2**32-1 or value is not True for key, value in trees.items())
            or ('staged_changes_observed' in observed and type(observed['staged_changes_observed']) is not bool)):
        raise ValueError('Invalid captured talent observation')
    definitions = {}; unknown = 0
    for key, node in nodes.items():
        if (not isinstance(key, str) or not key.isascii() or not key.isdecimal() or str(int(key)) != key
                or not 1 <= int(key) <= 2**32-1 or not isinstance(node, dict)
                or set(node) - {'entry_id', 'rank', 'tree_id', 'definition_id', 'purchased_rank', 'max_rank'}
                or any(not integer(node.get(field), 1) for field in ('entry_id', 'rank', 'tree_id'))
                or str(node['tree_id']) not in trees):
            raise ValueError('Invalid captured talent observation')
        fields = {'definition_id', 'purchased_rank', 'max_rank'} & node.keys()
        if not fields:
            unknown += 1
            continue
        if (len(fields) != 3 or not integer(node['definition_id'], 1) or not integer(node['max_rank'], 1)
                or not integer(node['purchased_rank']) or node['purchased_rank'] > node['max_rank']
                or node['definition_id'] in definitions):
            raise ValueError('Invalid or ambiguous captured talent definition')
        definitions[node['definition_id']] = node
    allocated=None
    if 'definition_allocation' in observed:
        allocated=_allocation_definitions(observed['definition_allocation'],nodes,trees,count,integer)
    source = talent_map()['classes'].get(class_name)
    if source is None:
        raise ValueError('Unsupported talent map class')
    value = player.get('talentsString')
    if ('talents_string' in player or not isinstance(value, str) or len(value) > 256
            or any(character not in '0123456789-' for character in value)):
        raise ValueError('Invalid simulator talent string')
    parts = value.split('-')
    if len(parts) > 3:
        raise ValueError('Invalid simulator talent string')
    parts += [''] * (3 - len(parts))
    ranks = []
    for part, size in zip(parts, source['tree_sizes']):
        if len(part) > size:
            raise ValueError('Simulator talent string exceeds pinned tree')
        ranks.extend(int(digit) for digit in part.ljust(size, '0'))
    mapped = {row['definition_id']: (min(rank, row['max_rank']), row['max_rank'])
              for row, rank in zip(source['talents'], ranks)}
    if allocated is None:
        report['unknown_node_count'] = unknown + len(definitions.keys() - mapped.keys())
    else:
        definitions=allocated
        report['basis']='captured_queried_definition_allocations_and_pinned_definitions'
        report['queried_definition_count']=len(definitions)
        report['unmapped_definition_count']=len(definitions.keys()-mapped.keys())
        report['missing_pinned_definition_count']=len(mapped.keys()-definitions.keys())
        report['unknown_node_count']=len({definitions[definition]['node_id'] for definition in definitions.keys()-mapped.keys()})
    staged = observed.get('staged_changes_observed')
    report['staged_changes_observed'] = staged
    for definition, node in sorted(definitions.items()):
        if definition not in mapped:
            continue
        rank, maximum = mapped[definition]
        if maximum != node['max_rank']:
            report['rank_limit_disagreement_count'] += 1
            continue
        report['compared_definition_count'] += 1
        if rank != node['purchased_rank']:
            report['conflicts'].append({'definition_id': definition, 'captured_purchased_rank': node['purchased_rank'],
                                        'template_effective_rank': rank})
    if staged is True:
        report['status'] = 'staged_changes'
    elif staged is False and report['conflicts']:
        report['status'] = 'mismatch'
    elif staged is False and report['compared_definition_count']:
        report['status'] = 'observed_ranks_match'
    if require_match and report['status'] in ('mismatch', 'staged_changes'):
        raise ConflictingTalentObservation(
            'Simulator talents conflict with saved purchased ranks or staged changes. Match the template to a committed capture or use --plan-only.')
    return report
