# Compilation Readiness

A smart contract's executable bundle does not run the moment it is accepted. Every evonode first prepares it (validates, links and compiles it locally), and the bundle activates only once enough of the network has reported that it is ready. This chapter describes the state Drive keeps for that mechanism from protocol version 17: the rounds, the reports, the cursors, the deadlines and the funds. The block event that evaluates rounds, the state transition that carries a report, the fee schedule and the classification of local failures arrive in later parts and get their own sections here.

## Why rounds are stored under their own key

The confirmed policy has three rules that shape the layout:

- One pending executable version per contract. A replacement atomically cancels the previous round and its activation timer, and reports for the old bundle cannot count for the new one.
- No automatic expiry. A round that never gathers enough reports stays pending forever.
- Bounded work per block. Every step the network takes for readiness (accepting a report, validating reporters against membership, activating, cleaning up) has a fixed cap.

A round can accumulate one report per evonode of the network, and nothing bounds that number except churn. If a replacement deleted the old round's reports in the same batch, the cost of a replacement would grow with the number of reports, and the pinned GroveDB prices a recursive subtree delete by its contents while its average-case estimator prices only the tree element. So a round lives under its own key beneath its contract, and the contract holds a pointer to the current round. Replacing a round is a pointer swap plus a fixed number of inserts and deletes; the old subtree stays on disk, unreachable through the pointer, until a bounded cleanup step drains it over as many blocks as it needs.

## Layout

Everything lives under the existing `Votes` root (`112`) in a new child `r`, and the funds beside the voting funds under the prefunded specialized balances root (`40`):

```text
112 Votes
 └─ r  Readiness (NormalTree)
     ├─ 0  contracts (NormalTree)                    key: contract_id (32)
     │      └─ contract_id (NormalTree)
     │           ├─ c          current round pointer   Item: round_id (32)
     │           └─ round_id   (NormalTree, one per open round; normally exactly one)
     │                ├─ 0  round record   Item: ReadinessRound (versioned, serialized)
     │                ├─ 1  reports        CountTree; key: pro_tx_hash (32) -> Item: ReadinessReportRecord
     │                └─ 2  scan cursor    Item: ReadinessScanCursor (absent when no walk is open)
     ├─ 1  deadlines (NormalTree)                    key: encode_u64(deadline_ms) (NormalTree)
     │      └─ contract_id (32) -> Item: round_id (32)
     ├─ 2  evaluation cursor                         Item: last contract_id visited by the block event
     └─ 3  retired rounds awaiting cleanup (NormalTree)
            key: round_id (32) -> Item: contract_id (32)
 40 PreFundedSpecializedBalances (SumTree)
 ├─ 128 voting funds (existing)
 └─ 129 readiness funds (SumTree)                    key: fund_id (32) -> SumItem
```

The keys `r`, `0` to `3`, `c` and `129` are provisional: the allocation register leaves new inner tags unallocated, and they are revised, if at all, before any network is asked to propose protocol version 17.

Paths and constants are in `packages/rs-drive/src/drive/votes/paths.rs`; the readiness fund key and paths are in `packages/rs-drive/src/drive/prefunded_specialized_balances/mod.rs`.

## The models

Defined in `packages/rs-dpp/src/voting/readiness/`, all versioned enums serialized with the platform serializer:

- `ReadinessRound` carries the contract, the round id, the bundle digest, the contract version the bundle activates, the preparation profile, when the bundle was accepted (committed block time and height), the status (`Pending` or `Crossed` with the crossing time and the activation deadline), the last evaluation mark (the core height of the membership view and the raw report count the round was last judged against), whether the fund ran short, the fund id and the payer. The round id is `hash_double(network_magic || contract_id || version || bundle_digest || accepted_at_height)`, so a replacement of the same bundle in a later block is a different round and a report signed for one network means nothing on another.
- `ReadinessReportRecord` is what is stored for one accepted report: the height it was accepted at and the profile it was compiled against. The count tree's own count is the raw number of distinct reporters; the record is there so the block event can judge a report later without the transition.
- `ReadinessScanCursor` is the persisted position of a paged walk over a round's reports, bound to the membership view (core height and eligible count) the walk started under.
- `ReadinessPayer` is the party the unused fund is refunded to. Only an identity can pay today; the contract-bucket owner is appended when typed storage flags land.

## Rounds

Opening a round (`open_readiness_round_operations`) writes the pointer, creates the round tree with its record and an empty reports count tree, creates the fund, and when a round was current retires it. Everything happens in one batch that the pinned GroveDB's consistency check accepts: every operation is on a distinct path and key, and no insert sits below a delete. An opening whose round id already has a tree under the contract is refused, whether that round is current or retired and awaiting cleanup: recreating the tree would leave the new round queued for cleanup.

Retiring a round (`retire_readiness_round_operations`, shared by replacement, cancellation and activation) never opens the reports tree. It queues the round under `[112, r, 3]`, drops the deadline entry when the round had crossed (with its per-time tree when no other round shares that time), empties the fund, charges the cleanup reserve to the epoch's processing pool and reports the remainder owed to the payer. The refund is returned rather than written because one batch may both refund the old payer and debit the new funding on the same identity, and two absolute balance writes on one key in one batch collapse; when the same identity funded both rounds the opening nets them into a single write, and otherwise it debits the new payer the full funding and credits the old payer its own refund. For the same reason the pool credit is an absolute rewrite of the epoch's processing pool item, so one retirement per applied batch is the contract of the helper.

An estimate reads no state, so opening and cancellation price the largest shape they can meet: a crossed round with a deadline entry and a time tree to remove, a fund to settle, the whole cleanup reserve credited to the pool, and a refund written to the retired round's payer as its own balance write.

Cancellation (`cancel_readiness_round_operations`) deletes the pointer and retires the round. Activation (`activate_readiness_round_operations`) does the same for a round that has crossed, is still the contract's current round and whose deadline the block has reached; routing the activated bundle into the contract's method tables is the caller's hook and lives outside Drive. In both cases the remainder is credited to the payer as unused preparation funding. That credit can repay the payer's debt, and only `apply_drive_operations` routes a repaid debt to the processing pool, so a block applies all three settlements as `ReadinessOperationType` batch operations (`OpenRound`, `CancelRound`, `ActivateRound`). It refuses a batch holding two of them, or one beside another identity balance or readiness fund write, which the absolute settlement writes would overwrite.

## Reports

`insert_readiness_report_operations` inserts one report into the current round's count tree if absent and says whether it was new. A retransmitted report is not new and writes nothing, so the count tree's count is the number of distinct reporters. The count is not an eligibility proof: the block event validates the reporters against the block's membership view before a crossing, and `prune_readiness_reports_operations` deletes the ones found ineligible.

`fetch_readiness_reports_page_operations` returns reports in key order, continuing after a given key, so the walk can be paged across blocks; `fetch_readiness_round_raw_count_operations` reads the count tree's count.

## Cursors, crossings and deadlines

A paged walk that does not finish in one block persists a `ReadinessScanCursor` under the round; a block whose membership view differs from the cursor's discards it and restarts, so a crossing is only ever committed from a walk completed against one coherent view. `update_readiness_round_evaluation_operations` rewrites the record with the last evaluation mark; a round whose mark matches the block's view and raw count is skipped at the cost of one read, and one whose mark differs is reconsidered, which is what makes a membership change reconsider every round even when the backlog spans several blocks.

`record_readiness_crossing_operations` marks the round crossed at the block's committed time with a deadline of `crossing_ms + clamp(crossing_ms - accepted_at_ms, 2 minutes, 1 hour)` (the bounds are `readiness_additional_wait_min_ms` and `readiness_additional_wait_max_ms` in `SystemLimits`) and queues the deadline under `[112, r, 1, encode_u64(deadline)]`, the same shape as the contested vote poll end-date queue. `fetch_readiness_rounds_due_operations` reads the entries at or before a time, oldest first; an entry whose round id is no longer the contract's current round is stale (the round was replaced or cancelled during the wait) and is dropped without activation. Retirement removes a crossed round's entry, and the per-time tree once it is empty: the due query walks the time keys under a limit and an empty time tree still spends it, so emptied trees ahead of a live deadline would hide that deadline from every block.

## Cleanup

`cleanup_retired_readiness_round_operations` runs one bounded step over the first queued retired round: it deletes up to a caller-given number of reports through the limited path-query delete and, once none remain, the record, the cursor and the now-empty count tree, then the round tree and, when no live round remains, the contract tree, and finally the queue entry. A round with more reports than the bound drains over several steps. The retired subtree has been unreachable since retirement, so the deferred deletion is invisible to every consensus rule; it only reclaims storage, paid by the cleanup reserve charged at retirement.

## Funds

A readiness fund is one sum item per round under `[40, 129]`, keyed by `hash_double("dashvm-readiness-fund-v1" || round_id)`. The five methods (`add_readiness_fund_operations`, `deduct_from_readiness_fund_operations`, `empty_readiness_fund_operations`, `fetch_readiness_fund`, `prove_readiness_fund`) are copies of their voting siblings on `[40, 128]`; the two families stay separate rather than sharing a path flag so the shipped voting generations remain byte-identical. Deductions keep a reserve untouchable so a round can always pay for its own deferred cleanup. Because the root `40` is a sum tree, readiness funds are inside the credit conservation check for free.

## Genesis and upgrade

`add_initial_vote_tree_main_structure_operations` generation 1 creates `[112, r]` with its children and `[40, 129]` at genesis. It is the vote setup that creates the fund tree, not the prefunded balances helper, because that helper is unversioned and genesis builds every lower layer in one batch. On a chain upgrading to protocol version 17, generation 3 of the protocol change hook creates the same elements with insert-if-not-exists through the same helper, and a test pins that a node born at 17 and a node upgraded from 16 hold byte-identical subtrees.

## Proofs

Three verifiers under `packages/rs-drive/src/verify/voting/` compile with the `verify` feature alone: `verify_readiness_round` (the pointer, the record and the raw count, from a merged proof the prover builds with the same queries), `verify_readiness_report` (one report by contract, round and reporter) and `verify_readiness_fund`. A query endpoint over them is client work and arrives separately.
