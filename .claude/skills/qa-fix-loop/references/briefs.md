# Briefs

A brief is everything its reader knows. Fill every `<…>` from the ledger, the QA home and the repository; leave none in. Keep the order: the goal and the limits first, then what to read, then the work, then how to report.

Never put a key or a recovery phrase in a brief. Give the file and the field: "`wallets.W3.mnemonic` in `$QA_HOME/secrets.json`".

- [Device pass](#device-pass)
- [Retest](#retest)
- [Fix thread](#fix-thread)
- [New evidence for a fix thread](#new-evidence-for-a-fix-thread)
- [Follow-up after a failed retest](#follow-up-after-a-failed-retest)
- [The scheduled tick](#the-scheduled-tick)

## Device pass

One per app. Role: test. Own worktree.

```markdown
You run a QA pass of <SwiftExampleApp | KotlinExampleApp> against the <network> network. Do the work yourself and start no subagents. This is TESTING ONLY: commit nothing, push nothing, open no pull request, file no issue. Defects are REPORTED with evidence, not fixed. A local, uncommitted patch in your own worktree is allowed only if the app cannot reach the network without it; put its diff in the report.

## Situation
- Network: <name>, Drive <version>, protocol version <n>. Nodes: <DAPI addresses>. <Quorum service, faucet and its shared limit, if any.>
- Commit to test: your worktree starts on an unrelated commit. Run `git fetch --tags origin`, then `git checkout --detach <sha>`, and confirm `git rev-parse --short=10 HEAD` prints `<sha>`. Use this exact commit even if the branch has moved.
- Read first: `<TEST_PLAN.md path>`, `$QA_HOME/fixtures.md` (which row uses which thing on chain), `$QA_HOME/README.md` if it exists, and the app's build guide. Drive the app with the `<simulator-control | emulator-control>` skill.

## Your device and wallet
- Device: <simulator name and UDID | emulator serial and AVD>. Leave every other simulator and emulator alone; other agents are using them.
- Wallet: <name> ONLY, at `<field>` in `$QA_HOME/secrets.json` (mode 0600). Masternode voting key for the voting rows: `<field>`.
- Do NOT restore or use <wallets reserved for people, and the other agent's wallet>. Do not broadcast from the fixture identities; reading them is fine.
- Never print a recovery phrase or a private key in a message, the report, a file name or a log you keep. Type it only into the app's own field for it, and keep no screenshot that shows it.
- Faucet: at most <n> requests, and only for a row that needs funds the wallet lacks.

## Build
<The app's build commands.> Run long builds as background commands with the longest timeout. Keep the Rust target directory inside your worktree. Check `df -h ~` first and stop if under <n> GB are free.

## What to test, in this order
1. Connect and load: the app reaches the network, the wallet restores and syncs, the identity shows its balance, keys, names, tokens and shielded balance. Everything already on chain must read correctly.
2. <The selected rows, by test plan id, most important first.>
3. With time left, continue through the remaining rows in plan order.

For each row follow the plan's steps in the UI, then confirm the result independently: a proved read with a read-only script (SDK of the network's protocol version, in `$QA_HOME/reports/<app>/tools/`), or the app's stored data. A row is PASS only when you saw the expected result. Otherwise FAIL (what happened: screenshot, log lines, consensus error code), BLOCKED (by what), or NOT RUN.

## Rules on the chain
- After any error that says the outcome is unconfirmed or "may have gone through", do NOT send again. Check with a proved read first.
- A proved read can fail for a few minutes after a validator rotation. Wait and read again.
- <Fixtures that are single use, and which of them are yours.>

## Report
Keep `$QA_HOME/reports/<app>.md` up to date as you go, from the first row, so a partial run still leaves a record. Append a section "## Pass on <ref> <sha> (<date>)":
- commit tested, how the app was built and pointed at the network;
- a table of every row you touched: id, title, time (UTC), result, evidence;
- counts per result;
- each failure in detail: steps to reproduce, the exact error text, the file and line if you traced the cause, and your judgement of where it belongs (the app, an SDK or FFI layer, the wallet, the chain, a fixture, or the test plan);
- the state of the wallet that this run changed.
Screenshots and logs go in `$QA_HOME/reports/<app>/`, numbered. Check a log for key material before you save it.

## Final message
Rows run and counts per result. Each failure in one line. Say plainly which results you verified on chain and which you only saw in the UI, and what you could not check. Leave the device running with the wallet loaded.
```

## Retest

Same shape as a device pass, narrower. One per app with a `RETEST` line.

```markdown
Goal: retest, in <app> on <device> against the <network> network, the items below, now that their fixes are merged. Nobody has tried these fixes in the app yet. Testing only: change no file in the repository, commit nothing, open no pull request, file no issue. If something still fails, report it with evidence; do not fix it.

## Read first
- `$QA_HOME/reports/<app>.md`, the whole file: how the earlier pass built and pointed the app, and the exact steps and screens for every item below.
- `<TEST_PLAN.md path>` for the rows, and the `<simulator-control | emulator-control>` skill.
- Each fix's description: `gh pr view <number>`. It says what the fix claims, and what it lists as not yet verified.

## Build
Your worktree starts on an unrelated commit: `git fetch origin <base>`, `git checkout --detach <sha>`, confirm with `git rev-parse --short=10 HEAD`. Build as the earlier pass did, target directory inside your worktree. <Name the fixes that change Rust: the native library must be rebuilt, not reused.>

Device: <name>, running the build of <previous sha> with <wallets> loaded. Install the new build OVER it so its data is kept; that is also the test of the data migration. If the new build will not start or loses data, record that with the log as a finding, then install clean and restore the wallet (`<field>` in `$QA_HOME/secrets.json`; never print it).

## Chain facts
<Network, versions.> Consensus code is unchanged between what the nodes run and your commit <or: these differences exist, and why they are safe>. Your first check is that the app connects and one proved read works.

## Rules
<Wallet, single-use fixtures, "may have gone through", proved reads after rotation, faucet, keys: as in the device pass.>

## What to retest (fix in brackets)
1. <Row or behaviour> [#<number>]: <exact steps, the expected result, and the proved read that confirms it. Name the fixture by its id.>
2. …
<n>. Quick regression after the update: wallets, contacts and loaded identities are still there, and balances match the chain.
<Known leftovers that were deliberately not fixed: "do not retest; do not report as new".>

## Record
Append "## Retest on <base> <sha>" to `$QA_HOME/reports/<app>.md`. Append only. A table: item, time (UTC), result (PASS / FAIL / NOT RUN), evidence. Then one line listing the wallet state this run changed.

## Final message
Per item: PASS, FAIL or NOT RUN with one line of evidence and the proved read behind each write. Any new defect with exact steps, error text, and file and line if traced. Whether the in-place update kept the data. What you verified on chain, what you only saw in the UI, and what you could not check.
```

## Fix thread

One per ledger item. Own worktree from the development branch. The thread's title starts as the defect in plain words.

```markdown
Fix a defect found on <date> by a QA pass of <app> at <ref> on the <network> network (protocol version <n>). You own this from diagnosis to an open pull request.

## The defect
<What the user does, what they see, what they should see. The exact error text. What still works for the same wallet and data: the controls narrow the cause.>
Cause as the QA agent read it (verify it yourself before changing anything): <file, function and lines, or "not traced">.
Evidence: `$QA_HOME/reports/<app>.md`, section "<name>", item <n>; screenshots `<paths>`. Read only.

## What a good fix looks like
<The behaviour after the fix, and the layer that owns it.> FFI, Swift and Kotlin wrappers marshal rather than decide: a rule that both apps need goes in Rust, once. Check whether <the other app> has the same defect. If it does, say so in the pull request, and fix it there only when the fix is small and the same shape; otherwise report it in your final message.

## How to work
- Work on branch `<branch>`, cut from the current `origin/<base>`. If your checkout is not on it: `git fetch origin <base>`, then `git switch --no-track -c <branch> origin/<base>`.
- Read `CLAUDE.md` at the repository root and follow it, then `book/src/contributing/coding-conventions.md`<, and the app's own guide>.
- Before coding, search open pull requests and issues for the same fix. If one exists, stop and report it.
- Confirm the cause from the code. If the report is wrong about it, fix the real cause and say what differed.
- Add a unit test that fails without the fix wherever the logic can be tested without a device. Keep the test plan row accurate if it describes this flow (<row ids>).
- Run only the targeted checks for what you touched, never a full suite: `cargo fmt --all`, then the tests and clippy of each Rust crate you changed, and the app's own build or unit tests. Keep any Rust target directory inside your worktree.
- Do not use a simulator or an emulator, and broadcast nothing: devices belong to the QA agents. Your fix is retested on a device after it merges.
- Stage explicit paths only. Commit no notes, plans or specs.

## Pull request (you are not finished until it is open, ready, linked and watched)
- Base `<base>`. Title in Conventional Commits form with a type and scope from `.github/workflows/pr.yml`. Body from the `pr-description` skill: it opens with "## Basic explanation" in plain words, and shows a before and after of what the user sees. Set the milestone of the release this branch is for.
- Describe behaviour. Use no labels from the QA report or the ledger; test plan row ids are fine.
- Draft only while code is still being written. Mark it ready for review as soon as the code is complete and the targeted checks pass. List what was not tried on a device under "Not yet verified"; never hold the pull request in draft for that.
- If your harness can link a pull request to this session, rename the session and watch the pull request, do all three: link it, rename to "#<number> <title>", watch it. Otherwise stay on it yourself until its checks finish and the first reviews are answered. Handle failed checks and review comments without asking: rerun a check once when the failure is the runner's, reply on each review thread, and do not mention a review bot by its @name (that queues a new bot run).
- Never merge it, and never post `/self-reviewed`: both are the maintainer's.
- Final message: the pull request URL, its head commit, the state of checks and reviews, and anything you found that belongs to another thread.
```

## New evidence for a fix thread

When a later pass shows more about a defect that already has a thread, send it there instead of starting a second thread.

```markdown
New evidence from a device retest of <row or behaviour> (<date>, <ref>), for you to check against the fix in #<number>. No new task beyond that check.
- <What was seen, with what makes it different from the first report.>
- <What was ruled out.>
Evidence: section "<name>" of `$QA_HOME/reports/<app>.md`; files `<paths>`.
If #<number> already covers this, reply in one line that it does and why. If it covers only part, extend it in the same pull request.
```

## Follow-up after a failed retest

Sent to the thread that wrote the fix.

```markdown
#<number> is merged (<sha>) and was retested on <device> in a build of `<base>` at <sha>. <What now works, confirmed how.> The same retest found <what still fails or what else showed up>. Please open a follow-up pull request for it.

## What the device shows
1. <Steps, what is shown, what the chain says, and the cause if the QA agent traced it: file and line.>
Evidence: `$QA_HOME/reports/<app>.md`, section "Retest on <base> <sha>"; screenshots `<paths>`. Read only.

## What to do
- New branch from the current `origin/<base>` in your worktree: `git fetch origin <base>`, `git switch --no-track -c <branch> origin/<base>`.
- Fix the class, not the one screen: find every place with the same shape and list in the pull request which were affected and which you checked and found fine.
- Fix it where it belongs. If the right fix is in a lower layer, say so; do not paper over it in the screen.
- A unit test that fails without the fix. Same rules as before for checks, devices and the pull request.
```

## The scheduled tick

The prompt of the scheduled check, bound to the coordinator's thread, every 20 to 30 minutes.

```markdown
/qa-fix-loop tick

QA home: <path>. Run the ledger's `sync` and `next`, and act on each line as the skill's step 6 says. Reply with one line per item that changed state since the last tick, and nothing for the others. When `next` prints DONE, do step 9 and delete this scheduled check.
```
