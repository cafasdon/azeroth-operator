"""Optional parser qualification against pinned installed code, without game APIs."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from tools.sync_exports import ordinary
from tools.tomtom_route import prepare_route

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT/'clients/forever/tomtom-format-evidence.json'

# No third-party code is copied here. The installed module runs with no game,
# file, network, process, loader or native-module APIs in its Lua environment.
HARNESS = r'''
local installed, commands = arg[1], arg[2]
assert(tonumber("1.1") ~= nil, "This fixture qualifies the dot numeric locale")
local rows = {}
local function forbidden() error("Unexpected addon/game action") end
local addon = {
    L = setmetatable({}, {__index=function(_, key) return key end}),
    private = {RoundCoords=forbidden}, NameToMapId={}, waypoints={},
    hbd = {mapData={[37]={}, [52]={}}},
}
function addon.hbd:GetLocalizedMap(id) return "Synthetic map " .. id end
function addon:AddWaypoint(map, x, y, opts)
    assert(opts.from == "TomTom/way")
    assert(type(opts.title) == "string" and not opts.title:find("\n", 1, true))
    rows[#rows+1] = {map, x, y, opts.title}
end
setmetatable(addon, {__index=function(_, name) error("Unexpected addon method " .. name) end})
local env = {
    SlashCmdList={}, ChatFrame1={AddMessage=forbidden},
    pairs=pairs, ipairs=ipairs, tonumber=tonumber, tostring=tostring, type=type,
    select=select, unpack=unpack, assert=assert, error=error,
    table={concat=table.concat, insert=table.insert}, string=string,
}
env._G = env
setmetatable(env, {__index=function(_, name) error("Unavailable game/global " .. name) end})
local chunk = assert(loadfile(installed))
setfenv(chunk, env)
chunk("TomTom", addon)
for line in io.lines(commands) do
    assert(line:match("^/tway #%d+ "))
    -- This is the same prefix removal used by TomTom's paste window.
    addon.SlashWayCommand(line:gsub("^%S+", ""))
end
for _, row in ipairs(rows) do
    print(string.format("%d\t%.17g\t%.17g\t%s", row[1], row[2], row[3], row[4]))
end
'''


@unittest.skipUnless(os.environ.get('AZEROTH_TOMTOM_DIR'), 'Explicit installed TomTom path required; never downloads')
class InstalledTomTomParserTests(unittest.TestCase):
    def test_real_slash_module_accepts_numbered_drafts_in_isolated_environment(self):
        folder = ordinary(Path(os.environ['AZEROTH_TOMTOM_DIR']))
        self.assertTrue(folder.is_absolute())
        evidence = json.loads(EVIDENCE.read_bytes())
        for name, expected in evidence['source_files'].items():
            path = ordinary(folder/name)
            self.assertLessEqual(path.stat().st_size, 256*1024)
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), expected)
        lua = ordinary(ROOT/'local-data/world-reference/pinned-source/tools/lua-binary/lua.exe')
        pin = json.loads((ROOT/'clients/forever/world.lock.json').read_bytes())
        self.assertEqual(hashlib.sha256(lua.read_bytes()).hexdigest(), pin['source_files']['tools/lua-binary/lua.exe'])
        steps = [
            dict(ui_map_id=37, x_percent=0, y_percent=100, label='First stop'),
            dict(ui_map_id=37, x_percent=0, y_percent=100, label='First stop'),
            dict(ui_map_id=52, x_percent=50.1234567, y_percent=34.8, label='Next map'),
            dict(ui_map_id=37, x_percent=51.2, y_percent=34.8, label='/run forbidden(); reset all'),
        ]
        # Ordinary numeric punctuation is transformed by the real parser.
        steps[0]['label'] = 'Stop 1. 2 and 3.4'
        draft = prepare_route('Route 1,2', steps, mode='all_pins')
        with tempfile.TemporaryDirectory(prefix='tomtom-parser-', dir=ROOT/'local-data') as temp:
            base = Path(temp)
            script, text = base/'parser.lua', base/'commands.txt'
            script.write_bytes(HARNESS.encode())
            text.write_bytes((draft['manual_import_text']+'\n').encode())
            env = {key: os.environ[key] for key in ('SystemRoot',) if key in os.environ}
            result = subprocess.run([str(lua), str(script), str(folder/'modules/slash.lua'), str(text)],
                cwd=base, env=env, capture_output=True, text=True, encoding='utf-8', timeout=15, check=True)
        rows = [line.split('\t', 3) for line in result.stdout.splitlines()]
        self.assertEqual(len(rows), len(steps))
        for row, planned in zip(rows, draft['manifest']):
            self.assertEqual(int(row[0]), planned['ui_map_id'])
            self.assertAlmostEqual(float(row[1]), float(planned['rendered_x_percent'])/100, places=14)
            self.assertAlmostEqual(float(row[2]), float(planned['rendered_y_percent'])/100, places=14)
            self.assertEqual(row[3], planned.get('display_pin_label_variants', {}).get('dot', planned['pin_label']))
        self.assertEqual(len({row[3] for row in rows}), len(steps))
        self.assertFalse(draft['waypoint_creation_performed'])
        self.assertFalse(draft['arrow_order_enforced'])


if __name__ == '__main__':
    unittest.main()
