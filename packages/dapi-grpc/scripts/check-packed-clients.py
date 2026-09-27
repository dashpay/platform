#!/usr/bin/env python3
"""Check that packing preserved the freshly generated DAPI clients."""
import json
from pathlib import Path
import sys
import tarfile

package = Path(__file__).resolve().parent.parent
expected = {p.relative_to(package).as_posix(): p.read_bytes()
            for p in (package / 'clients').rglob('*')
            if p.is_file() and p.relative_to(package).parts[3] in ('web', 'nodejs')}
found = 0
for archive in Path(sys.argv[1]).glob('*.tgz'):
    with tarfile.open(archive) as tar:
        manifest = tar.extractfile('package/package.json')
        if manifest is None or json.load(manifest)['name'] != '@dashevo/dapi-grpc':
            continue
        found += 1
        for name, content in expected.items():
            member = tar.extractfile('package/' + name)
            if member is None or member.read() != content:
                raise SystemExit(f'Packed client differs from generated source: {name}')
if found != 1:
    raise SystemExit(f'Expected one DAPI package archive, found {found}')
print(f'Verified {len(expected)} packed client files')
