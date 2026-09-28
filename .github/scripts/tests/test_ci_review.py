"""Exercise review-only routing using the actual privileged workflow script."""
import json
from pathlib import Path
import subprocess
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / ".github/workflows/ci-review.yml"
HEAD = "a" * 40


def run_review_request(files, **overrides):
    fixture = {
        "files": files,
        "pr": {
            "number": 42, "state": "open", "draft": False,
            "changed_files": len(files), "requested_reviewers": [],
            "user": {"login": "contributor"}, "head": {"sha": HEAD},
            "assignees": [{"login": "implementer"}],
        },
        "accept_request": overrides.pop("accept_request", True),
        "reviews": overrides.pop("reviews", []),
        "changes": overrides.pop("changes", {}),
    }
    fixture["pr"].update(overrides)
    script = textwrap.dedent(WORKFLOW.read_text().split("          script: |\n", 1)[1])
    harness = """
const fixture = JSON.parse(require('node:fs').readFileSync(0, 'utf8'));
const calls = [];
const context = {
  repo: { owner: 'dashpay', repo: 'platform' }, issue: { number: 42 },
  payload: { changes: fixture.changes },
};
const originalAssignees = JSON.stringify(fixture.pr.assignees);
const github = {
  rest: {
    pulls: {
      get: async () => ({ data: fixture.pr }),
      listFiles: Symbol('listFiles'),
      listReviews: Symbol('listReviews'),
      requestReviewers: async (params) => {
        calls.push({ method: 'requestReviewers', params });
        return { data: { requested_reviewers: [
          ...fixture.pr.requested_reviewers,
          ...(fixture.accept_request ? [{ login: 'ktechmidas' }] : []),
        ] } };
      },
    },
    issues: {
      addAssignees: async () => { throw new Error('Must never assign CI work'); },
      update: async () => { throw new Error('Must never change assignees'); },
    },
  },
  paginate: async (method, params) => {
    if (params.per_page !== 100) {
      throw new Error('Expected pagination');
    }
    if (method === github.rest.pulls.listFiles) {
      calls.push({ method: 'listFiles', params });
      return fixture.files;
    }
    if (method === github.rest.pulls.listReviews) {
      calls.push({ method: 'listReviews', params });
      return fixture.reviews;
    }
    throw new Error('Unexpected paginated endpoint');
  },
};
const core = {
  info: () => {},
  warning: (message) => calls.push({ method: 'warning', message }),
};
(async () => {
""" + script + """
})().then(() => {
  if (JSON.stringify(fixture.pr.assignees) !== originalAssignees) {
    throw new Error('Assignees must remain unchanged');
  }
  process.stdout.write(JSON.stringify(calls));
}).catch((error) => {
  process.stderr.write(error.message);
  process.exitCode = 1;
});
"""
    return subprocess.run(["node", "-e", harness], input=json.dumps(fixture),
                          text=True, capture_output=True)


class CIReviewTests(unittest.TestCase):
    def review_calls(self, files, **overrides):
        result = run_review_request(files, **overrides)
        self.assertEqual(result.returncode, 0, result.stderr)
        return [call for call in json.loads(result.stdout)
                if call["method"] == "requestReviewers"]

    def test_requests_review_for_ci_and_build_helpers_without_assigning_work(self):
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
                calls = self.review_calls([{"filename": path}])
                self.assertEqual(len(calls), 1)
                self.assertEqual(calls[0]["params"], {
                    "owner": "dashpay", "repo": "platform",
                    "pull_number": 42, "reviewers": ["ktechmidas"],
                })

    def test_does_not_route_ordinary_source_docs_or_lookalike_paths(self):
        files = [{"filename": path} for path in [
            "packages/rs-drive/src/lib.rs", "packages/dapi/scripts/api.js",
            "packages/swift-sdk/Sources/SwiftDashSDK/SDK.swift",
            "docs/ci-guide.md", "README.md", ".github-lookalike/test.yml",
            "scripts-lookalike/test.sh", "packages/new-sdk/src/build.sh",
        ]]
        self.assertEqual(self.review_calls(files), [])

    def test_routes_deleted_files_and_renames_out_of_ci(self):
        for file in [
            {"filename": ".github/workflows/old.yml", "status": "removed"},
            {"filename": "archive/old.yml", "status": "renamed",
             "previous_filename": ".github/workflows/old.yml"},
        ]:
            with self.subTest(file=file):
                self.assertEqual(len(self.review_calls([file])), 1)

    def test_finds_ci_beyond_the_actions_path_filter_limit(self):
        files = [{"filename": "docs/page-" + str(i) + ".md"} for i in range(350)]
        files.append({"filename": ".github/workflows/new.yml"})
        self.assertEqual(len(self.review_calls(files)), 1)

    def test_routes_conservatively_when_github_truncates_files(self):
        result = run_review_request([{"filename": "README.md"}], changed_files=3001)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call["method"] for call in json.loads(result.stdout)],
                         ["listFiles", "warning", "listReviews", "requestReviewers"])

    def test_preserves_other_reviewers_and_skips_a_pending_request(self):
        files = [{"filename": ".github/workflows/new.yml"}]
        self.assertEqual(len(self.review_calls(
            files, requested_reviewers=[{"login": "other"}])), 1)
        result = run_review_request(files, requested_reviewers=[{"login": "KTechMidas"}])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [])

    def test_waits_for_drafts_and_never_requests_self_review_or_closed_prs(self):
        for changes in [{"draft": True}, {"user": {"login": "KTechMidas"}}, {"state": "closed"}]:
            with self.subTest(changes=changes):
                result = run_review_request([{"filename": "Dockerfile"}], **changes)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout), [])

    def test_does_not_repeat_a_completed_review_of_the_same_revision(self):
        for state in ["APPROVED", "CHANGES_REQUESTED", "COMMENTED"]:
            with self.subTest(state=state):
                reviews = [{"user": {"login": "KTechMidas"}, "commit_id": HEAD, "state": state}]
                self.assertEqual(self.review_calls([{"filename": "Dockerfile"}], reviews=reviews), [])

    def test_requests_again_after_new_commits_or_a_dismissed_review(self):
        for commit_id, state in [("b" * 40, "APPROVED"), (HEAD, "DISMISSED")]:
            with self.subTest(commit_id=commit_id, state=state):
                reviews = [{"user": {"login": "ktechmidas"}, "commit_id": commit_id, "state": state}]
                self.assertEqual(len(self.review_calls(
                    [{"filename": "Dockerfile"}], reviews=reviews)), 1)

    def test_requests_again_if_retargeting_changes_the_diff_without_a_new_head(self):
        reviews = [{"user": {"login": "ktechmidas"}, "commit_id": HEAD, "state": "APPROVED"}]
        self.assertEqual(len(self.review_calls(
            [{"filename": "Dockerfile"}], reviews=reviews,
            changes={"base": {"ref": {"from": "v4.3-dev"}}})), 1)

    def test_ignores_reviews_from_deleted_or_other_accounts(self):
        reviews = [{"user": user, "commit_id": HEAD, "state": "APPROVED"}
                   for user in [None, {"login": "someone-else"}]]
        self.assertEqual(len(self.review_calls(
            [{"filename": "Dockerfile"}], reviews=reviews)), 1)

    def test_reports_if_github_does_not_accept_the_reviewer(self):
        result = run_review_request([{"filename": "Dockerfile"}], accept_request=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("GitHub did not request review from ktechmidas", result.stderr)


if __name__ == "__main__":
    unittest.main()
