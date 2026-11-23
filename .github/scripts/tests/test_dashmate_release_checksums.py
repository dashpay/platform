"""Record, generate, check and attach the SHA256SUMS of the Dashmate release packages."""
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
RELEASE = ROOT / '.github/workflows/release.yml'
SHA = 'bb78fa2220ae0d58f5a9b5c0f6b0027a292cc3d0'

# What the runner puts in place of the expressions these steps use.
EXPRESSIONS = {
    'github.token': 'token',
    'github.repository': 'dashpay/platform',
    'github.event.release.id': '123',
    'github.event.release.tag_name': 'v5.1.0',
    'github.sha': SHA,
}

# What each release leg publishes, by package type, as paths inside the leg's
# download of the build artifact.
PACKAGES = {
    'tarballs': ['dashmate-v5.1.0-3a5da22021-linux-arm64.tar.gz', 'dashmate-v5.1.0-3a5da22021-linux-x64.tar.gz'],
    'win': ['win32/dashmate-v5.1.0-3a5da22021-x64.exe'],
    'deb': ['deb/dashmate_5.1.0-1_amd64.deb', 'deb/dashmate_5.1.0-1_arm64.deb'],
    'macos': ['macos/dashmate-v5.1.0-3a5da22021-arm64.pkg', 'macos/dashmate-v5.1.0-3a5da22021-x64.pkg'],
}

ASSETS_CALL = 'api --paginate repos/dashpay/platform/releases/123/assets?per_page=100'

# gh needs GH_TOKEN in a workflow. `gh api` answers listings with the release assets in GH_ASSETS,
# filtered through real jq as gh's --jq would. An upload through the API keeps a copy of the file
# and answers with asset id GH_NEW_ID; reading that asset back reports the uploaded bytes' digest,
# or GH_ATTACHED_STATE / GH_ATTACHED_DIGEST ('null' for none). GH_FAIL fails the listings. The log
# has one line per call, without the jq filter.
GH_STUB = '''#!/usr/bin/env python3
import hashlib, json, os, shutil, subprocess, sys
args = sys.argv[1:]
jq = args[args.index('--jq') + 1] if '--jq' in args else None
with open(os.environ['GH_LOG'], 'a') as log:
    log.write(' '.join(a for a in args if jq is None or a not in ('--jq', jq)) + '\\n')
if not os.environ.get('GH_TOKEN'):
    print('gh: set the GH_TOKEN environment variable', file=sys.stderr)
    sys.exit(4)

def respond(text):
    result = subprocess.run(['jq', '-r', jq], input=text, capture_output=True, text=True)
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    sys.exit(result.returncode)

uploaded = os.path.join(os.environ['UPLOADED'], 'SHA256SUMS')
if args[:2] == ['api', '--paginate']:
    if os.environ.get('GH_FAIL'):
        print('gh: Server Error (HTTP 502)', file=sys.stderr)
        sys.exit(1)
    respond(os.environ['GH_ASSETS'])
if args[:3] == ['api', '--method', 'DELETE']:
    sys.exit(0)
if args[:3] == ['api', '--method', 'POST']:
    os.makedirs(os.environ['UPLOADED'], exist_ok=True)
    shutil.copy(args[args.index('--input') + 1], uploaded)
    respond(json.dumps({'id': int(os.environ.get('GH_NEW_ID', '4242')), 'state': 'uploaded'}))
if args[0] == 'api' and '/releases/assets/' in args[1]:
    with open(uploaded, 'rb') as handle:
        digest = 'sha256:' + hashlib.sha256(handle.read()).hexdigest()
    digest = os.environ.get('GH_ATTACHED_DIGEST', digest)
    respond(json.dumps({'id': int(args[1].rsplit('/', 1)[1]), 'state': os.environ.get('GH_ATTACHED_STATE', 'uploaded'),
                        'digest': None if digest == 'null' else digest}))
sys.exit(2)
'''


def job(name):
    """The text of one job of release.yml, up to the next job."""
    text = RELEASE.read_text()
    start = text.index(f'\n  {name}:\n')
    end = re.search(r'\n  [a-z][a-z0-9-]*:\n', text[start + 1:])
    return text[start:start + 1 + end.start()] if end else text[start:]


def step_block(name):
    """The lines of a step of a release.yml job, from its `- name:` to the next step."""
    lines = RELEASE.read_text().splitlines()
    start = lines.index('      - name: ' + name)
    block = [lines[start]]
    for line in lines[start + 1:]:
        if line.strip() and len(line) - len(line.lstrip()) <= 6:
            break
        block.append(line)
    return block


def scalar(value):
    """A plain or quoted YAML scalar's value; anything else is refused rather than misread."""
    if len(value) >= 2 and value[0] == value[-1] == "'":
        return value[1:-1].replace("''", "'")
    if len(value) >= 2 and value[0] == value[-1] == '"':
        return json.loads(value)
    if value[:1] in tuple('|>[{&*!\'"') or ' #' in value:
        raise ValueError(f'Unsupported YAML value: {value}')
    return value


def mapping(lines, header, indent):
    """The `key: value` entries under `header`."""
    if header not in lines:
        return {}
    entries = {}
    for line in lines[lines.index(header) + 1:]:
        if not line.startswith(' ' * indent) or line[indent] == ' ':
            break
        key, value = line.strip().split(': ', 1)
        entries[key] = scalar(value)
    return entries


def step_with(name):
    return mapping(step_block(name), '        with:', 10)


def step_env(name):
    """The step's own env, below the env of the job it belongs to."""
    checksums = job('release-dashmate-checksums')
    env = mapping(checksums.splitlines(), '    env:', 6) if '- name: ' + name + '\n' in checksums else {}
    return dict(env, **mapping(step_block(name), '        env:', 10))


def step_script(name):
    block = step_block(name)
    if '        run: |' not in block:
        raise ValueError(f'The step {name!r} has no `run: |` script')
    script = []
    for line in block[block.index('        run: |') + 1:]:
        if line.strip() and not line.startswith(' ' * 10):
            break
        script.append(line)
    return textwrap.dedent('\n'.join(script))


def evaluate(value, **context):
    """`value` with each ${{ expression }} replaced as the runner would; unknown ones raise."""
    return re.sub(r'\$\{\{\s*(.+?)\s*\}\}', lambda match: dict(EXPRESSIONS, **context)[match.group(1)], value)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


@unittest.skipUnless(shutil.which('jq') and shutil.which('shasum'), 'needs jq and shasum')
class ReleaseChecksumsTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / 'bin').mkdir()
        (self.tmp / 'bin/gh').write_text(GH_STUB)
        (self.tmp / 'bin/gh').chmod(0o755)
        (self.tmp / 'runner-temp').mkdir()

    def context(self, package_type):
        return {'runner.temp': str(self.tmp / 'runner-temp'), 'matrix.package_type': package_type}

    def run_step(self, name, cwd=None, assets=(), package_type=None, **knobs):
        """Run a step with the environment its YAML gives it, plus what the runner always sets.

        Nothing else leaks in: no LC_ALL unless the step sets one, under a locale whose collation
        differs from C. /bin/bash is the shell of the macOS leg, Apple's bash 3.2 there.
        """
        context = self.context(package_type) if package_type else {}
        env = {'PATH': f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}", 'HOME': str(self.tmp),
               'LANG': 'en_US.UTF-8', 'GITHUB_REPOSITORY': 'dashpay/platform',
               'GH_LOG': str(self.tmp / 'gh.log'), 'GH_ASSETS': json.dumps(list(assets)),
               'UPLOADED': str(self.tmp / 'uploaded'), **knobs}
        env.update({key: evaluate(value, **context) for key, value in step_env(name).items()})
        return subprocess.run(['/bin/bash', '-c', step_script(name)], capture_output=True, text=True, env=env,
                              cwd=cwd or self.tmp)

    def gh_calls(self):
        log = self.tmp / 'gh.log'
        return log.read_text().splitlines() if log.exists() else []

    def leg(self, package_type, files=None):
        """A release leg's download of its build artifact, with a hidden file the upload skips."""
        leg = self.tmp / package_type
        shutil.rmtree(leg, ignore_errors=True)
        contents = {}
        for path in files if files is not None else PACKAGES[package_type]:
            target = leg / 'release-artifacts' / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(path.encode() + b' built')
            contents[target.name] = target.read_bytes()
        (leg / 'release-artifacts/.DS_Store').parent.mkdir(parents=True, exist_ok=True)
        (leg / 'release-artifacts/.DS_Store').write_text('not uploaded')
        return leg, contents

    def record(self, package_type, files=None):
        """Run a leg's Record step; returns the file its upload step would take as the record."""
        leg, contents = self.leg(package_type, files)
        result = self.run_step('Record package checksums', cwd=leg, package_type=package_type)
        record = Path(evaluate(step_with('Upload package checksums')['path'], **self.context(package_type)))
        return result, record, contents

    def transfer_records(self):
        """Move the records as upload-artifact and download-artifact (pattern, merge-multiple) would."""
        upload, download = step_with('Upload package checksums'), step_with('Download package checksums')
        self.assertEqual(download.get('merge-multiple'), 'true')
        target = self.tmp / evaluate(download['path'])
        target.mkdir(exist_ok=True)
        # overwrite: true replaces an artifact of the same name, so each leg needs its own
        names = [evaluate(upload['name'], **self.context(package_type)) for package_type in PACKAGES]
        self.assertEqual(len(set(names)), len(names), names)
        for package_type in PACKAGES:
            context = self.context(package_type)
            source = Path(evaluate(upload['path'], **context))
            if source.exists() and fnmatch.fnmatchcase(evaluate(upload['name'], **context),
                                                       evaluate(download['pattern'])):
                shutil.copy(source, target)

    def record_all(self):
        published = {}
        for package_type in PACKAGES:
            result, _, contents = self.record(package_type)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            published.update(contents)
        self.transfer_records()
        return published

    @staticmethod
    def assets(published, **overrides):
        """The release's assets as the API lists them, with the other release jobs' assets beside them."""
        listed = [{'name': name, 'state': 'uploaded', 'digest': 'sha256:' + sha256(data)}
                  for name, data in published.items()]
        listed += [{'name': 'dash-sdk-android-5.1.0.aar', 'state': 'uploaded', 'digest': 'sha256:' + '0' * 64},
                   {'name': 'DashSDKFFI-5.1.0.xcframework.zip', 'state': 'uploaded', 'digest': None}]
        for name, change in overrides.items():
            listed = [dict(asset, **change) if asset['name'] == name else asset for asset in listed]
        return listed

    def test_should_record_each_uploaded_package_under_its_release_name(self):
        result, record, contents = self.record('deb')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(sorted(record.read_text().splitlines()),
                         sorted(f'{sha256(data)}  {name}' for name, data in contents.items()))

    def test_should_refuse_to_publish_anything_but_dashmate_packages(self):
        cases = {
            # a signature or checksum file from a build would replace the release's own
            'a SHA256SUMS': ['deb/dashmate_5.1.0-1_amd64.deb', 'deb/SHA256SUMS'],
            'a signature': ['deb/dashmate_5.1.0-1_amd64.deb', 'SHA256SUMS.asc'],
            # GitHub renames these characters on upload, so the name would not be the published one
            'a ~ in the name': ['deb/dashmate_5.1.0~rc.1-1_amd64.deb'],
            'a space in the name': ['deb/dashmate 5.1.0-1_amd64.deb'],
            'a newline in the name': ['deb/dashmate_5.1.0-1_amd64.deb\nSHA256SUMS'],
        }
        for case, files in cases.items():
            with self.subTest(case):
                result, record, _ = self.record('deb', files)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('::error::Refusing to publish', result.stdout)
        with self.subTest('a symlink, which the upload would follow'):
            leg, _ = self.leg('deb')
            (leg / 'release-artifacts/deb/dashmate_5.1.0-1_i386.deb').symlink_to('dashmate_5.1.0-1_amd64.deb')
            result = self.run_step('Record package checksums', cwd=leg, package_type='deb')
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('::error::Not a regular file', result.stdout)

    def test_should_refuse_a_leg_without_packages(self):
        result, _, _ = self.record('deb', [])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('::error::No packages to record', result.stdout)

    def test_should_list_every_package_once_sorted_by_name_in_sha256sum_format(self):
        published = self.record_all()

        result = self.run_step('Generate SHA256SUMS')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        sums = (self.tmp / 'SHA256SUMS').read_text()
        self.assertEqual(sums, ''.join(f'{sha256(published[name])}  {name}\n' for name in sorted(published)))
        # A user who downloads the packages checks them with the file as it is.
        downloads = self.tmp / 'downloads'
        downloads.mkdir()
        for name, data in published.items():
            (downloads / name).write_bytes(data)
        shutil.copy(self.tmp / 'SHA256SUMS', downloads)
        check = subprocess.run(['shasum', '-a', '256', '--check', '--strict', 'SHA256SUMS'], cwd=downloads,
                               capture_output=True, text=True)
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)

    def test_should_refuse_records_that_are_missing_malformed_or_collide(self):
        cases = {
            # the download does not fail when a record has expired
            'a package type without a record': lambda checksums: (checksums / 'dashmate-macos.sha256').unlink(),
            'a line that is not <sha256>  <name>': lambda checksums: (checksums / 'dashmate-win.sha256').write_text(
                'not-a-hash  dashmate-v5.1.0-3a5da22021-x64.exe\n'),
            'two packages published under one name': lambda checksums: (checksums / 'dashmate-win.sha256').write_text(
                f"{'a' * 64}  dashmate_5.1.0-1_amd64.deb\n"),
        }
        for case, damage in cases.items():
            with self.subTest(case):
                shutil.rmtree(self.tmp / 'checksums', ignore_errors=True)
                (self.tmp / 'SHA256SUMS').unlink(missing_ok=True)
                self.record_all()
                damage(self.tmp / 'checksums')

                result = self.run_step('Generate SHA256SUMS')

                self.assertNotEqual(result.returncode, 0)
                self.assertIn('::error::', result.stdout)
                self.assertFalse((self.tmp / 'SHA256SUMS').exists())

    def generated(self):
        published = self.record_all()
        result = self.run_step('Generate SHA256SUMS')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return published

    def test_should_accept_a_release_that_serves_exactly_the_recorded_packages(self):
        published = self.generated()
        assets = self.assets(published) + [{'name': 'SHA256SUMS', 'state': 'uploaded', 'digest': None}]

        result = self.run_step('Check the release serves the recorded packages', assets=assets)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.gh_calls(), [ASSETS_CALL])

    def test_should_refuse_a_release_whose_packages_differ_from_the_recorded_ones(self):
        published = self.generated()
        deb = 'dashmate_5.1.0-1_amd64.deb'
        cases = {
            'a package that was not attached': [a for a in self.assets(published) if a['name'] != deb],
            'a package with other bytes': self.assets(published, **{deb: {'digest': 'sha256:' + 'f' * 64}}),
            'an upload that did not finish': self.assets(published, **{deb: {'state': 'open'}}),
            'a package without a digest': self.assets(published, **{deb: {'digest': None}}),
            # e.g. the debs of an earlier Debian revision, still attached after a rebuild
            'a package left from an earlier build': self.assets(published) + [
                {'name': 'dashmate_5.1.0-2_amd64.deb', 'state': 'uploaded', 'digest': 'sha256:' + 'e' * 64}],
        }
        for case, assets in cases.items():
            with self.subTest(case):
                result = self.run_step('Check the release serves the recorded packages', assets=assets)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('::error::', result.stdout)
        with self.subTest('the release cannot be listed'):
            result = self.run_step('Check the release serves the recorded packages', assets=self.assets(published),
                                   GH_FAIL='1')
            self.assertNotEqual(result.returncode, 0)

    UPLOAD_CALL = ('api --method POST https://uploads.github.com/repos/dashpay/platform/releases/123/assets'
                   '?name=SHA256SUMS -H Content-Type: text/plain --input SHA256SUMS')

    def test_should_attach_sha256sums_and_record_the_asset_in_its_log(self):
        published = self.generated()
        sums = sha256((self.tmp / 'SHA256SUMS').read_bytes())
        previous = {'id': 77, 'name': 'SHA256SUMS', 'state': 'uploaded', 'digest': 'sha256:' + 'a' * 64}
        for case, assets, replaced in [('first upload', self.assets(published), []),
                                       ('a SHA256SUMS from an earlier run', self.assets(published) + [previous],
                                        ['api --method DELETE repos/dashpay/platform/releases/assets/77'])]:
            with self.subTest(case):
                (self.tmp / 'gh.log').unlink(missing_ok=True)

                result = self.run_step('Attach SHA256SUMS to the release', assets=assets)

                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.gh_calls(), [ASSETS_CALL, *replaced, self.UPLOAD_CALL,
                                                   'api repos/dashpay/platform/releases/assets/4242'])
                self.assertEqual((self.tmp / 'uploaded/SHA256SUMS').read_bytes(),
                                 (self.tmp / 'SHA256SUMS').read_bytes())
                self.assertIn(f'SHA256SUMS attached: asset 4242 sha256 {sums}', result.stdout.splitlines())
        # the signing script reads that line back from the job's log as this asset and sum
        script = (ROOT / 'scripts/release/sign_dashmate_checksums.sh').read_text()
        expression = re.search(r"sed -nE '([^']+)'", script).group(1)
        read = subprocess.run(['sed', '-nE', expression], capture_output=True, text=True,
                              input=f'2026-11-23T10:00:01.0000000Z SHA256SUMS attached: asset 4242 sha256 {sums}\n')
        self.assertEqual(read.stdout, f'4242 {sums}\n')

    def test_should_not_record_an_asset_that_is_not_the_file_it_uploaded(self):
        published = self.generated()
        for case, env in [('other bytes', {'GH_ATTACHED_DIGEST': 'sha256:' + 'f' * 64}),
                          ('no digest yet', {'GH_ATTACHED_DIGEST': 'null'}),
                          ('an upload that did not finish', {'GH_ATTACHED_STATE': 'open'}),
                          ('no asset id', {'GH_NEW_ID': '0'})]:
            with self.subTest(case):
                result = self.run_step('Attach SHA256SUMS to the release', assets=self.assets(published), **env)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('::error::', result.stdout)
                self.assertNotIn('SHA256SUMS attached: asset', result.stdout)

    def test_should_never_replace_a_signed_sha256sums(self):
        published = self.generated()
        writes = ('api --method',)
        for signature in ['SHA256SUMS.asc', 'SHA256SUMS.sig']:
            with self.subTest(signature):
                (self.tmp / 'gh.log').unlink(missing_ok=True)
                assets = self.assets(published) + [{'id': 77, 'name': 'SHA256SUMS', 'state': 'uploaded', 'digest': None},
                                                   {'id': 78, 'name': signature, 'state': 'uploaded', 'digest': None}]

                result = self.run_step('Attach SHA256SUMS to the release', assets=assets)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'::error::v5.1.0 is signed ({signature})', result.stdout)
                self.assertFalse(any(call.startswith(writes) for call in self.gh_calls()))
                self.assertFalse((self.tmp / 'uploaded').exists())
        with self.subTest('the release cannot be listed'):
            (self.tmp / 'gh.log').unlink(missing_ok=True)
            result = self.run_step('Attach SHA256SUMS to the release', assets=self.assets(published), GH_FAIL='1')
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(any(call.startswith(writes) for call in self.gh_calls()))

    def test_should_never_replace_the_packages_of_a_signed_release(self):
        step = 'Refuse to replace the packages of a signed release'
        result = self.run_step(step, assets=[{'name': 'SHA256SUMS'}])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.gh_calls(), [ASSETS_CALL])
        for signature in ['SHA256SUMS.asc', 'SHA256SUMS.sig']:
            with self.subTest(signature):
                result = self.run_step(step, assets=[{'name': 'SHA256SUMS'}, {'name': signature}])
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f'::error::The release is signed ({signature})', result.stdout)
        with self.subTest('the release cannot be listed'):
            self.assertNotEqual(self.run_step(step, assets=[], GH_FAIL='1').returncode, 0)

    def test_should_hash_after_signing_and_refuse_a_signed_release_before_uploading(self):
        legs = job('release-dashmate-packages')
        order = [legs.index('- name: ' + step) for step in [
            'Sign and notarize macOS installers', 'Refuse to replace the packages of a signed release',
            'Record package checksums', 'Upload package checksums', 'Upload artifacts to release']]
        self.assertEqual(order, sorted(order))
        # a re-run leg replaces its record, so the record describes what it published last
        self.assertEqual(step_with('Upload package checksums')['overwrite'], 'true')

    def test_should_compare_names_bytewise(self):
        # The C locale keeps the name check to ASCII and the sort order fixed. Set explicitly,
        # because a runner without the locale a test picks would hide its absence.
        for step in ['Record package checksums', 'Generate SHA256SUMS', 'Check the release serves the recorded packages']:
            with self.subTest(step):
                self.assertEqual(step_env(step).get('LC_ALL'), 'C')

    def test_should_require_a_record_from_every_package_type_of_the_matrix(self):
        matrix = re.findall(r'- package_type: (\S+)', job('release-dashmate-packages'))
        required = re.search(r'for package_type in ([^;]+); do', step_script('Generate SHA256SUMS')).group(1)
        self.assertEqual(sorted(required.split()), sorted(matrix))
        self.assertEqual(sorted(PACKAGES), sorted(matrix))

    def test_should_publish_checksums_only_after_every_build_and_release_leg_succeeded(self):
        # A release leg also runs after its build failed, and can then publish the packages an
        # earlier attempt built; checksums of those must not be published as the new build's.
        checksums = job('release-dashmate-checksums')
        needs = re.search(r'\n    needs:((?:\n      - \S+)+|[^\n]+)', checksums).group(1)
        self.assertIn('build-dashmate-packages', needs)
        self.assertIn('release-dashmate-packages', needs)
        condition = re.search(r'\n    if: >-\n((?:      .*\n)+)', checksums).group(1).split()
        self.assertEqual(' '.join(condition), "!cancelled() && github.event_name == 'release'"
                         " && needs.build-dashmate-packages.result == 'success'"
                         " && needs.release-dashmate-packages.result == 'success'")

    def test_should_publish_checksums_from_a_job_that_runs_no_repository_code(self):
        checksums = job('release-dashmate-checksums')
        uses = re.findall(r'uses: (\S+)', checksums)
        self.assertEqual(len(uses), 1)
        self.assertRegex(uses[0], r'^actions/download-artifact@[0-9a-f]{40}$')
        self.assertNotIn('checkout', checksums)
        self.assertIn('!cancelled()', checksums)


if __name__ == '__main__':
    unittest.main()
