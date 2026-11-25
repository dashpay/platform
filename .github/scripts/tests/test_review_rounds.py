"""Check how the review-rounds skill counts rounds and places findings.

The skill's diagnosis rests on these numbers: what counts as a round, whether a
finding sits on code the author changed since that reviewer's previous review,
and how large each response was. A reply counted as a round, a base merge
counted as the author's work, or a finding placed from a diff that is not the
difference between the two reviewed trees would point the diagnosis at the
wrong cause.
"""
import importlib.util
from pathlib import Path
import time
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
VIEW_PATCH = "@@ -29,2 +29,3 @@ fn view()\n context\n+    let label = input.trim();\n context"


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
            commit("h2", "Revert \"feat: delete first\"\n\nThis reverts commit abc.", "h1"),
            commit("h3", "fix: keep the old key", "h2"),
        ],
        "resolved": {1: True, 2: True, 5: False, 6: True, 7: True},
        "compares": {
            ("base", "h1"): compared({
                "src/store.rs": changed(40, 0, status="added"),
                "src/view.rs": changed(10, 5),
            }, status="diverged"),
            ("h1", "h2"): compared({"src/store.rs": changed(2, 1, PATCH)}),
            ("h2", "h3"): compared({"src/store.rs": changed(8, 4, UNDO)}),
        },
    }


def with_base_merge(raw, brought=("src/view.rs",)):
    """Put a merge from the base branch, changing `brought`, between h1 and h2."""
    raw["commits"][1:2] = [
        commit("merge", "Merge branch 'v5.1-dev'", "h1", "tip"),
        commit("h2", "fix: keep the old key", "merge"),
    ]
    raw["pr"]["commits"] = 4
    del raw["compares"][("h1", "h2")]
    raw["compares"][("h1", "merge")] = compared({path: changed(7, 3) for path in brought})
    return raw


def placed(report):
    return {thread["id"]: thread["place"] for entry in report["passes"] for thread in entry["threads"]}


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

    def test_should_measure_a_finding_against_its_own_reviewers_last_review(self):
        raw = history()
        raw["reviews"].append(review(6, "human", "COMMENTED", "h2", "2026-01-01T03:30:00Z"))
        raw["reviews"].append(review(7, "human", "COMMENTED", "h3", "2026-01-01T05:30:00Z"))
        raw["comments"].append(comment(8, 6, "human", "src/view.rs", 30, "Why trim here?"))
        raw["comments"].append(comment(9, 7, "human", "src/view.rs", 31, "Still unclear"))
        report = rounds.build_report(raw)
        # The first time a reviewer looks, nothing is "old code" to them.
        self.assertEqual(placed(report)[8], rounds.FIRST)
        self.assertEqual(placed(report)[9], rounds.OLD_CODE)
        self.assertEqual(report["later_findings"][rounds.OLD_CODE], 2)

    def test_should_size_a_response_by_what_the_author_changed(self):
        response = self.report["passes"][0]["response"]
        self.assertEqual([item["path"] for item in response["files"]], ["src/store.rs"])
        self.assertEqual((response["additions"], response["deletions"]), (2, 1))
        self.assertEqual(
            self.report["changed_since_first_review"],
            {"additions": 10, "deletions": 5, "undone": 1, "unavailable": 0},
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

    def test_should_cut_a_merge_from_the_base_branch_out_of_a_response(self):
        # The merge changed view.rs; the author changed store.rs after it.
        raw = with_base_merge(history())
        raw["compares"][("merge", "h2")] = compared({"src/store.rs": changed(2, 1, PATCH)})
        self.assertEqual(rounds.base_merges(raw["pr"], raw["commits"]), {"merge": True})
        report = rounds.build_report(raw)
        response = report["passes"][0]["response"]
        self.assertEqual([item["path"] for item in response["files"]], ["src/store.rs"])
        self.assertTrue(response["merged"])
        # The finding on view.rs is on code that changed since the last review,
        # but not by the author: neither "old code" nor the author's fix.
        self.assertEqual(
            [placed(report)[key] for key in (5, 6, 7)],
            [rounds.NEW_LINES, rounds.CHANGED_FILE, rounds.BASE_MERGE],
        )
        text = rounds.render(report, 0)
        self.assertIn("a merge from the base branch is left out", text)
        self.assertIn("1 new code, 1 changed file, 1 merged from base", text)

    def test_should_keep_the_authors_edit_to_a_file_the_base_merge_also_changed(self):
        # Base turns A into B and the author turns B into C. Subtracting line
        # counts would cancel the two; measuring after the merge keeps C.
        raw = with_base_merge(history())
        raw["compares"][("merge", "h2")] = compared({"src/view.rs": changed(1, 0, VIEW_PATCH)})
        report = rounds.build_report(raw)
        response = report["passes"][0]["response"]
        self.assertEqual(
            [(item["path"], item["additions"]) for item in response["files"]], [("src/view.rs", 1)]
        )
        self.assertEqual(placed(report)[7], rounds.NEW_LINES)
        self.assertEqual(placed(report)[5], rounds.OLD_CODE)

    def test_should_not_guess_at_a_file_when_the_merge_cannot_be_read(self):
        raw = with_base_merge(history())
        raw["compares"][("merge", "h2")] = compared({"src/store.rs": changed(2, 1, PATCH)})
        del raw["compares"][("h1", "merge")]
        report = rounds.build_report(raw)
        self.assertEqual(placed(report)[7], rounds.UNKNOWN)
        self.assertEqual(placed(report)[5], rounds.NEW_LINES)

    def test_should_trust_line_numbers_only_while_nothing_changed_the_file_after(self):
        # The author's edit comes before the merge. Its line numbers hold at
        # the reviewed head only if the merge left that file alone.
        def report_with(brought):
            raw = history()
            raw["commits"][1:2] = [
                commit("fix", "fix: keep the old key", "h1"),
                commit("h2", "Merge branch 'v5.1-dev'", "fix", "tip"),
            ]
            raw["pr"]["commits"] = 4
            del raw["compares"][("h1", "h2")]
            raw["compares"][("h1", "fix")] = compared({
                "src/store.rs": changed(2, 1, PATCH),
                "src/new.rs": changed(30, 0, status="added"),
            })
            raw["compares"][("fix", "h2")] = compared({path: changed(7, 3) for path in brought})
            raw["comments"].append(comment(8, 4, "bot", "src/new.rs", 5, "Check the length"))
            return rounds.build_report(raw)

        untouched = report_with(["src/view.rs"])
        self.assertEqual(placed(untouched)[5], rounds.NEW_LINES)
        self.assertEqual(placed(untouched)[8], rounds.NEW_LINES)
        self.assertEqual(untouched["passes"][0]["response"]["additions"], 32)
        shifted = report_with(["src/store.rs", "src/new.rs"])
        self.assertEqual(placed(shifted)[5], rounds.CHANGED_FILE)
        self.assertEqual(placed(shifted)[8], rounds.CHANGED_FILE)

    def test_should_keep_a_merge_of_another_branch_as_the_authors_change(self):
        # The merged-in commit is one of the pull request's own commits, so it
        # was not on the base branch. That stays true after the pull request is
        # merged, when every commit of it is reachable from the base branch.
        raw = history()
        raw["commits"][1:2] = [
            commit("tip", "feat: stacked work"),
            commit("merge", "Merge branch 'feat/stacked'", "h1", "tip"),
            commit("h2", "fix: keep the old key", "merge"),
        ]
        raw["pr"]["commits"] = 5
        raw["compares"][("h1", "h2")]["files"]["src/view.rs"] = changed(7, 3)
        self.assertEqual(rounds.base_merges(raw["pr"], raw["commits"]), {"merge": False})
        report = rounds.build_report(raw)
        response = report["passes"][0]["response"]
        self.assertEqual(
            sorted(item["path"] for item in response["files"]), ["src/store.rs", "src/view.rs"]
        )
        self.assertFalse(response["merged"])
        self.assertEqual(placed(report)[7], rounds.CHANGED_FILE)

    def test_should_say_so_when_a_merge_cannot_be_traced(self):
        # GitHub lists at most 250 commits; with a longer history a merged-in
        # commit that is not listed could be on either side.
        raw = history()
        raw["commits"][1:2] = [
            commit("merge", "Merge branch 'v5.1-dev'", "h1", "tip"),
            commit("h2", "fix: keep the old key", "merge"),
        ]
        raw["pr"]["commits"] = 400
        self.assertEqual(rounds.base_merges(raw["pr"], raw["commits"]), {"merge": None})
        report = rounds.build_report(raw)
        self.assertTrue(report["passes"][0]["response"]["unsure"])
        self.assertIn("may include a merge from the base branch", rounds.render(report, 0))

    def test_should_refuse_to_measure_across_a_rebase(self):
        # After a rebase GitHub compares from the merge base, not from the old
        # head: edits the rebase kept would look new, and a file put back to
        # its base version would drop out and look untouched.
        raw = history()
        raw["compares"][("h1", "h2")]["status"] = "diverged"
        report = rounds.build_report(raw)
        self.assertEqual(
            {thread["place"] for thread in report["passes"][1]["threads"]}, {rounds.UNKNOWN}
        )
        self.assertIsNone(report["passes"][0]["response"])
        self.assertEqual(
            report["changed_since_first_review"],
            {"additions": 8, "deletions": 4, "undone": 0, "unavailable": 1},
        )
        self.assertEqual(report["files"]["src/store.rs"]["responses"], 1)
        text = rounds.render(report, 0)
        self.assertIn(
            "unavailable (the history was rewritten in between, or a commit is no longer listed)", text
        )
        self.assertIn("1 of the changes between reviewed heads could not be measured", text)

    def test_should_not_measure_from_a_head_that_is_no_longer_listed(self):
        # A force-pushed head, or one past the 250 commits GitHub lists, has no
        # known place in the history, so a step from it could overlap another.
        raw = history()
        del raw["commits"][0]
        report = rounds.build_report(raw)
        self.assertEqual([entry["head"] for entry in report["passes"]], ["h1", "h2", "h3"])
        self.assertIsNone(report["passes"][0]["response"])
        self.assertEqual(placed(report)[5], rounds.UNKNOWN)
        self.assertEqual(report["passes"][1]["response"]["additions"], 8)
        self.assertEqual(report["changed_since_first_review"]["unavailable"], 1)

    def test_should_follow_the_commits_when_reviews_arrive_out_of_order(self):
        # Reviews land for h1, then h3, then h2. The responses are still
        # h1 to h2 and h2 to h3, each counted once.
        raw = history()
        raw["reviews"][3] = review(4, "bot", "CHANGES_REQUESTED", "h2", "2026-01-01T06:00:00Z")
        report = rounds.build_report(raw)
        self.assertEqual([entry["head"] for entry in report["passes"]], ["h1", "h2", "h3"])
        self.assertEqual([entry["round"] for entry in report["passes"]], [1, 2, None])
        self.assertEqual(
            report["changed_since_first_review"],
            {"additions": 10, "deletions": 5, "undone": 1, "unavailable": 0},
        )
        self.assertIn("from 2026-01-01 01:00 to 2026-01-01 06:00", rounds.render(report, 0))

    def test_should_judge_a_finding_by_what_its_reviewer_had_seen_at_the_time(self):
        # One reviewer reviews h1, then h3, then goes back to h2. When writing
        # the h3 finding they had seen only h1, so a line h2 added is new to
        # them. The h2 finding comes after h3, which cannot be compared forward.
        raw = history()
        raw["reviews"][3] = review(4, "bot", "CHANGES_REQUESTED", "h2", "2026-01-01T06:00:00Z")
        raw["reviews"].append(review(6, "bot", "COMMENTED", "h3", "2026-01-01T04:00:00Z"))
        raw["comments"].append(comment(8, 6, "bot", "src/store.rs", 11, "Still loses the key"))
        raw["compares"][("h1", "h3")] = compared({"src/store.rs": changed(9, 4, PATCH)})
        report = rounds.build_report(raw)
        self.assertEqual(placed(report)[8], rounds.NEW_LINES)
        self.assertEqual(
            {placed(report)[key] for key in (5, 6, 7)}, {rounds.UNKNOWN}
        )

    def test_should_keep_a_reviewers_baseline_across_two_reviews_of_one_head(self):
        raw = history()
        raw["reviews"].append(review(6, "bot", "COMMENTED", "h2", "2026-01-01T03:10:00Z"))
        raw["comments"].append(comment(8, 6, "bot", "src/view.rs", 30, "And here"))
        self.assertEqual(placed(rounds.build_report(raw))[8], rounds.OLD_CODE)

    def test_should_size_the_first_review_at_the_head_reviewed_first_in_time(self):
        raw = history()
        raw["reviews"][0]["submitted_at"] = "2026-01-01T09:00:00Z"
        self.assertIsNone(rounds.build_report(raw)["size_at_first_review"])
        raw["compares"][("base", "h2")] = compared({"src/store.rs": changed(42, 1)}, status="diverged")
        self.assertEqual(
            rounds.build_report(raw)["size_at_first_review"],
            {"files": 1, "additions": 42, "deletions": 1, "added_since": ["src/view.rs"]},
        )

    def test_should_follow_a_line_only_while_every_change_to_its_file_is_known(self):
        def change(patch, merged_files=()):
            return {
                "files": [{"path": "src/store.rs", "patches": [patch]}],
                "merged_files": set(merged_files),
            }

        self.assertEqual(rounds.undone_lines([change(PATCH), change(UNDO)])["src/store.rs"], 1)
        # A change that could not be measured may have removed the line and
        # written the same text elsewhere.
        self.assertEqual(rounds.undone_lines([change(PATCH), None, change(UNDO)])["src/store.rs"], 0)
        # So may a merge from the base branch that touched the file.
        merged = change(UNDO, ["src/store.rs"])
        self.assertEqual(rounds.undone_lines([change(PATCH), merged])["src/store.rs"], 0)
        after = change("@@ -1 +1 @@\n-x\n+y", ["src/store.rs"])
        self.assertEqual(rounds.undone_lines([change(PATCH), after, change(UNDO)])["src/store.rs"], 0)
        unreadable = {"files": [], "merged_files": None}
        self.assertEqual(rounds.undone_lines([change(PATCH), unreadable, change(UNDO)])["src/store.rs"], 0)

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
