"""Repack oclif's Dashmate debs under Debian versions that apt orders by release, adding the
apt repository's source file and keyring."""
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

# The control file oclif 4 writes for Dashmate, as shipped in v5.0.0-rc.5: Description is
# followed by Aliases.
CONTROL = '''Package: dashmate
Version: {version}
Section: main
Priority: standard
Architecture: {arch}
Maintainer: {maintainer}
Description: Distribution package for Dash node installation
Aliases: undefined
'''

SOURCES = 'etc/apt/sources.list.d/dashmate.sources'
KEYRING = 'usr/share/keyrings/dashmate-archive-keyring.gpg'

# The stanza infra specified for the repository. The suite stays `stable` for every version,
# prereleases included: switching to `testing` is the operator's choice, and the file is a
# conffile so that choice survives upgrades.
STANZA = '''Types: deb
URIs: https://apt.services.dash.org/dashmate
Suites: stable
Components: main
Architectures: amd64 arm64
Signed-By: /usr/share/keyrings/dashmate-archive-keyring.gpg
'''


def oclif_maintainer():
    """The Maintainer oclif writes: the package's author, or `undefined` when it has none."""
    author = json.loads((ROOT / 'packages/dashmate/package.json').read_text()).get('author')
    return 'undefined' if author is None else author


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


def payload_file(deb, path):
    tar = subprocess.run(['dpkg-deb', '--fsys-tarfile', str(deb)], check=True, capture_output=True).stdout
    with tarfile.open(fileobj=io.BytesIO(tar)) as archive:
        return archive.extractfile(f'./{path}').read()


def control_member(deb, name):
    """A file from the deb's control archive, with its mode."""
    tar = subprocess.run(['dpkg-deb', '--ctrl-tarfile', str(deb)], check=True, capture_output=True).stdout
    with tarfile.open(fileobj=io.BytesIO(tar)) as archive:
        member = archive.getmember(f'./{name}')
        return archive.extractfile(member).read().decode(), member.mode


def added_entries(keyring):
    """The entries the repack adds to oclif's payload: the two files and the directories
    holding them that oclif's payload has no entry for, all root owned."""
    root = (0, 0, 'root', 'root')
    directories = ['./etc', './etc/apt', './etc/apt/sources.list.d', './usr/share', './usr/share/keyrings']
    return ([(d, tarfile.DIRTYPE, 0o755, *root, 0, '') for d in directories]
            + [(f'./{SOURCES}', tarfile.REGTYPE, 0o644, *root, len(STANZA.encode()), ''),
               (f'./{KEYRING}', tarfile.REGTYPE, 0o644, *root, len(keyring), '')])


def four_byte_length_header(keyring):
    """The keyring with its first packet, a public key under the two-byte length header 0x99
    or the one-byte 0x98, re-encoded under the old format's four-byte length header 0x9a."""
    if keyring[0] == 0x98:
        length, body = keyring[1], keyring[2:]
    else:
        length, body = int.from_bytes(keyring[1:3], 'big'), keyring[3:]
    return bytes([0x9a]) + length.to_bytes(4, 'big') + body


def gpg(home, *args):
    return subprocess.run(['gpg', '--batch', '--no-options', '--homedir', str(home), *args], check=True,
                          capture_output=True, env=dict(os.environ, GNUPGHOME=str(home))).stdout


def field(deb, name):
    return subprocess.run(['dpkg-deb', '--field', str(deb), name], check=True, capture_output=True,
                          text=True).stdout.strip()


# gpg is not a reason to skip: the repack needs it, so a host without it fails loudly.
@unittest.skipUnless(shutil.which('dpkg-deb') and shutil.which('node'), 'needs dpkg-deb and node')
class RewriteDebVersionsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        """A throwaway signing key, exported the ways a keyring file could arrive."""
        # gpg-agent's socket lives in the home directory, and macOS caps socket paths at 104
        # bytes, which the default temporary directory there can exceed.
        home = Path(tempfile.mkdtemp(dir='/tmp' if os.path.isdir('/tmp') else None))
        try:
            gpg(home, '--passphrase', '', '--pinentry-mode', 'loopback', '--quick-gen-key',
                'Dashmate repack test <test@example.invalid>', 'ed25519', 'sign', 'never')
            cls.keyring = gpg(home, '--export')
            cls.armored_keyring = gpg(home, '--armor', '--export')
            cls.secret_key = gpg(home, '--passphrase', '', '--pinentry-mode', 'loopback',
                                 '--export-secret-keys')
            gpg(home, '--passphrase', '', '--pinentry-mode', 'loopback', '--quick-gen-key',
                'Another key <other@example.invalid>', 'ed25519', 'sign', 'never')
            cls.other_keyring = gpg(home, '--export', 'other@example.invalid')
        finally:
            subprocess.run(['gpgconf', '--homedir', str(home), '--kill', 'all'], capture_output=True)
            shutil.rmtree(home, ignore_errors=True)

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.package = self.tmp / 'package'
        self.dist = self.package / 'dist/deb'
        self.dist.mkdir(parents=True)
        # The repack reads its inputs under ROOT_PATH. Point that at a copy of the repository
        # layout holding copies of the real version script and source file, so each test
        # chooses the keyring without one ever being written into the repository. The version
        # script only runs as a command when invoked under its real path, so it is copied
        # rather than linked, under a path without symlinks (macOS's temporary directory is
        # one).
        self.root = self.tmp.resolve() / 'root'
        dashmate = self.root / 'packages/dashmate'
        (dashmate / 'deb').mkdir(parents=True)
        shutil.copytree(ROOT / 'packages/dashmate/scripts', dashmate / 'scripts')
        shutil.copy(ROOT / 'packages/dashmate/deb/dashmate.sources', dashmate / 'deb/dashmate.sources')
        self.use_keyring(self.keyring)

    def use_keyring(self, contents):
        path = self.root / 'packages/dashmate/deb/dashmate-archive-keyring.gpg'
        if contents is None:
            path.unlink(missing_ok=True)
        else:
            path.write_bytes(contents)

    def build_oclif_deb(self, version, arch, control=CONTROL, extra=None, link=None):
        """A deb shaped like oclif's: root owned, an executable, a data file and a symlink."""
        workspace = Path(tempfile.mkdtemp(dir=self.tmp))
        (workspace / 'DEBIAN').mkdir()
        (workspace / 'DEBIAN/control').write_text(control.format(version=version, arch=arch,
                                                                  maintainer=oclif_maintainer()))
        bin_dir = workspace / 'usr/lib/dashmate/bin'
        bin_dir.mkdir(parents=True)
        (bin_dir / 'dashmate').write_text('#!/bin/sh\n')
        (bin_dir / 'dashmate').chmod(0o755)
        (workspace / 'usr/lib/dashmate/package.json').write_text('{}\n')
        (workspace / 'usr/bin').mkdir(parents=True)
        os.symlink('../lib/dashmate/bin/dashmate', workspace / 'usr/bin/dashmate')
        for path, contents in (extra or {}).items():
            (workspace / path).parent.mkdir(parents=True, exist_ok=True)
            (workspace / path).write_text(contents)
        if link is not None:
            target, path = link
            (workspace / path).parent.mkdir(parents=True, exist_ok=True)
            os.symlink(target, workspace / path)
        deb = self.dist / f'dashmate_{version}_{arch}.deb'
        built = subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(workspace), str(deb)],
                               capture_output=True, text=True)
        self.assertEqual(built.returncode, 0, built.stderr)
        return deb

    def rewrite(self, semver, revision='1'):
        (self.package / 'package.json').write_text(f'{{"version": "{semver}"}}\n')
        return subprocess.run(
            ['bash', '-c', f'set -e\n{rewrite_function()}\nrewrite_deb_versions dist/deb'],
            cwd=self.package, capture_output=True, text=True,
            env=dict(os.environ, ROOT_PATH=str(self.root), DASHMATE_DEB_REVISION=revision, DASHMATE_DEB_EPOCH=''))

    def test_should_give_each_deb_the_debian_version_of_its_tag_and_keep_oclifs_payload(self):
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
                # the same compression, and the same files, modes and root ownership oclif built,
                # next to the apt repository's source file and keyring
                self.assertEqual(ar_members(deb), members)
                self.assertEqual(payload(deb), sorted(files + added_entries(self.keyring)))
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
            'no git sha in the version': ('4.1.0-1', CONTROL, 'Expected a git sha'),
            'no description to record the sha in': ('4.1.0.61be67f7bf-1',
                                                     CONTROL.replace('Description: Distribution package for Dash'
                                                                     ' node installation\n', ''),
                                                     'No Description field'),
            # valid for dpkg, but not the line the rewrite replaces
            'a Version line the rewrite does not recognise': ('4.1.0.61be67f7bf-1',
                                                              CONTROL.replace('Version: {version}',
                                                                              'Version:{version}'),
                                                              'Could not rewrite the Version field'),
            # what oclif writes when the package has no author
            'an undefined Maintainer': ('4.1.0.61be67f7bf-1', CONTROL.replace('{maintainer}', 'undefined'),
                                        'Expected a Maintainer'),
            'no Maintainer': ('4.1.0.61be67f7bf-1', CONTROL.replace('Maintainer: {maintainer}\n', ''),
                              'Expected a Maintainer'),
            # Debian requires the maintainer's address
            'a Maintainer without an address': ('4.1.0.61be67f7bf-1',
                                                CONTROL.replace('{maintainer}', 'Dash Core Team'),
                                                'Expected a Maintainer'),
            'an address with a second @': ('4.1.0.61be67f7bf-1',
                                           CONTROL.replace('{maintainer}', 'Dash Core Team <dev@@dash.org>'),
                                           'Expected a Maintainer'),
            # only a plain address is accepted, not a quoted local part
            'a quoted local part': ('4.1.0.61be67f7bf-1',
                                    CONTROL.replace('{maintainer}', 'Dash Core Team <"dev"@dash.org>'),
                                    'Expected a Maintainer'),
        }
        for case, (version, control, reason) in cases.items():
            with self.subTest(case):
                for leftover in self.dist.iterdir():
                    leftover.unlink()
                deb = self.build_oclif_deb(version, 'amd64', control)
                contents = deb.read_bytes()

                result = self.rewrite('4.1.0')

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(reason, result.stdout)
                self.assertEqual(sorted(p.name for p in self.dist.iterdir()), [deb.name])
                self.assertEqual(deb.read_bytes(), contents)

    def test_should_ship_the_stable_source_and_its_keyring_in_every_deb(self):
        self.build_oclif_deb('5.1.0.0f38a693a4-1', 'amd64')
        self.build_oclif_deb('5.1.0.0f38a693a4-1', 'arm64')

        # a prerelease still names `stable`
        result = self.rewrite('5.1.0-rc.1')

        self.assertEqual(result.returncode, 0, result.stderr)
        for arch in ['amd64', 'arm64']:
            with self.subTest(arch=arch):
                deb = self.dist / f'dashmate_5.1.0.rc.1-1_{arch}.deb'
                self.assertEqual(payload_file(deb, SOURCES).decode(), STANZA)
                # the key the source file names is the one shipped next to it
                self.assertEqual(payload_file(deb, KEYRING), self.keyring)

    def test_should_list_the_source_file_as_the_only_conffile(self):
        self.build_oclif_deb('5.1.0.0f38a693a4-1', 'amd64')

        self.assertEqual(self.rewrite('5.1.0').returncode, 0)

        # a final release names `stable` too
        self.assertEqual(payload_file(self.dist / 'dashmate_5.1.0-1_amd64.deb', SOURCES).decode(), STANZA)
        # without it dpkg would overwrite an operator's switch to `testing` on every upgrade;
        # the keyring is not one, so a renewed keyring reaches every host that upgrades
        self.assertEqual(control_member(self.dist / 'dashmate_5.1.0-1_amd64.deb', 'conffiles'),
                         (f'/{SOURCES}\n', 0o644))

    def test_should_leave_the_built_debs_alone_without_a_usable_keyring(self):
        cases = {
            'no keyring': (None, 'No apt repository keyring at'),
            'an empty file': (b'', 'is not a binary OpenPGP public keyring holding exactly one key'),
            'not a key': (b'not a keyring\n', 'is not a binary OpenPGP public keyring holding exactly one key'),
            # apt reads a `.gpg` keyring as binary only
            'an ASCII-armored key': (self.armored_keyring, 'is not a binary OpenPGP public keyring holding exactly one key'),
            'a secret key': (self.secret_key, 'is not a binary OpenPGP public keyring holding exactly one key'),
            # starts like a public key but gpg cannot read a key from it
            'a truncated key': (self.keyring[:8], 'is not a binary OpenPGP public keyring holding exactly one key'),
            # starts like a public key, but must never ship
            'a public key followed by its secret key': (self.keyring + self.secret_key,
                                                        'is not a binary OpenPGP public keyring holding exactly one key'),
            # gpg labels a version 3 secret key packet as a plain key packet, and fails to parse
            # this one; either is reason enough to refuse it
            'a public key followed by a version 3 secret key packet': (
                self.keyring + b'\x95\x01\x03', 'is not a binary OpenPGP public keyring holding exactly one key'),
            # a valid public key packet behind a header apt does not accept as a keyring
            'a key behind a four-byte length header': (
                four_byte_length_header(self.keyring), 'is not a binary OpenPGP public keyring holding exactly one key'),
            # infra hands over one key; a second one would be trusted for the repository too
            'two keys': (self.keyring + self.other_keyring,
                         'is not a binary OpenPGP public keyring holding exactly one key'),
        }
        for case, (keyring, reason) in cases.items():
            with self.subTest(case):
                for leftover in self.dist.iterdir():
                    leftover.unlink()
                self.use_keyring(keyring)
                deb = self.build_oclif_deb('5.1.0.0f38a693a4-1', 'amd64')
                contents = deb.read_bytes()

                result = self.rewrite('5.1.0')

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(reason, result.stdout)
                self.assertEqual(sorted(p.name for p in self.dist.iterdir()), [deb.name])
                self.assertEqual(deb.read_bytes(), contents)

    def test_should_refuse_a_deb_where_oclif_already_wrote_what_the_repack_adds(self):
        # writing over oclif's files would silently drop them, and writing through a symlinked
        # directory would leave the files out of the package
        cases = {
            'DEBIAN/conffiles': ({'DEBIAN/conffiles': '/usr/lib/dashmate/package.json\n'}, None,
                                 'already has DEBIAN/conffiles'),
            SOURCES: ({SOURCES: 'Types: deb\n'}, None, f'already has {SOURCES}'),
            KEYRING: ({KEYRING: 'key\n'}, None, f'already has {KEYRING}'),
            'a symlinked keyrings directory': ({'usr/lib/dashmate/keyrings/.keep': ''},
                                               ('../lib/dashmate/keyrings', 'usr/share/keyrings'),
                                               'has usr/share/keyrings as a symlink'),
        }
        for case, (extra, link, reason) in cases.items():
            with self.subTest(case):
                for leftover in self.dist.iterdir():
                    leftover.unlink()
                deb = self.build_oclif_deb('5.1.0.0f38a693a4-1', 'amd64', extra=extra, link=link)
                contents = deb.read_bytes()

                result = self.rewrite('5.1.0')

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(reason, result.stdout)
                self.assertEqual(sorted(p.name for p in self.dist.iterdir()), [deb.name])
                self.assertEqual(deb.read_bytes(), contents)

    def test_should_refuse_to_run_without_debs(self):
        self.assertNotEqual(self.rewrite('4.1.0').returncode, 0)


if __name__ == '__main__':
    unittest.main()
