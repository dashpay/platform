"""Keep a candidate-image wait off the time-limited light runner pool.

`runner-image.py select` can wait up to two hours for a PR's candidate image,
longer than a dash-x64-light job may run. A selector job may use the light
pool only through the `runner-image-changed` guard, which every path that
might wait turns into a GitHub-hosted runner.
"""
import importlib.util
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[3]
WORKFLOWS = ROOT / ".github/workflows"
spec = importlib.util.spec_from_file_location("routing_runner_image", ROOT / ".github/scripts/runner-image.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

GUARDED = "${{ !inputs.runner-image-changed && vars.DASH_LIGHT_RUNNER || 'ubuntu-24.04' }}"
FAIL_SAFE = "${{ needs.changes.outputs.runner-image-changed != 'false' }}"


def children(lines, indent):
    """Map each mapping key at `indent` spaces to the lines nested under it."""
    found, key = {}, None
    for line in lines:
        depth = len(line) - len(line.lstrip(" "))
        match = re.fullmatch(r"([\w-]+):.*", line[indent:]) if depth == indent else None
        if match:
            key = match.group(1)
            found[key] = []
        elif key and line.strip() and depth > indent:
            found[key].append(line)
    return found


def scalar(lines, indent, key):
    """Return the inline value of `key` at `indent` spaces, or None."""
    for line in lines:
        match = re.fullmatch(" " * indent + re.escape(key) + r":\s*(.*?)\s*", line)
        if match:
            return match.group(1)
    return None


def load(path):
    """Read a workflow, without comment lines, as its top-level blocks."""
    lines = [line for line in path.read_text().splitlines() if not line.lstrip().startswith("#")]
    return children(lines, 0)


def jobs(path):
    return children(load(path).get("jobs", []), 2)


def selector_jobs():
    """Yield (workflow, job name, job lines) for every runner-image selector."""
    for path in sorted(WORKFLOWS.glob("*.yml")):
        for name, lines in jobs(path).items():
            if any("runner-image.py select" in line for line in lines):
                yield path, name, lines


def guarded_workflows():
    return sorted({path for path, _, lines in selector_jobs() if scalar(lines, 4, "runs-on") == GUARDED})


class SelectRunnerRoutingTests(unittest.TestCase):
    def test_should_keep_every_selector_hosted_unless_guarded(self):
        """A selector reaches the light pool only through the runner-image guard."""
        for path, name, lines in selector_jobs():
            with self.subTest(workflow=path.name, job=name):
                runs_on = scalar(lines, 4, "runs-on") or ""
                self.assertTrue(runs_on == GUARDED or re.fullmatch(r"ubuntu-[\w.]+", runs_on), runs_on)
        self.assertLessEqual({"tests-rs-workspace.yml", "tests-rs-wallet.yml"},
                             {path.name for path in guarded_workflows()})

    def test_should_default_the_guard_to_a_hosted_runner(self):
        """A caller that does not pass the flag gets a hosted selector."""
        for path in guarded_workflows():
            with self.subTest(workflow=path.name):
                call = children(children(load(path)["on"], 2)["workflow_call"], 4)
                guard = children(call["inputs"], 6)["runner-image-changed"]
                self.assertEqual(scalar(guard, 8, "type"), "boolean")
                self.assertEqual(scalar(guard, 8, "default"), "true")

    def test_should_flag_exactly_the_manifest_the_selector_waits_on(self):
        """The changes job reports true for any PR that edits the AMD64 manifest."""
        changes = jobs(WORKFLOWS / "tests.yml")["changes"]
        outputs = children(changes, 4)["outputs"]
        self.assertEqual(scalar(outputs, 6, "runner-image-changed"),
                         "${{ steps.filter-rs-workflows.outputs.runner-image }}")
        text = "\n".join(changes) + "\n"
        step = re.split(r"\n {6}- ", text[text.index("id: filter-rs-workflows"):])[0] + "\n"
        paths = re.search(r"^( +)runner-image:\n((?:\1 +- .*\n)+)", step, re.MULTILINE)
        self.assertIsNotNone(paths)
        self.assertEqual(re.findall(r"- (\S+)", paths.group(2)), [runner.MANIFEST])

    def test_should_pass_the_guard_fail_safe_from_every_caller(self):
        """Only an explicit 'false' from the changes job puts an AMD64 selector on the light pool."""
        guarded = {path.name for path in guarded_workflows()}
        callers = set()
        for path in sorted(WORKFLOWS.glob("*.yml")):
            for name, lines in jobs(path).items():
                uses = scalar(lines, 4, "uses") or ""
                if not uses.startswith("./.github/workflows/") or Path(uses).name not in guarded:
                    continue
                callers.add(name)
                with self.subTest(workflow=path.name, job=name):
                    inputs = children(lines, 4).get("with", [])
                    allowed = [None, "true", FAIL_SAFE]
                    if scalar(inputs, 6, "runner-architecture") == "ARM64":
                        # An ARM64 selection returns before the AMD64 candidate
                        # wait (see test_runner_image's ARM64 tests).
                        allowed.append("false")
                    self.assertIn(scalar(inputs, 6, "runner-image-changed"), allowed)
        self.assertLessEqual({"rs-workspace-tests", "rs-arm64-image-tests", "rs-wallet-tests"}, callers)


if __name__ == "__main__":
    unittest.main()
