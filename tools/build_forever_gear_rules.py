"""Reproduce offline gear constraints from the authenticated local Forever source ZIP."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile
from tools.build_forever_talent_map import REVISION, SOURCE_SHA
from tools.joint_build_optimizer import request_type


def build(source):
    with source.open('rb') as stream:
        if hashlib.file_digest(stream,'sha256').hexdigest() != SOURCE_SHA:
            raise ValueError('Pinned source ZIP checksum mismatch')
    pool = request_type().DESCRIPTOR.file.pool
    enums = {kind:{v.name:v.number for v in pool.FindEnumTypeByName('proto.'+kind).values}
             for kind in ('Class','ArmorType','WeaponType','RangedWeaponType','Race','Spec','Profession','ItemType','ItemSlot','HandType')}
    result = dict(revision=REVISION,source_sha256=SOURCE_SHA,source_hashes={},enums=enums,classes={},slots={},race_factions={})
    with zipfile.ZipFile(source) as archive:
        def read(path):
            info = archive.getinfo(f'forever-{REVISION}/{path}')
            if info.file_size > 1024*1024:
                raise ValueError('Oversized source member')
            data = archive.read(info); result['source_hashes'][path] = hashlib.sha256(data).hexdigest()
            return data.decode()
        capabilities = read('ui/sim/player/classes/capabilities_auto_gen.ts')
        def number(token):
            kind, name = token.split('.')
            return enums[kind][name]
        for table,key in (('CLASS_ARMOR_TYPES','armor'),('CLASS_RANGED_WEAPON_TYPES','ranged'),('CLASS_RACES','races'),('CLASS_WEAPON_TYPES','weapons')):
            body = capabilities.split('export const '+table+':',1)[1].split('\n};',1)[0]
            rows = re.findall(r'\[(Class\.\w+)\]: \[(.*?)\],',body)
            if len(rows) != 9:
                raise ValueError('Unsupported class capabilities')
            for actor, values in rows:
                parsed = ({str(number(t)):b=='true' for t,b in re.findall(r'weaponType: (WeaponType\.\w+), canUseTwoHand: (true|false)',values)}
                          if key=='weapons' else [number(t) for t in re.findall(r'\w+\.\w+',values)])
                result['classes'].setdefault(str(number(actor)),{})[key] = parsed
        result['dual_wield'] = [t[4].lower()+t[5:] for t in re.findall(r'Spec\.(Spec\w+)',capabilities.split('export const SPEC_CAN_DUAL_WIELD',1)[1])]
        fit = read('sim/core/itemfit/itemfit.go')
        for kind,values in re.findall(r'proto.ItemType_(\w+):\s*\{([^}]+)\}',fit.split('func ItemSlots',1)[0]):
            result['slots'][str(enums['ItemType'][kind])] = [enums['ItemSlot'][s] for s in re.findall(r'proto.ItemSlot_(\w+)',values)]
        result['hand_slots'] = {}
        for hands,values in re.findall(r'case (.*?):\s*return \[\]proto.ItemSlot\{(.*?)\}',fit.split('func ItemSlots',1)[1].split('// The slots an enchant',1)[0]):
            for hand in re.findall(r'proto.HandType_(\w+)',hands):
                result['hand_slots'][str(enums['HandType'][hand])] = [enums['ItemSlot'][s] for s in re.findall(r'proto.ItemSlot_(\w+)',values)]
        races = read('tools/database/dbc/enums.go')
        for faction,name in ((1,'AlliedRaces'),(2,'HordeRaces')):
            names = re.search(name+r'\s+Race = ([^\n]+)',races).group(1).split(' | ')
            result['race_factions'].update({str(enums['Race']['Race'+name]):faction for name in names if 'Race'+name in enums['Race']})
        read('ui/sim/proto/items.ts'); read('sim/core/bulk/item_rules.go')
    data = (json.dumps(result,sort_keys=True,separators=(',',':'))+'\n').encode()
    if len(data)>128*1024 or len(result['slots'])!=13 or len(result['hand_slots'])!=4:
        raise ValueError('Unsupported gear rules')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-zip',type=Path,required=True); parser.add_argument('--check',type=Path,required=True)
    args = parser.parse_args(); data = build(args.source_zip)
    if args.check.read_bytes()!=data:
        parser.exit(1,'Bundled gear rules differ from pinned source.\n')
    print(json.dumps(dict(status='matches',sha256=hashlib.sha256(data).hexdigest())))


if __name__=='__main__':
    main()
