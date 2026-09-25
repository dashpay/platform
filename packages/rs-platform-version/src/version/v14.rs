use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v4::DOCUMENT_VERSIONS_V4;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v3::DPP_METHOD_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v2::STATE_TRANSITION_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_versions::v4::STATE_TRANSITION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_token_versions::v3::TOKEN_VERSIONS_V3;
use crate::version::dpp_versions::dpp_validation_versions::v5::DPP_VALIDATION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_query_versions::v3::DRIVE_ABCI_QUERY_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v2::DRIVE_ABCI_STRUCTURE_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v3::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::fee::v3::FEE_VERSION3;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v3::SYSTEM_DATA_CONTRACT_VERSIONS_V3;
use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_14: ProtocolVersion = 14;

/// v14 hosts six consensus changes:
///
/// 1. **Contract-level ranked aggregates**: an index can
///    declare that its groups are rankable by an aggregate, so a query like
///    "top 5 restaurants by average grade" is served from an ordered
///    secondary tree in O(log n + k) with a proof, instead of being rejected.
/// 2. **The shared-prefix aggregate index fix**: a data contract declaring
///    an aggregating (countable / summable) index that terminates at a
///    property which is also the prefix of a compound index (e.g. summable
///    `[a]` next to `[a, b]`) registered successfully but rejected every
///    document insert for most flag combinations, because Drive could not
///    legally hang the compound continuation tree under the aggregating
///    per-value tree. The v2 document index walkers (plus the v1 update
///    walker) that fix it gate here as well: tree types derive through a
///    shared continuation-demotion helper (provable count-bearing value
///    trees with compound continuations demote to `CountSumTree`, since
///    grovedb rejects count-suppressed children under provable count
///    parents by design) and continuation inserts route through the
///    completed zero-contribution wrapper matrix. No state migration is
///    needed: shapes without compound continuations produce bit-identical
///    operations, the broken shapes could never hold documents, and the
///    one previously-insertable shape the demotion changes (a provable
///    count-bearing value tree whose continuations were all sum-bearing —
///    insertable pre-v14 only through an unenforced grovedb batch guard)
///    simply gets `CountSumTree` value trees for values first seen at
///    v14+, which readers treat identically.
/// 3. **The contested vote poll index cross-check**: the index named by a
///    document create transition's prefunded voting balance keys the vote
///    poll, its stored info, its end-date entry and its prefunded
///    specialized balance, while the contested index the contender is
///    inserted under always comes from the document type. Up to v13 nothing
///    tied the two together, so a submitter could register and fund a
///    contest under a vote poll describing a different index than the one
///    the contest was created on — which halts the chain when that poll
///    ends — or open a contest for a document that is not a contested
///    resource at all. State validation also prevents a non-contested create
///    from occupying a live contested document's id before the contest winner
///    is awarded into primary storage. Drive's contested insert also recreates
///    an abstain or lock vote tree over the storage an earlier poll's cleanup
///    left orphaned (it only removed the trees that received votes), so a
///    resource can be contested again instead of failing with
///    `CorruptedContractIndexes`.
/// 4. **Relative daily withdrawal limit**: the flat 2000 Dash per 24 hours that
///    applied from v8 becomes 15% of the total credits Platform held a day ago
///    (`SYSTEM_LIMITS_V4.daily_withdrawal_limit_percent`, read by
///    `daily_withdrawal_limit` v2 through `DPP_METHOD_VERSIONS_V3`), never below
///    one maximal withdrawal (`max_withdrawal_amount`) so every accepted
///    withdrawal eventually fits and cannot block the pooling queue. The base is
///    capped at `max_daily_withdrawal_amount` (4000 Dash, Core's unlock capacity
///    per day under V24 as written); the credit inflows of the active window —
///    every credit mint, recorded per block by
///    `record_credit_inflows_for_withdrawals` in the credit inflows sum tree —
///    are added after the cap, so the limit counts net outflow and a matching
///    deposit -> withdraw cycle does not consume the capped budget of other
///    users (#4471). Outflow funded by same-window deposits may therefore
///    exceed the cap; this mirrors the net credit-pool rule Core adopts for V24
///    alongside this change (tracked in #4471), which must land before V24
///    activates. Both the inflows and the pooled reservations count over the
///    interval after the base snapshot only — an entry the snapshot already
///    reflects is neither added nor subtracted again. The base is
///    the total credits recorded at the latest block at least 24 hours before
///    the current one: `DRIVE_ABCI_METHOD_VERSIONS_V10` turns on
///    `record_total_credits_history_for_withdrawals`, which checks the total
///    credits every block once fees and epoch rewards are in, writes it under
///    the withdrawals tree keyed by block time whenever it changed (an entry
///    describes the total until the next one) and prunes entries older than the
///    one the limit reads, and `DRIVE_VERSION_V9`'s identity withdrawal table
///    bumps `calculate_current_withdrawal_limit` to 1 to read that lagged
///    value. Until an entry is a day old — the first day after activation — the
///    flat 2000 Dash keeps applying, so the lag cannot be skipped by inflating
///    the total before or at activation. The lag is the guardrail: a sudden
///    jump in the total credits does not raise the limit for a day. Amounts
///    already pooled in the last 24 hours keep counting against the maximum
///    exactly as before. Pre-V24 Core caps unlocks at `LimitAmountV22` (2000
///    Dash) per *block*, with the amount checked only at block level, so any
///    daily total is still minable across blocks; V24's 4000 Dash per 576-block
///    window matches the capped base and is raised to the same net rule before
///    activation (see above).
/// 5. **Time-range indexes**: an index can declare a `timeRange` transform
///    that buckets a required system timestamp (`$createdAt` /
///    `$updatedAt` / `$transferredAt`) into fixed-length, regularly-spaced,
///    optionally overlapping windows declared in seconds (`range` / `step`,
///    plus an optional `phase < step` alignment offset). Each grid gets its
///    own index subtree — the level is keyed by the property name qualified
///    with the grid — so several grids may bucket one timestamp side by
///    side. A document is stored once per containing bucket per grid (the
///    v2 insert/delete and v1 update walkers carry the fan-out; the
///    per-document write amplification is capped per index by
///    `SystemLimits::max_time_range_overlap_factor`), and the v1
///    `getDocuments` handler resolves the new `IN_TIME_RANGE` operator —
///    a typed `TimeRangeSelection` operand: `NEWEST`/`OLDEST` (resolved to
///    a bucket-start equality from committed block time) or `BY_START`
///    (naming any window, current or historic, by its grid-aligned start),
///    with a `grid` member naming one grid where several bucket the field
///    — making trending/leaderboard document and count/sum/avg queries
///    provable over the current or any named window. `unique: true` is
///    admitted only for non-overlapping windows (`range == step`) sourced
///    from the immutable `$createdAt`.
/// 6. **Deterministic token reward math**: `DistributionFunction::evaluate`
///    (logarithmic, inverted-logarithmic, exponential and polynomial perpetual
///    distributions) computes `ln`/`exp`/`pow` through the pinned pure-Rust
///    `libm` crate instead of the platform C library. musl's `log` takes an
///    FMA path on aarch64 and a non-FMA path on x86_64, so the two disagree by
///    1 ulp on some inputs; a contract owner could pick parameters whose reward
///    sat within that ulp of an integer, and `floor` then minted different
///    amounts on the two architectures, splitting the app hash both at claim
///    time and at contract registration (validation evaluates the start
///    value). Gated on `distribution_function_evaluate_version` so both
///    architectures switch at the same height; pre-v14 blocks replay on the
///    old math byte-for-byte. `log`/`exp` have no architecture dispatch and
///    `pow`'s only arch-touching call is the correctly-rounded `sqrt`, so the
///    result is bit-identical on every target Platform builds for. The goal
///    is determinism, not correct rounding: on a boundary tuple the host
///    libm (glibc, macOS) can still be 1 ulp away, so anything predicting
///    rewards with host math may differ from consensus by one unit.
///
/// The first two are orthogonal by construction: the ranked upgrade decides the
/// *property-name* tree type, the demotion decides the *value* tree type
/// one level below it, and a demoted `CountSumTree` value tree contributes
/// its (count, sum) to a ranked indexed parent exactly as the provable
/// variant did — so ranked secondaries keep ranking correctly over
/// shared-prefix shapes.
///
/// Until a contract uses the ranked or time-range grammar, the only v14
/// behavior changes are the shared-prefix fix, the contested-index
/// cross-check, the index-reorder schema-compatibility fix and the relative
/// daily withdrawal limit; everything else matches v13:
///
/// * `CONTRACT_VERSIONS_V6` points `document_type_schema` at the v3 document
///   meta-schema, which hosts the ranked index keywords
///   (`rankedCountable` / `rankedSummable` / `rankedAverageable`), the
///   `refersTo` reference keyword and the `timeRange` index transform. v13
///   keeps validating against meta-schema v2, where those keys are rejected
///   as unknown properties, so a pre-v14 contract cannot smuggle them in.
///   It also bumps `validate_schema_compatibility` to 1, which strips the
///   top-level `indices` key before diffing the old and new document type
///   schemas: index immutability is enforced by `validate_update` v1's
///   name-keyed comparison, so a contract update that merely reorders the
///   `indices` array validates cleanly instead of hitting the
///   unsupported-keyword hard error (an internal error under v13).
/// * `DRIVE_VERSION_V9` carries `DRIVE_DOCUMENT_METHOD_VERSIONS_V4`, adding
///   the `detect_ranked_mode` routing slot, plus the grove-method slots for
///   creating the three indexed tree variants and the verify-method slot for
///   `verify_ranked_top_k_proof`. All are 0 today. The same table bumps the
///   four index walkers to v2 and the document update walker to v1 for the
///   shared-prefix fix; those same walker versions carry the time-range
///   bucket fan-out, so both features gate on one table entry. It also sets
///   `insert_contested.fetch_charter_election_windows` to `Some(0)`: a
///   moderation election (an `electedCharter` contest) runs on its target
///   contract's join and vote windows, which the document create join check
///   and the contested insert read.
/// * `DRIVE_ABCI_QUERY_VERSIONS_V3` bumps
///   `document_query_helpers.compute_aggregate_mode_and_check_limit` 0 → 2,
///   opening two routes on the v1 document-query handler: the ranked path
///   (a grouped aggregate whose single `order_by` names the selected
///   aggregate — `ORDER BY <agg> [ASC|DESC] LIMIT n [OFFSET m]`) and the
///   boolean-`HAVING` range path (a grouped aggregate carrying exactly one
///   `having` clause on the selected aggregate — `GROUP BY p HAVING <agg>
///   <op> <value> LIMIT n`), the latter served as a value-bounded range
///   read of the covering ranked index's axis secondary. v13 and earlier
///   keep the v1 table and therefore keep rejecting both shapes, so
///   mixed-version networks agree across the upgrade.
/// * `DRIVE_ABCI_VALIDATION_VERSIONS_V10` bumps
///   `document_create_transition_structure_validation` 0 → 1, requiring a
///   contested create transition's prefunded voting balance to name the
///   same vote poll the document itself resolves to, and rejecting one on a
///   document that resolves to no contested index. It also bumps document
///   create state validation to 2, enforcing `refersTo` document references
///   and rejecting a non-contested create whose id is already present in the
///   contested tree. Document replace state validation 1 enforces the same
///   reference checks, re-validates a `refersTo: deletableDocument`
///   reference on every replace (a dead one must be repointed or cleared),
///   and lets an `immutable` one be cleared once its target is deleted.
///   Document create structure validation 1 and replace structure
///   validation 0 (extended in place) refuse a `distinctFrom` identifier
///   property equal to the value it must differ from
///   (`DocumentPropertyNotDistinctError`, 10419); transfer and purchase
///   structure validation 0, extended in place, judge the stored document's
///   `$ownerId` declarations against the new owner.
///   v13 keeps the v9 table and therefore keeps accepting all of these, so
///   replay of pre-upgrade blocks is unchanged.
/// * `DRIVE_ABCI_VALIDATION_VERSIONS_V10` also bumps the identity create from
///   addresses `advanced_structure` 0 → 1: a key whose proof of possession fails
///   is refused unpaid instead of charging the inputs a penalty, since the
///   address witnesses do not sign those proofs. v13 keeps the paid refusal of v0.
/// * `DOCUMENT_VERSIONS_V4` bumps `document_serialization_version` to
///   default 3: documents are stamped with the contract version their bytes
///   conform to (a varint after the format prefix), enabling the
///   `requiredSince` property keyword — a contract update may add a new
///   required property annotated with the version that update creates.
///   Documents stamped below a property's `requiredSince` keep the
///   presence-flagged layout they were written with, so the latest contract
///   alone reconstructs every stamp's layout and no historical contract
///   lookups are ever needed. Reads dispatch on the byte prefix, so
///   formats 0–2 (all pre-v14 documents) deserialize exactly as before with
///   an unstamped (pre-annotation) layout.
/// 7. **Client-side GroveDB proof envelope floor**:
///    `SYSTEM_LIMITS_V4.minimum_grovedb_proof_envelope_version` becomes 1, so
///    a client verifying with v14 tables rejects the legacy V0 proof
///    envelope before its bytes reach Drive (`drive-proof-verifier`,
///    `wasm-drive-verify`, and the nested compacted address proofs). V0's
///    item binding lets a prover return different item bytes under the same
///    authenticated root; every live network has emitted V1 envelopes since
///    v13 (grove version 3), so no honest response is affected.
/// 8. **Epoch-based perpetual distribution claims stop wrapping**:
///    `RewardDistributionType::max_cycle_moment` (the cap on how far one claim
///    may redeem, selected by
///    `TOKEN_VERSIONS_V3.reward_distribution_max_cycle_moment_version` 1)
///    computes `start + interval * cycles` in `u64` with saturating
///    arithmetic and narrows back to `EpochIndex` only after capping at the
///    last completed cycle moment (`current cycle moment - interval`, the
///    previous epoch for an interval of one as before; for wider intervals the
///    same cycles are paid, but the cap now sits on a cycle boundary, the only
///    shape in which `evaluate_interval`'s fixed-amount step count and its
///    per-cycle loop agree). Up to v13 the sum was taken in `u16`: a
///    fixed-amount function allows 32,767 cycles, so any epoch interval of
///    three or more with a nonzero start (or two with a start at epoch two
///    or later) pushed the cap past `u16::MAX`. Release builds wrap, the cap landed below the
///    start, `evaluate_interval` saw an empty range and the claim was
///    refused with `InvalidTokenClaimNoCurrentRewards` on every attempt. The
///    v0 arithmetic is kept, wrapping explicitly, so those refusals replay.
/// 9. **Evonode reward cycles weighted by the epochs they span**: the
///    per-cycle evaluator in `DistributionFunction::evaluate_interval` asks
///    the participation ratio for the epochs a cycle covers
///    (`TOKEN_VERSIONS_V3.distribution_function_cycle_epochs_version` 1:
///    `cycle moment - interval + 1 ..= cycle moment`). Up to v13 it passed the
///    cycle's step index as if it were an epoch, which coincides only for an
///    interval of one; for a wider interval it named epochs before the
///    distribution started, outside the epoch window the claim loads, and an
///    `EvonodesByParticipation` claim with a function other than a fixed
///    amount failed as an internal error (reachable only once item 8 let the
///    cap stop wrapping). Interval-one distributions are unchanged.
/// 10. **A zero epoch interval is rejected at registration**:
///     `RewardDistributionType::validate_structure_interval` v1
///     (`CONTRACT_VERSIONS_V6.token_versions.validate_structure_interval`)
///     refuses an `EpochBasedDistribution` with `interval: 0` with the new
///     `InvalidTokenDistributionEpochIntervalTooShortError` (code 10828) on
///     contract create and update. Up to v13 the epoch arm enforced nothing,
///     so such a contract registered and every claim on it failed as an
///     internal error, since no cycle can be computed from a zero step. Block
///     and time minimums are unchanged.
/// 11. **Gas paid by the contract owner**: a token-paid document action's
///     `gasFeesPaidBy` (offered by the document type's token cost, asked for by
///     the transition's `$tokenPaymentInfo`) is acted on. Both values were
///     carried but ignored up to v13, where the signer always paid. Batch
///     transform v2 resolves one payer for the batch (`GasFeesPaidBy::resolve`)
///     and reads the contract owner's balance into the action; batch advanced
///     structure v1 refuses a request the document type does not offer
///     (`GasFeesPaidByNotAllowedError`, 40129) or a batch naming two payers
///     (`InconsistentGasFeesPaidByInBatchError`, 40130); `validate_fees_of_event`
///     v1 judges the fee against the sponsor's balance, refusing an insisting
///     batch unpaid when it falls short (`GasSponsorInsufficientBalanceError`,
///     40222) and handing a preferring one back to the signer; `execute_event`
///     v1 charges whoever was admitted. The batch's signer only funds the
///     principal, and its minimum balance pre-check v1
///     (`identity_minimum_balance_pre_check`) asks no more of a batch that
///     requests sponsorship. A failed batch is never sponsored, so check tx
///     validates the state of a sponsored batch whose signer is under the fee
///     minimum in full, on the first check and on every recheck (mempool
///     policy, not consensus).
/// 12. **Optional token costs**: a document type's token cost may declare
///     `optional: true` (v3 meta-schema). A transition that leaves
///     `$tokenPaymentInfo` out then pays no token and its signer pays the gas
///     in credits, as on an action without a token cost (the base action
///     transformer waives the cost, and no sponsorship applies). With the
///     payment info present the token is charged exactly as for a required
///     cost, and too small a token balance stays a rejection. Contracts up to
///     v13 cannot carry the flag, so the waiver is inert before this version.
/// 13. **Pre-programmed distribution amounts are bounded**:
///     `TokenPreProgrammedDistribution::validate_amounts` rejects a release
///     whose amounts total more than `i64::MAX` with the new
///     `PreProgrammedDistributionAmountOverLimitError` (code 10277). It runs
///     on contract create (`DRIVE_ABCI_VALIDATION_VERSIONS_V10`'s create
///     `basic_structure` 2) and, for the tokens an update adds, on contract
///     update (`CONTRACT_VERSIONS_V6`'s `validate_update` 1). A release is
///     stored as a sum tree, so up to v13 such a create passed validation and
///     failed inside Drive as an internal error: never paid for, and stripped
///     from every proposal. An update failed the same way on a single amount
///     over the limit (its fee estimation takes the insert path), but was
///     accepted when only the total overflowed, since it wrote no distribution
///     storage; from v14 it writes it (`update_contract` 2) and would fail.
///     Tokens a contract already has are not judged, so a contract holding
///     such a token stays updatable.
/// 14. **Tokens of one contract sharing a pre-programmed release time**:
///     `DRIVE_TOKEN_METHOD_VERSIONS_V2` bumps
///     `add_pre_programmed_distributions` to 1, which queues the release-time
///     tree the tokens share once instead of once per token. Queued twice,
///     the batch is refused as an internal error by a node with
///     `batching_consistency_verification` on; the default is off, and there
///     GroveDB folds the identical inserts, so the stored state is unchanged
///     and only the processing fee drops, by the existence read the later
///     tokens no longer make.
///
/// 15. **Tokens added by a contract update are set up like registered ones**:
///     `update_contract` v2 (`DRIVE_CONTRACT_METHOD_VERSIONS_V4`) creates the
///     perpetual, pre-programmed and once-per-identity distribution storage
///     of a token the update adds, and mints its base supply to the token's
///     `newTokensDestinationIdentity`, or to the contract owner without one,
///     with the total supply starting at the base supply. v1 did neither: a
///     claim on such a token failed as an internal error, and the token sat at
///     a total supply of zero with nobody holding any of it. Nothing is minted
///     retroactively for a token an update added under an earlier version.
///
/// 16. **Contract moderation**: a data contract may declare, in its config,
///     a banlist and/or a suspension list of identities and who edits them
///     (the owner, or the owner and up to `SystemLimits::max_contract_moderators`
///     named identities, each of which must exist). `CONTRACT_VERSIONS_V6`
///     makes config V2 the config of every new contract (`max_version` and
///     `default_current_version` 2), which carries
///     the declaration; a contract create or update carrying a V2 config is
///     inactive before this version (`StateTransition::active_version_range`).
///     `DPP_VALIDATION_VERSIONS_V5.validate_config_update = 2` fixes the lists
///     a contract keeps at its creation: an update turns none on and none off,
///     and may only change the moderators.
///     `ContractUserModeration` (state transition type 24, gated by
///     `CONTRACT_USER_MODERATION_INITIAL_PROTOCOL_VERSION`) bans, unbans,
///     suspends until a block time (at most
///     `SystemLimits::max_contract_suspension_until`) and unsuspends one
///     identity, signed by the owner or a moderator with a CRITICAL key; a
///     ban and a suspension carry a reason, stored with the entry: a text of
///     at most `SystemLimits::max_contract_moderation_reason_length` bytes,
///     an optional code nothing checks, reserved for ban codes a contract may
///     declare in a later version, and up to
///     `SystemLimits::max_contract_moderation_reason_documents` documents the
///     reason is about, named by type and id and not looked up. A contract may also keep a warning list
///     (`[64, contract, 2] / 224`): a warn appends a warning, the block time and
///     a reason, to the identity's entry, at most
///     `SystemLimits::max_contract_warnings_per_identity` at a time, and a
///     clearWarnings deletes the entry; warnings bar nothing and are what a
///     status query and the identity's clients read;
///     `DRIVE_ABCI_VALIDATION_VERSIONS_V10` turns its gates on, moves the
///     contract update's basic structure to 2 and the contract create and
///     update state validation (already 1 here) checks the named moderators.
///     `batch_state_transition.contract_moderation_gate = Some(0)` makes the
///     batch transformer refuse, paid, the document transitions of a banned or
///     suspended signer, its deletions excepted, and collect a lapsed
///     suspension, which
///     `documents_batch_transition` 1 (`DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4`)
///     deletes when the batch executes; the same field gates the other
///     party of a transfer or a purchase, so a barred identity neither
///     receives nor sells a document. Token transitions are not gated.
///     `DRIVE_CONTRACT_METHOD_VERSIONS_V4` bumps `insert_contract` to 2,
///     which creates the list trees (`[64, contract, 2] / 128`, `/ 192` and `/ 224`, inside the contract's other tree), and
///     adds the `moderation` method table; the verify and
///     query tables gain the status and entries methods.
///
/// 17. **Document action fees and the contract fee claim**: a document type
///     may charge a fixed fee in credits for an action on one of its documents
///     (the `actionFees` keyword of the v3 document meta-schema, read by
///     `try_from_schema` 3), split between the contract's owner pot and its
///     moderators pot and priced as written or scaled by the fee multiplier of
///     the epoch. The fees of a document type never change (document type
///     `validate_update` 1), and a `moderators` part needs declared moderation
///     (contract create and update basic structure 2). Whoever pays the gas
///     pays the fee, the contract owner never into their own owner pot, and
///     only for an action that executes: `validate_fees_of_event` 1 and
///     `execute_event` 1 (`DRIVE_ABCI_METHOD_VERSIONS_V10`) settle the payer
///     and move the credits with the batch's own operations, outside the fee;
///     `apply_drive_operations` 1 merges every write of one identity balance,
///     fee pot or prefunded specialized balance in a batch into one, so a fee
///     leaving the balance a purchase price or a voting fund also leaves takes
///     both, and refuses a batch writing one token balance or supply twice.
///     Fee validation estimates for the payer it settles on and hands that
///     payer to `execute_event` 1.
///     `DRIVE_CONTRACT_METHOD_VERSIONS_V4` gains the `fee_pots` method table:
///     the pots are sum items under two sum trees of the prefunded specialized
///     balances (`[40, 64]` and `[40, 192]`), which `create_initial_state_structure`
///     4 and the upgrade to this version create, so they stay inside the total
///     credits the platform checks every block, and the epoch each pot was
///     last claimed in is an item of the contract's other tree (`32` and `96`).
///     `ContractFeeClaim` (state transition type 25, gated by
///     `CONTRACT_FEE_CLAIM_INITIAL_PROTOCOL_VERSION`) pays a pot out, at most
///     once per epoch each: the owner pot to the contract owner, the moderators
///     pot in equal shares to the moderation team, what the split leaves over
///     staying in the pot. `DRIVE_ABCI_VALIDATION_VERSIONS_V10` turns its gates
///     on, `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4` adds its converter, and
///     the verify table gains `verify_contract_fee_pots`.
/// 18. **Document ids commit to the identity contract nonce**: up to v13 a new
///     document's id hashed the contract, owner, document type and the entropy
///     of the create transition, and the create check only asks whether a
///     document exists under the id right now. The owner of a deleted
///     document could therefore create another one under the same id by
///     reusing the entropy, with different content, and everything that
///     referenced the id (a `refersTo` property, a like, a moderation removal
///     record) then pointed at the new content, which defeats
///     `documentsMutable: false` for a deletable document type.
///     `DOCUMENT_VERSIONS_V4` sets `generate_document_id` to 1: the id also
///     hashes a domain tag and the identity contract nonce of the create
///     transition, which is consumed at most once, so an id can be produced
///     at most once. The entropy stays in the hash (ids remain
///     unpredictable) and on the wire (the transition format is unchanged);
///     batch advanced structure validation 1, which only this version
///     selects, recomputes the id through `Document::generate_document_id`
///     and bills both passes of the double SHA-256 by the real preimage
///     length (4 blocks for most document type names) where v13 bills a
///     flat 2.
///     Ids of documents created before the upgrade can not be produced by
///     the new derivation either. A client that still derives the entropy
///     only id has every create rejected with
///     `InvalidDocumentTransitionIdError`. Every create path of the clients
///     in this repository derives through `Document::generate_document_id`:
///     `DocumentCreateTransitionV0::from_document` for dpp, rs-sdk and the
///     bindings built on them, and in wasm-dpp2 the `DocumentCreateTransition`
///     constructor (which also writes the id back onto the JavaScript
///     `Document`), `Document.generateId` with its `identityContractNonce`
///     argument, `setIdForCreation` and the `identityContractNonce`
///     constructor option.
/// 19. **Document deletion by moderators**: a document type of a contract
///     that declares moderation may set `canBeDeletedByModerators` (meta-schema
///     v3, fixed when the type is created, refused on a type that keeps
///     history, is indexOnly or restricts creation; for references such a type
///     is deletable, so a permanentDocument reference refuses it and a
///     deletableDocument reference accepts it). A moderation declaration may then keep
///     no list at all. `ContractUserModeration` gains the `DeleteDocument`
///     action: the owner or a moderator deletes a document of such a type,
///     except the owner's and the moderators' own, with a reason like a
///     ban's. The deletion leaves a record under the contract
///     (`[64, contract, 2] / 16 / <document type> / <document id>`: the
///     document's owner, the moderator, the block time and the reason), paid
///     for by the moderator and never deleted; `insert_contract` 2 creates
///     the records tree of each such document type, `update_contract` 2 the
///     tree of one an update adds, and either the tree above them with the
///     contract's first. The deleted document's
///     owner gets no storage refund: `apply_drive_operations = 1`
///     (`DRIVE_VERSION_V9`) attributes the removal of a batch that carries
///     the forfeiture to nobody, so the credits stay in the storage pools.
///     A record is final, since a document id is produced at most once (18).
///     Neither the type's deletion token cost nor its `actionFees` deletion
///     fee is charged.
///     The moderation method table, the verify table and the query table gain
///     the document removal methods (`getContractDocumentRemovals`).
///     `canBeDeletedByModeratorsFor` bounds the deletion in time: so many
///     seconds after a document's last modification (`$updatedAt`, or
///     `$createdAt` on a type whose documents never change; the type must
///     require its clock), past which no moderator deletes it, the
///     contract owner included (`DocumentModerationWindowElapsedError`); a
///     document's own owner still deletes it as `canBeDeleted` allows. A
///     replace opens the window again. Fixed with the type, like the flag.
///
/// 20. **Document transitions agree to their action fee**: version 2 of the
///     document base transition, the default from this version
///     (`STATE_TRANSITION_SERIALIZATION_VERSIONS_V3`) and inactive before it
///     (`StateTransition::active_version_range`, since earlier software
///     cannot decode it), carries an action fee agreement: the owner and moderators amounts the signer saw declared,
///     which must match the document type's exactly, and for a fee priced by
///     the fee multiplier the multiplier they knew with the increase, in
///     percent, they accept. Batch advanced structure 1
///     (`DRIVE_ABCI_VALIDATION_VERSIONS_V10`) refuses, as a paid nonce bump
///     that charges no fee, an action that charges a fee without an agreement
///     (40132), with one to other amounts or another pricing (40133), or
///     whose epoch's multiplier rose beyond the tolerance (40134), so a
///     contract whose fees change cannot make a signed transition pay them.
///     Check tx judges the agreements again on every recheck, off the action
///     the transformer rebuilt with the contract and the multiplier as they
///     are then, so a batch a block would refuse leaves the mempool instead
///     of failing there (mempool policy, not consensus).
///
/// 21. **Document restore by moderators**: the removal record a moderator's
///     deletion leaves (19) also holds a double SHA-256 of the document as
///     serialized under its type at the deletion (`ContractDocumentRemoval::
///     document_hash`), and `ContractUserModeration` gains the
///     `RestoreDocument` action: the owner or any current moderator brings
///     the document back, as it was, within
///     `SystemLimits::contract_document_restore_window_ms` (a week) of the
///     removal. The bytes must decode under the type and hash to what the
///     record holds; refused otherwise, or without a record (41119), past the
///     window (41120), on a hash mismatch (41121), once restored (41122), or
///     when another document took a value of one of the type's unique indexes
///     meanwhile (40105). The document goes back through the ordinary insert,
///     its storage flags naming its owner (the signer pays, the owner keeps
///     the refund of a later deletion), and the record is marked restored in
///     place (`ContractDocumentRemoval::restoration`: who, when) rather than
///     deleted; a restored document deleted again gets a fresh record in place
///     of the marked one, which the deletion transform reads to know. Neither
///     the type's creation token cost nor its `actionFees` creation fee is
///     charged, and no fee agreement is asked. `canBeDeletedByModerators` is
///     now also refused on a type with a contested index, whose deletions
///     could never be undone. The record grows on the wire
///     (`getContractDocumentRemovals`: `document_hash`, `restoration`).
///
/// 22. **Elected moderation teams, the declaration and the interim**: a data
///     contract may declare, when it is created, that its moderators are a team
///     elected by masternodes and evonodes (`ContractModerators::Elected`, a third kind
///     beside the owner and an appointed set, in the same config V2). The
///     declaration is frozen: the join and vote windows (one day to four weeks,
///     one week by default), in seconds and bounded by `SYSTEM_LIMITS_V4`;
///     whether the seat can be contested again once a team is seated
///     (`seatContestable`, required with no default), and for a contestable
///     seat the challenge cool-down (`challengeCoolDown`, in seconds, two weeks
///     to three years, refused on a seat that can not be contested; in Rust
///     one `Option<u32>`), which nothing reads until challenges come after
///     this version, a seat never being contested again here; an optional,
///     unbounded election delay in seconds after the contract's creation
///     before the first charter may be filed (`electionDelay`, read by the
///     `moderation: "electionOpen"` reference requirement of item 24); how many
///     members a seated team's leader may add after the election
///     (`maxAddedModerators`, 0 when left out, at most
///     `SYSTEM_LIMITS_V4.max_contract_moderation_added_moderators`, 15); the
///     document types the team moderates, each with the abilities the seated
///     team holds on it; who moderates until the first team is seated (the owner, an
///     appointed set, or nobody, with the moderated types not yet usable or
///     used unmoderated meanwhile); and whether the owner is protected from the
///     team. `validate_moderation_config` v0 checks
///     it against the contract's document types (10900), and
///     `validate_config_update` 2 refuses every change to it, and entering or
///     leaving elected moderation, with `DataContractConfigUpdateError`. The
///     interim moderators moderate and claim the pot as the merged kinds do;
///     with nobody named, nobody may claim the moderators pot, which
///     accumulates for the team to come, and with the types not yet usable
///     `contract_moderation_gate` v0 refuses, paid, every document transition
///     of a moderated type (`ContractModeratedDocumentTypeNotYetUsableError`,
///     41200) until a charter is seated (item 40).
///
/// 23. **Contested indexes without a Lock choice, and ties to the earliest
///     contender**: a contested unique index may declare `"resolution": 1`,
///     `ContestedIndexResolution::MasternodeVoteNoLocking` (meta-schema v3,
///     parser generation 3). Such a contest offers no Lock choice
///     (`VoteChoiceNotAllowedForVotePollError`, 40307, from `validate_state` 1
///     of the masternode vote) and always ends with a winner. Its end date is
///     the end of the join window until a second contender joins, when
///     `add_contested_document_for_contract_operations` 1 moves it to the full
///     poll duration, so a contest with a single contender is awarded without
///     the vote window. `check_for_ended_vote_polls` 1 awards a tie to the
///     **earliest** contender (creation time, block height, core height,
///     document id) for every resolution, where the shipped rule awarded the
///     latest; DPNS contests ending from this version on follow the new rule.
///
/// 24. **Contract references may require elected moderation, a minimum age, a
///     minimum time since the last update, an owner relation to the writer or
///     config flags of the referenced contract**: a `contract` `refersTo`
///     declaration may carry `contractRequirements`, what the referenced
///     contract must declare beyond existing, with `moderation: "elected"` or
///     `"electionOpen"` (elected, and the contract's own `electionDelay` since
///     its creation has passed, or it declares none),
///     `minimumAgeSeconds` (the contract's recorded creation time must be at
///     least that many seconds before the block time of the write),
///     `minimumSecondsSinceUpdate` (the same of the later of its creation and
///     last update times; a contract without a recorded creation time never
///     meets either), `owner` (`"self"`: the contract is owned by the
///     `$ownerId` of the referring document, `"other"`: by anyone else),
///     `readonly: true` (its config is read-only, so it can never be updated
///     again), `keepsHistory: true` (its config keeps history) and
///     `ownerProtected` (its elected moderation declaration protects the owner
///     from the team, or does not, as the value says; a contract without
///     elected moderation meets neither value) as the requirements
///     (meta-schema v3, `apply_property_reference` 0,
///     `ContractReferenceRequirements` on
///     `DocumentPropertyReferenceTarget::Contract`). The document reference
///     validation checks them against the contract it fetched for the
///     existence check and the write itself (its owner and block time), so
///     they cost no further read, and refuses the first unmet requirement with
///     `ReferencedContractRequirementNotMetError` (40135). A replace re-checks
///     them when it changes the reference. `owner` is judged against the
///     writer, which a transfer or a purchase changes without any write, so
///     on a document type whose documents can be transferred or traded a
///     declaration carrying it is re-checked, whole, on every replace, as a
///     `$ownerId` writer gate is: the new owner has to repoint the reference,
///     so registration refuses one held by an `immutable` property of such a
///     type. The other requirements are facts about the referenced contract and
///     never bring a reference back. A changed `contractRequirements` is an
///     incompatible schema change on update.
///
/// 25. **Typed arrays of scalars in document schemas**: a document property
///     may be `type: "array"` with an `items` element schema instead of
///     `byteArray` (meta-schema v3, `parse_typed_array` 0,
///     `DocumentPropertyType::TypedArray`). An element is an integer, a
///     number, a string, a boolean, a byte array or an identifier; objects
///     and arrays of arrays are refused. On the array `minItems` and
///     `maxItems` count elements, `maxItems` is required (with `minItems`
///     not above it) and at most `SYSTEM_LIMITS_V4.max_typed_array_items`
///     (1024), and `uniqueItems` refuses a document repeating an element. An
///     element's `enum` has members of the element type only (none on a byte
///     array or identifier element), and an integer element's `minimum` and
///     `maximum` are integers; the parser reads them so random documents stay
///     inside them. The array is stored inline, a varint element count followed by the
///     elements, each encoded exactly as a required scalar property of its
///     type: an identifier element is 32 raw bytes, an integer element takes
///     the width its bounds give it, a fixed-size byte array element is raw.
///     A contract update may not change how an element encodes
///     (`validate_update` 1). The array cannot be an index property or one
///     side of a `propertyAgreement`. Its identifier and byte array elements
///     are conversion paths (`find_identifier_and_binary_paths` 1). A byte array
///     refuses `items`, and an identifier (a byte array with the identifier
///     `contentMediaType`) now refuses `uniqueItems`, which would demand that
///     no byte repeat.
///
/// 26. **Distinct identifier properties**: the `distinctFrom` property
///     keyword (meta-schema v3, `apply_distinct_from` 0, `DistinctFrom` on
///     `DocumentProperty`) requires an identifier property's value to differ
///     from the value of a named property of the same document, or from the
///     document's `$ownerId`; on the `items` of a typed array of identifiers
///     it binds every element. A pure structure rule: document create
///     structure validation 1 and replace structure validation 0 call
///     `validate_distinct_from_properties` (`validate_distinct_from` 0) on the
///     transition's data and owner id after the schema validation, transfer
///     and purchase structure validation 0 call it on the stored document and
///     its new owner (the three generation-0 modules were extended in place:
///     the call is inert before this version, where no property carries the
///     keyword), and each refuses an equal pair with
///     `DocumentPropertyNotDistinctError` (10419); an absent named property
///     passes. The parser checks the target at contract
///     registration and update (it must exist, be an identifier and not be
///     the declaring property), and a changed `distinctFrom` is an
///     incompatible schema change on update.
///
/// 27. **`encryptedFor` on byte array properties**: a byte array property may
///     declare how its ciphertext was produced, so wallets read the recipe
///     from the contract instead of a side channel: `recipient` (an identifier
///     property of the same document type, or `$ownerId`), `recipientKey` and
///     `senderKey` (integer properties of the same type bounded to u32,
///     carrying key ids) and `scheme` (`ecdh-secp256k1-aes256-cbc`, the
///     dashpay contact request scheme: a 16-byte IV followed by AES-256-CBC
///     with PKCS7 padding under the ECDH shared key). Meta-schema v3 admits it
///     on byte arrays that are not identifiers, `apply_encrypted_for` 0 parses
///     it onto `DocumentProperty::encrypted_for` and checks the three named
///     properties exist with the right types at registration. Document create
///     structure validation 1 and replace structure validation 0 (extended in
///     place, inert before this version) call
///     `validate_encrypted_property_shapes` (`validate_encrypted_property_shapes`
///     0, `None` before this version) to check the ciphertext shape of every declared property a transition supplies,
///     at least the IV plus one block and a multiple of the block, and refuse
///     it with `InvalidEncryptedPropertyShapeError` (10420). Nothing else about
///     the ciphertext is verifiable on chain. A changed `encryptedFor` is an
///     incompatible schema change on update.
///
/// 28. **Property and document type names are word characters only**:
///     meta-schema v3 refuses `-` in a property name (top-level or nested,
///     and in the property paths of `refersTo` declarations) and generation
///     3 of the document type parser refuses it in a document type name,
///     under full validation. Every earlier meta-schema and generation
///     admitted `-`, which the dotted and `list[]` path syntax was never
///     written for; a census of every contract create and update on mainnet
///     and testnet (2026-09-23) found no name carrying one, so nothing stored
///     is affected. Stored contracts are read as they are.
///
/// 29. **Identity key references may require a purpose and a document type
///     bound**: an `identityPublicKey` `refersTo` declaration may carry
///     `keyRequirements`, what the referenced key must be beyond existing and
///     not being disabled, with `purpose` (the key's purpose, by its wire name,
///     any but `system`) and `boundTo` (the key's contract bounds must be
///     exactly the declaring contract and the named document type of it) as
///     the requirements (meta-schema v3, `apply_property_reference` 0,
///     `IdentityKeyReferenceRequirements` on
///     `DocumentPropertyReferenceTarget::IdentityPublicKey`).
///     `create_document_types_from_document_schemas` 1, edited in place (the
///     check is inert before this version, where no parsed reference carries
///     requirements), refuses a contract whose `boundTo` names a document type
///     it does not have or one no key of the required purpose can be bound
///     to, so the check never needs a second contract fetch and a declared
///     requirement can be met. The document reference validation
///     checks the requirements against the key it fetched for the existence
///     check, so they cost no further read, and refuses the first unmet one
///     with `ReferencedIdentityKeyRequirementNotMetError` (40136). A changed
///     `keyRequirements` is an incompatible schema change on update.
///
/// 30. **Key references on the key id property**: an `identityPublicKey`
///     `refersTo` declaration may sit on the key id property itself, an
///     integer with `minimum` 0 and `maximum` 4294967295 (a `KeyID` is a
///     `u32`), naming through `identityProperty` whose key the value is:
///     `"$ownerId"` (the writer), `"$creatorId"` (the document's creator,
///     only on a document type that records creator ids) or the path of an
///     identifier property of the same document type (which must exist, be
///     an identifier and not carry an `identityPublicKey` reference of its
///     own); the last two are checked at contract registration (40125). The
///     declaration takes no `keyIdProperty`; the identifier form is unchanged
///     and every other `refersTo` form stays identifier-only (meta-schema v3,
///     `apply_property_reference` 0, `DocumentPropertyType::KeyIdWithReference`
///     over `KeyReferenceIdentityProperty`). At document create and replace the
///     reference validation reads the key id from the property, resolves the
///     identity (the writer, the creator the action carries, or the named
///     property's value; a key id set while that property is unset, or on a
///     document that records no creator, one written before its type recorded
///     creator ids, being refused with 40125) and fetches that key, so the key
///     fetch is the only read; a key that does not exist refuses the write,
///     paid, with
///     `ReferencedIdentityKeyNotFoundError` (40123) and a disabled one with
///     `ReferencedIdentityKeyDisabledError` (40124), as for the identifier
///     form. A replace re-validates `$ownerId` touched or not, as the
///     `$ownerId` writer gate is, since the writer may not be the one who
///     wrote the key id; `$creatorId` when the key id changed; a property
///     path when the key id or that property changed (a transfer itself is
///     never checked: the reference governs writing, not holding). A
///     `keyIdProperty` may not name a property carrying this form (40125 at
///     registration). `keyRequirements` (item 29) sit on this form exactly
///     as on the identifier form, checked by the same key check and by the
///     same `boundTo` registration rule. Adding it to, removing it from or
///     changing it on an existing property is an incompatible schema change
///     on update, like the rest of a `refersTo`; a property an update adds
///     may carry it, so a `$creatorId` one can meet documents written before
///     their type recorded creator ids.
///
/// 31. **`refersTo` on the elements of a typed array**: an identifier element
///     of a typed array may carry a `refersTo` declaration on its `items`,
///     which every element then declares (meta-schema v3 `documentArrayItem`
///     reuses the property `refersTo` definition by `$ref` and refuses
///     `identityPublicKey` in both forms, which pair one key id with the
///     reference).
///     `parse_typed_array` 0 folds it into the element through the same
///     `apply_property_reference` 0 a scalar identifier goes through, so the
///     element is `IdentifierWithReference(target)` inside `item_type`, and
///     `DocumentPropertyType::reference` reports either kind. Contract
///     registration (`data_contract_reference_validation` 0) checks the
///     declaration as a single one, and document create state validation 2
///     and replace state validation 1 (`document_reference_validation` 0,
///     extended in place: both are only reached from protocol version 14,
///     where the element arm is the only new path) check every element as a
///     single reference, refusing the first that fails with that
///     reference's error (40120, 40127, 40135 and the rest), its path the
///     element's list path (`reasons[2]`). A replace re-validates the
///     elements of a changed list the stored list did not hold (the replace
///     action carries `stored_changed_values`), and all of them when a
///     property bound by a `propertyAgreement` changed, for a `$ownerId`
///     agreement or for `deletableDocument` elements. A repeated element and
///     a foreign contract holding the referenced document type are fetched
///     once per list. Registration caps the references one document
///     can carry at `SYSTEM_LIMITS_V4.max_references_per_document` (256; one
///     per property declaring a reference, key id references of item 30
///     included, `maxItems` per typed array of referencing elements;
///     backfilled into the earlier tables), and
///     refuses an `immutable` property holding a `deletableDocument`
///     reference no replace could clear (a typed array of them, or a single
///     one inside an immutable object), which could never be replaced once
///     a target is deleted, and a single top-level one that is also listed
///     under `immutableAllowSetting`, which a replace could clear once its
///     target is deleted and the next one set to another document. A changed
///     element `refersTo` is an incompatible schema change on update.
///
/// 32. **Document references resolved through a unique index**: a
///     `permanentDocument` `refersTo`, on an identifier property or on the
///     elements of a typed array (item 31), may carry a `lookup`
///     (meta-schema v3, `apply_property_reference` 0, parsed to the appended
///     `DocumentPropertyReferenceTarget::PermanentDocumentLookup`, so an id
///     reference keeps its variant and its encoding): the value is then
///     not the referenced document's id, and the referenced document is the
///     one the named unique index of the referenced document type finds for
///     a key assembled from the referring document. `keys` maps every index
///     property to a property path of the referring type, `$ownerId` or `.`
///     (the value, or the element, exactly once). A `deletableDocument`
///     reference may take one too (`DeletableDocumentLookup`, appended): it
///     then means a document with this key exists now, since the key may find
///     a later document once the one it found is deleted, so every replace
///     re-validates it, an immutable property may not hold it, and it is the
///     one deletable form a reference expression and `ownerRefersTo` (never
///     `creatorRefersTo`) take. Generation 3 of the parser
///     checks on every parse that each property a key reads is a stored,
///     required, single value of the referring type;
///     `create_document_types_from_document_schemas` 1, edited in place like
///     for item 29 (inert before this version, where no parsed reference
///     carries a lookup), checks a lookup into a document type of the same
///     contract under full validation (the index exists, is unique, carries
///     no `timeRange` and is not on an indexOnly type, the keys cover it
///     exactly, every source shares its index property's value kind, and the
///     key cannot move off the document it found: its schema properties are
///     immutable, and `$ownerId` is only a part on a type that is neither
///     transferable nor tradeable), and the contract reference validation
///     checks one into another contract, refusing it with
///     `ReferencedDocumentLookupInvalidError` (40137). The document
///     reference validation (generation 0, reached only from this version)
///     queries the index for each value's key, billed as a document fetch,
///     refuses a write with no match with `ReferencedEntityNotFoundError`
///     (40120, an element named by its list path), checks a
///     `propertyAgreement` against the document found, and on replace
///     re-validates when a property the key reads changed. A key may read
///     `$ownerId` only on a referring type that is neither transferable nor
///     tradeable, checked on every parse. A changed `lookup` is an
///     incompatible schema change on update. Chained queries and composite
///     by-id joins refuse a lookup reference as a join property, and
///     preallocated indexes are never bound through one.
/// 33. **Reference expressions (`anyOf` / `allOf`)**: a `refersTo`, on an
///     identifier property or on the elements of a typed array (item 31), may
///     be `{ "anyOf": [operand, ...] }`, holding if at least one operand
///     holds, or `{ "allOf": [operand, ...] }`, holding if every operand holds
///     for the same value, in place of one target (meta-schema v3, which
///     admits either combinator only as the declaration's one key,
///     `apply_property_reference` 0, parsed to the appended
///     `DocumentPropertyReferenceTarget::AnyOf` and `AllOf`, so every single
///     target keeps its variant and its encoding; decoding refuses a nesting
///     deeper than `MAX_REFERENCE_EXPRESSION_DECODE_DEPTH`, 16, so the bytes of
///     a consensus error cannot recurse without bound). An operand is a leaf,
///     an `identity`, a `permanentDocument` (by id or with a `lookup`, item
///     32), a `listElement`, a `deletableDocument` with a `lookup` (which
///     re-validates the expression on every replace), or an expression of the
///     other combinator; a list names two or more operands. `contract`,
///     `token`, `deletableDocument` by id and
///     `identityPublicKey` leaves, the key id form, a combinator directly
///     inside the same combinator and keys beside a combinator are refused on
///     every parse. Registration caps a list at
///     `SYSTEM_LIMITS_V4.max_reference_operands` (4) and the nesting at
///     `max_reference_expression_depth` (4 combinators on any path to a leaf),
///     both backfilled into the earlier tables, refuses two alike operands of
///     one list (a leaf naming the declaring contract explicitly counting as
///     the one omitting it), counts every leaf against
///     `max_references_per_document`, and checks each leaf as the same
///     declaration alone (`create_document_types_from_document_schemas` 1 and
///     `data_contract_reference_validation` 0, both walking
///     `DocumentPropertyReferenceTarget::leaves_with_paths`, which is the
///     declaration itself at an empty path for a single target, so their
///     output is unchanged where no expression can parse), a failing leaf
///     named by where it sits (`resignation.memberId.anyOf[1].allOf[0]`). The
///     document reference validation (`document_reference_validation` 0,
///     reached only from this version) evaluates each value operand by operand
///     in declared order: an `anyOf` stops at the first operand that holds and
///     otherwise refuses with the last operand's error, an `allOf` stops at the
///     first that fails and refuses with its error, so a refusal is always a
///     leaf's own error and no new error exists; every read is billed, the
///     failed operands' included. A `propertyAgreement` belongs to its leaf and
///     is checked only against that leaf's document. A replace re-validates an
///     expression when its value, or a property one of its leaves binds,
///     changed. A changed expression is an incompatible schema change on
///     update. Chained queries and composite by-id joins refuse an expression
///     join property, and preallocated indexes are never bound through one.
/// 34. **References on the document's writer or creator (`ownerRefersTo`,
///     `creatorRefersTo`)**: a document type may declare one `refersTo`
///     declaration of its own, under the doctype-level `ownerRefersTo`
///     keyword (meta-schema v3, which reuses the property declaration by
///     `$ref`), whose value is the document's `$ownerId`, the writer, instead
///     of a property's: a single target, or a reference expression (item 33)
///     whose every leaf is one of the targets that can hold a writer:
///     `identity`, and a `permanentDocument` found through a `lookup`, where
///     `.` is the writer (and, for the writer alone, a `deletableDocument`
///     found through one, item 32); `contract`, `token` and a document by id (which the
///     writer's identity id never is) and `identityPublicKey` (which needs a
///     key id) are refused, as a leaf too. Parser generation 3 reads it from
///     the stored schema once the core parse has run the meta-schema, on
///     every parse, through the same `apply_property_reference` 0 an
///     identifier property's goes through, onto
///     `DocumentTypeV2::owner_reference`, and refuses it on a type whose
///     documents can be transferred or traded, since neither is a write. Every
///     enumeration of a type's references goes through
///     `DocumentTypeRef::reference_declarations`, which yields it first: its
///     lookup's referring side is checked on every parse, a lookup into a
///     type of the same contract by
///     `create_document_types_from_document_schemas` 1 (edited in place like
///     for item 29, inert before this version, whose parsers never set an
///     owner reference), and the whole declaration at registration by the
///     contract reference validation (`data_contract_reference_validation` 0,
///     extended in place, only reached from this version), which names it
///     `<documentType>.$ownerId` and lets its `propertyAgreement` name the
///     writer on the referring side. It counts one against
///     `max_references_per_document`. Document create state validation 2 and
///     replace state validation 1 (`document_reference_validation` 0, extended
///     in place, both only reached from this version) check the writer against
///     the target exactly as a property's value is checked: on every create,
///     and on a replace under the rules of its target (a changed property its
///     lookup or a `propertyAgreement` reads, every replace for a `$ownerId`
///     pair), and refuse the write with the error the target reports for a
///     property (40120 and the rest) at the path `$ownerId`; an `identity`
///     target fetches nothing, the transition having proved the writer exists.
///     Adding, removing or changing it is an incompatible schema change on
///     update (`validate_schema_compatibility` 1 freezes it as the shared rule
///     set freezes `refersTo`). Its counterpart for a type whose documents can
///     be transferred or traded is `creatorRefersTo`, whose value is the
///     document's `$creatorId`, the creator, which never changes: the same
///     two targets (`.` the creator), only on a type that records creator ids
///     (`should_use_creator_id`: a transferable or tradeable type of a
///     format-1 contract), so a type declares at most one of the two; stored
///     as `DocumentTypeV2::creator_reference`, enumerated second by
///     `reference_declarations`, named `$creatorId` (and
///     `<documentType>.$creatorId` at registration), checked against the
///     writer on a create and the stored creator on a replace under the same
///     rules, never on a transfer or a purchase, and frozen on update the same
///     way.
/// 35. **References to an element of a list of a referenced document**: a
///     new `refersTo` target, `listElement` (meta-schema v3,
///     `apply_property_reference` 0, parsed to the appended
///     `DocumentPropertyReferenceTarget::ListElement`, so every earlier
///     variant keeps its encoding), on an identifier property, on the
///     elements of a typed array (item 31), as a leaf of a reference
///     expression (item 33), or on the writer or the creator (item 34, whose
///     identity then must be listed; a third target those two take next to
///     `identity` and a `permanentDocument` lookup, since an identity id can
///     be an element of a list of identities): the value must be an element
///     of the typed array
///     of identifiers `inList` held by one document of `documentType`, the
///     document whose `$id` the `propertyAgreement` pair with `$id` on the
///     referenced side reads from an identifier property of the referring
///     type (stored, optional or not; generation 3 of the parser checks it
///     under full validation). `$id` joins `$ownerId` and `$creatorId` as a
///     referenced-side agreement name for every document reference. In every
///     other respect a list element is a document reference: `contractId`,
///     `documentType` and its other agreement pairs are checked at
///     registration as a `permanentDocument`'s are (the type must forbid
///     deletion), and the list must be a stored typed array of identifiers
///     fixed once a document is written (the type is immutable or lists the
///     list's top-level property under `immutable`).
///     `create_document_types_from_document_schemas` 1, edited in place like
///     for items 29 and 32 (inert before this version, where no parsed
///     reference is a list element), checks a list in the same contract
///     under full validation, and the contract reference validation checks
///     one in another contract, refusing it with
///     `ReferencedDocumentListInvalidError` (40138). The document reference
///     validation (generation 0, reached only from this version) fetches the
///     list's document by the `$id` pair's value, once per write and shared
///     with any other reference of the same document (every by-id document
///     fetch of one write is now memoized), checks the other pairs against it,
///     and refuses a value the list does not hold, or one set while the `$id`
///     property is not, with `ReferencedEntityNotFoundError` (40120, the list
///     element declaration as its entity type, an element named by its list
///     path); the list is collected once, each value a set lookup. A replace
///     checks it again when its value or a referring side of any pair
///     changed, as every agreement is. Each value counts against
///     `SystemLimits::max_references_per_document` like every other
///     reference. A changed `listElement` is an incompatible schema change on
///     update.
///
///
/// 36. **Transient properties are never stored**: a transient property is
///     judged on the transition and dropped before its document is stored.
///     Up to v13 only a create dropped it and a replace stored whatever it
///     carried; `document_from_replace_transition_action` 1 (paired with the
///     contract-version stamp, edited in place, only selected by this
///     version) drops the transient values of a replace by top-level name as
///     a create does. The rules that read a stored value refuse a transient
///     one, by the property's path and every enclosing object's
///     (`is_transient`): at registration (parser generation 3 under full
///     validation) every `transient` entry must name a top-level property,
///     since Drive drops values by top-level name, and no index may read a
///     transient property, which every document would leave in the index's
///     null branch; a lookup's referenced side refuses such an index too
///     (`referenced_side_error`); the contract reference validation
///     (`data_contract_reference_validation` 0, extended in place, only
///     reached from this version) refuses a `propertyAgreement` whose
///     referenced property is transient, which no stored document carries,
///     and a key reference that stores the key id while its identity is
///     transient, in either form (`identityProperty` on the key id,
///     `keyIdProperty` on the identity). A transient referring side of an
///     agreement stays allowed: it is a write gate, judged on the
///     transition. Changing the `transient` list on contract update was an
///     unsupported keyword to the schema compatibility check, an internal
///     error that dropped the transition unpaid; `validate_schema_compatibility`
///     1 freezes the set of names it lists (sorted and deduplicated before
///     the diff, so a reordering is no change) as it freezes `refersTo`, an
///     incompatible schema change.
///     A census of every mainnet and testnet contract (2026-09-23) found
///     `transient` only on DPNS-shaped `domain` types, which are immutable,
///     index no transient property and list top-level properties only.
///
/// 37. **The moderation charters system contract**
///     (`SystemDataContract::ModerationCharters`, schema v1, the first piece of
///     decentralized moderation teams) carries seven document types, all
///     immutable, the four a charter is made of undeletable and the three team
///     changes deletable. A `reason` is a ground for a moderation
///     action, keyed by its owner and a three-letter `code` unique among the
///     owner's reasons. A `submittedCharter` is a leader's proposal to
///     moderate one contract on that contract's own terms: its
///     `targetContractId` refers to a contract declaring elected moderation
///     (item 24, `moderation: "elected"`, so teams form during the contract's
///     election delay), its `reasons` are a typed array (item 25) of
///     references to reasons (item 31), and it carries an optional
///     `moderatorsShare` and a `rewardSplit`. A `joinRequest` is an identity's
///     offer to serve on a proposal, one per identity per proposal, whose
///     `recipientId` must be the proposal's owner (`propertyAgreement`) and
///     name a decryption key bound to `submittedCharter` (item 29), whose
///     `senderKeyId` is an encryption key of the writer bound to `joinRequest`
///     (item 30) and whose `encryptedMessage` declares its envelope (item 27).
///     An `electedCharter` is a proposal put to the vote with its team: only
///     the proposal's owner may create one, for the proposal's own target
///     (`propertyAgreement`), its `targetContractId` requires
///     `moderation: "electionOpen"`, and its `members` are identities each of
///     which filed a join request for that proposal (item 32, a lookup through
///     the join request's unique index) and none of which is the leader
///     (item 26). Once a charter is seated, its leader adds members from the
///     same join requests (`addedModerator`, the same lookup) and takes them
///     back by deleting the addition, and removes elected members
///     (`removedModerator`, whose `memberId` is a `listElement` of the
///     charter's `members`), putting one back by deleting the removal; each
///     exists at most once per member and charter (unique indexes), so the
///     team that acts is the leader plus the elected members less the
///     removals plus the additions (`ElectedCharter::active_members`). A
///     member asks to leave with a deletable `resignationRequest`, which only
///     a member may file (`ownerRefersTo` with an `anyOf` of a `listElement`
///     into the elected charter's `members` and a `deletableDocument` lookup
///     of an `addedModerator`, items 32 to 35) and which carries a message
///     encrypted to the leader; the leader acts on it by deleting the addition
///     or removing an elected member. The cap on
///     additions, the target's `maxAddedModerators`, is a consensus rule of
///     item 40.
///     Its `byTargetContract` index is a contested unique index
///     with `"resolution": 1`, the masternode vote without a Lock choice of
///     item 23, so an elected charter create opens or joins the contest for
///     its target. `SYSTEM_DATA_CONTRACT_VERSIONS_V3` registers it
///     (`moderation_charters: 1`). A proposal holds no rule beyond its schema:
///     the reward split sums to 100 through the contract's
///     `propertyConstraints` rule (item 39), so 11001 is never produced, and
///     the description fits 4096 bytes through the schema's own `maxBytes`
///     (item 38); every document validation checks both. Genesis registers it
///     on chains born at this version (`create_genesis_state` v1, behind the
///     app-connect branch), `transition_to_version_14` inserts it on upgrade,
///     and the Drive system contract cache serves it from this version
///     (`MODERATION_CHARTERS_CONTRACT_INITIAL_PROTOCOL_VERSION`). Item 40 seats
///     the winning team.
///
/// 38. **`maxBytes` on strings**: a property keyword for the bound plain JSON
///     Schema cannot count, the most UTF-8 bytes a string may take
///     (`maxLength` counts characters, which are up to four bytes each). It
///     goes on a string property, or on the `items` of a typed array of
///     strings where it bounds every element, and is 1 to 65535 and no lower
///     than `minLength`, checked at registration. Meta-schema v3 admits it and
///     `apply_max_bytes` 0 folds it into `StringPropertySizes::max_bytes`, so
///     `max_byte_size`, `max_size` and random documents respect it. The
///     document validation (`DataContract::validate_document_properties` 0,
///     extended in place, inert before this version) calls
///     `validate_max_bytes_properties` (`validate_max_bytes` 0, `None` before
///     this version) after the JSON schema, on every create and replace and in
///     every client that validates a document, and refuses a longer value with
///     `DocumentPropertyMaxBytesExceededError` (10421, naming the element as
///     `tags[2]` for an item). On update it moves like `maxLength`: it may be
///     raised or removed, not added or lowered. The moderation charters
///     contract (item 37) declares it on the proposal's description, replacing
///     the charter-specific description check, its error 11002 and
///     `SystemLimits::max_moderation_charter_description_length`.
///
/// 39. **Property constraints**: the doctype-level `propertyConstraints`
///     keyword (meta-schema v3, `parse_property_constraints` 0) names rules a
///     document's integer properties must meet, each a comparison (`equal`,
///     `notEqual`, `lessThan`, `lessThanOrEqual`, `greaterThan`,
///     `greaterThanOrEqual`) of two integer expressions built from integer
///     literals, property paths and `add`, `subtract`, `multiply`, `divide`,
///     `modulo` and `power`. A property the document leaves out counts as 0,
///     or as the value of an `ifAbsent` operand naming it. Arithmetic is exact
///     `i128`: `divide` and `modulo` are Euclidean (the remainder is never
///     negative), and an overflow, a zero divisor, a negative exponent or a
///     value that is not an integer refuses the document rather than wrapping.
///     The parser checks that every path names an integer property that is
///     neither transient nor inside a transient object, and that no operand
///     nests deeper than `MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH` (64), on every
///     parse, and under full validation the limits
///     `SystemLimits::max_property_constraints` (16 rules) and
///     `max_property_constraint_nodes` (32 per rule).
///     `DataContract::validate_document_properties` 0 (extended in place, inert
///     before this version) calls `validate_property_constraints`
///     (`validate_property_constraints` 0) after the schema validation, so
///     document create and replace, and any client validating a document,
///     refuse a broken rule with `DocumentPropertyConstraintViolatedError`
///     (10422), naming the rule and why. The rules read no state and change
///     nothing stored. They are fixed when the document type is created: a
///     changed `propertyConstraints` is an incompatible schema change on
///     update. The moderation charters contract declares its first one: a
///     `submittedCharter`'s `rewardSplit` members add up to 100, replacing the
///     charter-specific check, whose error 11001 keeps its place in
///     `BasicError` but is never produced.
///
/// 40. **Elected moderation teams moderate from their stored charter**: seating
///     writes nothing. Awarding the contest of item 37 writes the winning
///     `electedCharter`, the only one ever stored for its target, so the
///     charter seated on an elected contract is the one the charter contract's
///     `byTargetContract` index finds, and the moderation paths read it, each
///     read a billed document query of the system contract. Once one is seated,
///     only its team moderates the contract: the leader (the charter's owner)
///     and the active members (its `members` less removals, plus additions),
///     each alone, found by one point read of the unique `removedModerator`
///     index for an elected member or of `addedModerator` for anyone else; the
///     interim moderators
///     are refused (41101). The team holds the abilities the declaration gives
///     it: a deletion or restore needs `deleteDocuments` on the type, a list
///     action the ability on some moderated type
///     (`ContractModerationAbilityNotGrantedError`, 41201). The leader and the
///     active members are protected (41102), with the owner when the
///     declaration says so, and the interim moderators no longer are. A
///     `notYetUsable` interim stops blocking the moderated types
///     (`contract_moderation_gate` v0). An `addedModerator` past the target's
///     `maxAddedModerators` additions the charter holds is refused,
///     paid (`ModerationCharterAddedModeratorLimitReachedError`, 41202), by a
///     hook in the batch's `validate_state` v0 that only a create of the
///     charter contract reaches. A document action on a moderated type may
///     agree to the seated proposal's `moderatorsShare` of the declared
///     moderators part (rounded down) instead of the whole, and is charged
///     that: the batch transformer (state v2) reads the charter and its
///     proposal only for such an agreement, and advanced structure validation
///     and every recheck judge it (`DocumentActionFeeModeratorsShareMismatchError`,
///     40139, for any other lower amount or with no seated charter). The
///     interim team's claim of the moderators pot is refused once a charter is
///     seated (41113). No table moves: every generation involved is unreleased,
///     but for the shipped batch `validate_state` v0, which no batch of an
///     earlier version reaches through the new hook.
///
/// 41. **A seated team's pot, action counts and reasons**: the leader or an
///     active member of a seated team claims the moderators pot for the team,
///     and it is split by the proposal's `rewardSplit`: the leader share to the
///     leader, the equal share between the other members (the leader's when it
///     has none), and the action share between the whole team by each one's
///     count of bans, suspensions, warnings and document deletions since the
///     last settle, equally when nobody acted. Every part rounds down and the
///     remainder stays in the pot. The counts are `member id -> u32` items
///     without storage flags under key `48` of an elected contract's other tree,
///     created with the contract (`insert_contract_moderation_trees` v0), and
///     every settle deletes them. An `addedModerator` or `removedModerator`
///     created or deleted settles the pot first, to the team as it was, by a
///     hook in the batch's `validate_state` v0 beside the cap on additions: it
///     ignores the once-per-epoch limit and writes no last claim. The proof of a
///     claim by a seated team's member, whom the contract does not name as a
///     recipient, shows the claimant's balance alone. A moderation reason gains
///     `reasonDocumentId` (tag bit 2 where it is stored), and a seated team's
///     ban, suspension, warning or deletion must name a `reason` document its
///     proposal lists (`ModerationReasonNotListedError`, 41203). No table moves
///     but the four
///     new Drive method slots, `0` at every version.
///
/// 42. **Repaid identity debt reaches the processing fee pool**: an identity
///     whose fee the balance could not fully cover keeps the unpaid processing
///     part as a debt (its negative credit balance), and credits it receives
///     while its balance is empty still repay that debt first. The repaid part
///     now goes to the processing fee pool of the epoch it is repaid in, where
///     the unpaid fee would have gone; before, it reached no balance the credit
///     sum counts. `add_to_identity_balance` 1 marks it with a
///     `LowLevelDriveOperation::RepaidIdentityDebt`, and every apply routes it:
///     `apply_drive_operations` 1 writes it to the pool after the batch (so it
///     adds to the end of block fee distribution the same batch may write,
///     unbilled), `apply_balance_change_from_fee_to_identity` 1, which now takes
///     the block info, writes it in its own batch (the fee paid is unchanged),
///     and `add_epoch_pool_to_proposers_payout_operations` 1 hands the epoch
///     payouts to the block's `apply_drive_operations` instead of converting
///     them to a plain grove batch, skips a share whose `payToId` has no
///     balance and caps each share at what is left of its masternode's payout.
///     An apply that meets one it does not route fails (`CorruptedCodeExecution`)
///     instead of dropping it. `apply_drive_operations` 1 also merges every
///     credit and debit one batch makes to an identity's balance into one net
///     write: each converts against the balance committed before the batch, so
///     a second write replaced the first and two credits to an indebted
///     identity repaid its debt twice.
///
/// The app-connect system contract (`SystemDataContract::AppConnect`, schema v1)
/// carries only the wallet's `loginKeyResponse`: a flat indexOnly entry keyed by
/// the app's ephemeral key hash and the responding identity, with the wallet's
/// ephemeral key and encrypted grant in `entryPayload`. Genesis registers it on
/// chains born at this version; `transition_to_version_14` inserts it on upgrade.
/// The Drive and trusted SDK caches serve it only from protocol version 14.
///
///
/// * `ShieldFromIdentity` (state transition type 21) activates:
///   `SHIELD_FROM_IDENTITY_INITIAL_PROTOCOL_VERSION = 14` gates it in
///   `is_allowed`, and `DRIVE_ABCI_VALIDATION_VERSIONS_V10` is the first
///   table whose `shield_from_identity_state_transition` row enables basic
///   structure, identity signature, and nonce validation. It moves credits
///   from an identity balance straight into the shielded pool: the funding
///   side is identity-signed like `IdentityCreditTransferToAddresses`, the
///   pool side is an outputs-only Orchard bundle like `Shield`, and the fee
///   is metered plus the shielded compute fee, paid from the identity.
///
/// * `IdentityTopUpFromShieldedPool` (state transition type 22) activates at the
///   same gate (`IDENTITY_TOP_UP_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION = 14`,
///   `DRIVE_ABCI_VALIDATION_VERSIONS_V10` row). It spends shielded notes like
///   `Unshield` and credits an EXISTING identity's balance instead of a platform
///   address: pool-paid flat fee (`compute_shielded_identity_top_up_fee`), no
///   platform signature, the target identity and gross amount bound into the
///   Orchard sighash, and no system-credit adjustment (pool and identity balances
///   are both conservation-equation terms).
///
/// The wire surface changes only additively: `GetDocumentsRequestV1`
/// already carries `selects` / `group_by` / `order_by` / `limit` /
/// `offset`; the ranked response is an additive `ResultData.ranked`
/// variant, whose `skipped` field is likewise additive; and the v1
/// where-clause operator enum gains `IN_TIME_RANGE = 11`, which pre-v14
/// servers reject as an unknown operator rather than misread (the v0 wire
/// has no time-range operator at all).
/// Contract-bound authentication keys activate through contract-bounds validation v2,
/// identity-signature validation v1 and batch advanced-structure v1. Identity creation
/// validates key bounds (state v1) and identity-update state v1 retains the contract
/// lookup fees; Drive identity methods v2 index and refresh the bound keys. The same v1
/// contract-info methods also store the current-key alias of a contract-level encryption or
/// decryption key bound under `MultipleReferenceToLatest` in its purpose subtree, where the
/// current-key query reads it; v0 wrote it one level up, where its sibling reference could
/// not resolve, so registering such a key failed inside Drive on every earlier version.
/// Contract group bounds on authentication keys ride the same versions: contract-bounds
/// validation v2 admits them, batch transform v2 resolves the member contract's group
/// memberships into the action (only for a group-bound signing key) for advanced-structure v1 to judge, and shielded-proof validation v1 refuses them in identity creation from the
/// shielded pool, whose sighash preimage layout predates them.
/// A transition carrying such a key is inactive before this version (`active_version_range`),
/// so earlier protocol versions reject it without charging, as a binary that cannot decode it does.
/// Authentication keys may carry a budget and an expiry (the version 1 public key format, which
/// `StateTransition::active_version_range` admits from 14). Key structure validation v1
/// (`STATE_TRANSITION_METHOD_VERSIONS_V2`) and `validate_identity_public_keys_limits` decide
/// which keys may carry them; Drive identity methods v2 write the remaining budget when the key
/// is added; identity-signature validation v1 refuses a key whose budget is spent;
/// `validate_fees_of_event` v1 refuses an expired key and a spend the remaining budget does not
/// cover (only metered processing may overshoot); `execute_event` v1 deducts what was spent.
/// Shielded-proof validation v1 refuses a key that carries a budget or an expiry in identity
/// creation from the shielded pool, whose sighash preimage does not cover the limits.
/// `IdentityKeyLimitsUpdate` (state transition type 23, gated by
/// `IDENTITY_KEY_LIMITS_UPDATE_INITIAL_PROTOCOL_VERSION`) raises a key's total budget, and the
/// remaining budget with it, or moves its expiry later; it only ever loosens limits. Signed by a
/// MASTER key or by a CRITICAL key without limits (`DRIVE_ABCI_VALIDATION_VERSIONS_V10` turns
/// its gates on; Drive identity methods v2 rewrite the key and raise the remaining budget).
pub const PLATFORM_V14: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_14,
    drive: DRIVE_VERSION_V9, // changed: drive document method versions v4 — v2 index walkers (shared-prefix aggregate indexes become insertable) + the detect_ranked_mode slot; contract method versions v4: the moderation list trees, the document removal record trees and the moderation method table; apply_drive_operations 1 (a moderator's document deletion refunds nobody; every write of one identity balance, fee pot or prefunded specialized balance in a batch merged into one; a batch writing one token balance or supply twice refused; repaid identity debt credited to the processing fee pool); index uniqueness gains validate_restored_document_uniqueness (a moderator's document restore)
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2, // changed: saved platform state structure 1 keeps masternodes and validator sets as one aux entry each
        methods: DRIVE_ABCI_METHOD_VERSIONS_V10, // changed: records the per-block total credits history for the daily withdrawal limit
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V10, // changed: contested-index cross-check + refersTo document reference validation; the ContractUserModeration gates and the batch transformer's contract_moderation_gate
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3, // changed: prune bound for the total credits history
        query: DRIVE_ABCI_QUERY_VERSIONS_V3, // changed: ranked + boolean-HAVING routing gate; the v1 handler also resolves IN_TIME_RANGE from committed block time
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5, // changed: validate_config_update 2 admits the contract moderation declaration of config V2
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3, // changed: the indexOnly delete-by-values kind (documentIndexOnlyDelete) joins the wire; the ContractUserModeration transition
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V2, // changed: public keys in creation may carry a budget or an expiry
        state_transitions: STATE_TRANSITION_VERSIONS_V4,
        contract_versions: CONTRACT_VERSIONS_V6, // changed: v3 document meta-schema hosts the ranked, refersTo, requiredSince and timeRange keywords; validate_structure_interval v1 rejects a zero epoch interval; config max_version 2 (the contract moderation declaration) and validate_moderation_config
        document_versions: DOCUMENT_VERSIONS_V4, // changed: document serialization format 3 — the contract version stamp that enables `requiredSince` properties
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V3, // changed: distribution_function_evaluate v1 — deterministic libm for token reward math; reward_distribution_max_cycle_moment v1: the epoch claim cap no longer wraps; distribution_function_cycle_epochs v1: evonode cycles weighted by the epochs they span
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3, // changed: daily_withdrawal_limit v2 — a percentage of the total credits a day ago
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3, // changed: DashPay v2 adds profile payment address fields (DIP-33); withdrawals v2 admits the terminal FAILED status
    // The TTL ephemeral-bytes rate (270 credits/byte to processing) rides
    // the shared storage table; it is dead below v14 (the `ttl` grammar
    // does not parse), so no table fork is needed.
    fee_version: FEE_VERSION3, // changed: contested document contribution reduced to 0.1 DASH; masternode vote cost reduced to 0.00002 DASH; moderation election fund of 0.5 DASH; registration surcharge for once-per-identity token distributions
    system_limits: SYSTEM_LIMITS_V4, // changed: daily withdrawal limit becomes 15% of the total credits a day ago + time-range overlap-factor cap (24) + time-range TTL cap (1 week) and per-write drop cap (32) + GroveDB proof envelope floor (V1); max_contract_moderators, max_contract_suspension_until, max_contract_moderation_reason_length, max_contract_warnings_per_identity, max_contract_moderation_reason_documents and contract_document_restore_window_ms (a week)
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::v13::PLATFORM_V13;

    #[test]
    fn should_change_only_the_contested_document_and_once_per_identity_fees_at_protocol_14() {
        for protocol_version in 1..14 {
            let version = PlatformVersion::get(protocol_version).expect("known protocol version");
            let fund_fees = &version.fee_version.vote_resolution_fund_fees;
            assert_eq!(
                fund_fees.contested_document_vote_resolution_fund_required_amount, 20_000_000_000,
                "protocol {protocol_version} must preserve the 0.2 DASH contribution"
            );
            assert_eq!(
                version
                    .fee_version
                    .vote_resolution_fund_fees
                    .contested_document_single_vote_cost,
                10_000_000,
                "protocol {protocol_version} must preserve the 0.0001 DASH vote"
            );
            // No moderation election exists before 14; its amount is the contested one, so
            // a shipped path choosing between the two cannot change what it charges
            assert_eq!(
                fund_fees.moderation_vote_resolution_fund_required_amount,
                fund_fees.contested_document_vote_resolution_fund_required_amount,
                "protocol {protocol_version}"
            );
        }

        let mut expected_fees = PLATFORM_V13.fee_version.clone();
        expected_fees
            .vote_resolution_fund_fees
            .contested_document_vote_resolution_fund_required_amount = 10_000_000_000;
        // A masternode vote costs a fifth of what it did: 0.00002 DASH from the contest's fund
        expected_fees
            .vote_resolution_fund_fees
            .contested_document_single_vote_cost = 2_000_000;
        // An application in a moderation election prefunds its masternode votes with 0.5 DASH
        expected_fees
            .vote_resolution_fund_fees
            .moderation_vote_resolution_fund_required_amount = 50_000_000_000;
        // The once-per-identity token distribution exists from protocol version 14 on, and a
        // token that uses it pays the surcharge of the other distribution kinds.
        assert_eq!(
            expected_fees
                .data_contract_registration
                .token_uses_once_per_identity_distribution_fee,
            0
        );
        expected_fees
            .data_contract_registration
            .token_uses_once_per_identity_distribution_fee = 10_000_000_000;
        assert_eq!(PLATFORM_V14.fee_version, expected_fees);
    }

    /// The ranked / boolean-HAVING routing gate lives in v14's own query
    /// table, so flipping it touches only v14: a v13 node keeps running
    /// the v0 helper, which rejects every non-empty HAVING, so a
    /// mixed-version network agrees until the upgrade vote carries.
    ///
    /// v14 selects the v2 helper, which routes the ranked shape
    /// (`ORDER BY <agg> LIMIT n`) to `dispatch_ranked_v1` and the
    /// boolean-HAVING range shape (exactly one `having` clause on the
    /// selected aggregate) to `dispatch_having_v1`. A change that made
    /// v13 non-zero here would be consensus-breaking for
    /// already-deployed nodes, which is exactly what the v13 half of
    /// this assertion guards.
    #[test]
    fn ranked_having_routing_gate_is_v14_only() {
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .query
                .document_query_helpers
                .compute_aggregate_mode_and_check_limit,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .query
                .document_query_helpers
                .compute_aggregate_mode_and_check_limit,
            2
        );
    }

    /// The ranked index keywords are gated by the meta-schema version, so v14
    /// must select meta-schema v3 while v13 stays on v2.
    /// Contested indexes without a Lock choice (item 23): the three method
    /// versions that read the resolution are selected by v14 only, so a v13
    /// replay keeps the shipped rules (a full poll for every contest, ties to
    /// the latest contender, a Lock vote accepted on any contest).
    #[test]
    fn no_locking_contests_are_selected_by_v14_only() {
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .methods
                .voting
                .check_for_ended_vote_polls,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .methods
                .voting
                .check_for_ended_vote_polls,
            1
        );
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .validation_and_processing
                .state_transitions
                .masternode_vote_state_transition
                .state,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .validation_and_processing
                .state_transitions
                .masternode_vote_state_transition
                .state,
            1
        );
        assert_eq!(
            PLATFORM_V13
                .drive
                .methods
                .document
                .insert_contested
                .add_contested_document_for_contract_operations,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive
                .methods
                .document
                .insert_contested
                .add_contested_document_for_contract_operations,
            1
        );
    }

    #[test]
    fn ranked_index_keywords_are_gated_by_meta_schema_v3() {
        assert_eq!(
            PLATFORM_V13
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .document_type_schema,
            2
        );
        assert_eq!(
            PLATFORM_V14
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .document_type_schema,
            3
        );
    }

    /// The ranked grammar lives in its own document-type parser generation
    /// rather than behind a version gate inside a shipped one, so v14 must
    /// select generation 3 while v13 stays on generation 2. Pinned here
    /// because it is the whole reason generations 0/1/2 can stay byte-identical
    /// to what consensus already ran: a historical block replayed at v13 is
    /// parsed by a generation that has never heard of the ranked keywords.
    /// The grove v4 cleanup gates (batch overwrite inspection + delete-tree
    /// actual-type cleanup) exist for the indexed trees that ranked indexes
    /// lay down, so v14 must select grove protocol 4 while v13 stays on 3.
    /// The gates are cost-neutral — they derive the old element from data the
    /// merk apply already loads — and the fee-constant tests pin identical
    /// fees on both sides of the boundary. Platform flows cannot themselves
    /// overwrite a ranked index (the flags are immutable on contract update
    /// and new indexes cannot be added to an existing document type), so the
    /// cleanup behavior itself is exercised by grovedb's own overwrite suites
    /// at the pinned revision; this test pins that v14 actually activates
    /// them.
    #[test]
    fn grove_v4_cleanup_gates_activate_at_v14() {
        assert_eq!(PLATFORM_V13.drive.grove_version.protocol_version, 3);
        assert_eq!(PLATFORM_V14.drive.grove_version.protocol_version, 4);
    }

    #[test]
    fn ranked_grammar_gets_its_own_parser_generation() {
        assert_eq!(
            PLATFORM_V13
                .dpp
                .contract_versions
                .document_type_versions
                .class_method_versions
                .try_from_schema,
            2
        );
        assert_eq!(
            PLATFORM_V14
                .dpp
                .contract_versions
                .document_type_versions
                .class_method_versions
                .try_from_schema,
            3
        );
    }

    /// v14 introduces the slots but activates none of them yet. If a later
    /// change flips one of these, it must do so deliberately — and update this
    /// test — rather than by inheriting a default.
    #[test]
    fn ranked_feature_slots_exist_but_are_dormant() {
        assert_eq!(
            PLATFORM_V14.drive.methods.document.query.detect_ranked_mode,
            0
        );
        assert_eq!(
            PLATFORM_V14.drive.methods.document.query.detect_having_mode,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive
                .methods
                .verify
                .document_ranked
                .verify_ranked_top_k_proof,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive
                .methods
                .verify
                .document_ranked
                .verify_having_range_proof,
            0
        );
        let grove = &PLATFORM_V14.drive.grove_methods.batch;
        assert_eq!(grove.batch_insert_empty_provable_count_indexed_tree, 0);
        assert_eq!(grove.batch_insert_empty_provable_sum_indexed_tree, 0);
        assert_eq!(
            grove.batch_insert_empty_provable_count_provable_sum_indexed_tree,
            0
        );
    }

    /// The contested vote poll index cross-check changes accept/reject
    /// behavior for document create transitions, so it lives in v14's own
    /// validation table: a v13 node keeps running structure validation v0,
    /// which validates only the prefunded amount and ignores the index name.
    /// A change that made v13 non-zero here would retroactively reject
    /// transitions already in the chain.
    #[test]
    fn contested_index_cross_check_is_v14_only() {
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_structure_validation,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_structure_validation,
            1
        );
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_state_validation,
            1
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_state_validation,
            2
        );
        assert_eq!(
            PLATFORM_V13
                .drive
                .methods
                .document
                .insert_contested
                .add_contested_vote_subtree_for_non_identities_operations,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive
                .methods
                .document
                .insert_contested
                .add_contested_vote_subtree_for_non_identities_operations,
            1
        );
    }
}
