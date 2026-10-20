"""Repack oclif's Dashmate debs under Debian versions that apt orders by release."""
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]

# The control file oclif 4 writes for Dashmate, as shipped in v5.0.0-rc.5: Description is
# followed by Aliases.
CONTROL = '''Package: dashmate
Version: {version}
Section: main
Priority: standard
Architecture: {arch}
Maintainer: undefined
Description: Distribution package for Dash node installation
Aliases:
'''


def rewrite_function():
    script = (ROOT / 'scripts/pack_dashmate.sh').read_text()
    start = script.index('rewrite_deb_versions() {\n')
    return script[start:script.index('\n}\n', start) + len('\n}\n')]


def ar_members(deb):
    """Names of the members of a deb's outer ar archive, which name its compression."""
    data = deb.read_bytes()
    assert data.startswith(b'!<arch>\n')
    names, offset = [], 8
    while offset < len(data):
        header = data[offset:offset + 60]
        names.append(header[:16].decode().strip().rstrip('/'))
        size = int(header[48:58].decode().strip())
        offset += 60 + size + size % 2
    return names


def payload(deb):
    """What dpkg installs: every entry's path, type, mode, owner, size and link target."""
    tar = subprocess.run(['dpkg-deb', '--fsys-tarfile', str(deb)], check=True, capture_output=True).stdout
    with tarfile.open(fileobj=io.BytesIO(tar)) as archive:
        return sorted((m.name, m.type, m.mode, m.uid, m.gid, m.uname, m.gname, m.size, m.linkname)
                      for m in archive.getmembers())


def field(deb, name):
    return subprocess.run(['dpkg-deb', '--field', str(deb), name], check=True, capture_output=True,
                          text=True).stdout.strip()


@unittest.skipUnless(shutil.which('dpkg-deb') and shutil.which('node'), 'needs dpkg-deb and node')
class RewriteDebVersionsTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.package = self.tmp / 'package'
        self.dist = self.package / 'dist/deb'
        self.dist.mkdir(parents=True)

    def build_oclif_deb(self, version, arch, control=CONTROL):
        """A deb shaped like oclif's: root owned, an executable, a data file and a symlink."""
        workspace = Path(tempfile.mkdtemp(dir=self.tmp))
        (workspace / 'DEBIAN').mkdir()
        (workspace / 'DEBIAN/control').write_text(control.format(version=version, arch=arch))
        bin_dir = workspace / 'usr/lib/dashmate/bin'
        bin_dir.mkdir(parents=True)
        (bin_dir / 'dashmate').write_text('#!/bin/sh\n')
        (bin_dir / 'dashmate').chmod(0o755)
        (workspace / 'usr/lib/dashmate/package.json').write_text('{}\n')
        (workspace / 'usr/bin').mkdir(parents=True)
        os.symlink('../lib/dashmate/bin/dashmate', workspace / 'usr/bin/dashmate')
        deb = self.dist / f'dashmate_{version}_{arch}.deb'
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(workspace), str(deb)],
                       check=True, capture_output=True)
        return deb

    def rewrite(self, semver, revision='1'):
        (self.package / 'package.json').write_text(f'{{"version": "{semver}"}}\n')
        return subprocess.run(
            ['bash', '-c', f'set -e\n{rewrite_function()}\nrewrite_deb_versions dist/deb'],
            cwd=self.package, capture_output=True, text=True,
            env=dict(os.environ, ROOT_PATH=str(ROOT), DASHMATE_DEB_REVISION=revision, DASHMATE_DEB_EPOCH=''))

    def test_should_give_each_deb_the_debian_version_of_its_tag_without_touching_the_payload(self):
        originals = {arch: self.build_oclif_deb('4.1.0.61be67f7bf-1', arch) for arch in ['amd64', 'arm64']}
        before = {arch: (ar_members(deb), payload(deb)) for arch, deb in originals.items()}

        result = self.rewrite('4.1.0-rc.3')

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sorted(p.name for p in self.dist.iterdir()),
                         ['dashmate_4.1.0.rc.3-1_amd64.deb', 'dashmate_4.1.0.rc.3-1_arm64.deb'])
        for arch, (members, files) in before.items():
            with self.subTest(arch=arch):
                deb = self.dist / f'dashmate_4.1.0.rc.3-1_{arch}.deb'
                # `~` keeps the release candidate below 4.1.0-1, so apt upgrades it to the release
                self.assertEqual(field(deb, 'Version'), '4.1.0~rc.3-1')
                self.assertEqual(field(deb, 'Architecture'), arch)
                # the same compression, and the same files, modes and root ownership oclif built
                self.assertEqual(ar_members(deb), members)
                self.assertEqual(payload(deb), files)
                self.assertTrue(all(entry[3:7] == (0, 0, 'root', 'root') for entry in files))

    def test_should_keep_the_git_sha_in_the_description_ahead_of_the_next_field(self):
        self.build_oclif_deb('4.1.0.61be67f7bf-1', 'amd64')

        self.assertEqual(self.rewrite('4.1.0').returncode, 0)

        control = subprocess.run(['dpkg-deb', '--info', str(self.dist / 'dashmate_4.1.0-1_amd64.deb'), 'control'],
                                 check=True, capture_output=True, text=True).stdout.splitlines()
        description = control.index('Description: Distribution package for Dash node installation')
        self.assertEqual(control[description + 1], ' Built from git commit 61be67f7bf.')
        self.assertTrue(control[description + 2].startswith('Aliases:'))

    def test_should_keep_dev_builds_below_beta_and_take_the_revision_from_the_environment(self):
        self.build_oclif_deb('5.1.0.0f38a693a4-1', 'amd64')

        self.assertEqual(self.rewrite('5.1.0-dev.1', revision='2').returncode, 0)

        deb = self.dist / 'dashmate_5.1.0.dev.1-2_amd64.deb'
        self.assertEqual(field(deb, 'Version'), '5.1.0~~dev.1-2')

    def test_should_leave_the_built_debs_alone_when_it_cannot_rewrite_them(self):
        cases = {
            'no git sha in the version': ('4.1.0-1', CONTROL),
            'no description to record the sha in': ('4.1.0.61be67f7bf-1',
                                                     CONTROL.replace('Description: Distribution package for Dash'
                                                                     ' node installation\n', '')),
            # valid for dpkg, but not the line the rewrite replaces
            'a Version line the rewrite does not recognise': ('4.1.0.61be67f7bf-1',
                                                              CONTROL.replace('Version: {version}',
                                                                              'Version:{version}')),
        }
        for case, (version, control) in cases.items():
            with self.subTest(case):
                for leftover in self.dist.iterdir():
                    leftover.unlink()
                deb = self.build_oclif_deb(version, 'amd64', control)
                contents = deb.read_bytes()

                self.assertNotEqual(self.rewrite('4.1.0').returncode, 0)
                self.assertEqual(sorted(p.name for p in self.dist.iterdir()), [deb.name])
                self.assertEqual(deb.read_bytes(), contents)

    def test_should_refuse_to_run_without_debs(self):
        self.assertNotEqual(self.rewrite('4.1.0').returncode, 0)


if __name__ == '__main__':
    unittest.main()
