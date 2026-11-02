#!/usr/bin/env python3
"""Verify or install the pinned native DAPI generators (no root or Docker)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import urllib.request

PACKAGE = Path(__file__).resolve().parent.parent
CONFIG = PACKAGE / 'codegen.json'
BINARIES = ('protoc', 'protoc-gen-grpc-java', 'grpc_objective_c_plugin', 'grpc_python_plugin')


def verify(destination, config):
    if json.loads((destination / 'lock.json').read_text()) != config['toolchain']:
        raise ValueError(f'Client generator versions differ at {destination}')
    for binary in BINARIES:
        if not os.access(destination / 'bin' / binary, os.X_OK):
            raise ValueError(f'Missing client generator: {binary}')
    version = subprocess.check_output([str(destination / 'bin/protoc'), '--version'], text=True).strip()
    if version != 'libprotoc ' + config['toolchain']['versions']['protobuf']:
        raise ValueError(f'Wrong client compiler: {version}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install', action='store_true', help='Build a user-local toolchain when absent')
    args = parser.parse_args()
    config = json.loads(CONFIG.read_text())
    revision = config['recipe_revision']
    cache = Path(os.environ.get('XDG_CACHE_HOME', str(Path.home() / '.cache')))
    local = cache / 'dash/client-codegen' / f'{revision}-{platform.system()}-{platform.machine()}'
    explicit = os.environ.get('DAPI_GRPC_TOOLCHAIN')
    if explicit:
        destination = Path(explicit).resolve()
    elif Path('/opt/client-codegen').exists():
        destination = Path('/opt/client-codegen')
    else:
        destination = local
    if destination.exists():
        verify(destination, config)
    elif args.install and not explicit:
        with tempfile.TemporaryDirectory(prefix='client-codegen-recipe-') as tmp:
            recipe = Path(tmp)
            for name, expected in config['recipe_files'].items():
                url = f"https://raw.githubusercontent.com/{config['recipe_repository']}/{revision}/client-codegen/{name}"
                with urllib.request.urlopen(url, timeout=120) as response:
                    content = response.read()
                if hashlib.sha256(content).hexdigest() != expected:
                    raise ValueError(f'Recipe checksum mismatch: {name}')
                (recipe / name).write_bytes(content)
            if json.loads((recipe / 'lock.json').read_text()) != config['toolchain']:
                raise ValueError('Recipe and Platform client toolchain locks differ')
            subprocess.run([sys.executable, str(recipe / 'build.py'), str(destination)],
                           check=True, stdout=sys.stderr)
        verify(destination, config)
    else:
        raise ValueError('Native client generators are missing. Run '
                         '`python3 packages/dapi-grpc/scripts/setup-codegen.py --install` '
                         '(requires Python 3.12+, CMake and a C++ compiler), or provision the matching runner image.')
    print(destination)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f'Client codegen: {error}', file=sys.stderr)
        sys.exit(1)
