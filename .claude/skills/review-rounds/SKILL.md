---
name: review-rounds
description: Find out why a pull request needed so many review rounds, decide whether the cause is architectural, and say what to change. Use when a PR has gone through many rounds of findings and fixes (about four or more), when someone asks why a PR keeps getting review comments, is stuck in review or is taking so long to land, before pushing yet another round of fixes to a long-running PR, and for a look back at a merged PR that was painful to review.
argument-hint: "[pr-number | pr-url]"
---

# Review Rounds

A pull request that keeps drawing findings round after round is usually telling
you something about its design, not about its reviewers. When a responsibility
sits in the wrong place, or one fact is kept in two places, every fix is local
and the next review finds the next case of the same problem. This skill reads
the review history, separates that situation from the ordinary ones (a large
diff, a noisy reviewer, a scope that grew), and gives advice that matches.

It is read-only. Do not push, comment, resolve threads or request reviews
because this skill ran. If the user wants the result posted, that is a separate
request.

## 1. Measure

Run the script from the repository root. With no argument it takes the current
branch's pull request; `--repo owner/name` reads another repository.

```bash
python3 .claude/skills/review-rounds/scripts/review_rounds.py <number-or-url>
```

A round is one head commit on which a reviewer other than the author opened a
new thread or requested changes. Replies in existing threads are not rounds.
The overview gives, per round, the new threads and the change the author made
before the next reviewed head, and overall:

- **New threads per round.** A healthy review decays fast (`17 4 1 0`). A flat
  or sawtooth tail (`17 7 3 8 1 4 2 2 2`) means fixes keep producing findings.
  `0*` is a round that still requested changes without a new thread: earlier
  findings stood, or the findings are only in the review summary. Threads
  undercount for reviewers that list findings in the summary, so read the
  summaries before trusting a low number.
- **Where later findings sit**, measured against the head the same reviewer
  had reviewed before. `new code` is a finding on lines written since then,
  `changed file` is on unchanged lines of a file that was changed since then,
  and `old code` is on code that reviewer had already seen and passed over.
- **Hot files.** Threads per file, in how many rounds, how many responses
  changed the file, and `undone`: lines one response added and a later one
  removed. A file found in most rounds, rewritten in most responses and with a
  high `undone` count is where the design problem lives, if there is one.
- **Long threads**, with who had the last word. A thread that went back and
  forth and ended disputed or withdrawn shows up here, even though GitHub lists
  it as resolved.
- **Growth.** Size at first review against size now, files that joined later,
  how much was changed since the first review, and commits named as reverts
  (a revert made by hand shows in `undone`, not in that count).

If the overview shows three rounds or fewer, say so and stop: there is nothing
to diagnose. Do not invent a problem to fill the report.

## 2. Read the evidence

The headlines in the overview are an index, not the evidence. The same script
prints the conversations in full, with every reply and the whole review
summary. Each run reads the pull request again (about half a minute for a long
one), so ask for everything you need in one call:

```bash
python3 .claude/skills/review-rounds/scripts/review_rounds.py <pr> \
  --threads <hot-file> <hot-file> --round <n> <n>
```

- Read the threads on the hot files, and the rounds that did not decay.
  Reading every thread of a long pull request is not needed; reading none of
  them is guessing.
- A reviewer may already have named a pattern across its own rounds, or
  proposed a split, in a review summary. Treat that as a lead to check against
  the code, not as the verdict.
- Read the hot files as they stand now, and the diffs between the reviewed
  heads the overview names (`git diff <head>..<next-head> -- <path>`; fetch
  `refs/pull/N/head` first when the commits are not local).
- Read the rules the code is measured against:
  `book/src/contributing/coding-conventions.md` for where each responsibility
  belongs in this repository, and the agent guide of the hot files' own package
  when it has one (a `CLAUDE.md` or `AGENTS.md` in or above their folder),
  which holds the rules local to it.

Then group the findings by what they are about, not by file or by round: for
example "partial failure between the two stores", "input that the binding
checks itself", "copy of a rule that drifted". Three to six themes usually
cover a pull request. Note for each theme how many findings it has and across
how many rounds.

## 3. Decide

For each theme ask one question: **is there a single structural change that
would have made these findings impossible, instead of fixing them one at a
time?** If yes, and the theme spans several rounds, the cause is architectural.
Name the change. The recurring shapes:

| Shape | What the review looks like |
| --- | --- |
| A responsibility in the wrong layer | Findings keep landing on a binding, wrapper or view that decides something. Reviewers say "move this to", "this belongs in". Each fix adds a branch to the caller. |
| One fact kept in two places | Findings about partial failure, rollback, ordering, stale values, races. The same finding returns under a new line number. Fixing one ordering exposes the next. |
| A rule copied instead of owned | "The other path still does X", "apply the same check here". The same fix lands at one more call site each round. |
| Cases listed instead of ruled out | Each round finds one more input or state the code did not think of. The type or interface allows states that should not exist. |
| A leaking interface | Callers must track, pass or sequence something the callee knows. Findings are about callers getting that wrong. |
| An abstraction that does not fit | Parameters, flags or special cases grow with each response. A high `undone` count: the same function written, taken back and written again. |
| More than one change in one PR | The diff grew during review, files joined late, and findings split cleanly into unrelated groups. |

A theme with no such change is not architectural, however many rounds it took.
The ordinary causes, and how they show:

- **Reviewer churn.** Many later findings on `old code`: the reviewer raised
  new points on code it had already passed, or two reviewers pulled in opposite
  directions (code taken back, then a finding asking for it again). The advice
  is about the review process, not the code.
- **One push per finding.** Many rounds in a short time, each with one or two
  threads and a small response. The round count overstates the trouble.
- **Suggestions taken as scope.** Later rounds are optional suggestions, each
  accepted, each adding code that draws its own suggestions.
- **Not ready at first review.** A large first round of plain bugs, missing
  tests or failing checks, then a fast decay.
- **Requirements moved.** The findings follow a decision that changed during
  review, visible in the thread replies.

Many small rounds in a few hours fit "one push per finding" and an
architectural cause equally well. What separates them is where the later
findings sit. Pushing often does not create findings: if the findings after the
first round were mostly on `new code`, the fixes produced them, and the cause
is in the design or in what was accepted as scope. If they were on code that
was already there in the first round, batching would have been enough.

Be honest about mixed cases. A pull request can have one architectural theme
and a tail of reviewer churn; report both with their share of the findings.
Check the diagnosis against the numbers before giving it: an architectural
cause should account for the hot files and for the rounds that did not decay.
If it does not, it is not the cause.

## 4. Advise

Advice depends on the state of the pull request.

- **Open, architectural cause.** Say what to restructure and where the
  responsibility should go. When the cause is a misplaced responsibility, load
  the `layer-ownership` skill and name the current and target owner the way it
  does (repository, crate or package, module). Say whether to do it in this
  pull request, or to split: land the part that reviewed cleanly and move the
  contested part to a follow-up built on the right structure. Recommend
  stopping the fix-per-finding loop until that is decided, since each further
  fix is spent on code that will be replaced. List the open findings the
  restructure makes moot, and the ones it does not.
- **Open, ordinary cause.** Say what finishes it: batch the remaining fixes
  into one push, decline optional suggestions with a reason or move them to an
  issue, split unrelated scope out, or settle the disputed point with a person.
- **Merged.** Say whether the structural problem is still in the code. If it
  is, describe the follow-up refactor and what it would cost. Either way, give
  the one thing to do differently next time, at the point it would have been
  cheap (usually: before the first review, when choosing where the code lives).

Keep the advice proportionate. A restructure has to be worth more than the
rounds it saves; for a small pull request near the end of its review, the
right advice is often "finish it, and fix the structure when this code is next
touched".

## Output

Lead with the verdict in one line: `architectural`, `partly architectural` or
`not architectural`, and the cause in a few words. Then:

1. **The numbers**: rounds, threads per round, where later findings sat, the
   hot files. Three or four lines.
2. **Themes**: each with its finding count, the rounds it spans, and two or
   three findings quoted or linked as evidence.
3. **Cause**: the structural change that would have prevented the main theme,
   or the ordinary cause, and how much of the review it explains.
4. **Advice**: what to do now, in order, and what to do differently next time.

State what was not read (threads seen only as headlines, a head that could not
be compared, a file list GitHub cut short), since a diagnosis from partial
history can miss the cause.
