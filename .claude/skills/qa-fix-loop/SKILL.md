---
name: qa-fix-loop
description: Run the SwiftExampleApp and KotlinExampleApp test plans on devices against a devnet, testnet or local network, start one fix thread with its own pull request for every defect found, and retest on a device when those pull requests merge, round after round until every row passes. Use for "run a QA pass", "test the apps on the devnet", "start fix threads for what failed", "the fixes merged, retest", "/qa-fix-loop", or whenever app testing should lead to pull requests and retests without being steered step by step.
argument-hint: "[start | tick | status | add <defect> | retest <ids>] [swift|kotlin|both] [network] [ref]"
---

# QA fix loop

Test the example apps on a simulator and an emulator, turn every defect into a pull request, and retest when it merges. The loop repeats until nothing fails:

```
preflight -> device pass -> verify -> triage -> fix threads -> pull requests
                 ^                                                  |
                 +---- retest on the merged build <--- merge -------+
```

You are the **coordinator**. You plan, brief, verify and keep the ledger. You do not drive a device and you do not fix code: QA agents do the first, fix threads the second.

## Authority

Being asked to run this skill is the go for everything the loop needs on a test network: starting agents and threads, broadcasting from the wallets assigned to agents, pushing fix branches and opening pull requests against the development branch. Do these without asking.

It is not the go for the following; each needs the user:

- Merging a pull request, or posting `/self-reviewed` (the author's own attestation).
- Anything on mainnet. The loop runs on devnet, testnet or a local network only.
- A defect that could halt or split the chain, or move funds without their owner. Tell the user privately. No thread, no pull request, no issue, and no description of it anywhere public.
- Filing issues, deleting files from the QA home, and choices the ledger marks `decision`.

## Arguments

| Argument | Meaning |
|---|---|
| `start` (default) | New loop: preflight, then the first device pass. |
| `tick` | One turn of the loop: sync the ledger and act on what is due. The scheduled check calls this. |
| `status` | Print the ledger table and what is due. Change nothing. |
| `add <defect>` | Put a defect somebody reported into the loop: triage it, then start its fix thread. |
| `retest <ids>` | Retest those merged items now (step 7) instead of waiting for the rest of the round. |

`swift`, `kotlin` or `both` picks the apps, then the network name, then the ref to test (default: the head of the development branch).

## The QA home

Everything the loop records lives outside the repository in one directory, `$QA_HOME` (default `~/.dash-qa/<network>`; use the one the user already has). Nothing in it is ever deleted: an old run is moved to `archive-<date>/`.

| Path | Holds | Writer |
|---|---|---|
| `ledger.json` | One item per defect, from found to passed. Written only through `scripts/ledger.mjs`. | coordinator |
| `secrets.json` (mode 0600) | Test wallets and keys. Agents get the path and the field name, never a value. | whoever seeds |
| `fixtures.md` | Which test plan row needs which thing on chain, and its id. See [references/fixtures.md](references/fixtures.md). | coordinator |
| `reports/<app>.md` | The app's QA record. Append only: one section per pass or retest. | that app's QA agent |
| `reports/<app>/` | Screenshots, logs and that agent's read-only check scripts. | that app's QA agent |

## Who does what

| Role | Runs as | Rules |
|---|---|---|
| QA agent, one per device | A child agent in its own worktree, detached at the tested commit. | Testing only: no commit, no pull request. Reports defects with evidence and does not fix them. |
| Fix thread, one per defect | A separate session in its own worktree from the development branch. | Owns the defect from diagnosis to an open pull request and handles its checks and review. Never uses a device. |

Which tool starts them depends on where you run:

| | T3 Code | Claude Code alone |
|---|---|---|
| QA agent | `delegate_task`, `isolation: "worktree"`, role `test` | `Agent` with `isolation: "worktree"` |
| Fix thread | `t3_thread_launch` with a worktree from `origin/<base>` and `startFromOrigin: true` | `Agent` with `isolation: "worktree"`, in the background |
| New evidence for a fix thread | `t3_thread_send` to that thread | `SendMessage` to that agent |
| The tick | `schedule_task`, an interval bound to this thread | `/loop 25m /qa-fix-loop tick` |

In any other harness: one agent per unit of work, each in its own `git worktree`, pull requests with `gh`, and a tick whenever you are woken or asked.

Model and effort are the user's choice. Without one, run QA agents and fix threads on a strong model at high effort, and give the strongest model to a defect whose cause is not traced or that crosses Rust, FFI and an app.

Write each brief from [references/briefs.md](references/briefs.md). An agent knows only its brief, so the brief carries the commit, the device, the wallet, the rows, the evidence paths and the rules.

## 1. Preflight

Answer from the QA home, the repository and the chain before asking the user anything. Ask only for what none of them can tell you: usually the network, the rows, and which wallets are reserved for people.

1. **Network.** Read its Drive version and protocol version from a node: `(await sdk.system.status()).toJSON()` in `@dashevo/evo-sdk` gives `version.software.drive` and `version.protocol.drive`. Stop if it is mainnet.
2. **Ledger.** If `$QA_HOME/ledger.json` exists, continue it. Otherwise:
   ```sh
   node .claude/skills/qa-fix-loop/scripts/ledger.mjs --home "$QA_HOME" init \
     --repo dashpay/platform --base <dev branch> --network <name> --deployed-ref <tag or sha the nodes run>
   ```
3. **Build against network.** `bash .claude/skills/qa-fix-loop/scripts/consensus_diff.sh <deployed-ref> <tested-ref>`. Exit 0 means a build of the tested ref can talk to the network. Exit 1 lists what differs in consensus, proof and node code: read it, and test a build of the deployed ref instead unless every difference is provably neutral.
4. **Check scripts match the network.** Independent reads use an SDK of the network's protocol version, never a newer or older one. `qa-contract/README.md` shows how to point a script at the workspace SDK build.
5. **Rows.** Resolve the request with the selection grammar in section 1 of each `TEST_PLAN.md` (`packages/swift-sdk/SwiftExampleApp/`, `packages/kotlin-sdk/KotlinExampleApp/`). With no request: rows that only the network's newest protocol version allows first, then one writing row per plan section, then the rest in plan order.
6. **Fixtures.** Every selected row needs its prerequisites on chain. Map them in `fixtures.md`. A row with no fixture is seeded now or recorded `BLOCKED`, never left to fail.
7. **Devices and wallets.** One simulator and one emulator, each with its own wallet. No identity has two broadcasters at once. Wallets reserved for people are not restored, used, or spent from, and nothing meant for them is used up. Check free disk space: each worktree builds its own Rust target directory.

## 2. Device pass

Start one QA agent per app, both in the same message so they run together. Use the **Device pass** brief. Then end the turn: each agent wakes you when it finishes. Do not poll.

A row's verdict is `PASS` only when the agent saw the expected result, `FAIL` with evidence, `BLOCKED` with what blocks it, or `NOT RUN`.

## 3. Verify

A report is a claim. Before acting on it:

- Read the whole new report section, not only the final message.
- For each defect, check that the evidence shows it: the screenshot, the log line, the consensus error.
- For each write that matters to a verdict, make your own proved read and note its height.
- Keep the difference visible in everything you write: what was confirmed on chain, and what was only seen in the UI.

## 4. Triage

Every failure and every new defect gets exactly one ledger item or joins one. Search open pull requests and issues first: a defect somebody is already fixing is `closed` with reason `duplicate` and its number in the note.

| What it is | Ledger | Then |
|---|---|---|
| A bug in one app | `found`, `--apps` that app | Fix thread. Its brief asks whether the other app has the same bug. |
| A bug in Rust, FFI or an SDK that both apps show | `found`, `--apps swift,kotlin` | One fix thread, fixed in the layer that owns it, never once per app. |
| Small UI or wording items | one item per app, up to about six | One fix thread for the batch. |
| A test plan row that is wrong | `found`, `--no-retest` | Fix thread that corrects the row. |
| A missing or used-up fixture | `closed`, reason `fixture` | Seed it, then run the row. |
| The network or its services (quorum service, faucet, payouts) | `closed`, reason `infra` | Tell the user who runs it. |
| Works as designed, or the fix is a product or design choice | `decision` | Ask the user. No thread until they answer. |
| Chain halt, fork or loss of funds | none | See Authority. |

```sh
L="node .claude/skills/qa-fix-loop/scripts/ledger.mjs --home $QA_HOME"
$L add --id kotlin-resume-after-pause --apps kotlin --rows TOK-13 \
  --title "Resume is never offered for a paused token" --source "reports/app-kotlin.md, defect 1"
```

Item ids are short and describe the behaviour. They are yours: a pull request describes what the user sees and never cites a ledger id or a report label. Test plan row ids are fine there.

## 5. Fix threads

Start one fix thread per `found` item, all in one message, with the **Fix thread** brief. Record each at once:

```sh
$L set kotlin-resume-after-pause state=threaded thread=<thread or agent id> branch=fix/kotlin-example-resume-after-pause model=<model>
```

A fix thread is finished only when its pull request is open against the development branch, ready for review, and watched by the thread itself. It is the thread that answers review comments and failed checks, not you.

Then schedule the tick (every 20 to 30 minutes) and end the turn.

## 6. The tick

```sh
$L sync && $L next
```

`sync` reads each item's pull request from GitHub and the head of the development branch. `next` prints one line per action that is due. Act on every line, then end the turn again.

| Line | Do |
|---|---|
| `THREAD <id>` | Start its fix thread (step 5). |
| `NUDGE <id>` | Read the thread. If it is still working, leave it. If it stopped with no pull request and no stated reason, send it one message saying it is not finished until the pull request is open, ready and watched, then `$L nudged <id>`. |
| `ASK <id>` | The pull request was closed unmerged. Ask the thread why, then close the item with a reason or re-thread it. |
| `RETEST <app>` | Every fix for that app is merged. Go to step 7. |
| `HOLD <app>` | Some fixes are merged and others are not. Wait, unless the open ones cannot change that app's build. |
| `RUNNING <app>` | A retest is on the device. Nothing to do. |
| `TRIAGE <id>` | A retest failed. Step 8. |
| `HUMAN <id>` | Put it in your next message to the user, once. Do not wait for the answer. |
| `DONE` | Step 9. |

A tick that changed nothing says nothing to the user.

## 7. Retest

A merged fix nobody has tried on a device is not a verified fix.

1. Take the head of the development branch from `sync` as the exact commit. Run `consensus_diff.sh` against the deployed ref again: other merges came in too.
2. Start one QA agent per app that has a `RETEST` line, with the **Retest** brief: the items, their rows, each fix's pull request number, and the fixture each one needs.
3. The new build is installed **over** the app that is on the device, so wallets and stored data stay. That also tests the data migration. A build that loses data is a defect.
4. Record the start, and the result of each item after verifying it as in step 3:
   ```sh
   $L retest-start --app kotlin --commit <sha> --ids kotlin-resume-after-pause,cosign-note --agent <task id>
   $L retest-result kotlin-resume-after-pause --app kotlin --result pass --evidence "proved read at height 1507: isPaused false"
   ```
   An item is `passed` when every app it names has passed.
5. A retest is where new defects show up. Each goes through step 4 and starts the next round.

## 8. When a retest fails

Send the new evidence to the fix thread that wrote the fix: it already knows the code. Ask for a follow-up pull request on a new branch from the current development branch, then `$L set <id> state=threaded branch=<new branch> pr=`. The item goes round again.

## 9. Finish

On `DONE`: remove the scheduled check, print `$L table`, and report:

- Per app: rows run, and counts of pass, fail, blocked and not run.
- Every item: its pull request and the commit it passed on.
- What is still open: `decision` items, rows not run and why, results that cannot be checked yet and the date they can.
- The state left behind: which devices are running which build with which wallets, and what the test wallets no longer have.

Record the runs on the QA contract with `qa-contract/src/submit-run.mjs` only when the user asks.

## Traps

Each of these has cost a run hours.

- **Wrong commit.** A child's worktree starts at the parent's commit, not the one to test. The brief has it detach at the exact SHA and print `git rev-parse --short=10 HEAD`.
- **Stale builds.** A Rust target directory shared between checkouts links crates from the other checkout. Each worktree keeps its own, and a retest of a Rust fix rebuilds the framework rather than reusing one.
- **"May have gone through".** After any error that says the outcome is unknown, nobody sends again. Read the chain first. A second send can pay twice.
- **Proved reads right after a validator rotation** can fail for a few minutes on a network with a quorum service. Wait and read again. It is not a defect in the app.
- **Fixtures get used up.** A pending action can be co-signed once, a pot claimed once per epoch. Give each agent its own, name them in the brief, and have each report list the state it changed.
- **A failing state can be the retest's fixture.** If a fix can only be shown from the broken state (a token left paused, a request left pending), leave it and say so in the item's note.
- **Slow clocks.** Contested names and polls end after hours or days. A result that cannot exist yet is `NOT RUN` with its date, not a failure.
- **Keys on screen.** Keys and recovery phrases go only into the app's own secure fields, are never printed, and never appear in a kept screenshot or log.
- **One writer per file.** Agents append to their own report. Only the coordinator writes the ledger.
- **A red check that is not the fix's own.** Runner download timeouts and emulator authentication errors pass on a rerun. The fix thread reruns once before reading a failure as real.
- **Two findings, one cause.** Before threading the second, check that the first thread's fix does not already cover it. Send the evidence to that thread instead.
