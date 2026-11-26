"""Check that an apt suite serves the debs a release run verified, with
scripts/release/check_dashmate_apt_publish.sh."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / 'scripts/release/check_dashmate_apt_publish.sh'
TAG = 'v5.1.0-rc.2'
# The bytes of the two debs the run verified, and their sha256s.
DEBS = {'amd64': b'the amd64 deb the run built', 'arm64': b'the arm64 deb the run built'}
AMD64 = hashlib.sha256(DEBS['amd64']).hexdigest()
ARM64 = hashlib.sha256(DEBS['arm64']).hexdigest()
REPOSITORY = 'https://apt.services.dash.org/dashmate'
SUMS = (f'{"c" * 64}  dashmate-v5.1.0-rc.2-3a5da22021-linux-x64.tar.gz\n'
        f'{AMD64}  dashmate_5.1.0.rc.2-1_amd64.deb\n'
        f'{ARM64}  dashmate_5.1.0.rc.2-1_arm64.deb\n').encode()


def stanza(arch, version, sha256, package='dashmate', size=None):
    """One record of a suite's Packages index, as the apt client in the container prints it. Its
    Size is that of the verified deb of its architecture unless given."""
    size = len(DEBS[arch]) if size is None else size
    return (f'Package: {package}\nArchitecture: {arch}\nVersion: {version}\nPriority: optional\n'
            f'Filename: pool/main/d/dashmate/dashmate_{version}_{arch}.deb\nSize: {size}\n'
            f'SHA256: {sha256}\nDescription: Distribution package for Dash node installation\n\n')


def indexes(amd64='', arm64=''):
    """The suite's two Packages indexes, each after the line that names its architecture."""
    return f'### index amd64\n{amd64}\n### index arm64\n{arm64}\n'


# The suite serves this release's two debs and an older release beside them.
SERVED = indexes(stanza('amd64', '5.1.0~rc.2-1', AMD64) + stanza('amd64', '5.1.0~rc.1-1', 'd' * 64),
                 stanza('arm64', '5.1.0~rc.2-1', ARM64) + stanza('arm64', '5.1.0~rc.1-1', 'e' * 64))

# What the apt client in the container must do before its output means anything: trust only the
# pinned archive keyring, fail on any index it cannot verify, and check the suite the signed
# Release names.
CONTAINER_MUST = ['### index $arch', 'signed-by=$keyring', 'sha256sum --check', 'APT::Update::Error-Mode=any',
                  '^Suite: ', '^Codename: ', 'apt-get indextargets', 'apt-helper cat-file']

# `gh release download` copies GH_SUMS into --dir, or fails with GH_FAIL. `docker run` prints
# DOCKER_OUTPUT and exits with DOCKER_EXIT. Each call is logged as a JSON list of its arguments.
GH_STUB = '''#!/usr/bin/env python3
import json, os, shutil, sys
args = sys.argv[1:]
with open(os.environ['CALLS'], 'a') as log:
    log.write(json.dumps(['gh'] + args) + '\\n')
if os.environ.get('GH_FAIL'):
    sys.exit(1)
assert args[:2] == ['release', 'download'], args
directory = args[args.index('--dir') + 1]
os.makedirs(directory, exist_ok=True)
shutil.copy(os.environ['GH_SUMS'], os.path.join(directory, 'SHA256SUMS'))
'''
# `curl` serves the repository's pool from CURL_POOL, by path, and fails as curl -f does on a 404.
CURL_STUB = '''#!/usr/bin/env python3
import json, os, shutil, sys
args = sys.argv[1:]
with open(os.environ['CALLS'], 'a') as log:
    log.write(json.dumps(['curl'] + args) + '\\n')
url, output = args[-1], args[args.index('-o') + 1]
limit = int(args[args.index('--max-filesize') + 1]) if '--max-filesize' in args else None
base = os.environ['REPOSITORY'] + '/'
source = os.path.join(os.environ['CURL_POOL'], url[len(base):]) if url.startswith(base) else ''
if not os.path.isfile(source):
    print(f'curl: (22) The requested URL returned error: 404', file=sys.stderr)
    sys.exit(22)
if limit is not None and os.path.getsize(source) > limit:
    print('curl: (63) Exceeded the maximum allowed file size', file=sys.stderr)
    sys.exit(63)
shutil.copy(source, output)
'''
DOCKER_STUB = '''#!/usr/bin/env python3
import json, os, sys
with open(os.environ['CALLS'], 'a') as log:
    log.write(json.dumps(['docker'] + sys.argv[1:]) + '\\n')
script = sys.argv[-1]
missing = [part for part in json.loads(os.environ['CONTAINER_MUST']) if part not in script]
if missing:
    print(f'container script lacks {missing}', file=sys.stderr)
    sys.exit(99)
sys.stdout.write(open(os.environ['DOCKER_OUTPUT']).read())
sys.exit(int(os.environ.get('DOCKER_EXIT', '0')))
'''


class CheckDashmateAptPublishTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / 'bin').mkdir()
        for name, stub in (('gh', GH_STUB), ('docker', DOCKER_STUB), ('curl', CURL_STUB)):
            (self.tmp / 'bin' / name).write_text(stub)
            (self.tmp / 'bin' / name).chmod(0o755)

    def pool(self, debs=None):
        """The repository's pool, holding the deb each served record names."""
        shutil.rmtree(self.tmp / 'pool', ignore_errors=True)
        for arch, data in (DEBS if debs is None else debs).items():
            path = self.tmp / 'pool' / f'pool/main/d/dashmate/dashmate_5.1.0~rc.2-1_{arch}.deb'
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)

    def check(self, *args, sums=SUMS, served=SERVED, debs=None, **env):
        (self.tmp / 'SHA256SUMS').write_bytes(sums)
        (self.tmp / 'served').write_text(served)
        self.pool(debs)
        environment = {'PATH': f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}", 'HOME': str(self.tmp),
                       'TMPDIR': str(self.tmp), 'CALLS': str(self.tmp / 'calls'),
                       'GH_SUMS': str(self.tmp / 'SHA256SUMS'), 'DOCKER_OUTPUT': str(self.tmp / 'served'),
                       'CONTAINER_MUST': json.dumps(CONTAINER_MUST), 'CURL_POOL': str(self.tmp / 'pool'),
                       'REPOSITORY': REPOSITORY, **env}
        return subprocess.run(['/bin/bash', str(SCRIPT), *args], capture_output=True, text=True,
                              env=environment, timeout=30)

    def calls(self):
        path = self.tmp / 'calls'
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def args(self, suite='testing', digest=None):
        return (f'--tag={TAG}', f'--suite={suite}', f'--sha256sums-sha256={digest or hashlib.sha256(SUMS).hexdigest()}')

    def assertRefused(self, result, message):
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(message, result.stderr)

    def test_should_pass_when_the_suite_serves_the_debs_the_run_verified(self):
        result = self.check(*self.args())

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f'amd64 5.1.0~rc.2-1 {AMD64}', result.stdout)
        self.assertIn(f'arm64 5.1.0~rc.2-1 {ARM64}', result.stdout)
        docker = [call for call in self.calls() if call[0] == 'docker']
        self.assertEqual(len(docker), 1)
        self.assertIn('SUITE=testing', docker[0])
        # each deb is downloaded from the path its record names and hashed
        self.assertEqual(sorted(call[-1] for call in self.calls() if call[0] == 'curl'),
                         [f'{REPOSITORY}/pool/main/d/dashmate/dashmate_5.1.0~rc.2-1_{arch}.deb'
                          for arch in ('amd64', 'arm64')])

    def test_should_refuse_a_deb_the_repository_does_not_serve_as_the_index_lists_it(self):
        # A signed index can list the right sha256 while the file itself is missing or other bytes.
        for case, debs, arch in (('a missing arm64 deb', {'amd64': DEBS['amd64']}, 'arm64'),
                                 ('other bytes for the amd64 deb', dict(DEBS, amd64=b'other bytes'), 'amd64')):
            with self.subTest(case):
                result = self.check(*self.args(), debs=debs)

                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn(f'the {arch} deb', result.stderr)
                self.assertIn('the release is not done, report it to infra', result.stderr)
                self.assertNotIn(f'testing serves the verified deb: {arch}', result.stdout)

    def test_should_refuse_a_deb_whose_size_is_not_the_one_the_index_lists(self):
        # apt rejects a deb whose size differs from its record, even with the right sha256.
        served = SERVED.replace(f'Size: {len(DEBS["amd64"])}\n', f'Size: {len(DEBS["amd64"]) + 1}\n', 1)
        result = self.check(*self.args(), served=served)

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(f'has {len(DEBS["amd64"])} bytes, not the {len(DEBS["amd64"]) + 1} its record lists', result.stderr)

    def test_should_cap_each_download_near_the_size_the_index_lists(self):
        # An oversized response stops the download instead of filling the disk.
        result = self.check(*self.args())

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        limits = {call[-1].rsplit('_', 1)[1]: int(call[call.index('--max-filesize') + 1])
                  for call in self.calls() if call[0] == 'curl'}
        self.assertEqual(limits, {f'{arch}.deb': len(data) + 4096 for arch, data in DEBS.items()})

        result = self.check(*self.args(), debs=dict(DEBS, amd64=DEBS['amd64'] + b'x' * 5000))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('could not download the amd64 deb', result.stderr)

    def test_should_refuse_a_record_without_a_plain_size(self):
        for size in ('', '0', '-5', '12x', '1e9'):
            with self.subTest(size):
                served = SERVED.replace(f'Size: {len(DEBS["amd64"])}\n', f'Size: {size}\n', 1)
                (self.tmp / 'calls').unlink(missing_ok=True)
                result = self.check(*self.args(), served=served)

                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn('the amd64 record in testing lists size', result.stderr)
                self.assertEqual([call for call in self.calls() if call[0] == 'curl'], [])

    def test_should_refuse_a_record_whose_file_name_is_not_a_pool_path(self):
        for filename in ('../../../etc/passwd', 'https://elsewhere.example/dashmate.deb', 'pool/a/../../x.deb', '',
                         # a real pool path with more after it is not that path
                         'pool/main/d/dashmate/dashmate_5.1.0~rc.2-1_amd64.deb extra',
                         'pool/main/d/dashmate/dashmate_5.1.0~rc.2-1_amd64.deb\t'):
            with self.subTest(filename):
                served = SERVED.replace('Filename: pool/main/d/dashmate/dashmate_5.1.0~rc.2-1_amd64.deb',
                                        f'Filename: {filename}')
                (self.tmp / 'calls').unlink(missing_ok=True)
                result = self.check(*self.args(), served=served)

                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn('the amd64 record in testing names', result.stderr)
                self.assertEqual([call for call in self.calls() if call[0] == 'curl'], [])

    def test_should_check_every_record_that_lists_the_verified_sha256(self):
        # Two records with the deb's sha256: both files must be there with those bytes.
        twice = SERVED.replace(stanza('amd64', '5.1.0~rc.1-1', 'd' * 64),
                               stanza('amd64', '5.1.0~rc.2-1', AMD64).replace('rc.2-1_amd64.deb', 'rc.2-1+b1_amd64.deb'))
        result = self.check(*self.args(), served=twice)

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('could not download the amd64 deb pool/main/d/dashmate/dashmate_5.1.0~rc.2-1+b1_amd64.deb',
                      result.stderr)

    def test_should_refuse_a_release_whose_sha256sums_is_not_the_verified_one(self):
        # The digest comes from the run's checksums job; any other manifest proves nothing.
        result = self.check(*self.args(digest='f' * 64))

        self.assertRefused(result, 'is not the one the Release Dashmate checksums job verified')
        self.assertEqual([call[0] for call in self.calls()], ['gh'])

    def test_should_refuse_a_suite_that_serves_other_bytes_or_nothing_for_an_architecture(self):
        cases = {
            'other bytes for arm64': (SERVED.replace(ARM64, '9' * 64), 'arm64', ARM64),
            # each architecture must serve its own deb, not the other one's
            'the two debs swapped': (indexes(stanza('amd64', '5.1.0~rc.2-1', ARM64),
                                             stanza('arm64', '5.1.0~rc.2-1', AMD64)), 'amd64', AMD64),
            # and from its own architecture's index, not the other one
            'the two indexes swapped': (indexes(stanza('arm64', '5.1.0~rc.2-1', ARM64),
                                                stanza('amd64', '5.1.0~rc.2-1', AMD64)), 'amd64', AMD64),
            # a record of another package with the same bytes does not count
            'only another package': (indexes(stanza('amd64', '5.1.0~rc.2-1', AMD64, package='dashmate-dbg'),
                                             stanza('arm64', '5.1.0~rc.2-1', ARM64, package='dashmate-dbg')),
                                     'amd64', AMD64),
            'no arm64 deb': (indexes(stanza('amd64', '5.1.0~rc.2-1', AMD64)), 'arm64', ARM64),
            'nothing at all': (indexes(), 'amd64', AMD64),
        }
        for case, (served, arch, sha256) in cases.items():
            with self.subTest(case):
                result = self.check(*self.args(), served=served)

                self.assertRefused(result, f'testing does not serve the {arch} deb that SHA256SUMS lists ({sha256})')

    def test_should_compare_sha256s_as_strings_not_as_numbers(self):
        # Two different all-digit sha256s are equal as floating-point numbers.
        listed, served = '1' * 64, '1' * 63 + '2'
        sums = SUMS.replace(AMD64.encode(), listed.encode())
        result = self.check(*self.args(digest=hashlib.sha256(sums).hexdigest()), sums=sums,
                            served=SERVED.replace(AMD64, served))

        self.assertRefused(result, f'testing does not serve the amd64 deb that SHA256SUMS lists ({listed})')

    def test_should_refuse_when_the_apt_client_fails(self):
        # A bad or expired signature fails apt-get update: that is a failed check, not a missing deb.
        result = self.check(*self.args(), DOCKER_EXIT='100')

        self.assertRefused(result, 'the apt client could not read the testing suite')

    def test_should_refuse_a_sha256sums_without_exactly_one_deb_per_architecture(self):
        two = SUMS + f'{"0" * 64}  dashmate_5.1.0.rc.2-2_amd64.deb\n'.encode()
        none = b''.join(line + b'\n' for line in SUMS.splitlines() if b'_arm64.deb' not in line)
        for case, sums, arch in (('two amd64 debs', two, 'amd64'), ('no arm64 deb', none, 'arm64')):
            with self.subTest(case):
                result = self.check(*self.args(digest=hashlib.sha256(sums).hexdigest()), sums=sums)

                self.assertRefused(result, f'SHA256SUMS of {TAG} does not list exactly one {arch} deb')

    def test_should_refuse_wrong_options_before_reaching_github(self):
        digest = hashlib.sha256(SUMS).hexdigest()
        for case, args in {
            'no tag': ('--suite=testing', f'--sha256sums-sha256={digest}'),
            'a bad tag': ('--tag=5.1.0', '--suite=testing', f'--sha256sums-sha256={digest}'),
            'an unknown suite': (f'--tag={TAG}', '--suite=main', f'--sha256sums-sha256={digest}'),
            'a short digest': (f'--tag={TAG}', '--suite=testing', f'--sha256sums-sha256={digest[:-1]}'),
            'an unknown option': (*self.args(), '--force'),
        }.items():
            with self.subTest(case):
                (self.tmp / 'calls').unlink(missing_ok=True)
                result = self.check(*args)

                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertEqual(self.calls(), [])

    def test_should_take_the_digest_in_upper_case_too(self):
        result = self.check(*self.args(digest=hashlib.sha256(SUMS).hexdigest().upper()))

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_be_clean_under_shellcheck(self):
        if not shutil.which('shellcheck'):
            self.skipTest('needs shellcheck')
        result = subprocess.run(['shellcheck', str(SCRIPT)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
