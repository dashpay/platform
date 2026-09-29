"""Draft image changes defer only Kotlin's candidate-dependent work until ready."""
import base64
import copy
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('draft_runner_image', ROOT / '.github/scripts/runner-image.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
FIXTURE = json.loads((Path(__file__).parent / 'fixtures/candidate-status-pr5151.json').read_text())
HEAD = FIXTURE['combined']['sha']


class DraftCandidateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.event = Path(self.temp.name) / 'event.json'
        self.output = Path(self.temp.name) / 'output'
        self.manifest = runner.read_manifest(ROOT / runner.MANIFEST)
        self.pr = {'number': 5151, 'state': 'open', 'head': {'sha': HEAD},
                   'changed_files': 1, 'draft': True}
        self.files = [{'filename': runner.MANIFEST}]
        self.calls = []
        self.statuses = []
        self.current = None

    def api(self, path):
        self.calls.append(path)
        if path == 'pulls/5151':
            if self.current is not None and self.calls.count(path) > 1:
                return self.current
            return self.pr
        if path == 'pulls/5151/files?per_page=100&page=1':
            return self.files
        if path == f'contents/{runner.MANIFEST}?ref={HEAD}':
            return {'content': base64.b64encode(json.dumps(self.manifest).encode()).decode()}
        if path == f'commits/{HEAD}/statuses?per_page=100&page=1':
            return self.statuses
        if path == 'actions/runs/' + str(FIXTURE['publisher_run']['id']):
            return FIXTURE['publisher_run']
        raise AssertionError('Unexpected API request: ' + path)

    def select(self, event=None, wait_seconds=0, defer_draft=True):
        self.event.write_text(json.dumps(event if event is not None else {'pull_request': self.pr}))
        self.output.unlink(missing_ok=True)
        with patch.dict(os.environ, {'GITHUB_EVENT_PATH': str(self.event)}), \
             patch.object(runner, 'api', side_effect=self.api), \
             patch.object(runner.time, 'sleep', side_effect=AssertionError('Must not wait for a draft publisher')):
            runner.select(self.manifest, 'kotlin', self.output, wait_seconds, defer_draft=defer_draft)
        return dict(line.split('=', 1) for line in self.output.read_text().splitlines())

    def test_draft_manifest_pr_defers_without_polling_or_runnable_labels(self):
        result = self.select()
        self.assertEqual(result, {'labels': '[]', 'image_changed': 'true', 'candidate_deferred': 'true'})
        self.assertEqual(self.calls, ['pulls/5151', 'pulls/5151/files?per_page=100&page=1'])

    def test_application_only_draft_keeps_ordinary_execution(self):
        self.files = [{'filename': 'packages/kotlin-sdk/src/application.kt'}]
        result = self.select()
        self.assertEqual(result['candidate_deferred'], 'false')
        self.assertEqual(result['image_changed'], 'false')
        self.assertEqual(json.loads(result['labels']), runner.ordinary_labels(self.manifest, 'kotlin'))
        self.assertFalse(any(p.startswith('commits/') for p in self.calls))

    def test_ready_after_old_deadline_starts_fresh_exact_candidate_selection(self):
        with patch.object(runner.time, 'monotonic', return_value=0):
            self.assertEqual(self.select(wait_seconds=7200)['candidate_deferred'], 'true')
        self.pr['draft'] = False
        self.statuses = copy.deepcopy(FIXTURE['statuses'])
        # More than two hours later: ready_for_review is a new selection, not a
        # rerun of the expired draft wait. Publisher provenance is still checked.
        with patch.object(runner.time, 'monotonic', return_value=10800):
            result = self.select({'action': 'ready_for_review', 'pull_request': self.pr})
        digest = FIXTURE['statuses'][0]['description'][7:]
        self.assertEqual(json.loads(result['labels'])[-1], f'platform-image-pr-5151-{HEAD}-{digest}-kotlin')
        self.assertEqual(result['candidate_deferred'], 'false')
        self.assertTrue(any(p.startswith('actions/runs/') for p in self.calls))

    def test_ready_without_publication_still_fails_not_false_success(self):
        self.pr['draft'] = False
        with self.assertRaisesRegex(ValueError, 'not published in time'):
            self.select()
        self.assertFalse(self.output.exists())

    def test_conversion_to_draft_while_waiting_defers(self):
        self.pr['draft'] = False
        self.current = dict(self.pr, draft=True)
        result = self.select(wait_seconds=60)
        self.assertEqual(result['candidate_deferred'], 'true')
        self.assertEqual(json.loads(result['labels']), [])

    def test_conversion_to_draft_at_final_publication_check_defers(self):
        self.pr['draft'] = False
        self.current = dict(self.pr, draft=True)
        self.statuses = copy.deepcopy(FIXTURE['statuses'])
        self.assertEqual(self.select()['candidate_deferred'], 'true')
        self.assertEqual(self.calls.count('pulls/5151'), 2)

    def test_stale_or_closed_drafts_fail_before_defer(self):
        event = {'pull_request': copy.deepcopy(self.pr)}
        for replacement in (dict(self.pr, state='closed'), dict(self.pr, head={'sha': 'f' * 40})):
            with self.subTest(replacement=replacement), patch.object(self, 'pr', replacement):
                with self.assertRaisesRegex(ValueError, 'superseded'):
                    self.select(event)
                self.assertFalse(self.output.exists())

    def test_push_and_dispatch_do_not_defer(self):
        for event in ({'ref': 'refs/heads/v4.3-dev'}, {'inputs': {}}):
            with self.subTest(event=event):
                self.assertEqual(self.select(event)['candidate_deferred'], 'false')
        self.assertEqual(self.calls, [])

    def test_unmodified_consumers_do_not_implicitly_change_behavior(self):
        with self.assertRaisesRegex(ValueError, 'not published in time'):
            self.select(defer_draft=False)
        self.assertFalse(self.output.exists())

    def test_cli_passes_explicit_defer_option(self):
        args = ['runner-image.py', 'select', '--kind', 'kotlin', '--output', str(self.output), '--defer-draft']
        with patch.object(sys, 'argv', args), patch.object(runner, 'select') as select:
            runner.main()
        self.assertTrue(select.call_args.args[-1])

    def test_kotlin_ready_event_and_candidate_only_gate_preserve_fork_boundary(self):
        workflow = (ROOT / '.github/workflows/kotlin-sdk-build.yml').read_text()
        trigger = re.search(r'  pull_request:\n    types: \[([^]]+)\]', workflow)
        self.assertIsNotNone(trigger)
        self.assertEqual({t.strip() for t in trigger[1].split(',')},
                         {'opened', 'synchronize', 'reopened', 'ready_for_review'})
        self.assertIn('candidate_deferred: ${{ steps.select.outputs.candidate_deferred }}', workflow)
        self.assertIn('select --kind kotlin --defer-draft', workflow)
        job = workflow.split('  kotlin-sdk-build:\n', 1)[1]
        condition = re.search(r'    if: >-\n(.*?)    timeout-minutes:', job, re.S)[1]
        # Evaluate the actual narrowly supported expression, not a copy of it.
        for event in ('pull_request', 'workflow_dispatch', 'push'):
            for same_repo, trusted_fork, deferred in ((a, b, c) for a in (False, True)
                                                     for b in (False, True) for c in (False, True)):
                expression = condition.strip()
                replacements = {
                    "github.event_name != 'pull_request'": event != 'pull_request',
                    'github.event.pull_request.head.repo.full_name == github.repository': same_repo,
                    "github.event.pull_request.head.repo.owner.login == 'thepastaclaw'": trusted_fork,
                    "needs.select-runner.outputs.candidate_deferred != 'true'": not deferred,
                }
                for text, value in replacements.items():
                    expression = expression.replace(text, str(value))
                expression = ' '.join(expression.replace('||', ' or ').replace('&&', ' and ').split())
                self.assertIsNotNone(re.fullmatch(r'[()\sTrueFalsondr]+', expression))
                self.assertEqual(eval(expression, {'__builtins__': {}}),
                                 (event != 'pull_request' or same_repo or trusted_fork) and not deferred)
        # The selector itself is NOT draft-skipped: it must detect whether a
        # draft actually changes the manifest before deferring its worker.
        selector = workflow.split('  select-runner:\n', 1)[1].split('  kotlin-sdk-build:\n', 1)[0]
        self.assertNotIn('pull_request.draft', selector)
        self.assertNotIn('candidate_deferred !=', selector)


if __name__ == '__main__':
    unittest.main()
