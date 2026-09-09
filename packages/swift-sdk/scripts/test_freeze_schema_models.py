#!/usr/bin/env python3
"""Tests for freeze_schema_models.py.

Run from the repository root:

    python3 -m unittest discover -s packages/swift-sdk/scripts -p 'test_*.py'

These cover the generator as a generator: that `--check` is an exact byte
comparison of the committed frozen files against what `FREEZES` and the
repository history produce, and that the copy is not truncated by a brace
inside a string or a comment. Whether the freeze is COMPLETE is not a
question this script can answer; see `DashModelMigrationTests`.
"""

import os
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import freeze_schema_models as gen  # noqa: E402

ROOT = gen.repo_root()


class CheckTests(unittest.TestCase):
    """`--check` proves one thing: the committed files are the generator's output."""

    @classmethod
    def setUpClass(cls):
        cls.files = gen.render_all(ROOT)

    def setUp(self):
        # A scratch root holding a copy of the committed frozen files.
        self.scratch = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.scratch)
        shutil.copytree(
            os.path.join(ROOT, gen.OUT_DIR), os.path.join(self.scratch, gen.OUT_DIR)
        )

    def test_the_committed_files_are_the_generators_output(self):
        self.assertEqual(len(self.files), 37)
        self.assertEqual(gen.check_problems(ROOT, self.files), [])

    def test_a_hand_edit_to_a_frozen_file_is_reported(self):
        path = f"{gen.OUT_DIR}/DashSchemaV1+PersistentWallet.swift"
        with open(os.path.join(self.scratch, path), "a", encoding="utf-8") as f:
            f.write("// edited by hand\n")
        self.assertEqual(gen.check_problems(self.scratch, self.files), [f"differs:  {path}"])

    def test_a_missing_and_a_stale_file_are_reported(self):
        missing = f"{gen.OUT_DIR}/DashSchemaV3+PersistentAssetLock.swift"
        stale = f"{gen.OUT_DIR}/DashSchemaV9+PersistentGhost.swift"
        os.rename(
            os.path.join(self.scratch, missing), os.path.join(self.scratch, stale)
        )
        self.assertEqual(
            gen.check_problems(self.scratch, self.files),
            [f"missing:  {missing}", f"stale:    {stale}"],
        )


class BlockEndTests(unittest.TestCase):
    """A brace that is not code must not end the copied block early."""

    def test_braces_in_strings_and_comments_do_not_count(self):
        lines = [
            "final class PersistentThing {",
            '    init() { let brace = "{"; _ = brace }',
            "    // a } in a comment",
            "    var late: Int",
            "}",
            "extension PersistentThing {}",
        ]
        self.assertEqual(gen.block_end(lines, 0), 4)

    def test_a_block_comment_is_refused_rather_than_misread(self):
        lines = ["final class PersistentThing {", "    /* { */", "}"]
        with self.assertRaises(SystemExit):
            gen.block_end(lines, 0)


if __name__ == "__main__":
    unittest.main()
