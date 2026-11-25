"""Check how the review-rounds skill counts rounds and places findings.

The skill's diagnosis rests on these numbers: what counts as a round, whether a
finding sits on code written in response to the previous review, and how large
each response was. A reply counted as a round, or a finding placed on the wrong
side of a change, would point the diagnosis at the wrong cause.
"""
import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location(
    "review_rounds", ROOT / ".claude/skills/review-rounds/scripts/review_rounds.py"
)
rounds = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rounds)

PATCH = (
    "@@ -10,4 +10,5 @@ fn store()\n context\n-old\n"
    "+    let kept = read_slot()?;\n+    }\n context\n context"
)
# Takes back the line PATCH wrote, and its closing brace, which is too short to count.
UNDO = (
    "@@ -10,5 +10,4 @@ fn store()\n context\n-    let kept = read_slot()?;\n-    }\n"
    "+    replace_slot()?;\n context\n context"
)


def review(review_id, user, state, head, at, body=""):
    return {
        "id": review_id, "user": {"login": user}, "state": state,
        "commit_id": head, "submitted_at": at, "body": body,
    }


def comment(comment_id, review_id, user, path, line, body, reply_to=None, start=None):
    return {
        "id": comment_id, "pull_request_review_id": review_id, "user": {"login": user},
        "path": path, "original_line": line, "original_start_line": start, "side": "RIGHT",
        "body": body, "created_at": f"2026-01-01T00:00:{comment_id:02d}Z",
        "in_reply_to_id": reply_to,
    }


def commit(sha, message, *parents):
    return {
        "sha": sha, "commit": {"message": message},
        "parents": [{"sha": parent} for parent in parents or ("p",)],
    }


def compared(files, status="ahead"):
    return {"status": status, "truncated": False, "files": files}


def changed(additions, deletions, patch=None, status="modified"):
    return {"status": status, "additions": additions, "deletions": deletions, "patch": patch}


def history():
    """Three reviewed heads: a first pass, a response that drew findings, an approval."""
    return {
        "repo": "dashpay/platform",
        "pr": {
            "number": 7, "title": "feat: store a key", "user": {"login": "author"},
            "state": "open", "merged_at": None, "created_at": "2026-01-01T00:00:00Z",
            "base": {"ref": "v5.1-dev", "sha": "base"}, "head": {"sha": "h3"},
            "changed_files": 2, "additions": 60, "deletions": 5, "commits": 3,
        },
        "reviews": [
            review(1, "bot", "CHANGES_REQUESTED", "h1", "2026-01-01T01:00:00Z", "Two problems."),
            review(2, "author", "COMMENTED", "h1", "2026-01-01T01:30:00Z"),
            review(3, "bot", "COMMENTED", "h1", "2026-01-01T01:40:00Z"),
            review(4, "bot", "CHANGES_REQUESTED", "h2", "2026-01-01T03:00:00Z"),
            review(5, "human", "APPROVED", "h3", "2026-01-01T05:00:00Z", "Looks right now."),
        ],
        "comments": [
            comment(1, 1, "bot", "src/store.rs", 12, "**🔴 Blocking: Keep the old key**\n\nDetail."),
            comment(2, 1, "bot", "src/view.rs", 30, "**Minor** · Quick win\n\n**Trim the input.**"),
            comment(3, 2, "author", "src/store.rs", 12, "Fixed.", reply_to=1),
            comment(4, 3, "bot", "src/store.rs", 12, "Thanks.", reply_to=1),
            comment(5, 4, "bot", "src/store.rs", 12, "Rollback loses the key", start=11),
            comment(6, 4, "bot", "src/store.rs", 14, "Name the error"),
            comment(7, 4, "bot", "src/view.rs", 30, "Trim the label too"),
        ],
        "files": [{"filename": "src/store.rs"}, {"filename": "src/view.rs"}],
        "commits": [
            commit("h1", "feat: store a key"),
            commit("h2", "Revert \"feat: delete first\"\n\nThis reverts commit abc."),
            commit("h3", "fix: keep the old key"),
        ],
        "resolved": {1: True, 2: True, 5: False, 6: True, 7: True},
        "base_merges": {},
        "compares": {
            ("base", "h1"): compared({
                "src/store.rs": changed(40, 0, status="added"),
                "src/view.rs": changed(10, 5),
            }),
            ("h1", "h2"): compared({
                "src/store.rs": changed(2, 1, PATCH),
                "Cargo.lock": changed(300, 200),
            }),
            ("h2", "h3"): compared({"src/store.rs": changed(8, 4, UNDO)}),
        },
    }


class ReviewRoundsTest(unittest.TestCase):
    def setUp(self):
        self.report = rounds.build_report(history())

    def test_should_count_a_reviewed_head_with_findings_as_one_round(self):
        self.assertEqual(self.report["rounds"], 2)
        self.assertEqual([entry["round"] for entry in self.report["passes"]], [1, 2, None])
        self.assertEqual([len(entry["threads"]) for entry in self.report["passes"]], [2, 3, 0])

    def test_should_not_count_replies_or_the_author_as_a_round(self):
        first = self.report["passes"][0]
        self.assertEqual([item["reviewer"] for item in first["reviews"]], ["bot"])
        self.assertEqual(first["threads"][0]["replies"], 2)
        self.assertNotIn("author", self.report["reviewers"])

    def test_should_place_later_findings_against_the_previous_response(self):
        places = [thread["place"] for thread in self.report["passes"][1]["threads"]]
        self.assertEqual(places, [rounds.NEW_LINES, rounds.CHANGED_FILE, rounds.OLD_CODE])
        self.assertEqual(
            {thread["place"] for thread in self.report["passes"][0]["threads"]}, {rounds.FIRST}
        )
        self.assertEqual(
            self.report["later_findings"],
            {rounds.NEW_LINES: 1, rounds.CHANGED_FILE: 1, rounds.OLD_CODE: 1},
        )

    def test_should_size_a_response_by_the_pull_requests_own_files(self):
        response = self.report["passes"][0]["response"]
        self.assertEqual([item["path"] for item in response["files"]], ["src/store.rs"])
        self.assertEqual((response["additions"], response["deletions"]), (2, 1))
        self.assertEqual(
            self.report["changed_since_first_review"], {"additions": 10, "deletions": 5, "undone": 1}
        )
        self.assertIsNone(self.report["passes"][2]["response"])
        self.assertTrue(self.report["passes"][2]["unchanged"])

    def test_should_rank_the_files_findings_return_to(self):
        store = self.report["files"]["src/store.rs"]
        self.assertEqual((store["threads"], store["rounds"], store["responses"]), (3, [1, 2], 2))
        self.assertEqual(store["undone"], 1)
        self.assertEqual(self.report["files"]["src/view.rs"]["responses"], 0)
        self.assertEqual(self.report["areas"]["src"]["threads"], 5)

    def test_should_report_growth_reverts_and_open_threads(self):
        self.assertEqual(
            self.report["size_at_first_review"],
            {"files": 2, "additions": 50, "deletions": 5, "added_since": []},
        )
        self.assertEqual(self.report["reverts"], 1)
        self.assertEqual(self.report["unresolved"], 1)

    def test_should_say_unknown_when_a_head_cannot_be_compared(self):
        raw = history()
        raw["compares"][("h1", "h2")] = None
        report = rounds.build_report(raw)
        self.assertEqual(
            {thread["place"] for thread in report["passes"][1]["threads"]}, {rounds.UNKNOWN}
        )
        self.assertIsNone(report["passes"][0]["response"])
        self.assertIn("unavailable", rounds.render(report, 0))

    def test_should_measure_a_finding_against_its_own_reviewers_last_review(self):
        raw = history()
        raw["reviews"].append(review(6, "human", "COMMENTED", "h2", "2026-01-01T03:30:00Z"))
        raw["reviews"].append(review(7, "human", "COMMENTED", "h3", "2026-01-01T05:30:00Z"))
        raw["comments"].append(comment(8, 6, "human", "src/view.rs", 30, "Why trim here?"))
        raw["comments"].append(comment(9, 7, "human", "src/view.rs", 31, "Still unclear"))
        report = rounds.build_report(raw)
        placed = {
            thread["id"]: thread["place"] for entry in report["passes"] for thread in entry["threads"]
        }
        # The first time a reviewer looks, nothing is "old code" to them.
        self.assertEqual(placed[8], rounds.FIRST)
        self.assertEqual(placed[9], rounds.OLD_CODE)
        self.assertEqual(report["later_findings"][rounds.OLD_CODE], 2)

    def test_should_leave_a_merge_from_the_base_branch_out_of_a_response(self):
        raw = history()
        raw["commits"].insert(1, commit("merge", "Merge branch 'v5.1-dev'", "h1", "tip"))
        raw["compares"][("h1", "h2")]["files"]["src/view.rs"] = changed(7, 3)
        raw["compares"][("h1", "merge")] = compared({"src/view.rs": changed(7, 3)})
        raw["base_merges"] = {"merge": True}
        report = rounds.build_report(raw)
        response = report["passes"][0]["response"]
        self.assertEqual([item["path"] for item in response["files"]], ["src/store.rs"])
        self.assertTrue(response["merged"])
        self.assertEqual(report["passes"][1]["threads"][2]["place"], rounds.OLD_CODE)
        self.assertIn("a merge from the base branch is left out", rounds.render(report, 0))
        passes = rounds.review_passes(raw["pr"], raw["reviews"], raw["comments"])
        self.assertIn(
            ("h1", "merge"),
            rounds.compare_pairs(raw["pr"], passes, raw["commits"], raw["base_merges"]),
        )

    def test_should_keep_a_merge_of_another_branch_as_the_authors_change(self):
        raw = history()
        raw["commits"].insert(1, commit("merge", "Merge branch 'feat/stacked'", "h1", "tip"))
        raw["compares"][("h1", "h2")]["files"]["src/view.rs"] = changed(7, 3)
        raw["base_merges"] = {"merge": False}
        report = rounds.build_report(raw)
        response = report["passes"][0]["response"]
        self.assertEqual(
            sorted(item["path"] for item in response["files"]), ["src/store.rs", "src/view.rs"]
        )
        self.assertFalse(response["merged"])
        self.assertEqual(report["passes"][1]["threads"][2]["place"], rounds.CHANGED_FILE)
        passes = rounds.review_passes(raw["pr"], raw["reviews"], raw["comments"])
        self.assertNotIn(
            ("h1", "merge"),
            rounds.compare_pairs(raw["pr"], passes, raw["commits"], raw["base_merges"]),
        )

    def test_should_call_a_response_partial_when_a_merge_cannot_be_traced(self):
        raw = history()
        raw["commits"].insert(1, commit("merge", "Merge branch 'gone'", "h1", "tip"))
        raw["base_merges"] = {"merge": None}
        report = rounds.build_report(raw)
        self.assertTrue(report["passes"][0]["response"]["truncated"])

    def test_should_flag_a_response_that_rewrote_history(self):
        raw = history()
        del raw["commits"][0]
        report = rounds.build_report(raw)
        self.assertTrue(report["passes"][0]["response"]["rewritten"])
        self.assertFalse(report["passes"][1]["response"]["rewritten"])
        self.assertIn("history was rewritten in between", rounds.render(report, 0))

    def test_should_mark_a_round_that_requested_changes_without_a_new_thread(self):
        raw = history()
        raw["reviews"].append(review(6, "bot", "CHANGES_REQUESTED", "h3", "2026-01-01T04:00:00Z", "Still 1 blocking."))
        text = rounds.render(rounds.build_report(raw), 0)
        self.assertIn("Rounds: 3 (heads reviewed: 3)", text)
        self.assertIn("New threads per round: 2 3 0*", text)

    def test_should_list_long_threads_with_their_last_word(self):
        raw = history()
        raw["comments"] += [
            comment(10, 2, "author", "src/store.rs", 12, "It cannot fail here.", reply_to=5),
            comment(11, 3, "bot", "src/store.rs", 12, "Still applies.", reply_to=5),
            comment(12, 2, "author", "src/store.rs", 12, "Changed in h3.", reply_to=5),
            comment(13, 3, "bot", "src/store.rs", 12, "Disputed, left to maintainers.", reply_to=5),
        ]
        report = rounds.build_report(raw)
        text = rounds.render(report, 0)
        self.assertIn("## Long threads", text)
        self.assertIn("4 replies (2 by the author)", text)
        self.assertIn("last word, bot: Disputed, left to maintainers.", text)
        full = "\n".join(rounds.render_rounds(report, [2]) + rounds.render_threads(report, ["view.rs"]))
        self.assertIn("-> author, 2026-01-01 00:00:\n     It cannot fail here.", full)
        self.assertIn("## 2 threads on paths containing 'view.rs'", full)
        with self.assertRaises(ValueError):
            rounds.render_rounds(report, [9])
        with self.assertRaises(ValueError):
            rounds.render_threads(report, ["missing.rs"])

    def test_should_cut_a_long_summary_at_a_line_and_say_where_the_rest_is(self):
        raw = history()
        raw["reviews"][0]["body"] = "First line of the summary.\n" + "x" * 80
        text = rounds.render(rounds.build_report(raw), 40)
        self.assertIn("  First line of the summary.\n  [81 more characters: --round 1]", text)

    def test_should_point_a_cut_summary_of_a_pass_without_findings_at_a_working_option(self):
        raw = history()
        raw["reviews"][4]["body"] = "Looks right now.\n" + "x" * 80
        report = rounds.build_report(raw)
        self.assertIn("[81 more characters: --body-chars -1]", rounds.render(report, 30))
        self.assertIn("x" * 80, rounds.render(report, -1))
        self.assertNotIn("Looks right now.", rounds.render(report, 0))

    def test_should_drop_sections_about_the_run_and_keep_findings(self):
        body = (
            "Two findings.\n<details>\n<summary>Review info</summary>\n"
            "<details><summary>Commits</summary>abc</details>\nconfiguration\n</details>\n"
            "<details>\n<summary>Follow-up candidates (1)</summary>\nName the error.\n</details>"
        )
        cleaned = rounds.clean(body)
        self.assertNotIn("configuration", cleaned)
        self.assertNotIn("abc", cleaned)
        self.assertIn("Follow-up candidates (1)", cleaned)
        self.assertIn("Name the error.", cleaned)

    def test_should_read_the_lines_a_patch_adds(self):
        self.assertEqual(rounds.added_lines(PATCH), {11, 12})
        self.assertEqual(rounds.added_lines(None), set())

    def test_should_keep_the_finding_and_drop_the_markup(self):
        body = (
            "<!-- bot v1 -->\n> ✅ **Resolved** at `abc`\n\n**🟡 Suggestion: Name the error**\n\n"
            "The caller cannot tell.\n\n<sub>source: model</sub>\n"
            "<details>\n<summary>🤖 Prompt for AI agents</summary>\n\nlong text\n</details>"
        )
        self.assertEqual(
            rounds.headline(body), "🟡 Suggestion: Name the error | The caller cannot tell."
        )
        self.assertEqual(rounds.severity_hint(rounds.headline(body)), "minor")
        self.assertEqual(rounds.severity_hint("🔴 Blocking: a minor point"), "blocking")
        self.assertIsNone(rounds.severity_hint("Trim the input."))

    def test_should_ask_for_each_step_between_reviewed_heads(self):
        raw = history()
        passes = rounds.review_passes(raw["pr"], raw["reviews"], raw["comments"])
        self.assertEqual(
            rounds.compare_pairs(raw["pr"], passes, raw["commits"], raw["base_merges"]),
            [("base", "h1"), ("h1", "h2"), ("h2", "h3")],
        )

    def test_should_render_the_report_as_text(self):
        text = rounds.render(self.report, 200)
        self.assertIn("Rounds: 2 (heads reviewed: 3)", text)
        self.assertIn("3 commits, 1 named as reverts", text)
        self.assertIn("1 lines written in one response were removed by a later one", text)
        self.assertIn("New threads per round: 2 3", text)
        self.assertIn("### Round 2: head h2", text)
        self.assertIn("[new code, unresolved] bot src/store.rs:12: Rollback loses the key", text)
        self.assertIn("### Pass without findings: head h3", text)
        self.assertIn("Change since this review: none", text)

    def test_should_resolve_a_pull_request_url_without_asking_github(self):
        self.assertEqual(
            rounds.target("https://github.com/dashpay/grovedb/pull/1042#discussion_r1", None),
            ("dashpay/grovedb", 1042),
        )
        self.assertEqual(rounds.target("#6057", "dashpay/platform"), ("dashpay/platform", 6057))

    def test_should_refuse_another_repository_without_a_pull_request(self):
        with self.assertRaises(ValueError):
            rounds.target(None, "dashpay/grovedb")


if __name__ == "__main__":
    unittest.main()
