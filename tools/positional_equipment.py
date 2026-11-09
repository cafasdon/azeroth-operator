"""Explicit, qualified positional equipment wire contract; no engine execution here."""
CONTRACT = 'azeroth_equipment_preserve_slots_v1'
ENGINE_SHA256 = '833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983'
CAPABILITY = {'contract':CONTRACT,'equipment_field':'preserveSlots','required_item_count':17,
    'legacy_default':'auto_place','native_qualification_performed':False}


def positional(spec):
    if not isinstance(spec, dict) or 'preserve_slots' in spec:
        raise ValueError('Invalid simulator equipment layout')
    if 'preserveSlots' in spec and spec['preserveSlots'] is not True:
        raise ValueError('Invalid simulator equipment layout')
    return spec.get('preserveSlots') is True


def request_requires_positional(request):
    return any(positional(player['equipment']) for party in request.get('raid',{}).get('parties',[])
        for player in party.get('players',[]) if player.get('equipment') is not None)
