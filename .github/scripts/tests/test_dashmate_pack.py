"""Keep the Dashmate package's own resolutions when it is packed from npm tarballs."""
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SKIP = 'https://registry.yarnpkg.com/@favware/skip-dependency/-/skip-dependency-1.2.1.tgz'


def tarball_resolutions_script():
    script = (ROOT / 'scripts/pack_dashmate.sh').read_text()
    start = script.index('if [ -n "${DASHMATE_NPM_TARBALLS_DIR:-}" ]; then\n')
    return script[start:script.index('\nfi\n', start) + len('\nfi\n')]


class PackDashmateTests(unittest.TestCase):
    # cpu-features compiles a native addon, and Apple does not notarize a
    # macOS package that holds one.
    def test_should_skip_cpu_features_in_the_dashmate_package(self):
        manifest = json.loads((ROOT / 'packages/dashmate/package.json').read_text())
        self.assertEqual(manifest['resolutions']['cpu-features'], SKIP)

    def test_should_keep_the_package_resolutions_when_packing_from_tarballs(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        package = tmp / 'package'
        tarballs = tmp / 'npm-packages'
        package.mkdir()
        tarballs.mkdir()
        (package / 'package.json').write_text(json.dumps({'name': 'dashmate', 'resolutions': {'cpu-features': SKIP}}))
        for name in ['dashmate', '@dashevo/wallet-lib']:
            manifest = json.dumps({'name': name}).encode()
            with tarfile.open(tarballs / (name.replace('/', '-') + '-1.0.0.tgz'), 'w:gz') as archive:
                info = tarfile.TarInfo('package/package.json')
                info.size = len(manifest)
                archive.addfile(info, io.BytesIO(manifest))

        subprocess.run(['bash', '-c', tarball_resolutions_script()], check=True, cwd=package,
                       env=dict(os.environ, DASHMATE_NPM_TARBALLS_DIR=str(tarballs)))

        wallet_lib = (tarballs / '@dashevo-wallet-lib-1.0.0.tgz').resolve()
        self.assertEqual(json.loads((package / 'package.json').read_text())['resolutions'],
                         {'cpu-features': SKIP, '@dashevo/wallet-lib': f'file:{wallet_lib}'})


if __name__ == '__main__':
    unittest.main()
