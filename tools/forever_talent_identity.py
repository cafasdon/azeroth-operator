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
    if set(observed) - {'config_id', 'nodes', 'observed_tree_ids', 'observed_node_count', 'staged_changes_observed'}:
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
    report['unknown_node_count'] = unknown + len(definitions.keys() - mapped.keys())
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
