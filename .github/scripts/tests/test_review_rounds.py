"""Check how the review-rounds skill counts rounds and reads review text.

The skill's diagnosis starts from these numbers: what counts as a round, which
files the findings return to, and which threads went back and forth. A reply
counted as a round, or a finding lost while cleaning a bot's markup, would
point the diagnosis at the wrong cause.
"""
import importlib.util
import json
from pathlib import Path
import time
import unittest


ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location(
    "review_rounds", ROOT / ".claude/skills/review-rounds/scripts/review_rounds.py"
)
rounds = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rounds)


def review(review_id, user, state, head, at, body=""):
    return {
        "id": review_id, "user": {"login": user}, "state": state,
        "commit_id": head, "submitted_at": at, "body": body,
    }


def comment(comment_id, review_id, user, path, line, body, reply_to=None):
    return {
        "id": comment_id, "pull_request_review_id": review_id, "user": {"login": user},
        "path": path, "original_line": line, "body": body,
        "created_at": f"2026-01-01T00:00:{comment_id:02d}Z", "in_reply_to_id": reply_to,
    }


def history():
    """Three reviewed heads: a first pass, a second round, an approval."""
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
            comment(5, 4, "bot", "src/store.rs", 12, "Rollback loses the key"),
            comment(6, 4, "bot", "src/store.rs", 14, "Name the error"),
            comment(7, 4, "bot", "src/view.rs", 30, "Trim the label too"),
        ],
        "commits": [
            {"sha": "h1", "commit": {"message": "feat: store a key"}},
            {"sha": "h2", "commit": {"message": "Revert \"feat: delete first\"\n\nThis reverts commit abc."}},
            {"sha": "h3", "commit": {"message": "fix: keep the old key"}},
        ],
        "resolved": {1: True, 2: True, 5: False, 6: True, 7: True},
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

    def test_should_list_a_summary_without_threads_apart_from_the_rounds(self):
        # A comment-only review can be four findings or a thank-you. It is not
        # counted, but it is not called a pass without findings either.
        raw = history()
        raw["reviews"][3] = review(4, "bot", "COMMENTED", "h2", "2026-01-01T03:00:00Z", "1. Rollback loses the key.")
        raw["comments"] = raw["comments"][:4]
        report = rounds.build_report(raw)
        self.assertEqual((report["rounds"], report["summary_only"]), (1, 1))
        self.assertTrue(report["passes"][1]["summary_only"])
        self.assertFalse(report["passes"][2]["summary_only"])
        text = rounds.render(report, 200)
        self.assertIn("1 more heads got a review summary with no thread and no change request", text)
        self.assertIn("### Summary only, not counted as a round: head h2", text)
        self.assertIn("  1. Rollback loses the key.", text)
        self.assertIn("### Pass without findings: head h3", text)

    def test_should_name_the_commits_to_compare_with_git(self):
        # The script measures no diff. It names, for each review, the commit
        # that reviewer looked at before and the commit reviewed next.
        passes = self.report["passes"]
        self.assertEqual([entry["next"] for entry in passes], ["h2", "h3", None])
        self.assertEqual(
            [review["previous"] for entry in passes for review in entry["reviews"]], [None, "h1", None]
        )
        text = rounds.render(self.report, 0)
        self.assertIn("bot CHANGES_REQUESTED (3 threads; reviewed h1 before)", text)
        self.assertIn("human APPROVED (0 threads; first review)", text)
        self.assertIn("Next reviewed commit or tip: h3", text)
        self.assertIn("This is the tip of the pull request.", text)
        self.assertIn("base v5.1-dev at base, tip h3", text)

    def test_should_say_when_a_reviewer_reviews_the_same_commit_again(self):
        # By the second review of h2 the reviewer had already seen h2, so its
        # findings are not on anything the author changed since.
        raw = history()
        raw["reviews"].append(review(6, "bot", "COMMENTED", "h2", "2026-01-01T03:10:00Z", "One more."))
        report = rounds.build_report(raw)
        self.assertEqual([item["previous"] for item in report["passes"][1]["reviews"]], ["h1", "h2"])
        self.assertIn("bot COMMENTED (0 threads; reviewed this commit before)", rounds.render(report, 0))

    def test_should_mark_a_finding_on_a_line_the_pull_request_removes(self):
        raw = history()
        raw["comments"][5]["side"] = "LEFT"
        raw["comments"][5]["original_start_line"] = 12
        raw["comments"][5]["original_commit_id"] = "h2"
        report = rounds.build_report(raw)
        thread = report["passes"][1]["threads"][1]
        self.assertEqual((thread["side"], thread["start_line"], thread["commit"]), ("LEFT", 12, "h2"))
        self.assertEqual(report["passes"][1]["threads"][0]["side"], "RIGHT")
        self.assertIn("- [on a removed line] bot src/store.rs:14: Name the error", rounds.render(report, 0))

    def test_should_rank_the_files_findings_return_to(self):
        store = self.report["files"]["src/store.rs"]
        self.assertEqual((store["threads"], store["rounds"]), (3, [1, 2]))
        self.assertEqual(self.report["files"]["src/view.rs"]["threads"], 2)
        self.assertEqual(self.report["areas"]["src"]["threads"], 5)
        self.assertEqual(rounds.area("packages/rs-drive/src/lib.rs"), "packages/rs-drive")
        self.assertEqual(rounds.area("AGENTS.md"), "AGENTS.md")

    def test_should_report_size_reverts_and_open_threads(self):
        self.assertEqual(self.report["size"], {"files": 2, "additions": 60, "deletions": 5})
        self.assertEqual(self.report["reverts"], 1)
        self.assertEqual(self.report["unresolved"], 1)

    def test_should_print_the_whole_report_as_json(self):
        report = json.loads(json.dumps(self.report))
        self.assertEqual(report["rounds"], 2)
        self.assertEqual(report["passes"][1]["threads"][0]["headline"], "Rollback loses the key")

    def test_should_mark_a_round_that_requested_changes_without_a_new_thread(self):
        raw = history()
        raw["reviews"].append(
            review(6, "bot", "CHANGES_REQUESTED", "h3", "2026-01-01T04:00:00Z", "Still 1 blocking.")
        )
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

    def test_should_say_which_sections_about_the_run_it_left_out(self):
        body = (
            "Two findings.\n<details>\n<summary>Review info</summary>\n"
            "<details><summary>Commits</summary>abc</details>\nconfiguration\n</details>\n"
            "<details>\n<summary>Follow-up candidates (1)</summary>\nName the error.\n</details>"
        )
        cleaned = rounds.clean(body)
        self.assertNotIn("configuration", cleaned)
        self.assertNotIn("abc", cleaned)
        self.assertEqual(cleaned.count(rounds.OMITTED), 1)
        self.assertIn('[section omitted: "Review info", ', cleaned)
        self.assertIn("--raw prints it]", cleaned)
        self.assertIn("Follow-up candidates (1)", cleaned)
        self.assertIn("Name the error.", cleaned)
        # Nothing is lost for good: --raw keeps the section.
        kept = rounds.clean(body, raw=True)
        self.assertIn("configuration", kept)
        self.assertNotIn(rounds.OMITTED, kept)
        raw = history()
        raw["raw"] = True
        raw["reviews"][0]["body"] = body
        self.assertIn("configuration", rounds.render(rounds.build_report(raw), -1))

    def test_should_say_when_an_unclosed_section_took_the_rest_of_the_comment(self):
        cleaned = rounds.clean("Two findings.\n<details><summary>Review info</summary>\nrun\n3. A third.")
        self.assertNotIn("A third.", cleaned)
        self.assertIn("to the end of the comment because the section is not closed", cleaned)
        self.assertIn("A third.", rounds.clean("<details><summary>Review info</summary>\n3. A third.", raw=True))

    def test_should_keep_text_that_only_begins_like_a_section(self):
        self.assertEqual(rounds.clean("<details and more\nA finding."), "<details and more\nA finding.")
        cleaned = rounds.clean("One.<details><summary>Review info</summary>x</details><details of the run\nTwo.")
        self.assertIn("<details of the run\nTwo.", cleaned)
        self.assertEqual(cleaned.count(rounds.OMITTED), 1)

    def test_should_read_a_section_title_inside_a_quote(self):
        cleaned = rounds.clean("> <details>\n> <summary><b>Run configuration</b></summary>\n> x\n> </details>")
        self.assertIn('[section omitted: "Run configuration"', cleaned)

    def test_should_clean_unclosed_markup_in_linear_time(self):
        # Tens of thousands of unclosed openers took minutes with a lazy regex per opener.
        body = (
            "<!-- " * 10000 + "<picture>" * 10000 + "<sub>" * 10000 + "<a " * 10000
            + "<details" * 10000 + "<details>" * 5000 + "<summary>" * 10000
        )
        started = time.monotonic()
        cleaned = rounds.clean(body)
        self.assertLess(time.monotonic() - started, 2)
        self.assertTrue(cleaned.startswith("<!--"))
        self.assertEqual(rounds.clean("a<!-- x -->b<sub>c</sub>d<picture><img></picture>e"), "abde")

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

    def test_should_render_the_report_as_text(self):
        text = rounds.render(self.report, 200)
        self.assertIn("Rounds: 2 (heads reviewed: 3)", text)
        self.assertIn("3 commits, 1 named as reverts, 2 files, +60 -5", text)
        self.assertIn("New threads per round: 2 3", text)
        self.assertIn("### Round 2: head h2", text)
        self.assertIn("- [unresolved] bot src/store.rs:12: Rollback loses the key", text)
        self.assertIn("- bot src/store.rs:14: Name the error", text)
        self.assertIn("### Pass without findings: head h3", text)

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
