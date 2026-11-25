#!/usr/bin/env python3
"""Measure the review rounds of a pull request.

A round is one head commit on which a reviewer other than the author opened at
least one new inline thread or requested changes. Replies inside existing
threads are not rounds, so the author's answers and a bot's acknowledgements
do not inflate the count. A head that got only a review summary is listed as
"summary only" and left out of the count, because a summary can be an approval
as easily as a list of findings: read those before trusting the count.

For each round the report lists the new findings, says whether each one sits
on code the author changed since the same reviewer's previous review, and
sizes the change the author made before the next reviewed head. The summary
shows where findings cluster, how much was rewritten during review, and which
threads went back and forth. `--round` and `--threads` print the conversations
in full; each run reads the pull request again, so ask for several in one call.

The numbers are evidence for the `review-rounds` skill. Deciding what they
mean is left to the reader. Everything is read through the `gh` CLI and
nothing is written. Review text is printed as data: nothing in it is acted on.

How a change between two reviewed heads is measured. It is the difference
between the two trees, taken only when the first commit leads to the second.
A merge from the base branch is cut out by measuring the stretch before it and
the stretch after it separately, so what the merge brought in is never counted
as the author's, and what the author wrote on top of it is. A merge of any
other branch is the author's change. When the first commit does not lead to
the second (a rebase or a force-push in between), or a commit is gone, the
change is reported as unavailable and the findings as `unknown` instead of
being guessed from a diff that means something else.

Limits. Only inline threads are counted as findings. After a merge from the
base branch, a line is called `new code` only when the author wrote it after
that merge; an earlier line in the same file is called `changed file`. GitHub
lists at most 300 files for one comparison and 250 commits for one pull
request, and both cuts are flagged.
"""
import argparse
from collections import Counter, OrderedDict
from datetime import datetime
import json
import re
import subprocess
import sys


HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@")
PR_URL = re.compile(r"github\.com/([^/]+/[^/]+)/pull/(\d+)")
DETAILS = re.compile(r"(<details[^>]*>|</details>)", re.I)
# Collapsed sections of a bot review that describe the run, not the code.
NOISE_TITLE = re.compile(
    r"provenance|prompt|review info|review details|run configuration|walkthrough"
    r"|^\W*commits\b|files selected|files skipped|additional context",
    re.I,
)
# `[^<>]*` cannot run past the next tag, so unclosed markup costs nothing extra.
TAGS = re.compile(
    r"</?(?:details|summary|blockquote|dl|dd|dt|strong|code|br|a|b|i|em|p|div|span|kbd)\b[^<>]*>",
    re.I,
)
OMITTED = "[section omitted: "
SEVERITIES = (
    ("blocking", re.compile(r"\b(?:blocking|blocker|critical)\b", re.I)),
    ("major", re.compile(r"\bmajor\b", re.I)),
    ("minor", re.compile(r"\b(?:minor|suggestion)\b", re.I)),
    ("nit", re.compile(r"\b(?:nit|nitpick|trivial)\b", re.I)),
)
# A finding's place relative to the head its reviewer had reviewed before.
FIRST, NEW_LINES, CHANGED_FILE, OLD_CODE, UNKNOWN = (
    "first pass", "new code", "changed file", "old code", "unknown",
)
PLACES = (NEW_LINES, CHANGED_FILE, OLD_CODE, UNKNOWN)
# A thread with this many replies is listed as a long thread.
LONG_THREAD = 4
# Shorter lines (braces, `else`, blank) say nothing about rework.
REWORK_LINE = 12
THREADS_QUERY = """
query($owner: String!, $name: String!, $number: Int!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes { isResolved comments(first: 1) { nodes { databaseId } } }
      }
    }
  }
}
"""


class GhError(RuntimeError):
    """A `gh` call failed."""


def gh(*args):
    result = subprocess.run(["gh", *args], capture_output=True, text=True)
    if result.returncode != 0:
        raise GhError(result.stderr.strip() or "gh " + " ".join(args) + " failed")
    return result.stdout


def gh_list(path):
    """Every element of a paginated list endpoint."""
    out = gh("api", "--paginate", path + "?per_page=100", "--jq", ".[]")
    return [json.loads(row) for row in out.splitlines() if row.strip()]


def login(item):
    return (item.get("user") or {}).get("login") or "ghost"


def drop_spans(text, start, end):
    """Remove every `start`...`end` span in one pass; an unclosed one stays."""
    kept, position = [], 0
    while True:
        opened = text.find(start, position)
        closed = text.find(end, opened + len(start)) if opened >= 0 else -1
        if closed < 0:
            break
        kept.append(text[position:opened])
        position = closed + len(end)
    kept.append(text[position:])
    return "".join(kept)


def without_noise(text):
    """Replace the collapsed sections that describe the run with a one-line note."""
    kept, dropping, size, name = [], [], 0, ""
    parts = DETAILS.split(text)
    for index, part in enumerate(parts):
        if part.lower().startswith("<details"):
            following = parts[index + 1] if index + 1 < len(parts) else ""
            title = re.search(r"<summary>(.*?)</summary>", following, re.S | re.I)
            title = re.sub(r"<[^<>]*>", "", title.group(1)).strip() if title else ""
            inside = bool(dropping and dropping[-1])
            if not inside and NOISE_TITLE.search(title):
                size, name = 0, " ".join(title.split())[:60]
            dropping.append(inside or bool(NOISE_TITLE.search(title)))
        elif part.lower() == "</details>":
            if dropping and dropping.pop() and not (dropping and dropping[-1]):
                kept.append(f'\n{OMITTED}"{name}", {size} characters; --raw prints it]\n')
        elif dropping and dropping[-1]:
            size += len(part)
        else:
            kept.append(part)
    return "".join(kept)


def clean(text, raw=False):
    """Keep what a comment says and drop its markup, icons and attributions.

    With `raw`, the collapsed sections about the run are kept as well.
    """
    text = text or ""
    if not raw:
        text = without_noise(text)
    for start, end in (("<!--", "-->"), ("<picture>", "</picture>"), ("<sub>", "</sub>")):
        text = drop_spans(text, start, end)
    text = TAGS.sub("", text)
    rows = [row.rstrip() for row in text.splitlines()]
    return re.sub(r"\n{3,}", "\n\n", "\n".join(row for row in rows if row.strip() != ">")).strip()


def headline(body, limit=200):
    """The first two lines of a comment that say something."""
    rows = (row.replace("**", "").strip(" \t·") for row in clean(body).splitlines())
    rows = [row for row in rows if row and not row.startswith((">", OMITTED))]
    text = " | ".join(rows[:2])
    return text if len(text) <= limit else text[: limit - 3].rstrip() + "..."


def severity_hint(text):
    """The first severity word the reviewer used, if any. A hint, not a fact."""
    found = [(match.start(), name) for name, pattern in SEVERITIES
             for match in [pattern.search(text)] if match]
    return min(found)[1] if found else None


def patch_lines(patch):
    """The lines a patch adds, with their new line numbers, and those it removes."""
    added, removed, current = [], [], None
    for row in (patch or "").splitlines():
        hunk = HUNK.match(row)
        if hunk:
            current = int(hunk.group(1))
        elif current is None or row.startswith("\\"):
            continue
        elif row.startswith("-"):
            removed.append(row[1:])
        else:
            if row.startswith("+"):
                added.append((current, row[1:]))
            current += 1
    return added, removed


def added_lines(patch):
    """Line numbers, in the new file, of the lines a patch adds."""
    return {number for number, _ in patch_lines(patch)[0]}


def area(path):
    """The package, or the top-level directory, a path belongs to."""
    parts = path.split("/")
    if parts[0] == "packages" and len(parts) > 2:
        return "/".join(parts[:2])
    return parts[0] if len(parts) > 1 else path


def review_passes(pr, reviews, comments, commits=(), raw=False):
    """Group the reviews other people left by the head commit they reviewed.

    The passes come back in the order of the commits they reviewed, whatever
    the order the reviews were submitted in; a head that is no longer among the
    pull request's commits comes first, in submission order. A pass with
    `findings` is a round. A review with no new thread, no body and no verdict
    is a reply.
    """
    author = login(pr)
    starters, replies = {}, {}
    for comment in sorted(comments, key=lambda item: item.get("created_at") or ""):
        if comment.get("in_reply_to_id"):
            replies.setdefault(comment["in_reply_to_id"], []).append(comment)
        elif login(comment) != author:
            starters.setdefault(comment.get("pull_request_review_id"), []).append(comment)

    passes = OrderedDict()
    for review in sorted(reviews, key=lambda item: item.get("submitted_at") or ""):
        state, reviewer = review.get("state"), login(review)
        if reviewer == author or state == "PENDING":
            continue
        threads = starters.get(review["id"], [])
        body = clean(review.get("body"), raw)
        if not threads and not body and state == "COMMENTED":
            continue
        head = review.get("commit_id") or "unknown"
        entry = passes.setdefault(head, {
            "head": head, "started": review.get("submitted_at"),
            "reviews": [], "threads": [], "findings": False, "summary_only": False,
        })
        entry["ended"] = review.get("submitted_at")
        requested = state == "CHANGES_REQUESTED"
        entry["findings"] = entry["findings"] or bool(threads) or requested
        # A summary with no thread and no verdict: findings or thanks, unknown.
        entry["summary_only"] = entry["summary_only"] or (state == "COMMENTED" and not threads)
        entry["reviews"].append({
            "reviewer": reviewer, "state": state, "threads": len(threads), "body": body,
        })
        for comment in threads:
            text = headline(comment.get("body"))
            answers = replies.get(comment["id"], [])
            entry["threads"].append({
                "id": comment["id"],
                "reviewer": reviewer,
                "path": comment["path"],
                "line": comment.get("original_line") or comment.get("line"),
                "start_line": comment.get("original_start_line"),
                "side": comment.get("side") or "RIGHT",
                "headline": text,
                "severity": severity_hint(text),
                "body": clean(comment.get("body"), raw),
                "replies": len(answers),
                "author_replies": sum(1 for answer in answers if login(answer) == author),
                "conversation": [
                    {"who": login(answer), "at": answer.get("created_at"),
                     "body": clean(answer.get("body"), raw)}
                    for answer in answers
                ],
            })

    position = {item["sha"]: index for index, item in enumerate(commits)}
    ordered = sorted(passes.values(), key=lambda entry: position.get(entry["head"], -1))
    seen = {}
    for entry in ordered:
        entry["summary_only"] = entry["summary_only"] and not entry["findings"]
        for thread in entry["threads"]:
            # The head this reviewer had reviewed before this one, if any.
            thread["since"] = seen.get(thread["reviewer"])
        for review in entry["reviews"]:
            seen[review["reviewer"]] = entry["head"]
    return ordered


def is_merge(commit):
    return len(commit.get("parents") or []) > 1


def base_merges(pr, commits):
    """For each merge commit: whether what it merged in came from the base branch.

    A pull request's commit list holds the commits that are not on its base,
    and GitHub keeps that list as it was once the pull request is merged. So a
    merged-in parent that is in the list is another branch's work, and one that
    is not was already on the base branch. None when the list is incomplete.
    """
    own = {item["sha"] for item in commits}
    complete = len(commits) >= pr["commits"]
    found = {}
    for item in filter(is_merge, commits):
        if any(parent["sha"] in own for parent in item["parents"][1:]):
            found[item["sha"]] = False
        else:
            found[item["sha"]] = True if complete else None
    return found


def stretches(pr, commits, old, new):
    """The parts of the history from `old` to `new` that are the author's work.

    Returns the (from, to) pairs to compare, whether a merge from the base
    branch was cut out, and whether a merge of unknown origin was left in.
    Returns None when `new` does not come after `old`.
    """
    order = [item["sha"] for item in commits]
    if old not in order or new not in order:
        # A commit GitHub no longer lists. The comparison itself will say
        # whether one leads to the other; merges in between cannot be seen.
        return [(old, new)], False, True
    if order.index(old) >= order.index(new):
        return None
    origins = base_merges(pr, commits)
    pairs, cut_out, unsure, start = [], False, False, old
    for item in commits[order.index(old) + 1: order.index(new) + 1]:
        from_base = origins.get(item["sha"], False)
        unsure = unsure or from_base is None
        if from_base:
            cut_out = True
            # The first parent is the pull request's side of the merge.
            before = item["parents"][0]["sha"]
            if before != start:
                pairs.append((start, before))
            start = item["sha"]
    if start != new:
        pairs.append((start, new))
    return pairs, cut_out, unsure


def steps(pr, passes):
    """Each reviewed head with the head that follows it; the last with the tip."""
    heads = [entry["head"] for entry in passes]
    return list(zip(heads, heads[1:] + [pr["head"]["sha"]]))


def compare_pairs(pr, passes, commits):
    """The (from, to) commit pairs whose difference the report needs."""
    known = [entry["head"] for entry in passes if entry["head"] != "unknown"]
    if not known:
        return []
    # From the merge base to the first reviewed head: the size at first review.
    pairs = [(pr["base"]["sha"], known[0])]
    wanted = steps(pr, passes) + [
        (thread["since"], entry["head"]) for entry in passes for thread in entry["threads"]
    ]
    for old, new in wanted:
        if old and old != new and "unknown" not in (old, new):
            pairs += (stretches(pr, commits, old, new) or ([],))[0]
    return list(OrderedDict.fromkeys(pairs))


def author_change(raw, old, new):
    """What the author changed from `old` to `new`, or None when that is unknowable."""
    plan = stretches(raw["pr"], raw["commits"], old, new)
    if plan is None:
        return None
    pairs, cut_out, unsure = plan
    files, cut = {}, False
    for index, pair in enumerate(pairs):
        change = raw["compares"].get(pair)
        # Only then is the comparison the difference between the two trees.
        if change is None or change["status"] not in ("ahead", "identical"):
            return None
        cut = cut or change["truncated"]
        # Line numbers hold at `new` only when nothing follows this stretch.
        last = index == len(pairs) - 1 and pair[1] == new
        for path, item in change["files"].items():
            row = files.setdefault(path, {
                "status": item["status"], "additions": 0, "deletions": 0, "patches": [],
            })
            row["additions"] += item["additions"]
            row["deletions"] += item["deletions"]
            if item["status"] == "added":
                row["status"] = "added"
            row["patches"].append(item.get("patch"))
            row["lines"] = added_lines(item["patch"]) if last and item.get("patch") else None
    return {"files": files, "merged": cut_out, "unsure": unsure, "cut": cut}


def place(thread, change):
    """Where a finding sits relative to what the author changed since the last review."""
    if change is None:
        return UNKNOWN
    changed = change["files"].get(thread["path"])
    if changed is None:
        return UNKNOWN if change["cut"] else OLD_CODE
    if changed["status"] == "added":
        return NEW_LINES
    if thread["side"] != "RIGHT" or not thread["line"] or changed["lines"] is None:
        return CHANGED_FILE
    first = thread["start_line"] or thread["line"]
    commented = set(range(min(first, thread["line"]), thread["line"] + 1))
    return NEW_LINES if commented & changed["lines"] else CHANGED_FILE


def response(change):
    """The size of what the author changed between two reviewed heads."""
    if change is None:
        return None
    files = [
        {"path": path, "additions": item["additions"], "deletions": item["deletions"],
         "patches": item["patches"]}
        for path, item in change["files"].items()
    ]
    files.sort(key=lambda item: -(item["additions"] + item["deletions"]))
    return {
        "files": files,
        "additions": sum(item["additions"] for item in files),
        "deletions": sum(item["deletions"] for item in files),
        "merged": change["merged"],
        "unsure": change["unsure"],
        "cut": change["cut"],
    }


def undone_lines(responses):
    """Per file, lines one response added that a later response removed."""
    written, undone = {}, Counter()
    for change in responses:
        for item in change["files"]:
            added, removed = [], []
            for patch in item["patches"]:
                more_added, more_removed = patch_lines(patch)
                added += more_added
                removed += more_removed
            earlier = written.setdefault(item["path"], Counter())
            for text in (row.strip() for row in removed):
                if earlier[text] > 0:
                    earlier[text] -= 1
                    undone[item["path"]] += 1
            # Counted after the removals, so a line moved inside one response
            # is not rework.
            earlier.update(
                text for text in (row.strip() for _, row in added) if len(text) >= REWORK_LINE
            )
    return undone


def build_report(raw):
    """Turn the fetched pull request data into the rounds report."""
    pr = raw["pr"]
    passes = raw.get("passes") or review_passes(
        pr, raw["reviews"], raw["comments"], raw["commits"], raw.get("raw", False)
    )
    resolved = raw.get("resolved")

    number = 0
    for entry, (head, following) in zip(passes, steps(pr, passes)):
        if entry["findings"]:
            number += 1
            entry["round"] = number
        else:
            entry["round"] = None
        for thread in entry["threads"]:
            thread["round"] = entry["round"]
            thread["place"] = (
                place(thread, author_change(raw, thread["since"], head))
                if thread["since"] else FIRST
            )
            thread["resolved"] = resolved.get(thread["id"]) if resolved else None
        entry["unchanged"] = following == head
        entry["response"] = (
            None if entry["unchanged"] or head == "unknown"
            else response(author_change(raw, head, following))
        )

    responses = [entry["response"] for entry in passes if entry["response"]]
    undone = undone_lines(responses)
    files, areas = {}, {}
    for entry in passes:
        for thread in entry["threads"]:
            for table, key in ((files, thread["path"]), (areas, area(thread["path"]))):
                row = table.setdefault(key, {"threads": 0, "rounds": set(), "responses": 0, "undone": 0})
                row["threads"] += 1
                row["rounds"].add(entry["round"])
    for item in (item for change in responses for item in change["files"]):
        if item["path"] in files:
            files[item["path"]]["responses"] += 1
    for path, count in undone.items():
        if path in files:
            files[path]["undone"] = count
    for table in (files, areas):
        for row in table.values():
            row["rounds"] = sorted(row["rounds"])
    for change in responses:
        for item in change["files"]:
            del item["patches"]

    first = raw["compares"].get((pr["base"]["sha"], passes[0]["head"])) if passes else None
    first_size = None
    if first and not first["truncated"]:
        first_size = {
            "files": len(first["files"]),
            "additions": sum(item["additions"] for item in first["files"].values()),
            "deletions": sum(item["deletions"] for item in first["files"].values()),
            "added_since": sorted(
                item["filename"] for item in raw["files"]
                if item["filename"] not in first["files"]
                and item.get("previous_filename") not in first["files"]
            ),
        }
    reviewers = {}
    for entry in passes:
        for review in entry["reviews"]:
            row = reviewers.setdefault(review["reviewer"], {"reviews": 0, "threads": 0})
            row["reviews"] += 1
            row["threads"] += review["threads"]
    threads = [thread for entry in passes for thread in entry["threads"]]
    return {
        "repo": raw["repo"],
        "number": pr["number"],
        "title": pr["title"],
        "author": login(pr),
        "state": "merged" if pr.get("merged_at") else pr["state"],
        "base": pr["base"]["ref"],
        "created": pr["created_at"],
        "size": {"files": pr["changed_files"], "additions": pr["additions"], "deletions": pr["deletions"]},
        "size_at_first_review": first_size,
        "commits": pr["commits"],
        "reverts": sum(
            1 for item in raw["commits"]
            if re.match(r"revert\b", item["commit"]["message"], re.I)
            or "This reverts commit" in item["commit"]["message"]
        ),
        "rounds": number,
        "summary_only": sum(1 for entry in passes if entry["summary_only"]),
        "passes": passes,
        "reviewers": reviewers,
        "threads": len(threads),
        "unresolved": sum(1 for thread in threads if thread["resolved"] is False) if resolved else None,
        "later_findings": dict(Counter(thread["place"] for thread in threads if thread["place"] != FIRST)),
        "changed_since_first_review": {
            "additions": sum(change["additions"] for change in responses),
            "deletions": sum(change["deletions"] for change in responses),
            "undone": sum(undone.values()),
            "unavailable": sum(
                1 for entry in passes if not entry["unchanged"] and not entry["response"]
            ),
        } if passes else None,
        "files": files,
        "areas": areas,
    }


def fetch_compare(repo, old, new):
    try:
        data = json.loads(gh("api", f"repos/{repo}/compare/{old}...{new}"))
    except GhError:
        return None
    files = data.get("files") or []
    return {
        "status": data.get("status"),
        # GitHub lists at most 300 files for one comparison.
        "truncated": len(files) >= 300,
        "files": {
            item["filename"]: {
                "status": item["status"], "additions": item["additions"],
                "deletions": item["deletions"], "patch": item.get("patch"),
            } for item in files
        },
    }


def fetch_resolved(repo, number):
    """Whether each thread is resolved, by its first comment's id."""
    owner, name = repo.split("/")
    resolved, after = {}, None
    try:
        while True:
            args = ["-f", f"owner={owner}", "-f", f"name={name}", "-F", f"number={number}"]
            if after:
                args += ["-f", f"after={after}"]
            page = json.loads(gh("api", "graphql", "-f", "query=" + THREADS_QUERY, *args))
            threads = page["data"]["repository"]["pullRequest"]["reviewThreads"]
            for node in threads["nodes"]:
                comments = node["comments"]["nodes"]
                if comments:
                    resolved[comments[0]["databaseId"]] = node["isResolved"]
            if not threads["pageInfo"]["hasNextPage"]:
                return resolved
            after = threads["pageInfo"]["endCursor"]
    except (GhError, KeyError, TypeError):
        return None


def fetch(repo, number, raw_text=False):
    base = f"repos/{repo}/pulls/{number}"
    raw = {
        "repo": repo,
        "raw": raw_text,
        "pr": json.loads(gh("api", base)),
        "reviews": gh_list(base + "/reviews"),
        "comments": gh_list(base + "/comments"),
        "files": gh_list(base + "/files"),
        "commits": gh_list(base + "/commits"),
        "resolved": fetch_resolved(repo, number),
    }
    raw["passes"] = review_passes(raw["pr"], raw["reviews"], raw["comments"], raw["commits"], raw_text)
    raw["compares"] = {
        pair: fetch_compare(repo, *pair)
        for pair in compare_pairs(raw["pr"], raw["passes"], raw["commits"])
    }
    return raw


def when(stamp):
    return stamp.replace("T", " ")[:16] if stamp else "?"


def span(start, end):
    if not start or not end:
        return "?"
    seconds = (datetime.fromisoformat(end.rstrip("Z")) - datetime.fromisoformat(start.rstrip("Z"))).total_seconds()
    days, rest = divmod(int(seconds), 86400)
    hours, minutes = rest // 3600, rest % 3600 // 60
    return f"{days}d {hours}h" if days else f"{hours}h {minutes:02d}m"


def size(item):
    files = item["files"]
    count = files if isinstance(files, int) else len(files)
    return f"{count} files, +{item['additions']} -{item['deletions']}"


def tags(thread):
    found = [thread["place"]] + [thread["severity"]] * bool(thread["severity"])
    if thread["resolved"] is False:
        found.append("unresolved")
    if thread["replies"] >= LONG_THREAD - 1:
        found.append(f"{thread['replies']} replies")
    return ", ".join(found)


def shorten(text, limit, hint):
    """Cut a long text at a line end and say how to read the rest."""
    if limit <= 0 or len(text) <= limit:
        return text
    cut = text.rfind("\n", 0, limit)
    kept = text[: cut if cut > limit // 2 else limit].rstrip()
    return f"{kept}\n[{len(text) - len(kept)} more characters: {hint}]"


def indent(text, prefix="  "):
    return [prefix + row for row in text.splitlines() if row.strip()]


def render_thread(thread):
    """A thread in full: the finding and every reply."""
    out = [
        "",
        f"#### {thread['path']}:{thread['line'] or '?'} "
        f"(round {thread['round']}, {thread['reviewer']}; {tags(thread)})",
    ] + indent(thread["body"])
    for answer in thread["conversation"]:
        out += [f"  -> {answer['who']}, {when(answer['at'])}:"] + indent(answer["body"], "     ")
    return out


def pass_label(entry):
    if entry["findings"]:
        return f"Round {entry['round']}"
    return "Summary only, not counted as a round" if entry["summary_only"] else "Pass without findings"


def render_pass(entry, last, body_chars, full=False):
    label = pass_label(entry)
    out = ["", f"### {label}: head {entry['head'][:8]}, {when(entry['started'])}"]
    out.append("Reviews: " + ", ".join(
        f"{review['reviewer']} {review['state']} ({review['threads']} threads)"
        for review in entry["reviews"]
    ))
    for thread in entry["threads"]:
        if full:
            out += render_thread(thread)
        else:
            out.append(
                f"- [{tags(thread)}] {thread['reviewer']} "
                f"{thread['path']}:{thread['line'] or '?'}: {thread['headline']}"
            )
    for review in entry["reviews"]:
        if review["body"] and (full or body_chars):
            # A pass without findings has no round number to ask for.
            hint = f"--round {entry['round']}" if entry["round"] else "--body-chars -1"
            body = review["body"] if full else shorten(review["body"], body_chars, hint)
            out += ["" if full else None, f"Summary by {review['reviewer']}:"] + indent(body)
    change = entry["response"]
    label = "Change since this review" if last else "Change before the next reviewed head"
    if change:
        top = ", ".join(
            f"{item['path'].rsplit('/', 1)[-1]} (+{item['additions']} -{item['deletions']})"
            for item in change["files"][:6]
        )
        notes = [
            note for flag, note in (
                ("merged", "a merge from the base branch is left out"),
                ("unsure", "may include a merge from the base branch: the commit list is incomplete"),
                ("cut", "partial: GitHub cut a file list short"),
            ) if change[flag]
        ]
        out.append(
            f"{label}: {size(change)}" + (f": {top}" if top else "")
            + (f" ({'; '.join(notes)})" if notes else "")
        )
    else:
        out.append(f"{label}: " + (
            "none" if entry["unchanged"]
            else "unavailable (the history was rewritten in between, or a commit is gone)"
        ))
    return [row for row in out if row is not None]


def render_header(report):
    return [
        f"# Review rounds: {report['repo']}#{report['number']}",
        f"{report['title']}",
        f"Author {report['author']}, {report['state']}, base {report['base']}, "
        f"{report['commits']} commits, {report['reverts']} named as reverts",
        "",
    ]


def render(report, body_chars=1000):
    out = render_header(report)
    passes = report["passes"]
    rounds = [entry for entry in passes if entry["findings"]]
    if not passes:
        return "\n".join(out + ["No review from anyone but the author yet."])
    started = min(entry["started"] for entry in passes)
    ended = max(entry["ended"] for entry in passes)
    out.append(
        f"Rounds: {report['rounds']} (heads reviewed: {len(passes)}), "
        f"from {when(started)} to {when(ended)} ({span(started, ended)})"
    )
    if report["summary_only"]:
        out.append(
            f"  ({report['summary_only']} more heads got a review summary with no thread and no change "
            "request. They are listed as 'Summary only' and are rounds if the summary carries findings: "
            "read them with --body-chars -1)"
        )
    out.append("New threads per round: " + " ".join(
        f"{len(entry['threads'])}" + "*" * (not entry["threads"]) for entry in rounds
    ))
    if any(not entry["threads"] for entry in rounds):
        out.append(
            "  (* changes requested with no new thread: earlier findings stood, "
            "or the findings are in the review summary)"
        )
    unresolved = "" if report["unresolved"] is None else f", {report['unresolved']} unresolved"
    out.append(f"Threads: {report['threads']}{unresolved}. Reviewers: " + ", ".join(
        f"{name} ({row['reviews']} reviews, {row['threads']} threads)"
        for name, row in report["reviewers"].items()
    ))
    later = report["later_findings"]
    if later:
        out.append(
            "Threads after a reviewer's first pass, against the head that reviewer saw before: "
            + ", ".join(f"{later[key]} {key}" for key in PLACES if key in later)
        )
    first = report["size_at_first_review"]
    out.append(
        f"Size now: {size(report['size'])}."
        + (f" At first review: {size(first)}; {len(first['added_since'])} files joined later."
           if first else " Size at first review unavailable.")
    )
    changed = report["changed_since_first_review"]
    now = report["size"]["additions"] + report["size"]["deletions"]
    churn = changed["additions"] + changed["deletions"]
    out.append(
        f"Changed by the author since the first review, summed step by step: "
        f"+{changed['additions']} -{changed['deletions']}"
        + (f" ({churn / now:.1f}x the diff as it stands)" if now else "")
        + f"; {changed['undone']} lines written in one response were removed by a later one"
    )
    if changed["unavailable"]:
        out.append(
            f"  ({changed['unavailable']} of the changes between reviewed heads could not be measured "
            "and are not in these numbers or in the tables below)"
        )

    total = report["rounds"]
    tables = (("Hot files", report["files"], 15), ("Areas", report["areas"], 10))
    for title, table, limit in tables if report["threads"] else ():
        hot = title == "Hot files"
        out += ["", f"## {title}", "threads  rounds    " + ("changed-in  undone  " if hot else "") + "path"]
        ranked = sorted(table.items(), key=lambda item: (-item[1]["threads"], item[0]))
        for path, row in ranked[:limit]:
            in_rounds = len([value for value in row["rounds"] if value])
            extra = f"{row['responses']:>10}  {row['undone']:>6}  " if hot else ""
            out.append(f"{row['threads']:>7}  {in_rounds:>2} of {total:<2}  {extra}{path}")
        if hot:
            out.append(
                "(changed-in: responses that changed the file. undone: lines one response "
                "added there and a later one removed)"
            )

    threads = [thread for entry in passes for thread in entry["threads"]]
    long_threads = sorted(
        (thread for thread in threads if thread["replies"] >= LONG_THREAD),
        key=lambda thread: -thread["replies"],
    )
    if long_threads:
        out += ["", "## Long threads"]
        for thread in long_threads[:10]:
            last = thread["conversation"][-1]
            out.append(
                f"- round {thread['round']}, {thread['path']}:{thread['line'] or '?'}: "
                f"{thread['replies']} replies ({thread['author_replies']} by the author). "
                f"{thread['headline'][:90]} | last word, {last['who']}: {headline(last['body'], 110)}"
            )

    out += ["", "## Rounds"]
    for entry in passes:
        out += render_pass(entry, entry is passes[-1], body_chars)
    return "\n".join(out)


def render_rounds(report, numbers):
    """The chosen rounds in full: every thread with its replies, and the summaries."""
    passes, out = report["passes"], []
    for number in numbers:
        chosen = [entry for entry in passes if entry["round"] == number]
        if not chosen:
            raise ValueError(f"no round {number}: the pull request has {report['rounds']}")
        out += render_pass(chosen[0], chosen[0] is passes[-1], 0, full=True)
    return out


def render_threads(report, needles):
    """Every thread on the files whose path contains one of `needles`, in full."""
    out = []
    for needle in needles:
        threads = [
            thread for entry in report["passes"] for thread in entry["threads"]
            if needle in thread["path"]
        ]
        if not threads:
            raise ValueError(f"no thread on a path containing {needle!r}")
        out += ["", f"## {len(threads)} threads on paths containing {needle!r}"]
        for thread in threads:
            out += render_thread(thread)
    return out


def target(argument, repo):
    """Resolve the repository and number from a URL, a number, or the branch."""
    match = PR_URL.search(argument or "")
    if match:
        return match.group(1), int(match.group(2))
    if repo and not argument:
        # The current branch's pull request belongs to the current repository.
        raise ValueError("give a pull request number or URL together with --repo")
    repo = repo or gh("repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner").strip()
    if argument:
        return repo, int(argument.lstrip("#"))
    return repo, int(gh("pr", "view", "--json", "number", "--jq", ".number").strip())


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("pr", nargs="?", help="number or URL; default: the current branch's pull request")
    parser.add_argument("--repo", help="owner/name; default: the current repository")
    parser.add_argument("--round", type=int, nargs="+", metavar="N", default=[],
                        help="print these rounds in full: their threads with every reply, and their review summaries")
    parser.add_argument("--threads", nargs="+", metavar="PATH", default=[],
                        help="print in full every thread on the files whose path contains PATH")
    parser.add_argument("--raw", action="store_true",
                        help="keep the collapsed sections of bot reviews that describe the run")
    parser.add_argument("--json", action="store_true", help="print the whole report as JSON")
    parser.add_argument("--body-chars", type=int, default=1000,
                        help="characters of each review summary in the overview; 0 for none, -1 for all")
    args = parser.parse_args()
    try:
        report = build_report(fetch(*target(args.pr, args.repo), args.raw))
        if args.json:
            text = json.dumps(report, indent=2)
        elif args.round or args.threads:
            text = "\n".join(
                render_header(report)[:-1]
                + render_rounds(report, args.round) + render_threads(report, args.threads)
            )
        else:
            text = render(report, args.body_chars)
    except (GhError, ValueError) as error:
        sys.exit(f"review_rounds: {error}")
    print(text)


if __name__ == "__main__":
    main()
