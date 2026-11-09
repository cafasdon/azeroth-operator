"""Build an opt-in exact-slot simulator from pinned local archives, without downloads."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import sys
from time import monotonic
import zipfile

from tools.sync_exports import ordinary

REVISION = '9dc32ac8ca0d8d1b76337861d38b93c0f884af11'
SOURCE_SHA = 'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712'
GO_SHA = 'f26081f438319757f8a1b46d7d0b2c5ec7583d4038e485e0aa725cfde7885fff'
PROTOC_SHA = '5a1b5350308309c9729ce4484a798981f281c10909144c4bdefd6a37688e4b1f'
PLUGIN_SHA = '974ecc9f65c335445167d6e4033f1e89f24b9d455511f5dae07b916ed6a47bb0'
FRAGMENTS = Path(__file__).with_name('forever_positional_engine')
CONTRACT = 'azeroth_equipment_preserve_slots_v1'


def require(value):
    if not value: raise ValueError('Invalid offline engine build input')


def sha(path):
    with path.open('rb') as handle: return hashlib.file_digest(handle, 'sha256').hexdigest()


def extract(archive, target, prefix, maximum):
    with zipfile.ZipFile(archive) as bundle:
        require(sum(row.file_size for row in bundle.infolist()) <= maximum)
        seen = set()
        for row in bundle.infolist():
            name = PurePosixPath(row.orig_filename)
            require(not name.is_absolute() and '\\' not in row.orig_filename and ':' not in row.orig_filename
                and '..' not in name.parts and name.parts and name.parts[0] == prefix
                and not stat.S_ISLNK(row.external_attr >> 16))
            destination = target.joinpath(*name.parts)
            require(destination.resolve().is_relative_to(target.resolve()))
            # Case aliases and duplicate archive members are never written twice.
            identity = str(destination).casefold()
            require(identity not in seen); seen.add(identity)
            if row.is_dir(): destination.mkdir(parents=True, exist_ok=True)
            else:
                destination.parent.mkdir(parents=True, exist_ok=True)
                with bundle.open(row) as source, destination.open('xb') as output:
                    while block := source.read(1024 * 1024): output.write(block)


def patch_source(source):
    database = source/'sim/core/database.go'
    original = 'func ProtoToEquipment(es *proto.EquipmentSpec) Equipment {\n\treturn NewEquipmentSet(ProtoToEquipmentSpec(es))\n}'
    text = database.read_text(encoding='utf-8'); require(text.count(original) == 1)
    database.write_text(text.replace(original, (FRAGMENTS/'constructor.go.txt').read_text(encoding='utf-8').rstrip()), encoding='utf-8')
    common = source/'proto/common.proto'
    original = 'message EquipmentSpec {\n\trepeated ItemSpec items = 1;\n}'
    text = common.read_text(encoding='utf-8'); require(text.count(original) == 1)
    common.write_text(text.replace(original, 'message EquipmentSpec {\n\trepeated ItemSpec items = 1;\n\tbool preserve_slots = 2;\n}'), encoding='utf-8')
    probe = source/'cmd/azeroth_equipment_probe/main.go'; probe.parent.mkdir()
    probe.write_bytes((FRAGMENTS/'probe.go.txt').read_bytes())
    capability = source/'cmd/wowsimcli/cmd/azeroth_equipment_contract.go'
    require(not capability.exists())
    capability.write_bytes((FRAGMENTS/'capability.go.txt').read_bytes())


def run_bounded(command, source, env, log, deadline):
    require(monotonic() < deadline)
    with log.open('ab') as output:
        process = subprocess.Popen(command, cwd=source, env=env, stdin=subprocess.DEVNULL,
            stdout=output, stderr=subprocess.STDOUT,
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP | subprocess.CREATE_NO_WINDOW)
        try:
            require(process.wait(timeout=deadline-monotonic()) == 0)
        except BaseException:
            if process.poll() is None:
                subprocess.run(['taskkill','/PID',str(process.pid),'/T','/F'], timeout=15,
                    stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=False)
            process.wait(timeout=15)
            raise


def build_environment(work, module_cache, cache):
    return {**os.environ, 'GOROOT':str(work/'go'), 'GOTOOLCHAIN':'local', 'GOPROXY':'off',
        'GOSUMDB':'off', 'CGO_ENABLED':'0', 'GOMODCACHE':str(module_cache),
        'GOPATH':str(work/'gopath'), 'GOCACHE':str(cache or work/'cache'),
        'GOENV':'off', 'GOWORK':'off', 'GO111MODULE':'on', 'GOTELEMETRY':'off',
        'GONOPROXY':'none', 'GONOSUMDB':'none', 'GOPRIVATE':'', 'GOVCS':'*:off',
        'GOOS':'windows', 'GOARCH':'amd64', 'GOAMD64':'v1', 'GOFLAGS':'',
        'GOEXPERIMENT':'', 'GOCACHEPROG':'', 'GODEBUG':''}


def build(args):
    require(sys.platform == 'win32')
    require(type(args.time_budget_seconds) is int and 1 <= args.time_budget_seconds <= 1800)
    for field, expected in [('source_zip', SOURCE_SHA), ('go_zip', GO_SHA), ('protoc', PROTOC_SHA), ('protoc_gen_go', PLUGIN_SHA)]:
        path = ordinary(getattr(args, field).absolute())
        require(path.is_file() and path.stat().st_size <= 100 * 1024 * 1024 and sha(path) == expected)
        setattr(args, field, path)
    args.module_cache = ordinary(args.module_cache.absolute()); require(args.module_cache.is_dir())
    cache = ordinary(args.build_cache.absolute()) if args.build_cache else None
    if cache is not None: require(cache.is_dir())
    work = ordinary(args.work_dir.absolute())
    require(work.parent.is_dir() and not work.exists())
    require(all(not path.is_relative_to(work) for path in [args.source_zip, args.go_zip, args.protoc, args.protoc_gen_go, args.module_cache]))
    deadline = monotonic() + args.time_budget_seconds
    work.mkdir(mode=0o700)
    extract(args.source_zip, work, 'forever-'+REVISION, 64*1024*1024)
    extract(args.go_zip, work, 'go', 300*1024*1024)
    source = work/('forever-'+REVISION); patch_source(source)
    artifacts = work/'artifacts'; artifacts.mkdir()
    env = build_environment(work, args.module_cache, cache)
    log = work/'build.log'
    def run(command):
        run_bounded(command, source, env, log, deadline)
    protos = sorted((source/'proto').glob('*.proto')); require(len(protos) == 17)
    run([str(args.protoc), '-I=./proto', '-I='+str(args.protoc.parent.parent/'include'),
        '--plugin=protoc-gen-go='+str(args.protoc_gen_go),
        '--go_opt=Mgoogle/protobuf/descriptor.proto=google.golang.org/protobuf/types/descriptorpb',
        '--go_out=./sim/core', *[str(path.relative_to(source)) for path in protos]])
    go = work/'go/bin/go.exe'
    for target, name in [('./cmd/wowsimcli','wowsimcli-positional.exe'), ('./cmd/azeroth_equipment_probe','equipment-probe.exe')]:
        run([str(go),'build','-tags=with_db','-trimpath','-buildvcs=false','-ldflags=-buildid=',
            '-o',str(artifacts/name),target])
    (artifacts/'LICENSE-wowsims.txt').write_bytes((source/'LICENSE').read_bytes())
    manifest = {'contract':CONTRACT, 'upstream_revision':REVISION,'source_archive_sha256':SOURCE_SHA,
        'go_archive_sha256':GO_SHA,'protoc_sha256':PROTOC_SHA,'protobuf_plugin_sha256':PLUGIN_SHA,
        'engine_sha256':sha(artifacts/'wowsimcli-positional.exe'),'probe_sha256':sha(artifacts/'equipment-probe.exe'),
        'constructor_sha256':sha(FRAGMENTS/'constructor.go.txt'),'probe_source_sha256':sha(FRAGMENTS/'probe.go.txt'),
        'capability_source_sha256':sha(FRAGMENTS/'capability.go.txt'),'builder_sha256':sha(Path(__file__)),
        'default_auto_placement_unchanged':True,'qualification_performed':False,
        'native_qualification_performed':False,'game_execution':False}
    require(monotonic() <= deadline)
    (artifacts/'build-manifest.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n',encoding='utf-8')
    return manifest


def main(argv=None):
    cli=argparse.ArgumentParser(description=__doc__)
    for name in ['source-zip','go-zip','protoc','protoc-gen-go','module-cache','work-dir']:
        cli.add_argument('--'+name,type=Path,required=True)
    cli.add_argument('--time-budget-seconds',type=int,default=600)
    cli.add_argument('--build-cache',type=Path,help='Optional existing trusted Go build cache')
    try: print(json.dumps(build(cli.parse_args(argv)),sort_keys=True))
    except KeyboardInterrupt: cli.exit(130,'Offline engine build interrupted; partial work remains in its fresh directory.\n')
    except Exception: cli.exit(1,'Offline engine build rejected or stopped; fresh work directory contains any partial build log.\n')


if __name__=='__main__': main()
