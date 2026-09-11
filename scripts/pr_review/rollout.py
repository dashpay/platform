"""Prepare repository-local files using a pinned shared policy implementation."""

import argparse
import json
from pathlib import Path
import re

from .main import ROOT, REPOSITORIES
from .policy import codeowners, validate_policy


def write_bundle(repository, engine_revision, destination):
    if repository not in REPOSITORIES - {'dashpay/platform'}:
        raise ValueError('Expected a configured external repository')
    if not re.fullmatch(r'[0-9a-f]{40}', engine_revision):
        raise ValueError('A full engine commit SHA is required')
    destination = Path(destination)
    if destination.exists():
        raise ValueError('Destination must not exist; existing work is never overwritten')
    name = repository.split('/')[1]
    policy_text = (ROOT / '.github/pr-review-policies' / f'{name}.json').read_text()
    policy = json.loads(policy_text)
    validate_policy(policy)
    if policy['repository'] != repository:
        raise ValueError('Policy repository mismatch')
    workflow = f'''name: PR review policy
on:
  pull_request_target:
    types: [opened, reopened, synchronize, ready_for_review, converted_to_draft, closed, edited]
  issue_comment:
    types: [created, edited, deleted]
  workflow_run:
    workflows: [PR review signal]
    types: [completed]
  push:
    paths: ['.github/pr-review-policy.json']
  workflow_dispatch:
  schedule:
    - cron: '*/15 * * * *'
permissions:
  contents: read
  pull-requests: write
  issues: write
  statuses: write
jobs:
  policy:
    uses: dashpay/platform/.github/workflows/pr-review-reusable.yml@{engine_revision}
    with:
      engine_revision: {engine_revision}
'''
    files = {
        '.github/pr-review-policy.json': policy_text,
        '.github/CODEOWNERS': codeowners(policy),
        '.github/workflows/pr-review-policy.yml': workflow,
        '.github/workflows/pr-review-signal.yml': (ROOT / '.github/workflows/pr-review-signal.yml').read_text(),
    }
    for relative, content in files.items():
        path = destination / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    unresolved = [a['id']+': '+', '.join(a['unresolved']) for a in policy['areas'] if a.get('unresolved')]
    note = f'''# {repository} review policy packet

Shared engine revision: `{engine_revision}`. This commit must be published in dashpay/platform before these workflows can run.

Review these files against the target repository's current default branch. The packet is not a patch application and must not overwrite unrelated local work. Keep the target repository's existing protections and readiness automation until its reviewed activation plan replaces them. `.github/CODEOWNERS` takes precedence over any root CODEOWNERS; inspect the resulting roster explicitly.

Writes are disabled unless the target repository sets PR_REVIEW_AUTOMATION_ENABLED=true. Create the ready-for-human label and verify both bot producers before enabling. Confirm permissions and target branches separately. Do not relax existing native approval requirements until the replacement status has been exercised.

Configuration blockers: {('; '.join(unresolved)) or 'none recorded; live preflight still required'}.
'''
    (destination / 'REVIEW_POLICY_ROLLOUT.md').write_text(note)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', required=True)
    parser.add_argument('--engine-revision', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(write_bundle(args.repo, args.engine_revision, args.output))


if __name__ == '__main__':
    main()
