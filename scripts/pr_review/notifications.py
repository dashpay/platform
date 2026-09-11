"""Build explicit Slack destinations from one immutable review snapshot."""

from collections import Counter
import json
import os
import re
import urllib.request

from .main import age


def payload(title, sections):
    # Slack plain_text blocks keep repository-controlled text from making mentions.
    texts = [title, *sections]
    blocks = [{'type': 'section', 'text': {'type': 'plain_text', 'text': text[:2900], 'emoji': False}}
              for text in texts[:49]]
    if len(texts) > 49:
        blocks.append({'type': 'section', 'text': {'type': 'plain_text',
                       'text': f'{len(texts) - 49} additional entries are in the full report.'}})
    return {'text': title, 'blocks': blocks, 'parse': 'none', 'unfurl_links': False, 'unfurl_media': False}


def pr_text(pr, now, blockers=False):
    text = (f"{pr['repository']}#{pr['number']} {pr.get('title', '')[:300]}\n"
            f"State: {pr['state']} · waiting: {age(pr.get('ready_since'), now)}\n")
    if blockers:
        text += '; '.join(pr.get('blockers', [])) + '\n'
    else:
        text += 'Reviewers: ' + ', '.join(pr.get('reviewers', [])) + '\n'
    return text + pr.get('url', '')


def build_delivery_plan(snapshot, config):
    """Return reviewable messages and all destination configuration errors."""
    errors = []
    messages = []
    if (not isinstance(config, dict) or set(config) != {'version', 'channel_id', 'users'}
            or type(config.get('version')) is not int or config['version'] != 1
            or not isinstance(config.get('users'), dict)):
        return {'messages': [], 'errors': ['Invalid Slack configuration']}
    channel = config.get('channel_id')
    if not isinstance(channel, str) or not re.fullmatch(r'[CG][A-Z0-9]+', channel):
        errors.append('Shared Slack channel ID is missing or invalid')
        channel = None
    mappings = {}
    for login, identity in config['users'].items():
        if not isinstance(login, str) or not login or login.lower() in mappings:
            errors.append('Duplicate or invalid GitHub login in Slack mapping')
            continue
        mappings[login.lower()] = identity
    grouped = {}
    for login in snapshot['roster']:
        identity = mappings.get(login.lower())
        if not isinstance(identity, str) or not re.fullmatch(r'[UW][A-Z0-9]+', identity):
            errors.append(f'Slack user ID missing or invalid for {login}')
            identity = 'unmapped:' + login.lower()
        grouped.setdefault(identity, set()).add(login.lower())
    now = snapshot['generated_at']
    rows = snapshot['pull_requests']
    unavailable = [f"Unavailable: {r['repository']} — {r.get('error', 'incomplete evidence')}"
                   for r in snapshot['repositories'] if not r['complete']]
    previews = [r['repository'] for r in snapshot['repositories'] if r.get('mode') == 'preview']
    mode_note = 'Preview policies: ' + ', '.join(previews) if previews else 'Canonical active policies'
    counts = Counter(r['state'] for r in rows)
    summary = 'States: ' + (', '.join(f'{state}: {count}' for state, count in sorted(counts.items())) or 'no known PRs')
    workload = [f"Workload warning: {w['author']} has {w['total']} PRs across repositories "
                '(five slots are enforced separately in each repository).'
                for w in snapshot['workload'] if w['over_limit']]
    queue = [pr_text(r, now) for r in rows if r['state'] == 'ready-for-human']
    if not queue:
        queue = ['No known PRs currently ready for human review.']
    blocked = [pr_text(r, now, True) for r in rows
               if r['state'] not in {'draft', 'ready-for-human', 'ready-to-merge'}]
    if blocked:
        blocked.insert(0, 'Author action needed — these PRs are not ready for human review')
    messages.append({'kind': 'channel', 'destination': channel,
                         'payload': payload(f'PR review summary — {now}',
                                            [mode_note, *unavailable, summary, *workload, *queue, *blocked])})
    for identity, logins in sorted(grouped.items()):
        reviews = [r for r in rows if r['state'] == 'ready-for-human'
                   and logins.intersection(u.lower() for u in r.get('reviewers', []))]
        own = [r for r in rows if r['author'].lower() in logins
               and r['state'] not in {'draft', 'ready-for-human', 'ready-to-merge'}]
        if not reviews and not own and not unavailable:
            continue
        sections = [mode_note, *unavailable]
        if reviews:
            sections += ['Reviews awaiting you (computed policy)', *[pr_text(r, now) for r in reviews]]
        if own:
            sections += ['Your PR blockers', *[pr_text(r, now, True) for r in own]]
        messages.append({'kind': 'personal', 'destination': None if identity.startswith('unmapped:') else identity, 'github_logins': sorted(logins),
                         'payload': payload(f'Your PR digest — {now}', sections)})
    return {'generated_at': now, 'complete': snapshot['complete'], 'messages': messages, 'errors': errors}


def deliver(plan):
    """Send once per destination; stop on any unacknowledged result."""
    if plan['errors']:
        raise ValueError('Slack configuration has errors; inspect delivery preview')
    token = os.environ.get('PR_REVIEW_SLACK_BOT_TOKEN', '')
    if os.environ.get('PR_REVIEW_SLACK_ENABLED') != 'true' or not token:
        raise ValueError('Slack delivery is disabled or the bot token is missing')
    destinations = [m['destination'] for m in plan['messages']]
    if len(destinations) != len(set(destinations)) or any(not re.fullmatch(r'[CGUW][A-Z0-9]+', d) for d in destinations):
        raise ValueError('Invalid or duplicate Slack destination')
    results = []
    stopped = False
    for message in plan['messages']:
        result = {'destination': message['destination'], 'state': 'not-attempted'}
        results.append(result)
        if stopped:
            continue
        request = urllib.request.Request('https://slack.com/api/chat.postMessage',
            data=json.dumps(dict(message['payload'], channel=message['destination'])).encode(),
            headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json; charset=utf-8'}, method='POST')
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                body = json.loads(response.read())
                if response.status == 200 and body.get('ok') is True and isinstance(body.get('ts'), str):
                    result.update(state='delivered', timestamp=body['ts'])
                elif response.status == 200 and body.get('ok') is False:
                    result.update(state='error', error='Slack rejected delivery; inspect app configuration')
                    stopped = True
                else:
                    result.update(state='uncertain', error='Slack acknowledgement incomplete; inspect before retrying')
                    stopped = True
        except Exception:
            # Neither response bodies nor transport exceptions may expose credentials.
            result.update(state='uncertain', error='Delivery uncertain; inspect Slack before retrying')
            stopped = True
    return results
