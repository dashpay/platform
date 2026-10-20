"""Execute the cache settings action with synthetic, credential-free inputs."""
import json
from pathlib import Path
import subprocess
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]


class LayerCacheTests(unittest.TestCase):
    def settings(self, shared=False, endpoint='https://cache.example.invalid'):
        action = (ROOT / '.github/actions/s3-layer-cache-settings/action.yaml').read_text()
        script = textwrap.dedent(action.split('        script: |\n', 1)[1])
        values = {'inputs.region': 'test-region', 'inputs.bucket': 'test-bucket',
                  'inputs.prefix': 'cache-layers/linux/amd64/', 'inputs.endpoint': endpoint,
                  'inputs.head_ref': 'refs/pull/42/merge', 'inputs.name': 'dashmate-helper',
                  'github.sha': 'a' * 40, 'inputs.mode': 'max',
                  'inputs.cache_to_name': str(shared).lower()}
        for key, value in values.items():
            script = script.replace('${{ ' + key + ' }}', value)
        self.assertNotIn('${{', script)
        harness = 'const output = {}; const core = {setOutput: (k, v) => output[k] = v};\n'
        result = subprocess.check_output(['node', '-e', harness + script
                                          + '\nconsole.log(JSON.stringify(output));'], text=True)
        return json.loads(result)

    def test_should_tolerate_only_export_errors_and_preserve_imports(self):
        settings = self.settings()
        self.assertTrue(settings['cache_to'].endswith(',ignore-error=true'))
        self.assertEqual(settings['cache_to'].count('ignore-error='), 1)
        imports = settings['cache_from'].splitlines()
        self.assertEqual(len(imports), 3)
        for item in imports:
            self.assertNotIn('ignore-error', item)
            self.assertNotIn('mode=', item)
            self.assertIn('prefix=cache-layers/linux/amd64/', item)
            self.assertIn('endpoint_url=https://cache.example.invalid', item)
        expected = 'dashmate-helper_sha_' + 'a' * 40 + ';dashmate-helper_tag_refs-pull-42-merge'
        self.assertIn('name=' + expected + ',ignore-error=true', settings['cache_to'])

    def test_should_keep_shared_manifest_writes_explicitly_opt_in(self):
        self.assertNotIn(';dashmate-helper,', self.settings()['cache_to'])
        self.assertIn(';dashmate-helper,ignore-error=true', self.settings(shared=True)['cache_to'])
        self.assertNotIn('endpoint_url=', self.settings(endpoint='')['cache_to'])

    def test_should_not_ignore_build_or_registry_publication_failures(self):
        action = (ROOT / '.github/actions/docker/action.yaml').read_text()
        build = action.split('    - name: Build and push Docker image', 1)[1]
        self.assertNotIn('continue-on-error', build)
        self.assertNotIn('ignore-error', build)
        self.assertIn('cache-to: ${{ steps.layer_cache_settings.outputs.cache_to }}', build)
        self.assertIn('name-canonical=true,push=true', build)
