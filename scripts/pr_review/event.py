"""Select bounded reconciliation work from event metadata, never event code."""

import json
import os
from pathlib import Path

from .main import run


def selections(kind, event):
    if kind in {'schedule', 'push', 'workflow_dispatch'}:
        return [['--batch-size', '3']]
    if kind == 'pull_request_target':
        numbers = [event['pull_request']['number']]
    elif kind == 'issue_comment':
        if 'pull_request' not in event['issue']:
            return []
        numbers = [event['issue']['number']]
    elif kind == 'workflow_run':
        numbers = [pr['number'] for pr in event['workflow_run']['pull_requests']]
    else:
        raise ValueError('Unsupported review event')
    if any(type(number) is not int or number <= 0 for number in numbers):
        raise ValueError('Invalid event PR number')
    return [['--pr', str(number)] for number in dict.fromkeys(numbers)]


def main():
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    options = ['sync', '--repo', os.environ['GITHUB_REPOSITORY']]
    if os.environ.get('PR_REVIEW_REPOSITORY_ROOT'):
        options += ['--repository-root', os.environ['PR_REVIEW_REPOSITORY_ROOT']]
    if os.environ.get('PR_REVIEW_AUTOMATION_ENABLED') == 'true':
        options.append('--apply')
    for selection in selections(os.environ['GITHUB_EVENT_NAME'], event):
        result = run(options + selection)
        if result:
            return result
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
