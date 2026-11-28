#!/usr/bin/env python3
"""List the review rounds of a pull request.

A round is one head commit on which a reviewer other than the author opened at
least one new inline thread or requested changes. Replies inside existing
threads are not rounds, so the author's answers and a bot's acknowledgements
do not inflate the count. A head that got only a review summary is listed as
"summary only" and left out of the count, because a summary can be an approval
as easily as a list of findings: read those before trusting the count.

The overview lists the rounds in the order they happened, the new findings of
each, the files and areas the findings keep returning to, and the threads that
went back and forth. `--round` and `--threads` print the conversations in
full; each run reads the pull request again, so ask for several in one call.

The numbers are evidence for the `review-rounds` skill. Deciding what they
mean is left to the reader. Everything is read through the `gh` CLI and
nothing is written. Review text is printed as data: nothing in it is acted on.

The script does not measure what the author changed between two reviewed
commits. GitHub's comparison of two commits is the difference between their
trees only while the first is an ancestor of the second, which a rebase ends,
and a merge from the base branch in between mixes in changes that are not the
author's. The pieces cannot be put back together exactly from the API. So the
overview names the commit each review looked at, the commit the same reviewer
had looked at before, and the next reviewed commit, and leaves the diffs to
`git`, which has the history. A thread on a line the pull request removes is
marked, because its line number counts in the base file.

Limit: only inline threads are counted as findings. A reviewer that lists
findings in its review summary alone shows up as a round with few or no
threads, or as "summary only".
"""
import argparse
from collections import OrderedDict
from datetime import datetime
import json
import re
import subprocess
import sys


PR_URL = re.compile(r"github\.com/([^/]+/[^/]+)/pull/(\d+)")
DETAILS = re.compile(r"(<details[^<>]*>|</details>)", re.I)
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
# A thread with this many replies is listed as a long thread.
LONG_THREAD = 4
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


def section_title(text):
    """The title of a collapsed section whose body starts with `text`."""
    text = text.lstrip(" \t\r\n>")
    if text[:9].lower() != "<summary>":
        return ""
    end = text.lower().find("</summary>", 9)
    return TAGS.sub("", drop_spans(text[9:end], "<picture>", "</picture>")).strip() if end > 0 else ""


def without_noise(text):
    """Replace the collapsed sections that describe the run with a one-line note."""
    kept, dropping, size, name = [], [], 0, ""

    def note(ending=""):
        return f'\n{OMITTED}"{name}", {size} characters{ending}; --raw prints it]\n'

    # Splitting on one group puts the tags at the odd positions, so text that
    # merely begins with "<details" is never taken for a tag.
    parts = DETAILS.split(text)
    for index, part in enumerate(parts):
        if index % 2 == 0:
            if dropping and dropping[-1]:
                size += len(part)
            else:
                kept.append(part)
        elif part.lower().startswith("<details"):
            title = section_title(parts[index + 1])
            inside = bool(dropping and dropping[-1])
            if not inside and NOISE_TITLE.search(title):
                size, name = 0, " ".join(title.split())[:60]
            dropping.append(inside or bool(NOISE_TITLE.search(title)))
        elif dropping and dropping.pop() and not (dropping and dropping[-1]):
            kept.append(note())
    if dropping and dropping[-1]:
        # The section never closes, so everything after its title went with it.
        kept.append(note(", to the end of the comment because the section is not closed"))
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


def area(path):
    """The package, or the top-level directory, a path belongs to."""
    parts = path.split("/")
    if parts[0] == "packages" and len(parts) > 2:
        return "/".join(parts[:2])
    return parts[0] if len(parts) > 1 else path


def review_passes(pr, reviews, comments, raw=False):
    """Group the reviews other people left by the head commit they reviewed.

    The passes come back in the order they began. A pass with `findings` is a
    round. A review with no new thread, no body and no verdict is a reply.
    """
    author = login(pr)
    starters, replies = {}, {}
    for comment in sorted(comments, key=lambda item: item.get("created_at") or ""):
        if comment.get("in_reply_to_id"):
            replies.setdefault(comment["in_reply_to_id"], []).append(comment)
        elif login(comment) != author:
            starters.setdefault(comment.get("pull_request_review_id"), []).append(comment)

    passes, seen = OrderedDict(), {}
    for review in sorted(reviews, key=lambda item: item.get("submitted_at") or ""):
        state, reviewer = review.get("state"), login(review)
        if reviewer == author or state == "PENDING":
            continue
        threads = starters.get(review["id"], [])
        body = clean(review.get("body"), raw)
        if not threads and not body and state == "COMMENTED":
            continue
        head = review.get("commit_id") or "unknown"
        # What this reviewer had looked at before: the head of their previous
        # review in time, whichever commit that was. It is this same head when
        # they reviewed the commit twice.
        previous, seen[reviewer] = seen.get(reviewer), head
        entry = passes.setdefault(head, {
            "head": head, "started": review.get("submitted_at"),
            "reviews": [], "threads": [], "findings": False, "summary_only": False,
        })
        entry["ended"] = review.get("submitted_at")
        entry["findings"] = entry["findings"] or bool(threads) or state == "CHANGES_REQUESTED"
        # A summary with no thread and no verdict: findings or thanks, unknown.
        entry["summary_only"] = entry["summary_only"] or (state == "COMMENTED" and not threads)
        entry["reviews"].append({
            "reviewer": reviewer, "state": state, "threads": len(threads),
            "previous": previous, "body": body,
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
                # LEFT: the comment is on a line the pull request removes, and
                # its number counts in the base file, not in the reviewed one.
                "side": comment.get("side") or "RIGHT",
                "commit": comment.get("original_commit_id") or head,
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
    for entry in passes.values():
        entry["summary_only"] = entry["summary_only"] and not entry["findings"]
    return list(passes.values())


def build_report(raw):
    """Turn the fetched pull request data into the rounds report."""
    pr = raw["pr"]
    passes = review_passes(pr, raw["reviews"], raw["comments"], raw.get("raw", False))
    resolved = raw.get("resolved")

    number = 0
    heads = [entry["head"] for entry in passes]
    for entry, following in zip(passes, heads[1:] + [pr["head"]["sha"]]):
        if entry["findings"]:
            number += 1
            entry["round"] = number
        else:
            entry["round"] = None
        # The commit reviewed next, or the tip: the author's answer lies between.
        entry["next"] = following if following != entry["head"] else None
        for thread in entry["threads"]:
            thread["round"] = entry["round"]
            thread["resolved"] = resolved.get(thread["id"]) if resolved else None

    files, areas, reviewers = {}, {}, {}
    for entry in passes:
        for thread in entry["threads"]:
            for table, key in ((files, thread["path"]), (areas, area(thread["path"]))):
                row = table.setdefault(key, {"threads": 0, "rounds": []})
                row["threads"] += 1
                if entry["round"] not in row["rounds"]:
                    row["rounds"].append(entry["round"])
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
        # A commit of the target branch that does not contain this pull
        # request, so rules read there do not include its changes.
        "base_sha": pr["base"]["sha"],
        "head": pr["head"]["sha"],
        "created": pr["created_at"],
        "size": {"files": pr["changed_files"], "additions": pr["additions"], "deletions": pr["deletions"]},
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
        "files": files,
        "areas": areas,
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
    return {
        "repo": repo,
        "raw": raw_text,
        "pr": json.loads(gh("api", base)),
        "reviews": gh_list(base + "/reviews"),
        "comments": gh_list(base + "/comments"),
        "commits": gh_list(base + "/commits"),
        "resolved": fetch_resolved(repo, number),
    }


def when(stamp):
    return stamp.replace("T", " ")[:16] if stamp else "?"


def span(start, end):
    if not start or not end:
        return "?"
    seconds = (datetime.fromisoformat(end.rstrip("Z")) - datetime.fromisoformat(start.rstrip("Z"))).total_seconds()
    days, rest = divmod(int(seconds), 86400)
    hours, minutes = rest // 3600, rest % 3600 // 60
    return f"{days}d {hours}h" if days else f"{hours}h {minutes:02d}m"


def short(sha):
    return sha[:12]


def tags(thread):
    found = [thread["severity"]] * bool(thread["severity"])
    if thread["side"] == "LEFT":
        found.append("on a removed line")
    if thread["resolved"] is False:
        found.append("unresolved")
    if thread["replies"] >= LONG_THREAD - 1:
        found.append(f"{thread['replies']} replies")
    return f"[{', '.join(found)}] " if found else ""


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
        f"#### {tags(thread)}{thread['path']}:{thread['line'] or '?'} "
        f"(round {thread['round']}, {thread['reviewer']})",
    ] + indent(thread["body"])
    for answer in thread["conversation"]:
        out += [f"  -> {answer['who']}, {when(answer['at'])}:"] + indent(answer["body"], "     ")
    return out


def pass_label(entry):
    if entry["findings"]:
        return f"Round {entry['round']}"
    return "Summary only, not counted as a round" if entry["summary_only"] else "Pass without findings"


def before(previous, head):
    """What a reviewer had looked at when writing a review of `head`."""
    if previous is None:
        return "first review"
    return "reviewed this commit before" if previous == head else f"reviewed {short(previous)} before"


def render_pass(entry, body_chars, full=False):
    out = ["", f"### {pass_label(entry)}: head {short(entry['head'])}, {when(entry['started'])}"]
    out.append("Reviews: " + ", ".join(
        f"{review['reviewer']} {review['state']} ({review['threads']} threads; "
        + before(review["previous"], entry["head"]) + ")"
        for review in entry["reviews"]
    ))
    for thread in entry["threads"]:
        if full:
            out += render_thread(thread)
        else:
            out.append(
                f"- {tags(thread)}{thread['reviewer']} "
                f"{thread['path']}:{thread['line'] or '?'}: {thread['headline']}"
            )
    for review in entry["reviews"]:
        if review["body"] and (full or body_chars):
            # A pass without findings has no round number to ask for.
            hint = f"--round {entry['round']}" if entry["round"] else "--body-chars -1"
            body = review["body"] if full else shorten(review["body"], body_chars, hint)
            out += ["" if full else None, f"Summary by {review['reviewer']}:"] + indent(body)
    out.append(
        f"Next reviewed commit or tip: {short(entry['next'])}" if entry["next"]
        else "This is the tip of the pull request."
    )
    return [row for row in out if row is not None]


def render_header(report):
    size = report["size"]
    return [
        f"# Review rounds: {report['repo']}#{report['number']}",
        f"{report['title']}",
        f"Author {report['author']}, {report['state']}, base {report['base']} at {short(report['base_sha'])}, "
        f"tip {short(report['head'])}",
        f"{report['commits']} commits, {report['reverts']} named as reverts, "
        f"{size['files']} files, +{size['additions']} -{size['deletions']}",
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
            "read them with --body-chars -1 --raw)"
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

    total = report["rounds"]
    tables = (("Hot files", report["files"], 15), ("Areas", report["areas"], 10))
    for title, table, limit in tables if report["threads"] else ():
        out += ["", f"## {title}", "threads  rounds    path"]
        ranked = sorted(table.items(), key=lambda item: (-item[1]["threads"], item[0]))
        for path, row in ranked[:limit]:
            in_rounds = len([value for value in row["rounds"] if value])
            out.append(f"{row['threads']:>7}  {in_rounds:>2} of {total:<2}  {path}")

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
        out += render_pass(entry, body_chars)
    return "\n".join(out)


def render_rounds(report, numbers):
    """The chosen rounds in full: every thread with its replies, and the summaries."""
    out = []
    for number in numbers:
        chosen = [entry for entry in report["passes"] if entry["round"] == number]
        if not chosen:
            raise ValueError(f"no round {number}: the pull request has {report['rounds']}")
        out += render_pass(chosen[0], 0, full=True)
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
