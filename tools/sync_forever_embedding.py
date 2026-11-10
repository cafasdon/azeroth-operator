"""Explicit download of the checksum-pinned public embedding model."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

from tools.forever_knowledge import MODEL_LOCK, file_digest
from tools.sync_exports import ordinary


def sync(destination):
    destination = ordinary(destination.absolute())
    destination.mkdir(parents=True, exist_ok=True)
    changed = 0
    for name, expected in MODEL_LOCK['files'].items():
        target = ordinary(destination / name)
        if target.exists() and file_digest(target) == expected:
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='.embedding-', dir=target.parent) as stage:
            download = Path(stage) / 'file'
            url = f"https://huggingface.co/{MODEL_LOCK['provider']}/resolve/{MODEL_LOCK['revision']}/{name}"
            subprocess.run(['curl', '--disable', '--fail', '--silent', '--show-error', '--location',
                '--proto', '=https', '--proto-redir', '=https', '--max-time', '120',
                '--max-filesize', str(128 * 1024 * 1024), '--output', str(download), url],
                check=True, timeout=125)
            if file_digest(download) != expected:
                raise ValueError('Embedding download checksum mismatch')
            os.replace(download, target)
            changed += 1
    return changed


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument('--output', type=Path, required=True)
    args = cli.parse_args()
    print(f'Pinned embedding files updated: {sync(args.output)}')


if __name__ == '__main__':
    main()
