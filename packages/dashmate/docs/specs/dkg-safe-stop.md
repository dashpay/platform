# DKG-safe stop

How dashmate decides whether a masternode can be stopped without risking a PoSe penalty, and how `--safe` waits until it can. What operators see is described in [`stop`](../commands/stop.md#stopping-a-masternode-safely); this page explains the rule behind it and the Dash Core behaviour it relies on.

The check runs before Core is stopped by `stop`, `restart`, `group stop` and `group restart` on a masternode. `stop`, `restart` and `group stop` take `--force` to skip it; `group restart` has no `--force`. `stop` and `restart` take `--platform`, which leaves Core running and needs no check.

## Why stopping can cost a penalty

A DKG (distributed key generation) session creates the keys of a new quorum. It runs in phases of `dkgPhaseBlocks` blocks each, starting at the session's start block (`quorumHeight`). The final commitment lists which members took part properly; members left out of it get a PoSe penalty.

What decides whether a member is left out is sent by the end of the Commit phase, `5 × dkgPhaseBlocks` blocks after the start: 10 blocks for most quorum types, 20 for the 400-member ones. Finalizing and mining the commitment does not need this node. dashmate calls those first `5 × dkgPhaseBlocks` blocks the session's *active window*. Dash Core 24.1 uses the same window for its own count of active sessions.

Core joins a session only when it processes the session's start block while running. A node that is down at the start, or restarted during a session, misses that session. So stopping is unsafe:

- while a session this node is a member of is in its active window;
- from 6 blocks before such a session starts (`MIN_BLOCKS_BEFORE_DKG`), as a restart may not finish in time.

In rotated quorum types (`llmq_60_75`, `llmq_test_dip0024` and `llmq_devnet_dip0024`), each quorum index starts on its own block, `cycleBase + quorumIndex`, and its active window is measured from that block, not from the cycle's shared mining window.

## The rule

`getDkgStopBlockers` turns Core's DKG data into *blockers*: height ranges `[from, until)` in which stopping is unsafe. `null` for `from` means "already", `null` for `until` means "no known end". Stopping is safe at a height no blocker covers. Blockers that start above the tip do not block yet; they only feed the estimate of when stopping becomes safe.

| Reason | From Core data | Blocks |
|---|---|---|
| `upcoming` | an `upcoming_dkgs` entry this node is a member of, or whose membership Core could not determine | from 6 blocks before its start to the end of its active window |
| `imminent` | no `upcoming_dkgs` (Core 23, or Core does not know this node's proTxHash): any DKG may include this node | from 6 blocks before the next DKG start Core reports (`next_dkg`), with no known end |
| `session` | a `dkgstatus` session record inside its active window | until the end of that window |
| `new-round` | a record one `dkgInterval` old, within the next round's active window: that round has started and Core has not replaced the record yet, so this node may be a member again | until the end of the next round's active window |
| `unrecorded` | Core older than 24.1 only: `dkgstatus` `quorumConnections` shows a current session started with no record for it, and Core did not announce it without this node | until the end of its active window |
| `active-count` | Core 24.1+ only: `active_dkgs > 0` | with no known end |
| `unreadable` | data dashmate cannot interpret: an unknown LLMQ type, a malformed entry | with no known end |

`LLMQ_TYPES` holds `dkgInterval` and `dkgPhaseBlocks` of every type. It mirrors Dash Core's `src/llmq/params.h` and must be kept in sync with it. A type it lacks is unreadable wherever it could block: in a session record, in a current session of `quorumConnections` (read on Core older than 24.1), and in an announcement that may include this node. An announcement without this node is ignored whatever its type. `dkgstatus` names types by name, `upcoming_dkgs` by numeric id.

## Core versions

dashmate reads Core's numeric `version` (`10000 × major + 100 × minor + patch`) from `getnetworkinfo`, once per plain stop and on every poll of `--safe`, not from the configured image. A floating image tag such as `dashpay/dashd:24` can move to a new minor version on pull, and the running binary may differ from the configured image.

- **Core 23** reports no `upcoming_dkgs`, so the `imminent` rule applies to every DKG start.
- **Core 24.0** lists upcoming DKGs with this node's membership once their work block, 8 blocks before the cycle starts, is mined. Every DKG starting within the 6-block margin is therefore listed. All indices of a rotated cycle share the work block of the cycle base. Its `active_dkgs`, however, counts the DKG worker's local session records, which outlive their sessions by a whole interval. dashmate does not treat it as a blocker; it only compares it with the number of records to catch records changing during a read.
- **Core 24.1+** counts in `active_dkgs` the sessions this node is a member of, or whose membership it cannot determine, from their start block to the end of their Commit phase (dashpay/dash#7812). While Core knows the node's proTxHash the count is authoritative, so no confirmation is needed; without one Core reports 0 and no `upcoming_dkgs`. `MEMBERSHIP_ACTIVE_DKGS_CORE_VERSION` (240100) marks this. Should that change ever be backported to a 24.0.x release, this constant would not see it, and dashmate would read that release as 24.0.

Session records are read on every version. On Core 24.1, if Core loses the node's proTxHash mid-session (a ProUpServTx moving its IP elsewhere, or an operator key change or revocation), `active_dkgs` drops while the session keeps running; its record still shows it.

On Core older than 24.1, the record of a new session appears with a delay. At the start block, Core erases the previous round's record, computes the new session's members, and only then records the session if this node is a member. At the start of a rotation cycle it computes all indices first. Until then nothing in the record shows the session, which is why the `new-round` and `unrecorded` blockers and the confirmation below exist.

## Reading Core consistently

Each RPC reads Core's tip on its own, so one check reads `getblockchaininfo`, `quorum dkginfo`, `quorum dkgstatus` and `getblockchaininfo` again, in that order. The check is *unsettled*, and is tried again, when:

- the tip hash changed between the two `getblockchaininfo` reads;
- on Core older than 24.1, `active_dkgs` differs from the number of records `dkgstatus` lists: the worker changed its records between the reads.

**Syncing Core.** When both `getblockchaininfo` reads report initial block download, Core takes no part in DKGs. It is in initial block download while it reindexes or loads blocks, while its tip has less than the minimum chain work, or while its tip is more than 6 hours old (the default `-maxtipage`); once out, it stays out until restarted. Session data then does not apply, but announced DKGs (`upcoming`, `imminent`) still do, as Core may finish syncing in time for them, and so does unreadable data from any source. The tip may move during these reads. The check is kept if the chain read last still has every work block Core's membership answers name (`workBlockHeight`, `workBlockHash`), and is unsettled otherwise. `next_dkg` is counted from the tip read before `dkginfo`, the lowest one Core may have used.

**Reorgs.** A check can be given the tip of an earlier one. It looks that block up on the chain it read, after its DKG reads. If the chain is now lower than that block, or has a different block at its height, the check is unsettled and reports a reorg. `--safe` then drops the DKGs it saw announced without this node; the blockers it remembers stay.

## Confirmation on Core older than 24.1

Because a new session's record appears with a delay, a safe result on Core older than 24.1 is checked again `SAFE_STOP_CONFIRMATION_DELAY_MS` (15 seconds) after the first check finished. By then every block the first check saw is old enough for Core to have recorded its sessions.

The confirming check must find the first check's tip still on the chain; a lower or replaced tip makes the result unsettled. A safe confirming result must also be at most 6 blocks above the first check's tip, as the first check ruled out a DKG of this node starting within those 6 blocks; more makes it unsettled. A blocked confirming result stays blocked, and a Core that restarted into initial block download has no such limit.

The DKGs the first check saw stay in the result when the confirming read is discarded, when Core changed its records during it, and when Core restarted into initial block download in between.

A syncing Core and Core 24.1+ need no confirmation.

## Plain stop and `--safe`

**Plain stop** checks up to 3 times while the result is unsettled; its attempts share no memory. If the last result is blocked, it refuses with what blocks the stop and the earliest safe block; if still unsettled, it says Core's DKG state kept changing. For unreadable data it suggests updating dashmate and never `--safe`, as waiting does not help. It suggests `--safe` and `--force` only when the command has them. A failed RPC call ends it at once.

**`--safe`** checks every 10 seconds, reading Core's version each time, as Core may have been replaced. It stops the node only once a check is safe and nothing it remembers blocks. It remembers:

- every blocker with a known start and end a check reported, announced DKGs and running sessions alike, until its active window ends, even after Core stops reporting it, as after a restart. It takes in the first check's before a confirmation starts, so a check cut short loses nothing, and skips the confirmation when a remembered blocker already covers the first check's height.
- the DKGs Core announced without this node, so that their sessions are not mistaken for unrecorded ones. These are dropped on a reorg, as on another branch the node may be a member after all.
- the tip of its last check, for the reorg check above.

It gives up and leaves the node running when data stays unreadable for 3 checks in a row, or when Core fails to answer 6 checks in a row, about a minute. Ctrl+C also ends the wait and leaves the node running.

**Local networks.** Group commands stop a group's nodes in reverse order, so a local network's miner, the seed node set up last, stops before its masternodes. With the miner down no block comes, so a wait could never end. And with every masternode of the network going down together, their DKG produces no commitment to penalize them in. So masternodes of a group whose miner the same command stopped skip the check.

## Progress and estimate

While `--safe` waits, it shows the earliest height no blocker covers and the blockers between the tip and that height, or only those at the tip when that height is unknown, each session once. It shows "unknown" when a blocker with no known end is in the way. That height is a lower bound: Core announces a node's membership only 8 blocks before the cycle starts, so a DKG not announced yet can push it out.

Time estimates use the nominal block time: 150 seconds on mainnet and testnet, `core.devnet.powTargetSpacing` on devnets, none on local networks. The output is updated only when its text changes, so logs without a terminal get one line per change.

On Core 24.1+ the `active-count` line is left out when the running sessions dashmate knows of, from records or remembered announcements, account for the count. This affects only the display and the estimate; the count still blocks.

## Known limits

- **Plain-stop retries.** Plain stop's attempts share no memory. A Core that restarts into initial block download during a 15-second confirmation could forget an announced DKG between attempts. Restarting into initial block download needs a tip over 6 hours old, or a reindex.
- **Fast-syncing Core.** A syncing Core's announcement list covers DKGs up to 8 blocks past the tip `dkginfo` read. If the tip moved further during the reads, a DKG beyond that is not seen. Such a Core is usually catching up on blocks over 6 hours old, whose DKGs the network ran long ago. The gap matters only when the network itself made no block for 6 hours, or when Core is near the end of a reindex, where the blocks it connects can be recent.
- **Group commands.** These stop nodes one after another, each waiting for its own window, while the nodes already stopped stay down.
- **Local network with its miner already down.** The exemption applies only when the same command stopped the miner. `stop --safe` on one local masternode whose miner is already stopped waits for blocks that never come.

## Code

| File | Role |
|---|---|
| `src/core/quorum/getDkgStopBlockers.js` | the rule, `LLMQ_TYPES`, version switch, estimate |
| `src/core/quorum/checkMasternodeSafeToStop.js` | consistent reads, syncing Core, reorgs, confirmation |
| `src/core/quorum/waitForDKGWindowPass.js` | the `--safe` loop and what it remembers |
| `src/core/quorum/formatDkgStopStatus.js` | progress and refusal text |
| `src/listr/tasks/stopNodeTaskFactory.js` | plain-stop retries, local-network exemption, block time |
| `src/commands/stop.js`, `restart.js`, `group/stop.js`, `group/restart.js` | flags each command offers; group order, miner first |
| `src/constants.js` | `MIN_BLOCKS_BEFORE_DKG` |
