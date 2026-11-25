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
# A signer the script accepts, and a key it does not.
KEY = '19E460D55429A91B40E9E7C5EDBC0AA6BC1AA13C'
STRANGER = 'C0FFEE0123456789ABCDEF0123456789ABCDEF01'
SUBKEY = '0123456789ABCDEF0123456789ABCDEF01234567'
RUN_ID = 77
HEAD_SHA = 'bb78fa2220ae0d58f5a9b5c0f6b0027a292cc3d0'
OTHER_SHA = '3d8365918935a2cad03b04fa70d8bb9d07f9a849'
# The release PR's merge commit, which the tag names and the release run built.
COMMIT = f'--commit={HEAD_SHA}'
CHECKSUMS_JOB = 501


# The dashmate package jobs of a release.yml attempt, all finished and green.
DASHMATE_JOBS = [
    {'id': CHECKSUMS_JOB - 2, 'name': 'Build Dashmate packages (deb, ubuntu-24.04)', 'status': 'completed',
     'conclusion': 'success'},
    {'id': CHECKSUMS_JOB - 1, 'name': 'Release Dashmate packages (deb, ubuntu-24.04)', 'status': 'completed',
     'conclusion': 'success'},
    {'id': CHECKSUMS_JOB, 'name': 'Release Dashmate checksums', 'status': 'completed', 'conclusion': 'success'}]


def dashmate_jobs(offset=0, **changes):
    """DASHMATE_JOBS with ids moved by `offset` and some jobs changed, by name."""
    return [dict(job, id=job['id'] + offset, **changes.get(job['name'], {})) for job in DASHMATE_JOBS]


def checksums_log(*records):
    """The log of a Release Dashmate checksums job: its script, as GitHub shows it, then the records."""
    lines = ['\ufeff2026-11-23T10:00:00.0000000Z ##[group]Run set -euo pipefail',
             '2026-11-23T10:00:00.0100000Z \x1b[36;1mecho "SHA256SUMS attached: asset ${id} sha256 ${sum}"\x1b[0m',
             '2026-11-23T10:00:00.0200000Z ##[endgroup]',
             # output that quotes the record's wording is not a record
             '2026-11-23T10:00:00.0300000Z ##[error]dashmate.deb SHA256SUMS attached: asset 999 sha256 ' + 'e' * 64]
    lines += [f'2026-11-23T10:00:01.0000000Z SHA256SUMS attached: asset {asset} sha256 {sum_}'
              for asset, sum_ in records]
    return '\n'.join(lines) + '\n'

PACKAGES = ['dashmate-v5.1.0-3a5da22021-arm64.pkg', 'dashmate-v5.1.0-3a5da22021-linux-x64.tar.gz',
            'dashmate-v5.1.0-3a5da22021-x64.exe', 'dashmate_5.1.0-1_amd64.deb', 'dashmate_5.1.0-1_arm64.deb']

# The release lives in STATE (JSON) and its files in RELEASE_FILES. --jq goes through real jq,
# as gh's own filter would. An API path containing GH_PARTIAL is answered and then fails, as a
# paginated listing does when a later page cannot be read. GH_AFTER_UPLOAD changes the release
# right after the upload (a package replaced, the tag moved, the run restarted, the checksums job
# re-run and failed or passed, or a newer run of another or the same commit); GH_READ_BACK_FAILS fails downloading the signature back, and
# GH_UPLOAD_FAILS the upload itself. GH_AFTER_DOWNLOAD changes the release once the packages are
# downloaded, as a re-run or another maintainer could while this one signs. GH_UPLOAD_FAILS fails
# the upload: with 1 nothing is attached, with `created` or `partial` the asset is created with
# the uploaded bytes, completely or not, with `signed-first` another signature is attached just
# before it, and with `unlisted` the assets cannot be listed afterwards.
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
    release['assets'].append({'id': max(a['id'] for a in release['assets']) + 1, 'name': name,
                              'state': 'uploaded', 'uploader': {'login': uploader},
                              'digest': 'sha256:' + hashlib.sha256(data).hexdigest()})
    with open(os.path.join(files, name), 'wb') as handle:
        handle.write(data)

def new_attempt(status):
    """Re-run the newest run: a new attempt whose jobs are copies under new ids, as GitHub makes
    them, the dashmate ones `status` (None: finished, carried over)."""
    run = state['runs'][0]
    old = '%d/%d' % (run['id'], run['run_attempt'])
    run['run_attempt'] += 1
    run['status'] = 'completed' if status is None else 'queued'
    run.pop('attempt_status', None)
    jobs = []
    for job in state['jobs'][old]:
        copy = dict(job, id=job['id'] + 100)
        if str(job['id']) in state['logs']:
            state['logs'][str(copy['id'])] = state['logs'][str(job['id'])]
        if status is not None and 'Dashmate' in job['name']:
            copy.update(status=status, conclusion=None)
        jobs.append(copy)
    state['jobs']['%d/%d' % (run['id'], run['run_attempt'])] = jobs

def apply(change):
    """A change to the release or its runs, as a re-run or another maintainer could make it."""
    if change == 'signed':
        asset('SHA256SUMS.asc', b'another signature', 'maintainer')
    if change == 'sums-replaced':
        asset('SHA256SUMS', b'0' * 64 + b'  dashmate_5.1.0-2_amd64.deb\\n', 'github-actions[bot]')
    if change == 'package-replaced':
        asset('dashmate_5.1.0-1_amd64.deb', b'rebuilt', 'github-actions[bot]')
    if change == 'sums-reuploaded':
        with open(os.path.join(files, 'SHA256SUMS'), 'rb') as handle:
            asset('SHA256SUMS', handle.read(), 'github-actions[bot]')
    if change == 'job-rerun':
        new_attempt(None)
    if change == 'run-restarted':
        new_attempt('queued')
    if change == 'sdk-finished':
        state['runs'][0]['status'] = 'completed'
    if change == 'tag-moved':
        state['tag_commit'] = '3d8365918935a2cad03b04fa70d8bb9d07f9a849'

def rerun_after(**counters):
    """Request a re-run of the newest run after the n-th call of a kind, n given by the named variable."""
    for counter, variable in counters.items():
        state[counter] = state.get(counter, 0) + 1
        if os.environ.get(variable) and state[counter] == int(os.environ[variable]):
            new_attempt('queued')
        save()

# A change the gpg stub left to happen while the maintainer signed (GPG_AFTER_SIGN).
if state.get('after_sign'):
    apply(state.pop('after_sign'))
    save()

if args[0] == 'api':
    # GH_ESCAPE_GUARD: a gh that refuses a response with terminal escape sequences unless
    # --allow-escape-sequences is given; any other gh does not know that option.
    guard = bool(os.environ.get('GH_ESCAPE_GUARD'))
    if '--help' in args:
        # As long as gh's own help, so a reader that stops at the first match closes the pipe early.
        print('Usage: gh api <endpoint> [flags]')
        if guard:
            print('      --allow-escape-sequences   Allow terminal escape sequences in the response')
        for number in range(5000):
            print(f'  --option-{number} string   Another option of gh api')
        sys.exit(0)
    allow = '--allow-escape-sequences' in args
    if allow and not guard:
        print('unknown flag: --allow-escape-sequences', file=sys.stderr)
        sys.exit(1)
    jq = option('--jq')
    path = [a for a in args[1:] if a not in ('--paginate', '--allow-escape-sequences', '--jq', jq)][0]
    if path == repo + 'releases/tags/' + release['tag_name']:
        respond(release, jq)
    if path == repo + 'releases/%d/assets?per_page=100' % release['id']:
        if state.get('unlisted'):
            print('gh: Server Error (HTTP 502)', file=sys.stderr)
            sys.exit(1)
        assets = list(release['assets'])
        rerun_after(assets_listing='GH_RERUN_AFTER_ASSET_LISTING')
        respond(assets, jq)
    if path == repo + 'commits/refs/tags/' + release['tag_name']:
        if state['tag_commit'] is None:
            print('gh: No commit found for SHA: refs/tags/%s (HTTP 422)' % release['tag_name'], file=sys.stderr)
            sys.exit(1)
        tagged = state['tag_commit']
        rerun_after(tag_lookup='GH_RERUN_AFTER_TAG_LOOKUP')
        respond({'sha': tagged}, jq)
    if path.startswith(repo + 'actions/workflows/release.yml/runs?'):
        query = dict(part.split('=', 1) for part in path.split('?', 1)[1].split('&'))
        runs = [dict(run) for run in state['runs'] if query.get('event', run['event']) == run['event']
                and query.get('branch', run['head_branch']) == run['head_branch']]
        # GH_RERUN_WHILE_LISTED: right after the first listing a re-run of the newest run is queued
        # (1), has already finished (finished), or is queued or running before the attempt number
        # moves on (stale, stale-running)
        rerun = os.environ.get('GH_RERUN_WHILE_LISTED')
        if rerun and not state.get('rerun_requested'):
            state['rerun_requested'] = True
            if rerun.startswith('stale'):
                state['runs'][0].update(status='in_progress' if rerun == 'stale-running' else 'queued',
                                        attempt_status='completed')
            else:
                new_attempt(None if rerun == 'finished' else 'queued')
            save()
        # GH_FINISH_WHILE_LISTED: the newest run finishes, its SDK builds done, right after the first listing
        if os.environ.get('GH_FINISH_WHILE_LISTED') and not state.get('finished'):
            state['finished'] = True
            state['runs'][0]['status'] = 'completed'
            save()
        # GH_RELIST_FAILS: every second listing, the one that checks nothing changed, answers and then fails
        state['listings'] = state.get('listings', 0) + 1
        save()
        if os.environ.get('GH_RELIST_FAILS') and state['listings'] % 2 == 0:
            text = json.dumps({'workflow_runs': runs})
            sys.stdout.write(subprocess.run(['jq', '-r', jq], input=text, capture_output=True, text=True).stdout)
            print('gh: Server Error (HTTP 502)', file=sys.stderr)
            sys.exit(1)
        respond({'workflow_runs': runs[:int(query.get('per_page', 30))]}, jq)
    match = re.fullmatch(repo + r'actions/runs/(\\d+)', path)
    if match:
        run = next(run for run in state['runs'] if run['id'] == int(match.group(1)))
        respond({'id': run['id'], 'status': run['status'], 'run_attempt': run['run_attempt']}, jq)
    match = re.fullmatch(repo + r'actions/runs/(\\d+)/attempts/(\\d+)', path)
    if match:
        # the newest attempt reports the run's status unless `attempt_status` says otherwise, as
        # when a re-run is queued before the run's attempt number moves on
        run = next(run for run in state['runs'] if run['id'] == int(match.group(1)))
        # GH_FINISH_BEFORE_ATTEMPT_READ: the run finishes, its SDK builds done, before its attempt is read
        if os.environ.get('GH_FINISH_BEFORE_ATTEMPT_READ') and not state.get('finished'):
            state['finished'] = True
            run['status'] = 'completed'
            save()
        newest = int(match.group(2)) == run['run_attempt']
        respond({'run_attempt': int(match.group(2)),
                 'status': run.get('attempt_status', run['status']) if newest else 'completed'}, jq)
    match = re.fullmatch(repo + r'actions/runs/(\\d+)/attempts/(\\d+)/jobs\\?per_page=100', path)
    if match:
        respond({'jobs': state['jobs'].get(match.group(1) + '/' + match.group(2), [])}, jq)
    match = re.fullmatch(repo + r'actions/jobs/(\\d+)/logs', path)
    if match and match.group(1) in state['logs'] and not os.environ.get('GH_NO_LOG'):
        if guard and not allow and '\x1b' in state['logs'][match.group(1)]:
            print('the response contains terminal escape sequences', file=sys.stderr)
            sys.exit(1)
        sys.stdout.write(state['logs'][match.group(1)])
        sys.exit(0)
    not_found()

if args[:2] == ['release', 'download']:
    if args[2] != release['tag_name'] or option('--repo') != 'dashpay/platform':
        not_found()
    patterns = [args[i + 1] for i, a in enumerate(args) if a == '--pattern']
    if os.environ.get('GH_READ_BACK_FAILS') and 'SHA256SUMS.asc' in patterns:
        print('gh: Server Error (HTTP 502)', file=sys.stderr)
        sys.exit(1)
    names = [a['name'] for a in release['assets'] if any(fnmatch.fnmatchcase(a['name'], p) for p in patterns)]
    if not names:
        print('no assets match the file pattern', file=sys.stderr)
        sys.exit(1)
    os.makedirs(option('--dir'), exist_ok=True)
    for name in names:
        shutil.copy(os.path.join(files, name), os.path.join(option('--dir'), name))
    change = os.environ.get('GH_AFTER_DOWNLOAD')
    if change and not state.get('changed') and any(p.startswith('dashmate') for p in patterns):
        state['changed'] = True
        apply(change)
        save()
    sys.exit(0)

if args[:2] == ['release', 'upload']:
    if args[2] != release['tag_name'] or option('--repo') != 'dashpay/platform':
        not_found()
    name = os.path.basename(args[3])
    failure = os.environ.get('GH_UPLOAD_FAILS')
    if failure == 'signed-first':
        asset(name, b'another signature', 'maintainer')
        save()
    if failure in ('1', 'unlisted', 'created', 'partial'):
        if failure in ('created', 'partial'):
            with open(args[3], 'rb') as handle:
                asset(name, handle.read(), 'maintainer')
            if failure == 'partial':
                release['assets'][-1]['state'] = 'starter'
        state['unlisted'] = failure == 'unlisted'
        save()
        print('gh: connection reset by peer', file=sys.stderr)
        sys.exit(1)
    if any(a['name'] == name for a in release['assets']) and '--clobber' not in args:
        print('HTTP 422: Validation Failed (already_exists)', file=sys.stderr)
        sys.exit(1)
    with open(args[3], 'rb') as handle:
        data = handle.read()
    asset(name, b'tampered' if os.environ.get('GH_UPLOAD_TAMPERED') else data, 'maintainer')
    after = os.environ.get('GH_AFTER_UPLOAD')
    if after == 'package-replaced':
        asset('dashmate_5.1.0-1_amd64.deb', b'rebuilt', 'github-actions[bot]')
    if after == 'tag-moved':
        state['tag_commit'] = '3d8365918935a2cad03b04fa70d8bb9d07f9a849'
    if after == 'run-restarted':
        new_attempt('queued')
    if after in ('checksums-job-rerun', 'checksums-job-rerun-green'):
        # a re-run of the checksums job alone makes a new attempt of the run; the job sees the
        # signature and fails, or passes
        new_attempt(None)
        run = state['runs'][0]
        if after == 'checksums-job-rerun':
            next(job for job in state['jobs']['%d/%d' % (run['id'], run['run_attempt'])]
                 if job['name'] == 'Release Dashmate checksums')['conclusion'] = 'failure'
    if after in ('run-of-another-commit', 'newer-run'):
        # a newer run whose dashmate jobs are done
        run = state['runs'][0]
        newer = dict(run, id=run['id'] + 1, run_attempt=1)
        if after == 'run-of-another-commit':
            newer['head_sha'] = '3d8365918935a2cad03b04fa70d8bb9d07f9a849'
        state['runs'].insert(0, newer)
        state['jobs']['%d/1' % newer['id']] = [
            dict(job, id=job['id'] + 1000) for job in state['jobs']['%d/%d' % (run['id'], run['run_attempt'])]]
    save()
    sys.exit(0)

print('gh stub: unexpected call', file=sys.stderr)
sys.exit(2)
'''

# A signature records who signed and the sha256 of what was signed. GPG_AFTER_SIGN leaves a change
# to the release for the gh stub to make on its next call, as while the passphrase is entered. GPG_SIGNER makes gpg sign
# with another key than the one asked for; GPG_SUBKEY reports a signing subkey of the primary;
# GPG_EXPIRED, GPG_TWO_SIGNATURES and GPG_EXTRA_FIELD shape the status lines as gpg would for
# an expired key, a gpg.conf that adds a second signer, and a later gpg version; GPG_BAD_SIGNATURE
# signs other bytes, so the verification fails.
GPG_STUB = '''#!/usr/bin/env python3
import hashlib, json, os, sys
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
    if os.environ.get('GPG_AFTER_SIGN'):
        with open(os.environ['STATE']) as handle:
            state = json.load(handle)
        state['after_sign'] = os.environ['GPG_AFTER_SIGN']
        with open(os.environ['STATE'], 'w') as handle:
            json.dump(state, handle)
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
# tests give it: every line must be well formed, every listed file present with those bytes;
# GNU sha256sum skips blank lines, shasum does not. HASH_TOOL=shasum makes
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
    if not line and tool == 'sha256sum':
        continue
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
        self.script = SCRIPT
        self.files = self.tmp / 'release'
        self.files.mkdir()
        self.packages = {name: name.encode() + b' built' for name in PACKAGES}
        self.release({name: data for name, data in self.packages.items()})

    def release(self, files, sums=None, sums_uploader='github-actions[bot]', extra_assets=(), runs=None, jobs=None,
                receipt=None, tag_commit=HEAD_SHA, logged=(CHECKSUMS_JOB,)):
        """A published release: its files, a SHA256SUMS of `sums` (default: the files), the run behind it.

        `receipt` is what the checksums job's log records it attached, as `(asset id, sha256)`: by
        default this release's SHA256SUMS; False for no record, a list for several, in the log of
        each job in `logged`. `jobs` is keyed by `<run id>/<attempt>`; a run's newest attempt is
        its `run_attempt`, 1 unless given. The record of tool calls starts afresh, so each subtest
        sees only its own.
        """
        (self.tmp / 'calls').unlink(missing_ok=True)
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
        for number, asset in enumerate(assets):
            asset.setdefault('id', 1000 + number)
        sums_asset = next(a for a in assets if a['name'] == 'SHA256SUMS')
        if receipt is None:
            receipt = (sums_asset['id'], sha256(text))
        records = [] if receipt is False else receipt if isinstance(receipt, list) else [receipt]
        state = {
            'release': {'id': 123, 'tag_name': TAG, 'assets': assets},
            'runs': [dict({'run_attempt': 1}, **run) for run in (runs if runs is not None else [
                {'id': RUN_ID, 'head_branch': TAG, 'head_sha': HEAD_SHA, 'event': 'release', 'status': 'completed'}])],
            'jobs': jobs if jobs is not None else {f'{RUN_ID}/1': DASHMATE_JOBS},
            'logs': {str(job): checksums_log(*records) for job in logged},
            # the commit the tag names, None for a tag GitHub cannot resolve
            'tag_commit': tag_commit,
        }
        assert all('/' in key for key in state['jobs']), state['jobs']
        (self.tmp / 'state.json').write_text(json.dumps(state))

    def state(self):
        return json.loads((self.tmp / 'state.json').read_text())

    def accepting(self, *fingerprints):
        """A copy of the script whose list of accepted signers is `fingerprints`."""
        text = SCRIPT.read_text()
        start = text.index('ALLOWED_SIGNERS=(\n')
        end = text.index('\n)\n', start)
        copy = self.tmp / 'sign_dashmate_checksums.sh'
        copy.write_text(text[:start] + 'ALLOWED_SIGNERS=(\n' + ''.join(f'  {f}\n' for f in fingerprints)
                        + text[end + 1:])
        return copy

    def edit_asset(self, name, **fields):
        """Change an asset; a changed SHA256SUMS is recorded as the one the checksums job attached."""
        state = self.state()
        for asset in state['release']['assets']:
            if asset['name'] == name:
                asset.update(fields)
                if name == 'SHA256SUMS' and 'digest' in fields:
                    state['logs'][str(CHECKSUMS_JOB)] = checksums_log((asset['id'], fields['digest'][7:]))
        (self.tmp / 'state.json').write_text(json.dumps(state))

    def sign(self, *options, **env):
        # /bin/bash: maintainers on macOS may run it under Apple's bash 3.2.
        # A locale whose collation differs from C's, where the system has it.
        env = dict(os.environ, PATH=f"{self.tmp / 'bin'}{os.pathsep}{os.environ['PATH']}", LC_ALL='en_US.UTF-8',
                   TMPDIR=str(self.tmp / 'temp'), STATE=str(self.tmp / 'state.json'),
                   RELEASE_FILES=str(self.files), CALLS=str(self.tmp / 'calls'), **env)
        if not options:
            options = (f'--tag={TAG}', f'--key={KEY}', COMMIT)
        return subprocess.run(['/bin/bash', str(self.script), *options], capture_output=True, text=True, env=env)

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
        # nothing was uploaded, so the message must not send the maintainer to delete a signature
        self.assertNotIn('SHA256SUMS.asc is attached', result.stderr)



@unittest.skipUnless(shutil.which('jq'), 'needs jq')
class SignDashmateChecksumsTests(ReleaseFixture):
    def test_should_read_the_job_log_with_any_gh_version(self):
        # gh 2.105 refuses a log with the runner's colour codes unless told to allow them; older
        # versions do not know that option.
        for guard in ['', '1']:
            with self.subTest(refuses_escape_sequences=bool(guard)):
                self.release(self.packages)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GH_ESCAPE_GUARD=guard)

                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(len(self.uploads()), 1)

    def test_should_check_and_sign_but_not_upload_without_the_flag(self):
        result = self.sign()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.uploads(), [])
        self.assertIn(f'scripts/release/sign_dashmate_checksums.sh --tag={TAG} --key={KEY} --commit={HEAD_SHA} --upload',
                      result.stdout)
        # every line checked strictly, nothing missing skipped, and the downloaded file signed
        self.assertEqual([c for c in self.calls('sha256sum') if '--check' in c], ['sha256sum --check --strict SHA256SUMS'])
        [signing] = self.signings()
        self.assertTrue(signing.startswith(f'gpg --armor --detach-sign --local-user {KEY} --output '), signing)
        self.assertTrue(signing.endswith('/release/SHA256SUMS'), signing)
        self.assertEqual(len([c for c in self.calls('gpg') if '--verify' in c]), 1)
        # SHA256SUMS first, then by name each package it lists, into a directory of their own that
        # is removed afterwards
        downloads = [c for c in self.calls('gh') if c.startswith('gh release download')]
        self.assertEqual(len(downloads), 2)
        self.assertEqual(downloads[0].split(' --dir ')[0], f'gh release download {TAG} --repo dashpay/platform --pattern SHA256SUMS')
        self.assertEqual(downloads[1].split(' --dir ')[0], f'gh release download {TAG} --repo dashpay/platform '
                         + ' '.join(f'--pattern {name}' for name in sorted(PACKAGES)))
        for download in downloads:
            self.assertIn(f"--dir {self.tmp / 'temp'}/dashmate-sign.", download)
        self.assertEqual(list((self.tmp / 'temp').iterdir()), [])

    def test_should_take_the_fingerprint_with_spaces_and_in_lower_case(self):
        spaced = ' '.join(KEY[i:i + 4] for i in range(0, 40, 4)).lower()

        result = self.sign(f'--tag={TAG}', f'--key={spaced}', COMMIT)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f'--local-user {KEY} ', self.signings()[0])

    def test_should_attach_the_signature_and_read_it_back_with_the_flag(self):
        result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload')

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        [upload] = self.uploads()
        self.assertTrue(upload.endswith('/release/SHA256SUMS.asc --repo dashpay/platform'), upload)
        self.assertNotIn('--clobber', upload)
        attached = [a['name'] for a in self.state()['release']['assets']]
        self.assertIn('SHA256SUMS.asc', attached)
        # read back from the release after the upload
        downloads = [c for c in self.calls('gh') if c.startswith('gh release download')]
        self.assertEqual(len(downloads), 3)
        self.assertIn('--pattern SHA256SUMS.asc', downloads[2])
        self.assertIn(f'Attached SHA256SUMS.asc to {TAG}', result.stdout)
        self.assertEqual(list((self.tmp / 'temp').iterdir()), [])

    def test_should_refuse_wrong_options_before_reaching_github(self):
        cases = {
            'no tag': (f'--key={KEY}', COMMIT),
            'not a release tag': ('--tag=5.1.0', f'--key={KEY}', COMMIT),
            'a tag with more after it': (f'--tag={TAG}/other', f'--key={KEY}', COMMIT),
            'a tag with a second line': (f'--tag={TAG}\n../other', f'--key={KEY}', COMMIT),
            'a key with a second line': (f'--tag={TAG}', f'--key={KEY}\n{SUBKEY}', COMMIT),
            'no key': (f'--tag={TAG}', COMMIT),
            'a short key id, which can match another key': (f'--tag={TAG}', '--key=EDBC0AA6BC1AA13C', COMMIT),
            'an unknown option': (f'--tag={TAG}', f'--key={KEY}', COMMIT, '--force'),
            'no commit': (f'--tag={TAG}', f'--key={KEY}'),
            'a short commit, which can match another one': (f'--tag={TAG}', f'--key={KEY}', f'--commit={HEAD_SHA[:12]}'),
            'a commit with a second line': (f'--tag={TAG}', f'--key={KEY}', f'{COMMIT}\n{OTHER_SHA}'),
            'a branch instead of a commit': (f'--tag={TAG}', f'--key={KEY}', '--commit=v5.1-dev'),
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

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload')

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

    def test_should_refuse_a_sha256sums_the_checksums_job_did_not_attach(self):
        own = sha256(''.join(f'{sha256(data)}  {name}\n' for name, data in sorted(self.packages.items())).encode())
        others = {name: data + b' other build' for name, data in self.packages.items()}
        sums_id = 1000 + len(PACKAGES)
        cases = {
            # attached by the release workflow and consistent with the packages it lists, but not
            # the asset the checksums job recorded
            'another SHA256SUMS that matches other packages': (others, (999, own), {}, 'job 501 of run 77 attached'),
            # the same bytes, attached again as another asset
            'the same SHA256SUMS as another asset': (self.packages, (999, own), {}, 'job 501 of run 77 attached'),
            'a record of other bytes under the same asset': (self.packages, (sums_id, 'e' * 64), {},
                                                             'job 501 of run 77 attached'),
            'no record in the job log': (self.packages, False, {}, 'does not record exactly one SHA256SUMS'),
            'two records in the job log': (self.packages, [(sums_id, own), (sums_id, own)], {},
                                           'does not record exactly one SHA256SUMS'),
            'a job log that cannot be read': (self.packages, None, {'GH_NO_LOG': '1'}, 'could not read the log'),
        }
        for case, (files, receipt, env, message) in cases.items():
            with self.subTest(case):
                for leftover in self.files.iterdir():
                    leftover.unlink()
                self.release(files, receipt=receipt)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', **env)

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])

    def test_should_sign_once_the_dashmate_jobs_are_done_while_the_sdk_builds_run(self):
        newest = {'id': RUN_ID, 'head_branch': TAG, 'head_sha': HEAD_SHA, 'event': 'release', 'status': 'in_progress'}
        sdk = [{'id': 600, 'name': 'Release Kotlin SDK / Build Android SDK', 'status': 'in_progress', 'conclusion': None},
               {'id': 601, 'name': 'Release Swift SDK / Build XCFramework', 'status': 'queued', 'conclusion': None},
               # the Docker image is not one of the packages SHA256SUMS lists
               {'id': 602, 'name': 'Release Dashmate Helper image / Build linux/arm64 image', 'status': 'in_progress',
                'conclusion': None}]
        cases = {
            'the run still building the SDKs': ({'runs': [newest], 'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS + sdk}}, {}),
            # the dashmate jobs carried over into the new attempt, under new ids
            'a re-run of an SDK build only': ({
                'runs': [dict(newest, run_attempt=2)],
                'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID}/2': dashmate_jobs(100) + sdk[:1]},
                'logged': (CHECKSUMS_JOB + 100,)}, {}),
            'an older run of the tag still building its SDKs': ({
                'runs': [dict(newest, status='completed'), dict(newest, id=RUN_ID - 1)],
                'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID - 1}/1': dashmate_jobs(-100) + sdk}}, {}),
            'the SDK builds finishing while it signs': (
                {'runs': [newest], 'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS + sdk}}, {'GH_AFTER_DOWNLOAD': 'sdk-finished'}),
            'the SDK builds finishing between two listings of the runs': (
                {'runs': [newest], 'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS + sdk}}, {'GH_FINISH_WHILE_LISTED': '1'}),
            'the SDK builds finishing before the attempt is read': (
                {'runs': [newest], 'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS + sdk}}, {'GH_FINISH_BEFORE_ATTEMPT_READ': '1'}),
            # an SDK job waiting on its environment makes the run wait; the dashmate jobs are done
            'a run waiting on an SDK deployment': ({'runs': [dict(newest, status='waiting')], 'jobs': {
                f'{RUN_ID}/1': DASHMATE_JOBS + [{'id': 603, 'name': 'Release Kotlin SDK / Deploy to Maven Central',
                                                 'status': 'waiting', 'conclusion': None}]}}, {}),
        }
        for case, (fields, env) in cases.items():
            with self.subTest(case):
                self.release(self.packages, **fields)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', **env)

                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(len(self.uploads()), 1)

    def test_should_refuse_until_the_dashmate_jobs_of_the_newest_attempt_succeeded(self):
        newest = {'id': RUN_ID, 'head_branch': TAG, 'head_sha': HEAD_SHA, 'event': 'release', 'status': 'in_progress'}
        unfinished = {'status': 'in_progress', 'conclusion': None}
        win = {'id': CHECKSUMS_JOB - 3, 'name': 'Build Dashmate packages (win, ubuntu-24.04)'}
        cases = {
            'no run for the tag': ({'runs': []}, f'found no release.yml run for {TAG}'),
            'the checksums job failed': ({'jobs': {f'{RUN_ID}/1': dashmate_jobs(**{
                'Release Dashmate checksums': {'conclusion': 'failure'}})}}, f'run {RUN_ID} attempt 1 did not succeed (failure)'),
            'no checksums job': ({'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS[:2]}}, 'did not succeed (not found)'),
            'two checksums jobs': ({'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS + dashmate_jobs(10)[2:]}},
                                   'has more than one Release Dashmate checksums job'),
            'the checksums job still running': ({'runs': [newest], 'jobs': {f'{RUN_ID}/1': dashmate_jobs(**{
                'Release Dashmate checksums': unfinished})}}, f'Release Dashmate checksums (run {RUN_ID} attempt 1, in_progress)'),
            # an earlier attempt's success does not count while a newer attempt is pending
            'a dashmate job re-run in a newer attempt': ({'runs': [dict(newest, run_attempt=2)], 'jobs': {
                f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID}/2': dashmate_jobs(100, **{
                    'Release Dashmate packages (deb, ubuntu-24.04)': unfinished,
                    'Release Dashmate checksums': {'status': 'pending', 'conclusion': None}})}},
                f'Release Dashmate packages (deb, ubuntu-24.04) (run {RUN_ID} attempt 2, in_progress)'),
            'a newer attempt whose jobs are not listed yet': ({'runs': [dict(newest, run_attempt=2)],
                                                               'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS}},
                                                              f'run {RUN_ID} attempt 2 did not succeed (not found)'),
            # GitHub shows a requested re-run as a queued run before its attempt number moves on
            'a re-run queued before its attempt number moves on': (
                {'runs': [dict(newest, status='queued', attempt_status='completed')]},
                f'run {RUN_ID} (queued; attempt 1: completed)'),
            # an unfinished run counts only while both it and the attempt it names are in progress
            'a queued run whose attempt reports in progress': (
                {'runs': [dict(newest, status='queued', attempt_status='in_progress')]},
                f'run {RUN_ID} (queued; attempt 1: in_progress)'),
            'a run in progress on an attempt that has finished': (
                {'runs': [dict(newest, attempt_status='completed')]}, f'run {RUN_ID} (in_progress; attempt 1: completed)'),
            # an active run creates its dashmate jobs only once the jobs they need have finished
            'an older active run whose dashmate jobs are not created yet': ({
                'runs': [dict(newest, status='completed'), dict(newest, id=RUN_ID - 1)],
                'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID - 1}/1': [
                    {'id': 500, 'name': 'Build NPM packages / Build NPM packages', 'status': 'in_progress',
                     'conclusion': None}]}},
                f'Release Dashmate checksums (run {RUN_ID - 1} attempt 1, not created yet)'),
            'a newer attempt whose checksums job failed': ({'runs': [dict(newest, status='completed', run_attempt=2)], 'jobs': {
                f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID}/2': dashmate_jobs(100, **{
                    'Release Dashmate checksums': {'conclusion': 'failure'}})}},
                f'run {RUN_ID} attempt 2 did not succeed (failure)'),
            # a re-published release starts another run, whose legs upload by tag
            'a dashmate job of an older run still going': ({
                'runs': [dict(newest, status='completed'), dict(newest, id=RUN_ID - 1)],
                'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID - 1}/1': dashmate_jobs(-100, **{
                    'Release Dashmate packages (deb, ubuntu-24.04)': unfinished,
                    'Release Dashmate checksums': {'status': 'queued', 'conclusion': None}})}},
                f'Release Dashmate packages (deb, ubuntu-24.04) (run {RUN_ID - 1} attempt 1, in_progress)'),
            'an older run re-running a dashmate job': ({
                'runs': [dict(newest, status='completed'), dict(newest, id=RUN_ID - 1, run_attempt=2)],
                'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS, f'{RUN_ID - 1}/1': dashmate_jobs(-100),
                         f'{RUN_ID - 1}/2': dashmate_jobs(-200, **{
                             'Release Dashmate packages (deb, ubuntu-24.04)': {'status': 'queued', 'conclusion': None}})}},
                f'Release Dashmate packages (deb, ubuntu-24.04) (run {RUN_ID - 1} attempt 2, queued)'),
            # runs of other tags are not this release's; of this tag's runs the newest counts
            'the newest run of the tag failed its checksums': ({
                'runs': [{'id': 90, 'head_branch': 'v5.0.9', 'head_sha': HEAD_SHA, 'event': 'release', 'status': 'completed'},
                         {'id': 78, 'head_branch': TAG, 'head_sha': HEAD_SHA, 'event': 'release', 'status': 'completed'},
                         {'id': RUN_ID, 'head_branch': TAG, 'head_sha': HEAD_SHA, 'event': 'release', 'status': 'completed'}],
                'jobs': {'90/1': DASHMATE_JOBS, f'{RUN_ID}/1': DASHMATE_JOBS,
                         '78/1': dashmate_jobs(-50, **{'Release Dashmate checksums': {'conclusion': 'failure'}})}},
                'job of run 78 attempt 1 did not succeed (failure)'),
        }
        # however far the checksums job got, a dashmate job of the attempt that is not finished
        for status in ['queued', 'in_progress', 'waiting', 'pending', 'requested']:
            cases[f'a dashmate build leg {status}'] = (
                {'runs': [newest], 'jobs': {f'{RUN_ID}/1': DASHMATE_JOBS + [dict(win, status=status, conclusion=None)]}},
                f'Build Dashmate packages (win, ubuntu-24.04) (run {RUN_ID} attempt 1, {status})')
        for case, (fields, message) in cases.items():
            with self.subTest(case):
                self.release(self.packages, **fields)

                result = self.sign()

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])
        for rerun in ['1', 'finished', 'stale', 'stale-running']:
            with self.subTest('a re-run requested while the runs are read', rerun=rerun):
                self.release(self.packages)

                result = self.sign(GH_RERUN_WHILE_LISTED=rerun)

                self.assertRefused(result, f'the release.yml runs of {TAG} changed while they were checked')
                self.assertEqual(self.signings(), [])
        with self.subTest('a second listing of the runs that fails'):
            self.release(self.packages)

            result = self.sign(GH_RELIST_FAILS='1')

            self.assertRefused(result, f'could not list the release.yml runs of {TAG} again')
            self.assertEqual(self.signings(), [])

    def test_should_check_the_run_again_right_before_signing(self):
        # The packages take a while to download; without --upload, too, nothing is signed for a
        # release whose run or tag changed meanwhile.
        for change, message in [('run-restarted', f'run {RUN_ID} (queued; attempt 2: queued)'),
                                ('job-rerun', 'checksums job changed while SHA256SUMS was being signed'),
                                ('tag-moved', f'{TAG} names {OTHER_SHA}, not {HEAD_SHA}')]:
            with self.subTest(change):
                self.release(self.packages)

                result = self.sign(GH_AFTER_DOWNLOAD=change)

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])
        # the run is the last thing read: a re-run requested while the tag is looked up is seen
        with self.subTest('a re-run requested while the tag is looked up'):
            self.release(self.packages)

            result = self.sign(GH_RERUN_AFTER_TAG_LOOKUP='2')

            self.assertRefused(result, f'run {RUN_ID} (queued; attempt 2: queued)')
            self.assertEqual(self.signings(), [])

    def test_should_refuse_a_release_not_built_from_the_commit_given(self):
        newest = {'id': RUN_ID, 'head_branch': TAG, 'event': 'release', 'status': 'completed'}
        cases = {
            # the tag was moved after the run that built the packages
            'a tag that names another commit': ({'tag_commit': OTHER_SHA}, f'{TAG} names {OTHER_SHA}, not {HEAD_SHA}'),
            'a tag GitHub cannot resolve': ({'tag_commit': None}, f'could not resolve {TAG} to a commit'),
            'a run of another commit': ({'runs': [dict(newest, head_sha=OTHER_SHA)], 'tag_commit': OTHER_SHA},
                                        f'release.yml run {RUN_ID} built {OTHER_SHA}, not {HEAD_SHA}'),
            # an older run of the commit given does not count when the newest is another
            'an older run of the commit, a newer of another': ({'runs': [
                dict(newest, head_sha=OTHER_SHA), dict(newest, id=RUN_ID - 1, head_sha=HEAD_SHA)]},
                f'release.yml run {RUN_ID} built {OTHER_SHA}, not {HEAD_SHA}'),
        }
        for case, (fields, message) in cases.items():
            with self.subTest(case):
                self.release(self.packages, **fields)

                result = self.sign()

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])
        with self.subTest('given in upper case'):
            self.release(self.packages)
            result = self.sign(f'--tag={TAG}', f'--key={KEY}', f'--commit={HEAD_SHA.upper()}')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_refuse_a_listing_that_failed_part_way(self):
        for listing, message in [('/assets?', f'could not list the assets of {TAG}'),
                                 ('/runs?', f'could not list the release.yml runs of {TAG}'),
                                 ('/jobs?', f'could not list the jobs of release.yml run {RUN_ID}')]:
            with self.subTest(listing):
                (self.tmp / 'calls').unlink(missing_ok=True)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GH_PARTIAL=listing)

                self.assertRefused(result, message)
                self.assertEqual(self.signings(), [])

    def test_should_download_no_package_until_sha256sums_lists_exactly_the_ones_served(self):
        # SHA256SUMS is checked against the names and digests GitHub reports for the release's
        # dashmate assets before any package is downloaded.
        deb = 'dashmate_5.1.0-1_amd64.deb'
        others = {name: data for name, data in self.packages.items() if name != deb}
        listed = ''.join(f'{sha256(data)}  {name}\n' for name, data in sorted(self.packages.items())).encode()
        lines = listed.splitlines(keepends=True)
        cases = {
            'a package with other bytes': (dict(self.packages, **{deb: b'other bytes'}), self.packages, None),
            'a listed package not attached': (others, self.packages, None),
            'an attached package not listed': (self.packages, others, None),
            'a line that is not <sha256>  <name>': (self.packages, None, listed + b'not a checksum line\n'),
            'an empty SHA256SUMS': (self.packages, None, b''),
            'a duplicated line': (self.packages, None, listed + lines[0]),
            'a package listed twice and another not at all': (
                self.packages, None, b''.join(sorted(lines[:-1] + lines[:1], key=lambda line: line.split(b'  ')[1]))),
            'lines out of order': (self.packages, None, b''.join(reversed(lines))),
            # GNU sha256sum --check --strict passes both
            'a blank line at the end': (self.packages, None, listed + b'\n'),
            'no newline at the end': (self.packages, None, listed[:-1]),
        }
        for case, (files, sums, text) in cases.items():
            with self.subTest(case):
                for leftover in self.files.iterdir():
                    leftover.unlink()
                self.release(files, sums=sums)
                if text is not None:
                    (self.files / 'SHA256SUMS').write_bytes(text)
                    self.edit_asset('SHA256SUMS', digest='sha256:' + sha256(text))

                result = self.sign(HASH_CHECK_LENIENT='1')

                self.assertRefused(result, 'the dashmate packages v5.1.0 serves are not the ones its SHA256SUMS lists')
                self.assertEqual([c.split(' --dir ')[0] for c in self.calls('gh') if c.startswith('gh release download')],
                                 [f'gh release download {TAG} --repo dashpay/platform --pattern SHA256SUMS'])
                self.assertEqual(self.signings(), [])
                self.assertEqual(list((self.tmp / 'temp').iterdir()), [])
        with self.subTest('a release without packages'):
            for leftover in self.files.iterdir():
                leftover.unlink()
            self.release({})

            self.assertRefused(self.sign(HASH_CHECK_LENIENT='1'), f'SHA256SUMS of {TAG} lists no packages')

    def test_should_refuse_packages_that_do_not_match_every_line(self):
        # The bytes downloaded, not only the digests GitHub reports, must match every line.
        deb = 'dashmate_5.1.0-1_amd64.deb'
        with self.subTest('a package whose bytes are not the digest GitHub reports'):
            self.release(dict(self.packages, **{deb: b'other bytes'}), sums=self.packages)
            self.edit_asset(deb, digest='sha256:' + sha256(self.packages[deb]))

            self.assertRefused(self.sign(), 'do not match its SHA256SUMS')
            self.assertEqual(self.signings(), [])
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

    def test_should_refuse_to_run_without_any_accepted_signer(self):
        self.script = self.accepting()

        result = self.sign()

        self.assertRefused(result, 'ALLOWED_SIGNERS lists no key')
        self.assertEqual(self.calls(), [])

    def test_should_refuse_a_key_the_apt_repository_does_not_accept(self):
        result = self.sign(f'--tag={TAG}', f'--key={STRANGER}', COMMIT, '--upload')

        self.assertRefused(result, f'{STRANGER} is not one of the signers this script accepts')

    def test_should_accept_a_signature_made_by_a_subkey_of_the_key(self):
        for key in [KEY, SUBKEY]:
            with self.subTest(key=key):
                # a later gpg may append fields to VALIDSIG; the primary key stays the twelfth
                result = self.sign(f'--tag={TAG}', f'--key={key}', COMMIT, GPG_SIGNER=KEY, GPG_SUBKEY=SUBKEY,
                                   GPG_EXTRA_FIELD=' 0' * 2)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_stop_before_uploading_when_the_release_changed_meanwhile(self):
        cases = {
            'another maintainer signed it': ('signed', f'{TAG} already carries SHA256SUMS.asc'),
            'a re-run replaced SHA256SUMS': ('sums-replaced', 'changed while it was being signed'),
            'a re-run replaced a package': ('package-replaced', 'are not the ones its SHA256SUMS lists'),
            'SHA256SUMS uploaded again': ('sums-reuploaded', 'changed while it was being signed'),
            'the checksums job ran again': ('job-rerun', 'checksums job changed while SHA256SUMS was being signed'),
            'a re-run started': ('run-restarted', f'run {RUN_ID} (queued; attempt 2: queued)'),
            'the tag was moved': ('tag-moved', f'{TAG} names {OTHER_SHA}, not {HEAD_SHA}'),
        }
        for case, (change, message) in cases.items():
            with self.subTest(case):
                self.release(self.packages)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GH_AFTER_DOWNLOAD=change)

                self.assertRefused(result, message)
        # the run is the last thing read before the upload, after the release's assets
        with self.subTest('a re-run requested while the assets are listed before the upload'):
            self.release(self.packages)

            result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GH_RERUN_AFTER_ASSET_LISTING='2')

            self.assertRefused(result, f'run {RUN_ID} (queued; attempt 2: queued)')
        # while the passphrase is entered: after the signature, before the upload
        for change, message in [('run-restarted', f'run {RUN_ID} (queued; attempt 2: queued)'),
                                ('job-rerun', 'checksums job changed while SHA256SUMS was being signed'),
                                ('tag-moved', f'{TAG} names {OTHER_SHA}, not {HEAD_SHA}')]:
            with self.subTest(f'{change} while signing'):
                self.release(self.packages)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GPG_AFTER_SIGN=change)

                self.assertRefused(result, message)
                self.assertEqual(len(self.signings()), 1)

    def test_should_name_the_attached_signature_when_a_check_after_the_upload_fails(self):
        # The signature stays attached; the message says so and how to remove it before a retry.
        cases = {
            'a package changed right after the upload': (
                {'GH_AFTER_UPLOAD': 'package-replaced'}, 'are not the ones its SHA256SUMS lists'),
            'the release serves another signature': (
                {'GH_UPLOAD_TAMPERED': '1'}, 'serves another SHA256SUMS.asc than the one signed here'),
            'the signature cannot be read back': (
                {'GH_READ_BACK_FAILS': '1'}, 'could not download SHA256SUMS and SHA256SUMS.asc back'),
            # the release must still be the one approved at --commit when it reports success
            'the tag was moved during the upload': (
                {'GH_AFTER_UPLOAD': 'tag-moved'}, f'{TAG} names {OTHER_SHA}, not {HEAD_SHA}'),
            'a re-run started during the upload': (
                {'GH_AFTER_UPLOAD': 'run-restarted'}, f'run {RUN_ID} (queued; attempt 2: queued)'),
            'the checksums job re-run and failing during the upload': (
                {'GH_AFTER_UPLOAD': 'checksums-job-rerun'}, f'run {RUN_ID} attempt 2 did not succeed (failure)'),
            'the checksums job re-run and passing during the upload': (
                {'GH_AFTER_UPLOAD': 'checksums-job-rerun-green'},
                'the release.yml run or its Release Dashmate checksums job changed while SHA256SUMS.asc was uploaded'),
            'a run of another commit during the upload': (
                {'GH_AFTER_UPLOAD': 'run-of-another-commit'},
                f'release.yml run {RUN_ID + 1} built {OTHER_SHA}, not {HEAD_SHA}'),
            'a newer run of the same commit during the upload': (
                {'GH_AFTER_UPLOAD': 'newer-run'},
                'the release.yml run or its Release Dashmate checksums job changed while SHA256SUMS.asc was uploaded'),
        }
        for case, (env, reason) in cases.items():
            with self.subTest(case):
                self.release(self.packages)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', **env)

                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertEqual(len(self.uploads()), 1)
                self.assertIn(reason, result.stderr)
                self.assertIn(f'SHA256SUMS.asc is attached to {TAG}', result.stderr)
                self.assertIn(f'gh release delete-asset {TAG} SHA256SUMS.asc --repo dashpay/platform', result.stderr)

    def test_should_say_what_is_attached_when_the_upload_of_the_signature_fails(self):
        with self.subTest('nothing attached'):
            self.release(self.packages)

            result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GH_UPLOAD_FAILS='1')

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn(f'the upload of SHA256SUMS.asc to {TAG} failed and nothing is attached; run this again',
                          result.stderr)
        # A signature nobody has checked yet, or what is attached when that is unknown: the
        # maintainer looks before changing anything, and is not told to delete it.
        for case, env, message in [
                ('another signature attached first', {'GH_UPLOAD_FAILS': 'signed-first'},
                 f'{TAG} carries a SHA256SUMS.asc that is not the one signed here'),
                ('an upload that did not finish', {'GH_UPLOAD_FAILS': 'partial'},
                 f'{TAG} now carries SHA256SUMS.asc (starter) instead of a complete SHA256SUMS.asc'),
                ('assets that cannot be listed afterwards', {'GH_UPLOAD_FAILS': 'unlisted'}, 'could not be listed'),
                ('a signature that cannot be downloaded to compare it',
                 {'GH_UPLOAD_FAILS': 'created', 'GH_READ_BACK_FAILS': '1'},
                 'the SHA256SUMS.asc now attached could not be downloaded')]:
            with self.subTest(case):
                self.release(self.packages)

                result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', **env)

                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn(message, result.stderr)
                self.assertIn(f'gh release view {TAG} --repo dashpay/platform', result.stderr)
                self.assertNotIn('delete-asset', result.stderr)
                if case == 'another signature attached first':
                    self.assertEqual((self.files / 'SHA256SUMS.asc').read_bytes(), b'another signature')
        with self.subTest('the signature made here attached all the same'):
            self.release(self.packages)

            result = self.sign(f'--tag={TAG}', f'--key={KEY}', COMMIT, '--upload', GH_UPLOAD_FAILS='created')

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn(f'{TAG} serves the SHA256SUMS.asc signed here', result.stdout)
            self.assertIn(f'Attached SHA256SUMS.asc to {TAG}', result.stdout)

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
        self.script = self.accepting(self.primary)


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
                result = self.sign(f'--tag={TAG}', f'--key={key}', COMMIT, GNUPGHOME=str(self.home))
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_should_refuse_a_second_signature_from_gpg_conf(self):
        (self.home / 'gpg.conf').write_text(f'local-user {self.other}\n')

        result = self.sign(f'--tag={TAG}', f'--key={self.primary}', COMMIT, GNUPGHOME=str(self.home))

        self.assertRefused(result, 'exactly one signature')


if __name__ == '__main__':
    unittest.main()
