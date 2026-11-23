"""Check, sign and attach a release's SHA256SUMS with scripts/release/sign_dashmate_checksums.sh."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / 'scripts/release/sign_dashmate_checksums.sh'
TAG = 'v5.1.0'
KEY = 'ABCDEF0123456789ABCDEF0123456789ABCDEF01'
SUBKEY = '0123456789ABCDEF0123456789ABCDEF01234567'
RUN_ID = 77

PACKAGES = ['dashmate-v5.1.0-3a5da22021-arm64.pkg', 'dashmate-v5.1.0-3a5da22021-linux-x64.tar.gz',
            'dashmate-v5.1.0-3a5da22021-x64.exe', 'dashmate_5.1.0-1_amd64.deb', 'dashmate_5.1.0-1_arm64.deb']

# The release lives in STATE (JSON) and its files in RELEASE_FILES. --jq goes through real jq,
# as gh's own filter would. An API path containing GH_PARTIAL is answered and then fails, as a
# paginated listing does when a later page cannot be read. GH_AFTER_UPLOAD changes the release
# right after the upload. GH_AFTER_DOWNLOAD changes the release once the packages are
# downloaded, as a re-run or another maintainer could while this one signs.
GH_STUB = '''#!/usr/bin/env python3
import fnmatch, hashlib, json, os, re, shutil, subprocess, sys
args = sys.argv[1:]
with open(os.environ['CALLS'], 'a') as log:
    log.write('gh ' + ' '.join(a.replace('\\n', ' ') for a in args) + '\\n')
state_path, files = os.environ['STATE'], os.environ['RELEASE_FILES']
with open(state_path) as handle:
    state = json.load(handle)
release = state['release']
repo = 'repos/dashpay/platform/'

def save():
    with open(state_path, 'w') as handle:
        json.dump(state, handle)

def respond(value, jq):
    text = json.dumps(value)
    if jq is None:
        print(text)
        sys.exit(0)
    result = subprocess.run(['jq', '-r', jq], input=text, capture_output=True, text=True)
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    if os.environ.get('GH_PARTIAL') and os.environ['GH_PARTIAL'] in path:
        print('gh: Server Error (HTTP 502)', file=sys.stderr)
        sys.exit(1)
    sys.exit(result.returncode)

path = ''

def option(name):
    return args[args.index(name) + 1] if name in args else None

def not_found():
    print('gh: Not Found (HTTP 404)', file=sys.stderr)
    sys.exit(1)

def asset(name, data, uploader):
    release['assets'] = [a for a in release['assets'] if a['name'] != name]
    release['assets'].append({'name': name, 'state': 'uploaded', 'uploader': {'login': uploader},
                              'digest': 'sha256:' + hashlib.sha256(data).hexdigest()})
    with open(os.path.join(files, name), 'wb') as handle:
        handle.write(data)

if args[0] == 'api':
    jq = option('--jq')
    path = [a for a in args[1:] if a not in ('--paginate', '--jq', jq)][0]
    if path == repo + 'releases/tags/' + release['tag_name']:
        respond(release, jq)
    if path == repo + 'releases/%d/assets?per_page=100' % release['id']:
        respond(release['assets'], jq)
    if path.startswith(repo + 'actions/workflows/release.yml/runs?'):
        query = dict(part.split('=', 1) for part in path.split('?', 1)[1].split('&'))
        runs = [run for run in state['runs'] if query.get('event', run['event']) == run['event']
                and query.get('branch', run['head_branch']) == run['head_branch']]
        respond({'workflow_runs': runs[:int(query.get('per_page', 30))]}, jq)
    match = re.fullmatch(repo + r'actions/runs/(\\d+)/jobs\\?per_page=100', path)
    if match and match.group(1) in state['jobs']:
        respond({'jobs': state['jobs'][match.group(1)]}, jq)
    not_found()

if args[:2] == ['release', 'download']:
    if args[2] != release['tag_name'] or option('--repo') != 'dashpay/platform':
        not_found()
    patterns = [args[i + 1] for i, a in enumerate(args) if a == '--pattern']
    names = [a['name'] for a in release['assets'] if any(fnmatch.fnmatchcase(a['name'], p) for p in patterns)]
    if not names:
        print('no assets match the file pattern', file=sys.stderr)
        sys.exit(1)
    os.makedirs(option('--dir'), exist_ok=True)
    for name in names:
        shutil.copy(os.path.join(files, name), os.path.join(option('--dir'), name))
    change = os.environ.get('GH_AFTER_DOWNLOAD')
    if change and not state.get('changed'):
        state['changed'] = True
        if change == 'signed':
            asset('SHA256SUMS.asc', b'another signature', 'maintainer')
        if change == 'sums-replaced':
            asset('SHA256SUMS', b'0' * 64 + b'  dashmate_5.1.0-2_amd64.deb\\n', 'github-actions[bot]')
        if change == 'package-replaced':
            asset('dashmate_5.1.0-1_amd64.deb', b'rebuilt', 'github-actions[bot]')
        if change == 'run-restarted':
            state['runs'][0]['status'] = 'in_progress'
        save()
    sys.exit(0)

if args[:2] == ['release', 'upload']:
    if args[2] != release['tag_name'] or option('--repo') != 'dashpay/platform':
        not_found()
    name = os.path.basename(args[3])
    if any(a['name'] == name for a in release['assets']) and '--clobber' not in args:
        print('HTTP 422: Validation Failed (already_exists)', file=sys.stderr)
        sys.exit(1)
    with open(args[3], 'rb') as handle:
        data = handle.read()
    asset(name, b'tampered' if os.environ.get('GH_UPLOAD_TAMPERED') else data, 'maintainer')
    if os.environ.get('GH_AFTER_UPLOAD') == 'package-replaced':
        asset('dashmate_5.1.0-1_amd64.deb', b'rebuilt', 'github-actions[bot]')
    save()
    sys.exit(0)

print('gh stub: unexpected call', file=sys.stderr)
sys.exit(2)
'''

# A signature records who signed and the sha256 of what was signed. GPG_SIGNER makes gpg sign
# with another key than the one asked for; GPG_SUBKEY reports a signing subkey of the primary;
# GPG_EXPIRED, GPG_TWO_SIGNATURES and GPG_EXTRA_FIELD shape the status lines as gpg would for
# an expired key, a gpg.conf that adds a second signer, and a later gpg version; GPG_BAD_SIGNATURE
# signs other bytes, so the verification fails.
GPG_STUB = '''#!/usr/bin/env python3
import hashlib, os, sys
args = sys.argv[1:]
with open(os.environ['CALLS'], 'a') as log:
    log.write('gpg ' + ' '.join(args) + '\\n')

def digest(path):
    with open(path, 'rb') as handle:
        return hashlib.sha256(handle.read()).hexdigest()

if len(args) == 7 and args[:3] == ['--armor', '--detach-sign', '--local-user'] and args[4] == '--output':
    if os.environ.get('GPG_SIGN_FAILS'):
        print('gpg: signing failed: No secret key', file=sys.stderr)
        sys.exit(2)
    with open(args[5], 'w') as handle:
        handle.write('-----BEGIN PGP SIGNATURE-----\\n%s\\n%s\\n-----END PGP SIGNATURE-----\\n'
                     % (os.environ.get('GPG_SIGNER', args[3]),
                        'f' * 64 if os.environ.get('GPG_BAD_SIGNATURE') else digest(args[6])))
    sys.exit(0)

if len(args) == 5 and args[:3] == ['--status-fd', '1', '--verify']:
    with open(args[3]) as handle:
        lines = handle.read().splitlines()
    if len(lines) != 4 or lines[2] != digest(args[4]):
        print('[GNUPG:] NEWSIG')
        print('[GNUPG:] BADSIG 0123456789ABCDEF Maintainer')
        print('gpg: BAD signature', file=sys.stderr)
        sys.exit(1)
    signer = lines[1]
    for _ in range(2 if os.environ.get('GPG_TWO_SIGNATURES') else 1):
        print('[GNUPG:] NEWSIG')
        print('[GNUPG:] %s %s Maintainer <maintainer@example.org>'
              % ('EXPKEYSIG' if os.environ.get('GPG_EXPIRED') else 'GOODSIG', signer[-16:]))
        print('[GNUPG:] VALIDSIG %s 2026-11-23 1795000000 0 4 0 1 10 00 %s%s'
              % (os.environ.get('GPG_SUBKEY', signer), signer, os.environ.get('GPG_EXTRA_FIELD', '')))
    print('gpg: Good signature from "Maintainer"', file=sys.stderr)
    sys.exit(0)

print('gpg stub: unexpected arguments', file=sys.stderr)
sys.exit(2)
'''

# Installed as sha256sum and as shasum. The check follows --check --strict on the inputs these
# tests give it: every line must be well formed, every listed file present with those bytes. HASH_TOOL=shasum makes
# sha256sum report itself as the one macOS ships, which is not GNU's. HASH_CHECK_LENIENT passes
# an empty file, as that one does; duplicated lines pass every one of them.
HASH_STUB = '''#!/usr/bin/env python3
import hashlib, os, re, sys
tool, args = os.path.basename(sys.argv[0]), sys.argv[1:]
with open(os.environ['CALLS'], 'a') as log:
    log.write(tool + ' ' + ' '.join(args) + '\\n')

def digest(path):
    with open(path, 'rb') as handle:
        return hashlib.sha256(handle.read()).hexdigest()

if tool == 'sha256sum' and args == ['--version']:
    print('sha256sum (Darwin) 1.0' if os.environ.get('HASH_TOOL') == 'shasum' else 'sha256sum (GNU coreutils) 9.4')
    sys.exit(0)
if tool == 'shasum':
    if args[:2] != ['-a', '256']:
        sys.exit(2)
    args = args[2:]
if len(args) == 1:
    print('%s  %s' % (digest(args[0]), args[0]))
    sys.exit(0)
if args[:2] != ['--check', '--strict'] or len(args) != 3:
    print('hash stub: unexpected arguments', file=sys.stderr)
    sys.exit(2)
with open(args[2]) as handle:
    lines = handle.read().splitlines()
good = bool(lines) or bool(os.environ.get('HASH_CHECK_LENIENT'))
for line in lines:
    match = re.fullmatch(r'([0-9a-f]{64})  (\\S+)', line)
    if not match:
        print('%s: improperly formatted SHA256 checksum line' % args[2], file=sys.stderr)
        good = False
    elif not os.path.isfile(match.group(2)):
        print('%s: FAILED open or read' % match.group(2))
        good = False
    elif digest(match.group(2)) != match.group(1):
        print('%s: FAILED' % match.group(2))
        good = False
    else:
        print('%s: OK' % match.group(2))
sys.exit(0 if good else 1)
'''


def sha256(data):
    return hashlib.sha256(data).hexdigest()


class ReleaseFixture(unittest.TestCase):
    """A published release with a stubbed gh, gpg and hash tools; no tests of its own."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / 'bin').mkdir()
        for name, body in [('gh', GH_STUB), ('gpg', GPG_STUB), ('sha256sum', HASH_STUB), ('shasum', HASH_STUB)]:
            (self.tmp / 'bin' / name).write_text(body)
            (self.tmp / 'bin' / name).chmod(0o755)
        (self.tmp / 'temp').mkdir()
        self.files = self.tmp / 'release'
        self.files.mkdir()
        self.packages = {name: name.encode() + b' built' for name in PACKAGES}
        self.release({name: data for name, data in self.packages.items()})

    def release(self, files, sums=None, sums_uploader='github-actions[bot]', extra_assets=(), runs=None, jobs=None):
        """A published release: its files, a SHA256SUMS of `sums` (default: the files), the run behind it."""
        for name, data in files.items():
            (self.files / name).write_bytes(data)
        listed = files if sums is None else sums
        text = ''.join(f'{sha256(data)}  {name}\n' for name, data in sorted(listed.items())).encode()
        (self.files / 'SHA256SUMS').write_bytes(text)
        # GitHub lists assets in upload order, not by name.
        assets = [{'name': name, 'state': 'uploaded', 'uploader': {'login': 'github-actions[bot]'},
                   'digest': 'sha256:' + sha256(data)} for name, data in reversed(list(files.items()))]
        assets.append({'name': 'SHA256SUMS', 'state': 'uploaded', 'uploader': {'login': sums_uploader},
                       'digest': 'sha256:' + sha256(text)})
        assets.append({'name': 'DashSDKFFI-5.1.0.xcframework.zip', 'state': 'uploaded',
                       'uploader': {'login': 'github-actions[bot]'}, 'digest': 'sha256:' + '0' * 64})
        assets += list(extra_assets)
        state = {
            'release': {'id': 123, 'tag_name': TAG, 'assets': assets},
            'runs': runs if runs is not None else [
                {'id': RUN_ID, 'head_branch': TAG, 'event': 'release', 'status': 'completed'}],
            'jobs': jobs if jobs is not None else {str(RUN_ID): [
                {'name': 'Release Dashmate packages (deb, ubuntu-24.04)', 'status': 'completed',
                 'conclusion': 'success'},
                {'name': 'Release Dashmate checksums', 'status': 'completed', 'conclusion': 'success'}]},
        }
        (self.tmp / 'state.json').write_text(json.dumps(state))

    def state(self):
        return json.loads((self.tmp / 'state.json').read_text())

    def edit_asset(self, name, **fields):
        state = self.state()
        for asset in state['release']['assets']:
            if asset['name'] == name:
                asset.update(fields)
        (self.tmp / 'state.json').write_text(json.dumps(state))

    def sign(self, *options, **env):
        # /bin/bash: maintainers on macOS may run it under Apple's bash 3.2.
        # A locale whose collation differs from C's, where the system has it.
        env = dict(os.environ, PATH=f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}", LC_ALL='en_US.UTF-8',
                   TMPDIR=str(self.tmp / 'temp'), STATE=str(self.tmp / 'state.json'),
                   RELEASE_FILES=str(self.files), CALLS=str(self.tmp / 'calls'), **env)
        if not options:
            options = (f'--tag={TAG}', f'--key={KEY}')
        return subprocess.run(['/bin/bash', str(SCRIPT), *options], capture_output=True, text=True, env=env)

    def calls(self, tool=None):
        path = self.tmp / 'calls'
        lines = path.read_text().splitlines() if path.exists() else []
        return [line for line in lines if tool is None or line.split(' ', 1)[0] == tool]

    def uploads(self):
        return [call for call in self.calls('gh') if call.startswith('gh release upload')]

    def signings(self):
        return [call for call in self.calls('gpg') if '--detach-sign' in call]

    def assertRefused(self, result, message):
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('Error: ', result.stderr)
        self.assertIn(message, result.stderr)
        self.assertEqual(self.uploads(), [])



@unittest.skipUnless(shutil.which('jq'), 'needs jq')
class SignDashmateChecksumsTests(ReleaseFixture):
    def test_should_check_and_sign_but_not_upload_without_the_flag(self):
        result = self.sign()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.uploads(), [])
        self.assertIn(f'scripts/release/sign_dashmate_checksums.sh --tag={TAG} --key={KEY} --upload', result.stdout)
        # every line checked strictly, nothing missing skipped, and the downloaded file signed
        self.assertEqual([c for c in self.calls('sha256sum') if '--check' in c], ['sha256sum --check --strict SHA256SUMS'])
        [signing] = self.signings()
        self.assertTrue(signing.startswith(f'gpg --armor --detach-sign --local-user {KEY} --output '), signing)
        self.assertTrue(signing.endswith('/release/SHA256SUMS'), signing)
        self.assertEqual(len([c for c in self.calls('gpg') if '--verify' in c]), 1)
        # the download went into a directory of its own, removed afterwards
        downloads = [c for c in self.calls('gh') if c.startswith('gh release download')]
        self.assertEqual(len(downloads), 1)
        self.assertIn(f"--dir {self.tmp / 'temp'}/dashmate-sign.", downloads[0])
        self.assertEqual(list((self.tmp / 'temp').iterdir()), [])

    def test_should_take_the_fingerprint_with_spaces_and_in_lower_case(self):
        spaced = ' '.join(KEY[i:i + 4] for i in range(0, 40, 4)).lower()

        result = self.sign(f'--tag={TAG}', f'--key={spaced}')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f'--local-user {KEY} ', self.signings()[0])

    def test_should_attach_the_signature_and_read_it_back_with_the_flag(self):
        result = self.sign(f'--tag={TAG}', f'--key={KEY}', '--upload')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        [upload] = self.uploads()
        self.assertTrue(upload.endswith('/release/SHA256SUMS.asc --repo dashpay/platform'), upload)
        self.assertNotIn('--clobber', upload)
        attached = [a['name'] for a in self.state()['release']['assets']]
        self.assertIn('SHA256SUMS.asc', attached)
        # read back from the release after the upload
        downloads = [c for c in self.calls('gh') if c.startswith('gh release download')]
        self.assertEqual(len(downloads), 2)
        self.assertIn('--pattern SHA256SUMS.asc', downloads[1])
        self.assertIn(f'Attached SHA256SUMS.asc to {TAG}', result.stdout)
        self.assertEqual(list((self.tmp / 'temp').iterdir()), [])

    def test_should_refuse_wrong_options_before_reaching_github(self):
        cases = {
            'no tag': (f'--key={KEY}',),
            'not a release tag': ('--tag=5.1.0', f'--key={KEY}'),
            'a tag with more after it': (f'--tag={TAG}/other', f'--key={KEY}'),
            'a tag with a second line': (f'--tag={TAG}\n../other', f'--key={KEY}'),
            'a key with a second line': (f'--tag={TAG}', f'--key={KEY}\n{SUBKEY}'),
            'no key': (f'--tag={TAG}',),
            'a short key id, which can match another key': (f'--tag={TAG}', '--key=EDBC0AA6BC1AA13C'),
            'an unknown option': (f'--tag={TAG}', f'--key={KEY}', '--force'),
        }
        for case, options in cases.items():
            with self.subTest(case):
                result = self.sign(*options)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn('Error: ', result.stderr)
                self.assertEqual(self.calls(), [])

    def test_should_refuse_a_release_that_is_already_signed(self):
        for signature in ['SHA256SUMS.asc', 'SHA256SUMS.sig', 'SHA256SUMS.minisig']:
            with self.subTest(signature):
                self.release(self.packages, extra_assets=[
                    {'name': signature, 'state': 'uploaded', 'uploader': {'login': 'maintainer'}, 'digest': None}])

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', '--upload')

                self.assertRefused(result, f'{TAG} already carries {signature}')
                self.assertEqual(self.signings(), [])

    def test_should_refuse_a_sha256sums_the_release_workflow_did_not_attach(self):
        cases = {
            'no SHA256SUMS': ({'state': 'deleted'}, f'{TAG} has no SHA256SUMS'),
            'uploaded by a person': ({'uploader': {'login': 'maintainer'}}, 'not by the release workflow'),
            'an upload that did not finish': ({'state': 'open'}, 'not fully uploaded'),
        }
        for case, (fields, message) in cases.items():
            with self.subTest(case):
                self.release(self.packages)
                if case == 'no SHA256SUMS':
                    state = self.state()
                    state['release']['assets'] = [a for a in state['release']['assets'] if a['name'] != 'SHA256SUMS']
                    (self.tmp / 'state.json').write_text(json.dumps(state))
                else:
                    self.edit_asset('SHA256SUMS', **fields)

                result = self.sign()

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])

    def test_should_refuse_until_the_release_run_has_finished_green(self):
        jobs = {str(RUN_ID): [{'name': 'Release Dashmate checksums', 'status': 'completed', 'conclusion': 'failure'}]}
        cases = {
            'no run for the tag': ({'runs': []}, f'found no release.yml run for {TAG}'),
            # a re-run attempt in progress can still replace packages or SHA256SUMS
            'an attempt in progress': ({'runs': [{'id': RUN_ID, 'head_branch': TAG, 'event': 'release',
                                                  'status': 'in_progress'}]}, f'{RUN_ID} (in_progress)'),
            # a re-published release starts another run, whose legs upload by tag
            'an older run still going': ({'runs': [
                {'id': RUN_ID, 'head_branch': TAG, 'event': 'release', 'status': 'completed'},
                {'id': RUN_ID - 1, 'head_branch': TAG, 'event': 'release', 'status': 'queued'}]},
                f'{RUN_ID - 1} (queued)'),
            'the checksums job failed': ({'jobs': jobs}, 'did not succeed (failure)'),
            # runs of other tags are not this release's; of this tag's runs the newest counts
            'the newest run of the tag failed its checksums': ({
                'runs': [{'id': 90, 'head_branch': 'v5.0.9', 'event': 'release', 'status': 'completed'},
                         {'id': 78, 'head_branch': TAG, 'event': 'release', 'status': 'completed'},
                         {'id': RUN_ID, 'head_branch': TAG, 'event': 'release', 'status': 'completed'}],
                'jobs': {'90': [{'name': 'Release Dashmate checksums', 'status': 'completed', 'conclusion': 'success'}],
                         '78': [{'name': 'Release Dashmate checksums', 'status': 'completed', 'conclusion': 'failure'}],
                         str(RUN_ID): [{'name': 'Release Dashmate checksums', 'status': 'completed',
                                        'conclusion': 'success'}]}},
                'job of run 78 did not succeed (failure)'),
            'no checksums job': ({'jobs': {str(RUN_ID): []}}, 'did not succeed (not found)'),
        }
        for case, (fields, message) in cases.items():
            with self.subTest(case):
                self.release(self.packages, **fields)

                result = self.sign()

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])

    def test_should_refuse_a_listing_that_failed_part_way(self):
        for listing, message in [('/assets?', f'could not list the assets of {TAG}'),
                                 ('/runs?', f'could not list the release.yml runs of {TAG}'),
                                 ('/jobs?', f'could not list the jobs of release.yml run {RUN_ID}')]:
            with self.subTest(listing):
                (self.tmp / 'calls').unlink(missing_ok=True)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', '--upload', GH_PARTIAL=listing)

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])

    def test_should_refuse_packages_that_do_not_match_every_line(self):
        deb = 'dashmate_5.1.0-1_amd64.deb'
        others = {name: data for name, data in self.packages.items() if name != deb}
        cases = {
            'a package with other bytes': (dict(self.packages, **{deb: b'other bytes'}), self.packages,
                                           'do not match its SHA256SUMS'),
            'a listed package not attached': (others, self.packages, 'do not match its SHA256SUMS'),
            'an attached package not listed': (self.packages, others, 'is not one line per dashmate package it serves'),
        }
        for case, (files, listed, message) in cases.items():
            with self.subTest(case):
                for leftover in self.files.iterdir():
                    leftover.unlink()
                self.release(files, sums=listed)

                result = self.sign()

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])
        with self.subTest('a line that is not <sha256>  <name>'):
            self.release(self.packages)
            sums = self.files / 'SHA256SUMS'
            sums.write_bytes(sums.read_bytes() + b'not a checksum line\n')
            self.edit_asset('SHA256SUMS', digest='sha256:' + sha256(sums.read_bytes()))

            self.assertRefused(self.sign(), 'do not match its SHA256SUMS')
        for case, text in [('an empty SHA256SUMS', b''), ('a duplicated line', None)]:
            with self.subTest(case):
                self.release(self.packages)
                sums = self.files / 'SHA256SUMS'
                if text is None:
                    text = sums.read_bytes() + sums.read_bytes().splitlines(keepends=True)[0]
                sums.write_bytes(text)
                self.edit_asset('SHA256SUMS', digest='sha256:' + sha256(text))

                result = self.sign(HASH_CHECK_LENIENT='1')

                self.assertRefused(result, 'is not one line per dashmate package it serves')
                self.assertEqual(self.signings(), [])
                self.assertEqual(list((self.tmp / 'temp').iterdir()), [])
        with self.subTest('a package listed twice and another not at all'):
            self.release(self.packages)
            sums = self.files / 'SHA256SUMS'
            lines = sums.read_bytes().splitlines(keepends=True)
            text = b''.join(sorted(lines[:-1] + lines[:1], key=lambda line: line.split(b'  ')[1]))
            sums.write_bytes(text)
            self.edit_asset('SHA256SUMS', digest='sha256:' + sha256(text))

            self.assertRefused(self.sign(), 'is not one line per dashmate package it serves')
        with self.subTest('a download that is not the file the release lists'):
            self.release(self.packages)
            self.edit_asset('SHA256SUMS', digest='sha256:' + 'f' * 64)

            self.assertRefused(self.sign(), 'is not the file the release lists')

    def test_should_refuse_a_signature_it_cannot_verify(self):
        with self.subTest('gpg cannot sign'):
            self.assertRefused(self.sign(GPG_SIGN_FAILS='1'), f'gpg could not sign SHA256SUMS with {KEY}')
        with self.subTest('a signature that does not verify'):
            self.assertRefused(self.sign(GPG_BAD_SIGNATURE='1'), 'gpg did not verify')
        with self.subTest('signed with another key'):
            self.assertRefused(self.sign(GPG_SIGNER=SUBKEY),
                               f'was not made by {KEY} but by {SUBKEY} (primary key {SUBKEY})')
        with self.subTest('a key that has expired'):
            self.assertRefused(self.sign(GPG_EXPIRED='1'), 'no good signature')
        with self.subTest('a second signature, e.g. from gpg.conf'):
            self.assertRefused(self.sign(GPG_TWO_SIGNATURES='1'), 'exactly one signature')

    def test_should_accept_a_signature_made_by_a_subkey_of_the_key(self):
        for key in [KEY, SUBKEY]:
            with self.subTest(key=key):
                # a later gpg may append fields to VALIDSIG; the primary key stays the twelfth
                result = self.sign(f'--tag={TAG}', f'--key={key}', GPG_SIGNER=KEY, GPG_SUBKEY=SUBKEY,
                                   GPG_EXTRA_FIELD=' 0' * 2)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_stop_before_uploading_when_the_release_changed_meanwhile(self):
        cases = {
            'another maintainer signed it': ('signed', f'{TAG} already carries SHA256SUMS.asc'),
            'a re-run replaced SHA256SUMS': ('sums-replaced', 'changed while it was being signed'),
            'a re-run replaced a package': ('package-replaced', 'are not the ones its SHA256SUMS lists'),
            'a re-run started': ('run-restarted', f'{RUN_ID} (in_progress)'),
        }
        for case, (change, message) in cases.items():
            with self.subTest(case):
                self.release(self.packages)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', '--upload', GH_AFTER_DOWNLOAD=change)

                self.assertRefused(result, message)

    def test_should_fail_when_a_package_changes_right_after_the_upload(self):
        result = self.sign(f'--tag={TAG}', f'--key={KEY}', '--upload', GH_AFTER_UPLOAD='package-replaced')

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('are not the ones its SHA256SUMS lists', result.stderr)
        self.assertEqual(len(self.uploads()), 1)

    def test_should_fail_when_the_attached_signature_does_not_verify(self):
        result = self.sign(f'--tag={TAG}', f'--key={KEY}', '--upload', GH_UPLOAD_TAMPERED='1')

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(f'{TAG} serves another SHA256SUMS.asc than the one signed here', result.stderr)
        self.assertEqual(len(self.uploads()), 1)

    def test_should_check_with_shasum_where_sha256sum_is_not_gnu(self):
        result = self.sign(HASH_TOOL='shasum')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('shasum -a 256 --check --strict SHA256SUMS', self.calls('shasum'))
        self.assertEqual(self.calls('sha256sum'), ['sha256sum --version'])

    def test_should_be_clean_under_shellcheck(self):
        if not shutil.which('shellcheck'):
            self.skipTest('needs shellcheck')
        result = subprocess.run(['shellcheck', str(SCRIPT)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout)


@unittest.skipUnless(all(shutil.which(tool) for tool in ['jq', 'gpg', 'gpgconf', 'gpg-agent']),
                     'needs jq, gpg and gpg-agent')
class SignWithGnuPGTests(ReleaseFixture):
    """The real gpg, so the status lines the script reads are the ones gpg writes."""

    def setUp(self):
        super().setUp()
        (self.tmp / 'bin/gpg').unlink()
        # Short, because gpg-agent's socket lives in the home directory.
        self.home = Path(tempfile.mkdtemp(dir='/tmp', prefix='gpg'))
        self.addCleanup(shutil.rmtree, self.home, True)
        self.addCleanup(subprocess.run, ['gpgconf', '--remove-socketdir'], env=self.gpg_env(), capture_output=True)
        self.addCleanup(subprocess.run, ['gpgconf', '--kill', 'all'], env=self.gpg_env(), capture_output=True)
        self.home.chmod(0o700)
        # A primary key that only certifies, signing through a subkey, as maintainers keep them.
        self.primary = self.new_key('Maintainer <maintainer@example.org>', 'cert')
        self.gpg('--quick-add-key', self.primary, 'ed25519', 'sign', 'never')
        self.subkey = self.fingerprints(self.primary)[1]
        self.other = self.new_key('Other <other@example.org>', 'sign')

    def gpg_env(self):
        return dict(os.environ, GNUPGHOME=str(self.home))

    def gpg(self, *args):
        return subprocess.run(['gpg', '--batch', '--pinentry-mode', 'loopback', '--passphrase', '', *args],
                              env=self.gpg_env(), check=True, capture_output=True, text=True).stdout

    def fingerprints(self, key):
        listing = self.gpg('--with-colons', '--list-keys', key)
        return [line.split(':')[9] for line in listing.splitlines() if line.startswith('fpr:')]

    def new_key(self, uid, usage):
        self.gpg('--quick-gen-key', uid, 'ed25519', usage, 'never')
        return self.fingerprints(uid)[0]

    def test_should_take_the_primary_key_or_its_signing_subkey(self):
        for key in [self.primary, self.subkey]:
            with self.subTest(key=key):
                result = self.sign(f'--tag={TAG}', f'--key={key}', GNUPGHOME=str(self.home))
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_refuse_a_second_signature_from_gpg_conf(self):
        (self.home / 'gpg.conf').write_text(f'local-user {self.other}\n')

        result = self.sign(f'--tag={TAG}', f'--key={self.primary}', GNUPGHOME=str(self.home))

        self.assertRefused(result, 'exactly one signature')


if __name__ == '__main__':
    unittest.main()
