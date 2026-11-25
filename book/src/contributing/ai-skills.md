# AI Skills

The repository ships a set of skills for AI coding agents. A skill is a folder
with a `SKILL.md` file: instructions for one kind of task, sometimes with a
script or a reference document beside it. The agent loads a skill when the task
matches its description, or when you ask for it by name (`/release`,
`/review-rounds 6057`).

The skills live in `.claude/skills/`, where Claude Code finds them.
`.agents/skills`, where Codex looks for a repository's skills, is a link to the
same folder, so both use one set. (`.codex/skills` is an older link to the same
place, kept for tools that still read it.) The skills are checked in, so
everyone who works in the repository with an agent gets them, and a change to
one goes through review like any other change.

## The skills

| Skill | What it does | Use it when |
| --- | --- | --- |
| `emulator-control` | Drives and inspects KotlinExampleApp on a booted Android emulator: taps, typing, screenshots, the view tree, funding, full identity and DashPay flows against testnet. | Reproducing a KotlinExampleApp bug, or checking an SDK change end to end on Android. |
| `layer-ownership` | Places a change in the right repository, crate and module, following the [coding conventions](coding-conventions.md). Flags business logic in bindings, duplicated validation and implementation details that leak to API clients. | Planning or writing code, and reviewing a pull request for where its code lives. |
| `pr-description` | Writes the pull request title and description for the current branch from the project's template, opening with the basic explanation. | Opening a pull request. |
| `protocol-upgrade-test` | Reviews and tests a Drive protocol upgrade: version maps, activation, first-block transitions, persisted data, and a rolling Dashmate upgrade on a local network. | Comparing release refs, changing what happens on the first block of a protocol version, or rehearsing an upgrade. |
| `prs` | Reports your open pull requests and the ones waiting on you across the Dash repositories: what each is blocked on and how long it has waited. | Checking the review queue. |
| `qa-fix-loop` | Runs the SwiftExampleApp and KotlinExampleApp test plans on devices against a devnet, testnet or local network, starts one fix thread with its own pull request for every defect, and retests when those pull requests merge, until every row passes. | Running a QA pass on the example apps, or retesting after fixes merged. |
| `release` | Cuts a Platform release from start to finish: the `yarn release` pull request, the GitHub release, its assets, and the signed dashmate checksums. | Publishing a stable version or a prerelease. |
| `review-rounds` | Reads a pull request's review history, finds why it needed so many rounds, decides whether the cause is architectural, and says what to change. | A pull request has gone through about four rounds or more, or is stuck in review. |
| `simulator-control` | Drives and inspects SwiftExampleApp on the booted iOS simulator: taps, typing, screenshots, SwiftData, logs, the accessibility tree. | Reproducing a SwiftExampleApp bug, or checking the app's stored state against what it shows. |

Each skill's own `SKILL.md` is the reference for how it works. This table only
says what exists.

## Review rounds

`review-rounds` deserves a note, because it encodes a habit of this project. A
pull request that keeps drawing findings, round after round, is usually not
suffering from picky reviewers. More often a responsibility sits in the wrong
layer, or one fact is kept in two places, and each fix is local, so the next
review finds the next case of the same problem.

The skill measures the history first: the rounds, the new findings in each,
the files that findings and rewrites concentrate in, how much code was written
in one response and removed by a later one, and whether later findings sit on
code written since the same reviewer last looked. Then it asks one question of
each group of findings: is there a single structural change that would have
made them impossible? If there is, the cause is architectural and the advice is
to restructure, or to split the pull request, instead of answering one more
round. If there is not, it names the ordinary cause: a diff that grew during
review, one push per finding, optional suggestions taken as scope, or a
reviewer raising new points on code it had already passed.

A change between two reviewed commits is measured only when the first leads to
the second, with merges from the base branch cut out. After a rebase or a
force-push the script reports the change as unavailable instead of guessing.

The measuring part also runs by itself, for the overview or for the full
conversations of a round or a file:

```bash
python3 .claude/skills/review-rounds/scripts/review_rounds.py 6057
python3 .claude/skills/review-rounds/scripts/review_rounds.py 6057 --round 2 --threads KeychainManager.swift
```

It reads through the `gh` CLI and writes nothing.

## Adding a skill

1. Create `.claude/skills/<name>/SKILL.md`. Its front matter needs `name`
   (the folder's name) and `description`. The description is what the agent
   reads to decide whether to load the skill, so say what the skill does and
   when to use it.
2. Keep instructions that every task needs in `AGENTS.md` instead. A skill is
   for one kind of task and is loaded only for it.
3. Put anything the skill runs in a `scripts/` folder beside `SKILL.md`, and
   test it like other code.
4. Add a row to the table above. A test in `.github/scripts/tests` fails when
   a skill is missing from this page or the page lists one that does not exist.
