"""Read and reconcile Platform's PR review policy using trusted repository data."""

import argparse
import hashlib
import json
import os
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path
import sys

from .github import GitHub, GitHubError, parse_controller_state
from .policy import admit, codeowners, effective_admission, evaluate, fingerprint, validate_policy

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / '.github' / 'pr-review-policy.json'
CONTEXT = 'Platform PR policy'
REPOSITORIES = {'dashpay/platform', 'dashpay/rust-dashcore', 'dashpay/tenderdash',
                'dashpay/grovedb', 'dashpay/dash-evo-tool'}


def utc_now():
    return datetime.now(timezone.utc).isoformat(timespec='seconds').replace('+00:00', 'Z')


def context_fingerprint(prs):
    fields = ('number', 'author', 'head', 'base', 'base_sha', 'draft', 'state')
    values = [tuple(pr.get(k) for k in fields) for pr in prs]
    return hashlib.sha256(json.dumps(sorted(values), sort_keys=True).encode()).hexdigest()


def load_histories(api, selected):
    def history(pr):
        comments = api.comments(pr['number'])
        state, comment_id = parse_controller_state(comments)
        if state is not None and state['number'] != pr['number']:
            raise GitHubError('Controller admission history belongs to another PR')
        return dict(pr, comments=comments, controller_state=state,
                    controller_comment_id=comment_id, lifecycle_at=api.activity(pr['number']))
    with ThreadPoolExecutor(max_workers=4) as pool:
        return list(pool.map(history, selected))


def admission_conflicts(policy, candidates):
    counts = {}
    for pr in candidates:
        if (pr['state'] == 'open' and not pr['draft'] and pr['base'] in policy['target_branches']
                and effective_admission(pr)):
            author = pr['author'].lower()
            counts[author] = counts.get(author, 0) + 1
    return {author for author, count in counts.items() if count > policy['max_active_prs']}


def admission_fingerprint(candidates):
    return sorted((p['number'], effective_admission(p), p.get('lifecycle_at')) for p in candidates)


def periodic_batch(prs, size, epoch_seconds=None):
    """Rotate a bounded slice without a persisted scheduler cursor."""
    if type(size) is not int or size < 1:
        raise ValueError('Batch size must be positive')
    ordered = sorted(prs, key=lambda p: p['number'])
    if not ordered:
        return []
    seconds = datetime.now(timezone.utc).timestamp() if epoch_seconds is None else epoch_seconds
    start = (int(seconds // 900) * size) % len(ordered)
    return [ordered[(start + offset) % len(ordered)] for offset in range(min(size, len(ordered)))]


def collect(api, policy, number=None, apply=False, reconcile_author=False, batch_size=None):
    """Load global admission history, then full evidence for requested PRs."""
    prs = api.open_prs()
    selected = [p for p in prs if p['base'] in policy['target_branches']]
    batch_numbers = None
    if batch_size is not None:
        if number is not None:
            raise ValueError('Batch selection cannot be combined with a PR selector')
        batch = periodic_batch(selected, batch_size)
        batch_numbers = {p['number'] for p in batch}
        authors = {p['author'].lower() for p in batch}
        selected = [p for p in selected if p['author'].lower() in authors]
    if number is not None:
        principal = next((p for p in prs if p['number'] == number), None) or api.pull(number)
        selected = [p for p in selected if p['author'].lower() == principal['author'].lower()]
    try:
        candidates = load_histories(api, selected)
        requested = [p for p in candidates if number is None or p['number'] == number]
        if number is not None and reconcile_author:
            slots = admit(policy, candidates, utc_now())
            conflicts = admission_conflicts(policy, candidates)
            requested = [p for p in candidates if p['number'] == number
                         or bool(slots.get(p['number'])) != bool(effective_admission(p))
                         or p['author'].lower() in conflicts]
        if batch_numbers is not None:
            requested = [p for p in requested if p['number'] in batch_numbers]
        if number is not None and not requested and not reconcile_author:
            raise GitHubError(f'PR #{number} is not open on a configured target branch')
        with ThreadPoolExecutor(max_workers=4) as pool:
            snapshots = list(pool.map(lambda p: api.snapshot(p['number'], policy), requested))
        return prs, candidates, snapshots
    except GitHubError:
        if apply:
            # Admission depends on all candidates, so incomplete history invalidates
            # every known active head, even when only one PR was requested.
            for pr in selected:
                try:
                    current = api.pull(pr['number'])
                    if current['state'] == 'open' and current['base'] in policy['target_branches']:
                        api.post_status(current['head'], 'error', 'Incomplete policy evidence; reconciliation required')
                except GitHubError:
                    print(f"PR #{pr['number']}: unable to publish evidence error status", file=sys.stderr)
        raise


def state_record(pr, result, context):
    return {key: result.get(key) for key in
            ('number', 'head', 'admitted_at', 'ready_since', 'state')} | {
                'version': 1, 'evidence': fingerprint(pr), 'context': context}


def state_body(result):
    reasons = result.get('blockers') or ['All policy requirements are satisfied.']
    return '\n'.join([
        '### Platform PR review',
        f"State: **{result['state']}** · commit `{result['head']}`",
        '', *[f'- {reason}' for reason in reasons],
        '', 'Self-review is an author attestation for this exact commit:',
        f"`/self-reviewed {result['head']}`",
        '', 'This report does not bypass CI or repository protection rules.',
    ])


def publish(api, policy, pr, result, context_prs, apply=False, candidates=None):
    """Publish only after revalidating the PR and its policy evidence."""
    if not apply:
        return
    candidates = context_prs if candidates is None else candidates
    actionable = result['state'] in {'ready-for-human', 'ready-to-merge'}

    def admission_valid(expected):
        current_prs = api.open_prs()
        if context_fingerprint(current_prs) != context:
            return False
        # Only this author's histories can change this PR's admission decision.
        relevant = [p for p in current_prs if p['base'] in policy['target_branches']
                    and p['author'].lower() == pr['author'].lower()]
        histories = load_histories(api, relevant)
        baseline = [p for p in expected if p['author'].lower() == pr['author'].lower()]
        if admission_fingerprint(histories) != admission_fingerprint(baseline):
            return False
        if pr['author'].lower() in admission_conflicts(policy, histories):
            return result['status'] != 'success'
        slots = admit(policy, histories, result.get('admitted_at') or utc_now())
        return slots.get(pr['number']) == result.get('admitted_at')

    identity = ('head', 'base', 'base_sha', 'draft', 'state')

    def identity_matches():
        current = api.pull(pr['number'])
        return all(current.get(k) == pr.get(k) for k in identity)

    if not identity_matches():
        return
    context = context_fingerprint(context_prs)
    if actionable and fingerprint(api.snapshot(pr['number'], policy)) != fingerprint(pr):
        api.post_status(pr['head'], 'pending', 'Review evidence changed; reconciliation required')
        return
    if actionable and not admission_valid(candidates):
        api.post_status(pr['head'], 'pending', 'PR admission context changed; reconciliation required')
        return

    desired = state_record(pr, result, context)

    def finish(expected):
        # Admission history reads can be slow. Read this PR's review evidence
        # after those reads so a dismissed approval is not reused from before them.
        valid_admission = admission_valid(expected)
        final = api.snapshot(pr['number'], policy)
        if not valid_admission or fingerprint(final) != fingerprint(pr):
            api.post_status(pr['head'], 'pending', 'Review evidence changed; reconciliation required')
            return
        check = evaluate(policy, final, result.get('admitted_at'), utc_now())
        if result['status'] == 'success' and check['status'] != 'success':
            api.post_status(pr['head'], 'pending', 'Policy changed; reconciliation required')
            return
        api.post_status(pr['head'], result['status'], result['state'])

    ready = result['state'] == 'ready-for-human'
    requested = set(pr.get('requested_reviewers', []))
    missing = [u for u in result.get('reviewers', []) if u not in requested] if ready else []
    label_correct = ('ready-for-human' in pr.get('labels', [])) == ready
    if pr.get('controller_state') == desired and label_correct and not missing:
        # Read current evidence on every run, but avoid churning comments and labels.
        if actionable:
            finish(candidates)
        else:
            api.post_status(pr['head'], result['status'], result['state'])
        return

    api.post_status(pr['head'], 'pending', 'Evaluating current review policy')
    if not identity_matches():
        return
    api.upsert_state(pr['number'], desired, state_body(result), pr.get('controller_comment_id'))
    if not identity_matches():
        return desired
    api.set_ready_label(pr['number'], ready, pr.get('labels', []))
    if missing:
        room = max(0, 15 - len(requested))
        if len(missing) > room:
            print(f"PR #{pr['number']}: reviewer request capacity reached; "
                  f"{len(missing) - room} request(s) deferred", file=sys.stderr)
        if room:
            if not identity_matches():
                return desired
            api.request_reviewers(pr['number'], missing[:room])

    if not actionable:
        if identity_matches():
            api.post_status(pr['head'], result['status'], result['state'])
        return desired

    # Review decisions and comments can change without changing the commit SHA.
    expected = [dict(p, controller_state=desired) if p['number'] == pr['number'] else p for p in candidates]
    finish(expected)
    return desired


def age(since, now):
    if not since:
        return 'not recorded'
    try:
        seconds = max(0, (datetime.fromisoformat(now.replace('Z', '+00:00'))
                          - datetime.fromisoformat(since.replace('Z', '+00:00'))).total_seconds())
    except (ValueError, TypeError):
        return 'unknown'
    hours = int(seconds // 3600)
    return f'{hours // 24}d {hours % 24}h' if hours >= 24 else f'{hours}h'


def cell(value):
    return str(value).replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;').replace('|', '&#124;').replace('\n', ' ')


def selected_rows(rows, user=None):
    return [r for r in rows if not user or r['author'].lower() == user.lower()
            or user.lower() in [x.lower() for x in r.get('reviewers', [])]]


def render_report(rows, now, user=None):
    lines = [f'# Platform PR reviews — {now}', '',
             'Read-only snapshot. Waiting time is for the current actionable review cycle.', '',
             '| PR | Author | Areas | State | Awaiting review | Next action |',
             '| --- | --- | --- | --- | --- | --- |']
    for r in selected_rows(rows, user):
        next_action = '; '.join(r.get('blockers', [])) or 'Policy satisfied; check remaining GitHub gates'
        if r.get('reviewers'):
            next_action += '; reviewers: ' + ', '.join(r['reviewers'])
        link = f"[#{r['number']}: {cell(r.get('title', ''))}]({r.get('url', '')})"
        lines.append('| ' + ' | '.join([link, cell(r['author']), cell(', '.join(r.get('areas', []))),
                                      cell(r['state']), age(r.get('ready_since'), now), cell(next_action)]) + ' |')
    if not selected_rows(rows, user):
        lines += ['', 'No matching PRs.']
    return '\n'.join(lines) + '\n'


def evaluate_snapshots(policy, context, candidates, snapshots, now):
    """Use the same policy decisions for local enforcement and combined reports."""
    admissions = admit(policy, candidates, now)
    conflicts = admission_conflicts(policy, candidates)
    rows = []
    head_counts = Counter(pr['head'] for pr in context)
    for pr in snapshots:
        result = evaluate(policy, pr, admissions.get(pr['number']), now)
        if pr['author'].lower() in conflicts:
            result.update(state='configuration-error', status='error', reviewers=[], ready_since=None)
            result['admitted_at'] = effective_admission(pr)
            result['blockers'].append('More than five persisted author admissions; repair inconsistent history explicitly')
        if head_counts[pr['head']] > 1:
            result.update(state='configuration-error', status='error', reviewers=[], ready_since=None)
            result['blockers'].append('Another open PR shares this head; commit-scoped status is ambiguous')
        result['repository'] = policy['repository']
        rows.append(result)
    return rows


def run(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['validate', 'codeowners', 'report', 'sync'])
    parser.add_argument('--repo', default='dashpay/platform')
    parser.add_argument('--repository-root', type=Path)
    parser.add_argument('--policy', type=Path)
    parser.add_argument('--pr', type=int)
    parser.add_argument('--batch-size', type=int)
    parser.add_argument('--user')
    parser.add_argument('--format', choices=['markdown', 'json'], default='markdown')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args(argv)
    if args.pr is not None and args.pr <= 0:
        parser.error('--pr must be positive')
    if args.batch_size is not None and (args.batch_size < 1 or args.command != 'sync' or args.pr is not None):
        parser.error('--batch-size requires sync, a positive size, and no --pr')
    if args.apply and args.command != 'sync':
        parser.error('--apply is only valid for sync')
    if args.apply and (args.repo not in REPOSITORIES
                       or os.environ.get('GITHUB_ACTIONS') != 'true'
                       or os.environ.get('GITHUB_REPOSITORY') != args.repo
                       or os.environ.get('PR_REVIEW_AUTOMATION_ENABLED') != 'true'):
        parser.error('apply is restricted to the enabled repository Actions workflow')
    repository_root = args.repository_root.resolve() if args.repository_root else ROOT
    policy_path = args.policy or (repository_root / '.github/pr-review-policy.json' if args.repository_root else POLICY)
    if args.apply and args.policy and args.policy.resolve() != repository_root / '.github/pr-review-policy.json':
        parser.error('apply requires the canonical policy in the target repository checkout')
    try:
        policy = json.loads(policy_path.read_text())
        validate_policy(policy, repository_root)
        if policy['repository'] != args.repo:
            raise ValueError('Repository must match the checked-out policy')
    except (ValueError, OSError):
        if args.apply:
            # Broken configuration cannot identify its scope reliably. Revoke
            # known open heads using the independently authorized repository.
            api = GitHub(args.repo)
            try:
                known = api.open_prs()
            except GitHubError:
                known = []
                print('Unable to discover heads for configuration error statuses', file=sys.stderr)
            for pr in known:
                try:
                    api.post_status(pr['head'], 'error', 'Invalid policy configuration; inspect workflow log')
                except GitHubError:
                    print(f"PR #{pr['number']}: unable to publish configuration error status", file=sys.stderr)
        raise
    if args.command == 'validate':
        print('Policy schema and paths are valid.')
        unresolved = [(a['id'], a['unresolved']) for a in policy['areas'] if a.get('unresolved')]
        if unresolved:
            print('Activation blockers: ' + json.dumps(unresolved))
        return 0
    if args.command == 'codeowners':
        generated = codeowners(policy)
        if args.check:
            locations = [repository_root / path for path in
                         ('.github/CODEOWNERS', 'CODEOWNERS', 'docs/CODEOWNERS')]
            effective = next((path for path in locations if path.is_file()), locations[0])
            if effective.read_text() != generated:
                raise GitHubError('CODEOWNERS differs from canonical policy; regenerate it')
        else:
            print(generated, end='')
        return 0

    api = GitHub(args.repo)
    context, candidates, snapshots = collect(api, policy, args.pr, apply=args.apply,
                                           reconcile_author=args.command == 'sync', batch_size=args.batch_size)
    now = utc_now()
    rows = evaluate_snapshots(policy, context, candidates, snapshots, now)
    for pr, result in zip(snapshots, rows):
        if args.command == 'sync':
            try:
                if args.apply:
                    # Verify setup explicitly; never create labels as a side effect.
                    api.request('GET', f'repos/{args.repo}/labels/ready-for-human')
                written = publish(api, policy, pr, result, context, args.apply, candidates=candidates)
                if written:
                    for candidate in candidates:
                        if candidate['number'] == pr['number']:
                            candidate['controller_state'] = written
            except GitHubError:
                if args.apply:
                    api.post_status(pr['head'], 'error', 'Policy reconciliation failed; inspect workflow log')
                raise
    rows.sort(key=lambda r: (r['state'] != 'ready-for-human', r.get('ready_since') or now, r['number']))
    if args.format == 'json':
        print(json.dumps({'generated_at': now, 'pull_requests': selected_rows(rows, args.user)}, indent=2))
    else:
        print(render_report(rows, now, args.user), end='')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(run())
    except (GitHubError, ValueError, KeyError, OSError) as exc:
        print(f'PR review policy error: {exc}', file=sys.stderr)
        sys.exit(1)
