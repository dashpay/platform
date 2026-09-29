"""Exact ordinary image pools must coexist with legacy development branches."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('ordinary_runner_image', ROOT / '.github/scripts/runner-image.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class OrdinaryImageRoutingTests(unittest.TestCase):
    def setUp(self):
        self.legacy = runner.read_manifest(Path(__file__).parent / 'fixtures/legacy-runner-requirements.json')
        self.new = copy.deepcopy(self.legacy)
        self.new['recipe_revision'] = '1be03edb7f4f18548d525b17dfac6ee31db17c99'
        self.assertEqual(runner.fingerprint(self.legacy), 'd272d01bcf3dfa620bbab1e9f31c3e1987862d33ec876bbad83c80f29de41a58')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.event = Path(self.temp.name) / 'event.json'
        self.output = Path(self.temp.name) / 'output'

    def select(self, manifest, kind='rust', event=None, files=None, arch=None, validation=False):
        self.event.write_text(json.dumps(event or {}))
        pr = {'number': 99, 'state': 'open', 'head': {'sha': 'a' * 40}, 'changed_files': len(files or [])}
        def api(path):
            if path == 'pulls/99': return pr
            if path == 'pulls/99/files?per_page=100&page=1': return [{'filename': p} for p in files or []]
            raise AssertionError('Unexpected API request: ' + path)
        with patch.dict(os.environ, {'GITHUB_EVENT_PATH': str(self.event)}), patch.object(runner, 'api', side_effect=api):
            runner.select(manifest, kind, self.output, 0, arch, validation)
        result = dict(line.split('=', 1) for line in self.output.read_text().splitlines())
        return json.loads(result['labels']), result['image_changed']

    def test_legacy_rust_kotlin_npm_labels_remain_unchanged(self):
        for kind, labels in [('rust', ['self-hosted', 'Linux', 'rust-ci']), ('kotlin', ['self-hosted', 'kotlin-ci']), ('npm', ['self-hosted', 'npm-pr'])]:
            with self.subTest(kind=kind): self.assertEqual(self.select(self.legacy, kind), (labels, 'false'))

    def test_new_recipe_never_uses_generic_legacy_capacity(self):
        for kind in ('rust', 'kotlin', 'npm'):
            with self.subTest(kind=kind):
                labels, changed = self.select(self.new, kind)
                self.assertEqual(labels, ['self-hosted', 'Linux', 'X64', f'platform-image-manifest-{runner.fingerprint(self.new)}-{kind}'])
                self.assertEqual(changed, 'false')
                self.assertTrue(set(labels).isdisjoint({'rust-ci', 'kotlin-ci', 'npm-pr', 'rust-ci-validation'}))
                self.assertFalse(any(label.startswith('platform-image-pr-') for label in labels))

    def test_changed_requirements_even_with_legacy_recipe_need_exact_pool(self):
        manifest = copy.deepcopy(self.legacy)
        manifest['requirements']['versions']['protoc'] = '32.1'
        labels, _ = self.select(manifest)
        self.assertEqual(labels[-1], 'platform-image-manifest-' + runner.fingerprint(manifest) + '-rust')

    def test_new_base_unchanged_application_pr_uses_versioned_capacity(self):
        labels, changed = self.select(self.new, event={'pull_request': {'number': 99, 'head': {'sha': 'a' * 40}}}, files=['Cargo.lock'])
        self.assertEqual(labels[-1], 'platform-image-manifest-' + runner.fingerprint(self.new) + '-rust')
        self.assertEqual(changed, 'false')

    def test_arm64_change_cannot_reset_new_amd64_pool_to_generic(self):
        labels, changed = self.select(self.new, event={'pull_request': {'number': 99, 'head': {'sha': 'a' * 40}}}, files=[runner.ARM64_MANIFEST])
        self.assertEqual(labels[-1], 'platform-image-manifest-' + runner.fingerprint(self.new) + '-rust')
        self.assertEqual(changed, 'false')

    def test_explicit_arm64_pools_are_not_amd64_versioned_pools(self):
        for validation, kind_label in [(False, 'rust-ci'), (True, 'rust-ci-validation')]:
            labels, _ = self.select(self.new, arch='ARM64', validation=validation)
            self.assertEqual(labels, ['self-hosted', 'Linux', 'ARM64', kind_label])

    def test_explicit_x64_uses_exact_manifest_pool(self):
        labels, _ = self.select(self.new, arch='X64')
        self.assertEqual(labels[:3], ['self-hosted', 'Linux', 'X64'])
        self.assertEqual(labels[-1], 'platform-image-manifest-' + runner.fingerprint(self.new) + '-rust')

    def test_full_manifest_label_is_stable_under_key_order_only(self):
        reordered = dict(reversed(list(self.new.items())))
        self.assertEqual(self.select(self.new), self.select(reordered))


if __name__ == '__main__':
    unittest.main()
