"""Owner-bound, saved-capture gear and talent experiments; never control the game."""
import argparse
import asyncio
from collections import Counter, OrderedDict
from copy import deepcopy

from tools import build_search as wire, gear_builds as gear, talent_builds as talents
from tools import joint_build_optimizer as joint, positional_equipment as layout
from tools import run_forever_sim as adapter
from tools.owned_build_source import OwnedBuildSource, read_json, saved_call
from tools.sync_exports import absolute, ordinary

ERROR = 'Owned build request unavailable, changed, unsupported or over budget'
STORAGE = ('equipment', 'bags', 'character_bank')
SLOTS = ('head', 'neck', 'shoulder', 'back', 'chest', 'wrist', 'hands', 'waist',
         'legs', 'feet', 'finger1', 'finger2', 'trinket1', 'trinket2', 'main_hand', 'off_hand', 'ranged')
FLAGS = dict(live=False, active_character_verified=False, current_ownership_verified=False,
             native_legality_verified=False, simulation_model_verified=False, global_optimum_verified=False,
             all_owned_items_covered=False, game_execution=False, snapshot_reference_verified=True,
             exact_snapshot_binding_verified=True)


def bounded(value, maximum=wire.MAX_INPUT_BYTES):
    if len(wire.encoded(value).encode('ascii')) > maximum:
        raise ValueError(ERROR)
    return deepcopy(value)


def window(offset, limit):
    wire.text(offset, wire.OFFSET_PATTERN, 80)
    wire.integer(limit, 64)


def native_names(original, canonical):
    """Keep explicit proto3 defaults while adopting canonical native enum names."""
    if isinstance(original, dict) and isinstance(canonical, dict):
        return {key: native_names(value, canonical[key]) if key in canonical else value
                for key, value in original.items()}
    if isinstance(original, list) and isinstance(canonical, list):
        return [native_names(a,b) for a,b in zip(original,canonical,strict=True)]
    return canonical


def locations(snapshot):
    rows, coverage = {}, {}
    for storage in STORAGE:
        section = snapshot['sections'].get(storage)
        coverage[storage] = dict(status='unavailable' if section is None else 'observed',
                                 observed_locations=0, known_physical_count=0, unknown_count_locations=0)
        groups = {None: section} if storage == 'equipment' else section
        for container, slots in (groups or {}).items():
            for slot, item in (slots or {}).items():
                count = item.get('count', 1 if storage == 'equipment' else None)
                if type(count) is not int or not 1 <= count <= 2**31-1:
                    count = None
                rows[(storage, container, slot)] = (item, count)
                coverage[storage]['observed_locations'] += 1
                coverage[storage]['known_physical_count'] += count or 0
                coverage[storage]['unknown_count_locations'] += count is None
    return rows, coverage


class OwnedBuildOptimizer:
    def __init__(self, source, catalogue, probe, engine, cache=None):
        self.source = source
        self.catalogue, self.probe, self.engine = (ordinary(absolute(str(p))) for p in (catalogue, probe, engine))
        self.cache, self._cases = cache, OrderedDict()

    def _binding(self, selected):
        return wire.digest(dict(snapshot_id=selected['snapshot_id'], snapshot=selected['snapshot'],
                                selection_basis=selected['selection_basis'], config=self.source.config,
                                paths={k: str(getattr(self.source, k, '')) for k in ('reader', 'root', 'export', 'database')}))

    def _remember(self, report, manifest, selected, character, binding):
        if self._binding(selected) != binding:
            raise ValueError(ERROR)
        if manifest is None:
            return bounded(report, wire.MAX_BYTES-1024)
        record = bounded(dict(report=report, manifest=manifest, binding=binding,
                              character=character), 4*1024*1024)
        key = report['search_id']
        self._cases.pop(key, None)
        self._cases[key] = record
        while len(self._cases) > 8 or sum(len(wire.encoded(v)) for v in self._cases.values()) > 4*1024*1024:
            self._cases.popitem(last=False)
        return bounded(report, wire.MAX_BYTES-1024)

    async def _inventory(self, session, selected, raw):
        copies, variants, slots, reasons, seen = Counter(), {}, {}, Counter(), set()
        for storage in STORAGE:
            expected = {key for key in raw if key[0] == storage}
            if not expected:
                continue
            offset = 0
            while True:
                page = await saved_call(session, 'review_forever_owned_gear', dict(
                    snapshot_id=selected['snapshot_id'], storage=storage, offset=offset, limit=20))
                if page['snapshot_id'] != selected['snapshot_id'] or page['filtered_item_locations'] != len(expected):
                    raise ValueError(ERROR)
                entries = page['items']
                if not entries or len(entries) > 20:
                    raise ValueError(ERROR)
                for row in entries:
                    key = (row['storage'], row['container'], row['slot'])
                    if key not in expected or key in seen or row['observed'] != raw[key][0]:
                        raise ValueError(ERROR)
                    seen.add(key)
                    candidates = row.get('catalogue_modifier_candidates') or {}
                    spec, count = candidates.get('simulator_item_spec'), raw[key][1]
                    if row.get('simulator_item_resolution') != 'resolved' or spec is None or count is None:
                        reasons['unknown_count' if count is None else row.get('simulator_item_resolution', 'unresolved')] += 1
                        continue
                    spec = gear.item_spec(spec)
                    if not spec or spec['id'] != row['observed']['item_id']:
                        raise ValueError(ERROR)
                    ref = wire.digest(spec)
                    variants[ref] = spec
                    copies[ref] += count
                    slots[ref] = row['catalogue_base_item']['simulator_slots']
                offset += len(entries)
                if page['has_more'] is not (offset < len(expected)):
                    raise ValueError(ERROR)
                if not page['has_more']:
                    break
        if seen != set(raw):
            raise ValueError(ERROR)
        return [dict(item=variants[k], count=copies[k]) for k in sorted(copies)], slots, dict(reasons)

    async def prepare(self, character=None, model=None, gear_choices=None, talent_choices=None, offset='0', limit=16):
        window(offset, limit)
        arguments = bounded(dict(character=character, model=model, gear_choices=gear_choices, talent_choices=talent_choices))
        character, model, gear_choices, talent_choices = (arguments[k] for k in arguments)
        selected = deepcopy(self.source.select(character))
        binding = self._binding(selected)
        snapshot = selected['snapshot']
        raw, coverage = locations(snapshot)
        stats = snapshot['sections'].get('stats', {})
        reasons = []
        if type(stats.get('level')) is not int or stats['level'] != 60:
            reasons.append('unsupported_character_level')
        if model is None:
            reasons.append('missing_simulation_model')
        missing = sorted(set(('equipment', 'stats', 'professions', 'talents')) - snapshot['sections'].keys())
        if missing:
            reasons.append('missing_capture_sections')
        if len(raw) > 2048:
            reasons.append('inventory_location_budget')
        report = dict(status='plan_only', plan_only=True, blocked_reasons=reasons, missing_capture_sections=missing,
                      search_id=wire.digest(dict(binding=binding, arguments=arguments)),
                      snapshot_id=selected['snapshot_id'], character_level=stats.get('level'),
                      selection=dict(character=snapshot['character'], client=snapshot['client'],
                                     selection_basis=selected['selection_basis'], age_seconds=selected['age_seconds'],
                                     captured_at=snapshot['captured_at']), storage_coverage=coverage,
                      excluded_storage=['account_bank', 'mail_attachments', 'historical_banks', 'other_characters'],
                      observed_item_locations=len(raw), simulation_performed=False, objective_evaluated=False,
                      total_decimal='0', accepted_reference_constraints_verified=False, **FLAGS)
        if reasons:
            return self._remember(report, None, selected, character, binding)
        wire.exact(model, ('raid', 'encounter', 'simOptions', 'type'))
        message_type = joint.request_type()
        joint.check_message(message_type.DESCRIPTOR, model)
        model = native_names(model, joint.json_format.MessageToDict(joint.json_format.ParseDict(model, message_type())))
        if model['type'] != 'SimTypeIndividual':
            raise ValueError(ERROR)
        async with asyncio.timeout(120):
            async with self.source.session(self.catalogue) as session:
                baseline = await saved_call(session, 'review_equipment_selection', dict(
                    snapshot_id=selected['snapshot_id'], selections=[], positional_simulator_equipment=True))
                if (baseline['character'] != snapshot['character'] or baseline['client'] != snapshot['client']
                        or baseline.get('captured_character_level') != stats['level']):
                    raise ValueError(ERROR)
                inventory, slots, unresolved = await self._inventory(session, selected, raw)
        report.update(resolved_variant_count=len(inventory), resolved_physical_count=sum(v['count'] for v in inventory),
                      unresolved_item_locations=sum(unresolved.values()), unresolved_reasons=unresolved)
        if baseline.get('checks', {}).get('simulator_equipment_spec') is None:
            reasons.append('unresolved_captured_equipment')
            return self._remember(report, None, selected, character, binding)
        player = adapter.single_player(model['raid'])
        identity = adapter.check_character_identity(player, baseline.get('captured_character_identity'))
        profession = adapter.check_profession_identity(player, baseline.get('captured_professions'))
        spec = adapter.player_spec(player)
        class_name = adapter.SPEC_CLASSES[spec]
        talent = adapter.check_talent_identity(player, baseline.get('captured_talent_observation'), class_name)
        if set(identity.values()) != {'matches'} or profession['status'] != 'matches':
            reasons.append('unresolved_character_identity_or_professions')
        if (talent.get('basis') != 'captured_queried_definition_allocations_and_pinned_definitions'
                or talent.get('missing_pinned_definition_count') != 0 or talent.get('unmapped_definition_count') != 0
                or talent['rank_limit_disagreement_count'] or talent['unknown_node_count']
                or talent.get('staged_changes_observed') is not False or talent['conflicts']):
            reasons.append('incomplete_captured_talent_allocation')
        if reasons:
            return self._remember(report, None, selected, character, binding)
        request = adapter.request_from(model, baseline, dict(family='forever', section='encounters',
                                       simulation_input=dict(encounter=model['encounter'], simOptions=model['simOptions'])))
        request = native_names(request, joint.canonical_request(request, dict(class_id=adapter.CLASS_IDS[class_name],specialization=spec), joint.request_type()))
        player = adapter.single_player(request['raid'])
        names = {number: name for number, name in adapter.PROFESSIONS.values()}
        context = dict(class_id=adapter.CLASS_IDS[class_name], specialization=spec, race=player['race'], level=60,
                       professions=[names[n] for n in profession['captured_profession_ids']])
        base = gear.equipment_spec(player['equipment'])
        if gear_choices is None:
            gear_choices = []
            for slot, name in enumerate(SLOTS):
                values = {wire.digest(base['items'][slot]): base['items'][slot]}
                for row in inventory:
                    key = wire.digest(row['item'])
                    if name in slots[key]:
                        values[key] = row['item']
                gear_choices.append(dict(slot=slot, items=list(values.values())))
        gears = gear.generate_gear_builds(context, base, inventory, gear_choices, catalogue_path=self.catalogue,
                                           probe_path=self.probe, limit=128, scan_limit=4096)
        trees = talents.generate_talent_builds(class_name, 60, player['talentsString'],
                                               [] if talent_choices is None else talent_choices, limit=128, scan_limit=4096)
        report['generation'] = {name: {k: result[k] for k in ('total_decimal', 'examined_count', 'accepted_count',
                                   'rejected_count', 'next_offset', 'declared_matrix_complete', 'rejection_reasons')}
                                for name, result in (('gear', gears), ('talents', trees))}
        report['generation']['gear'].update(unresolved_count=gears['unresolved_count'], unresolved_reasons=gears['unresolved_reasons'])
        if not gears['axis'] or not trees['axis']:
            reasons.append('no_accepted_builds')
            return self._remember(report, None, selected, character, binding)
        case = dict(snapshot_id=selected['snapshot_id'], client=snapshot['client'], character_level=stats['level'],
                    class_id=context['class_id'], specialization=spec, objective='dps', engine_sha256=layout.ENGINE_SHA256,
                    data_manifest_sha256=joint.CATALOGUE_SHA256, encounter_sha256=wire.digest(request['encounter']),
                    constraints_sha256=wire.digest(dict(source=binding, inventory=inventory,
                        context=context, gear_rules=gear.RULES_SHA256, talent_rules=talents.RULES_SHA256,
                        gear_search=gears['search_id'], talent_search=trees['search_id'])))
        manifest = dict(case=case, axes=[gears['axis'], trees['axis']], base_request=request,
                        references=gears['references'] | trees['references'])
        plan, _, _, _ = joint.preflight(manifest, window_mode=True, offset=offset, limit=limit)
        report.update(status='ready', plan_only=True, search_id=plan['search_id'], total_decimal=plan['total_decimal'],
                      start_decimal=plan['start_decimal'], end_decimal=plan['end_decimal'], next_offset=plan['next_offset'],
                      axes=manifest['axes'], accepted_reference_hashes=sorted(manifest['references']),
                      constraints_sha256=case['constraints_sha256'], accepted_reference_constraints_verified=True)
        if self._binding(self.source.select(character)) != binding:
            raise ValueError(ERROR)
        return self._remember(report, manifest, selected, character, binding)

    async def evaluate(self, expected_search_id, character=None, offset='0', limit=16):
        wire.text(expected_search_id, wire.HASH_PATTERN, 64)
        window(offset, limit)
        record = self._cases.get(expected_search_id)
        if record is None or character != record['character']:
            raise ValueError(ERROR)
        selected = self.source.select(character)
        if self._binding(selected) != record['binding']:
            raise ValueError(ERROR)
        self._cases.move_to_end(expected_search_id)
        if record['manifest'] is None:
            if offset != '0':
                raise ValueError(ERROR)
            return bounded(record['report'], wire.MAX_BYTES-1024)
        result = joint.evaluate_joint_build_window(record['manifest'], str(self.engine), offset=offset, limit=limit,
                                                   expected_search_id=expected_search_id, cache=self.cache)
        result.update(selection=record['report']['selection'], storage_coverage=record['report']['storage_coverage'],
                      accepted_reference_constraints_verified=True, plan_only=False, **FLAGS)
        result['coverage'] = 'Only this accepted saved-capture matrix window was evaluated; no live ownership or global optimum claim.'
        return bounded(result, wire.MAX_BYTES-1024)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('prepare',))
    for name in ('source', 'catalogue', 'probe', 'engine', 'input'):
        parser.add_argument('--'+name, required=True)
    args = parser.parse_args()
    try:
        source = OwnedBuildSource(read_json(args.source, 64*1024))
        request = read_json(args.input, wire.MAX_INPUT_BYTES)
        if type(request) is not dict or set(request)-{'character', 'model', 'gear_choices', 'talent_choices', 'offset', 'limit'}:
            raise ValueError(ERROR)
        optimizer = OwnedBuildOptimizer(source, args.catalogue, args.probe, args.engine)
        print(wire.encoded(asyncio.run(optimizer.prepare(**request))))
    except Exception:
        parser.exit(1, ERROR+'\n')


if __name__ == '__main__':
    main()
