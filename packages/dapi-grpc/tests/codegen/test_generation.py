"""Integration regressions; run with `yarn workspace @dashevo/dapi-grpc exec python3 -m unittest discover -s tests/codegen -v`."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

PACKAGE = Path(__file__).resolve().parents[2]


def snapshot(path):
    return {str(p.relative_to(path)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in path.rglob('*') if p.is_file()}


class GenerationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.toolchain = Path(subprocess.check_output(
            ['python3', str(PACKAGE / 'scripts/setup-codegen.py')], text=True).strip())

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='client generation with spaces ')
        self.addCleanup(self.tmp.cleanup)
        self.package = Path(self.tmp.name) / 'dapi-grpc'
        shutil.copytree(PACKAGE, self.package, ignore=shutil.ignore_patterns('.clients-build.*'))

    def run_build(self, toolchain=None):
        env = dict(os.environ, DAPI_GRPC_TOOLCHAIN=str(toolchain or self.toolchain))
        return subprocess.run(['bash', str(self.package / 'scripts/build.sh')],
                              env=env, capture_output=True, text=True)

    def test_should_generate_all_languages_and_be_idempotent(self):
        manual = self.package / 'clients/core/v0/web/handwritten.txt'
        manual.write_text('preserve me')
        result = self.run_build()
        self.assertEqual(result.returncode, 0, result.stderr)
        clients = self.package / 'clients'
        for name in ('core/v0/java/org/dash/platform/dapi/v0/CoreGrpc.java',
                     'core/v0/objective-c/Core.pbrpc.h', 'core/v0/python/core_pb2_grpc.py',
                     'platform/v0/web/platform_pb_service.d.ts', 'drive/v0/nodejs/drive_pbjs.js'):
            self.assertTrue((clients / name).is_file(), name)
        first = snapshot(clients)
        result = self.run_build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(snapshot(clients), first)
        self.assertEqual(manual.read_text(), 'preserve me')

    def test_should_preserve_clients_when_a_late_generator_fails(self):
        broken = Path(self.tmp.name) / 'broken-toolchain'
        (broken / 'bin').mkdir(parents=True)
        shutil.copy2(self.toolchain / 'lock.json', broken / 'lock.json')
        for binary in (self.toolchain / 'bin').iterdir():
            if binary.name != 'grpc_python_plugin':
                (broken / 'bin' / binary.name).symlink_to(binary)
        (broken / 'include').symlink_to(self.toolchain / 'include', target_is_directory=True)
        plugin = broken / 'bin/grpc_python_plugin'
        plugin.write_text('#!/bin/sh\nexit 23\n')
        plugin.chmod(0o755)
        before = snapshot(self.package / 'clients')
        result = self.run_build(broken)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Plugin failed', result.stderr)
        self.assertEqual(snapshot(self.package / 'clients'), before)
        self.assertEqual(list(self.package.glob('.clients-build.*')), [])

    def test_should_reject_mismatched_toolchain_before_touching_clients(self):
        broken = Path(self.tmp.name) / 'wrong-version'
        broken.mkdir()
        (broken / 'lock.json').write_text(json.dumps({'versions': {'protobuf': '32.0'}}))
        before = snapshot(self.package / 'clients')
        result = self.run_build(broken)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('versions differ', result.stderr)
        self.assertEqual(snapshot(self.package / 'clients'), before)
