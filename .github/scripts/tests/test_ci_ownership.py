"""Exercise the actual privileged workflow script with a mocked GitHub API."""
import json
from pathlib import Path
import subprocess
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / ".github/workflows/ci-ownership.yml"


def run_assignment(files, **overrides):
    fixture = {
        "files": files,
        "pr": {
            "number": 42, "state": "open", "draft": True,
            "changed_files": len(files), "assignees": [],
        },
        "accept_assignment": True,
    }
    fixture["accept_assignment"] = overrides.pop("accept_assignment", True)
    fixture["pr"].update(overrides)
    script = textwrap.dedent(WORKFLOW.read_text().split("          script: |\n", 1)[1])
    harness = """
const fixture = JSON.parse(require('node:fs').readFileSync(0, 'utf8'));
const calls = [];
const context = { repo: { owner: 'dashpay', repo: 'platform' }, issue: { number: 42 } };
const github = {
  rest: {
    pulls: {
      get: async () => ({ data: fixture.pr }),
      listFiles: Symbol('listFiles'),
    },
    issues: {
      addAssignees: async (params) => {
        calls.push({ method: 'addAssignees', params });
        return { data: { assignees: [
          ...fixture.pr.assignees,
          ...(fixture.accept_assignment ? [{ login: 'ktechmidas' }] : []),
        ] } };
      },
    },
  },
  paginate: async (method, params) => {
    if (method !== github.rest.pulls.listFiles || params.per_page !== 100) {
      throw new Error('Expected paginated pull-request files');
    }
    calls.push({ method: 'paginate', params });
    return fixture.files;
  },
};
const core = {
  info: () => {},
  warning: (message) => calls.push({ method: 'warning', message }),
};
(async () => {
""" + script + """
})().then(() => {
  process.stdout.write(JSON.stringify(calls));
}).catch((error) => {
  process.stderr.write(error.message);
  process.exitCode = 1;
});
"""
    result = subprocess.run(["node", "-e", harness], input=json.dumps(fixture),
                            text=True, capture_output=True)
    return result


class CIOwnershipTests(unittest.TestCase):
    def assignment_calls(self, files, **overrides):
        result = run_assignment(files, **overrides)
        self.assertEqual(result.returncode, 0, result.stderr)
        return [call for call in json.loads(result.stdout)
                if call["method"] == "addAssignees"]

    def test_routes_ci_and_build_helpers_including_draft_prs(self):
        for path in [
            ".github/workflows/new.yml", ".github/actions/new/action.yml",
            ".github/scripts/new.py", ".github/runner-requirements.json",
            ".github/package-filters/new.yml", ".github/CODEOWNERS",
            ".cargo/config.toml", ".devcontainer/Dockerfile", "ci/new.sh",
            "scripts/release/release.sh", "Dockerfile", ".codecov.yml",
            "rust-toolchain.toml", "package.json", "Cargo.toml",
            "packages/dapi-grpc/scripts/setup-codegen.py",
            "packages/swift-sdk/scripts/freeze_appstore_release.py",
            "packages/swift-sdk/build_ios.sh", "packages/swift-sdk/setup_ios_build.sh",
            "packages/swift-sdk/run_tests.sh", "packages/swift-sdk/run_integration_tests.sh",
            "packages/swift-sdk/SwiftExampleApp/Scripts/generate_bindings.sh",
            "packages/platform-test-suite/bin/test.sh",
            "packages/kotlin-sdk/build_android.sh", "packages/wasm-drive-verify/build.sh",
            "packages/wasm-sdk/scripts/build.sh", "packages/new-sdk/publish.sh",
        ]:
            with self.subTest(path=path):
                calls = self.assignment_calls([{"filename": path}])
                self.assertEqual(len(calls), 1)
                self.assertEqual(calls[0]["params"], {
                    "owner": "dashpay", "repo": "platform",
                    "issue_number": 42, "assignees": ["ktechmidas"],
                })

    def test_does_not_route_ordinary_source_docs_or_lookalike_paths(self):
        files = [{"filename": path} for path in [
            "packages/rs-drive/src/lib.rs", "packages/dapi/scripts/api.js",
            "packages/swift-sdk/Sources/SwiftDashSDK/SDK.swift",
            "docs/ci-guide.md", "README.md", ".github-lookalike/test.yml",
            "scripts-lookalike/test.sh", "packages/new-sdk/src/build.sh",
        ]]
        self.assertEqual(self.assignment_calls(files), [])

    def test_routes_deleted_files_and_renames_out_of_ci(self):
        for file in [
            {"filename": ".github/workflows/old.yml", "status": "removed"},
            {"filename": "archive/old.yml", "status": "renamed",
             "previous_filename": ".github/workflows/old.yml"},
        ]:
            with self.subTest(file=file):
                self.assertEqual(len(self.assignment_calls([file])), 1)

    def test_finds_ci_beyond_the_actions_path_filter_limit(self):
        files = [{"filename": "docs/page-" + str(i) + ".md"} for i in range(350)]
        files.append({"filename": ".github/workflows/new.yml"})
        self.assertEqual(len(self.assignment_calls(files)), 1)

    def test_routes_conservatively_when_github_truncates_files(self):
        result = run_assignment([{"filename": "README.md"}], changed_files=3001)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call["method"] for call in json.loads(result.stdout)],
                         ["paginate", "warning", "addAssignees"])

    def test_preserves_existing_assignees_and_does_not_repeat_assignment(self):
        files = [{"filename": ".github/workflows/new.yml"}]
        self.assertEqual(len(self.assignment_calls(files, assignees=[{"login": "other"}])), 1)
        result = run_assignment(files, assignees=[{"login": "KTechMidas"}])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [])

    def test_does_not_assign_a_pr_that_closed_since_the_event(self):
        result = run_assignment([{"filename": "Dockerfile"}], state="closed")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [])

    def test_reports_if_github_silently_rejects_the_assignee(self):
        result = run_assignment([{"filename": "Dockerfile"}], accept_assignment=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("GitHub did not assign ktechmidas", result.stderr)


if __name__ == "__main__":
    unittest.main()
