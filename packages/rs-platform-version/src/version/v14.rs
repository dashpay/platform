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
use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
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

/// v14 hosts thirty-one consensus changes:
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
///    withdrawal eventually fits and cannot block the pooling queue. The base has
///    no fixed cap: what Core will mine bounds pooling through the Core-anchored
///    limit of note 74 instead. The credit inflows of the active window — every
///    credit mint, recorded per block by `record_credit_inflows_for_withdrawals`
///    in the credit inflows sum tree — are added to the base, so the limit
///    counts net outflow and a matching deposit -> withdraw cycle does not
///    consume the budget of other users (#4471), mirroring Core v24's net
///    credit-pool rule. Both the inflows and the pooled reservations count over the
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
///    daily total is still minable across blocks; V24 limits the net drop of
///    its credit pool per 576-block window, which note 74 follows.
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
/// * `DRIVE_ABCI_QUERY_VERSIONS_V2` bumps
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
/// 7. **Client-side GroveDB proof envelope floor (not a version-table
///    entry)**: clients refuse the legacy V0 proof envelope at every protocol
///    version through
///    `drive::verify::grovedb_proof_envelope::MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION`,
///    so nothing about it is gated on v14. The note keeps its number so the
///    later notes keep theirs.
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
///     suspended signer, its deletions excepted (and its retractions, item 72),
///     and collect a lapsed
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
///     that declares moderation may set `moderatorAbilities.delete` (meta-schema
///     v3, fixed when the type is created, refused on a type that keeps
///     history, is indexOnly or restricts creation; for references such a type
///     is no longer permanent, so a permanentDocument reference refuses it, and
///     a moderatedDocument or a deletableDocument reference takes it, see 64). A
///     moderation declaration may then keep
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
///     `moderatorAbilities.deleteWithin` bounds the deletion in time: so many
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
///     charged, and no fee agreement is asked. `moderatorAbilities.delete` is
///     now also refused on a type with a contested index, whose deletions
///     could never be undone. The record grows on the wire
///     (`getContractDocumentRemovals`: `document_hash`, `restoration`).
///
/// 22. **Elected moderation teams, the declaration and the interim**: a data
///     contract may declare, when it is created, that its moderators are a team
///     elected by masternodes and evonodes (`ContractModerators::Elected`, a third kind
///     beside the owner and an appointed set, in the same config V2). The
///     declaration is frozen: the join and vote windows (at most four weeks, at
///     least one day on mainnet and 0 elsewhere, one week by default), in
///     seconds and bounded by `SYSTEM_LIMITS_V4`;
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
///     a target is deleted, and a single top-level one frozen only under a
///     condition (see 66), which a replace could clear once its target is
///     deleted and a later one the condition leaves free set to another
///     document. A changed
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
///     immutable, none an optional `deletableDocument` reference by id, which
///     a replace may clear once its document is deleted (item 73), and
///     `$ownerId` is only a part on a type that is neither transferable nor
///     tradeable), and the contract reference validation
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
///     document's properties must meet, each a condition: a comparison
///     (`equal`, `notEqual`, `lessThan`, `lessThanOrEqual`, `greaterThan`,
///     `greaterThanOrEqual`) of two integer expressions built from integer
///     literals, paths of integer or boolean properties (a boolean reading as 1
///     for true and 0 for false), `add`, `subtract`, `multiply`, `divide`,
///     `modulo` and `power`, `min` and `max` over two or more operands and
///     `abs` over one, and sizes: `length` and `byteLength`, the characters and
///     UTF-8 bytes of a string property, and `count`, the items
///     of an array or byte array property, each 0 for a property the document
///     leaves out, and the system times and heights `$createdAt`, `$updatedAt`
///     and `$transferredAt` (block times in milliseconds), each also with
///     `BlockHeight` or `CoreBlockHeight` appended, of the document's creation,
///     last update (create, replace, price update) and last transfer (create,
///     transfer, purchase), which a rule may read only on a type listing them
///     in `required`; `in`, whether an integer expression takes one of two or
///     more distinct integer values; `equal` or `notEqual` of a string property
///     and a `{ "const": string }` or of two bare paths naming string
///     properties, or `in` of a string property and two or more distinct
///     strings, a string the document leaves out equalling no constant and no
///     other string unless an `ifAbsent` gives it a string default
///     (`{ "ifAbsent": ["status", "open"] }`, whose default an `enum` must list
///     too); `equal`, `notEqual` or `in` of an identifier property (one
///     declaring `refersTo` included) or of `$ownerId`, the document's owner,
///     likewise, with base58 identifier constants or another identifier operand
///     and no default, an identifier the document leaves out equalling none;
///     `startsWith` or `endsWith`, whether a string (a constant or a string
///     property, at least one a property) starts or ends with another, byte for
///     byte; `contains`, whether a typed array property holds an element equal
///     to an integer expression, a string or an identifier operand (a constant,
///     a property, or `$ownerId`), as its elements are, an array the document
///     leaves out holding nothing; `present` or `absent` naming a property of
///     any type, whether the document holds it (the one way to tell a property
///     left out from one set to 0); `anyOf` or `allOf` over two or more
///     conditions; `not` over one; `ifThen` over two (the second holding
///     whenever the first does, evaluated only then) or `ifThenElse` over three
///     (the second when the first holds, the third when it does not, only the
///     branch taken evaluated), no two alike; `notIn`, an `in` negated in as
///     many nodes. In an operand, a property the document leaves out counts as
///     0, or as the value of an `ifAbsent` operand naming it. `countOf` and
///     `sumOf` operands read a total from state: how many documents of a type
///     of the same contract match a filter (keys of that type or `$ownerId`,
///     values read from the document written), or an integer property's total
///     over them, as the count or sum tree will keep it once the write is done;
///     the batch transformer reads them into the action
///     (`Drive::fetch_property_constraint_aggregate`, billed; in place in
///     transformer 0, reading nothing before this version), a transfer or
///     purchase reads again those depending on the owner, a price update those
///     of the rules it judges, and a rule reading one it is not given is not
///     judged by an SDK pre-check and an error in consensus, which reads them
///     all.
///     Arithmetic is exact `i128`: `divide` and `modulo` are Euclidean (the
///     remainder is never negative), and an overflow, a zero divisor, a
///     negative exponent or a value that is not an integer refuses the document
///     rather than wrapping. Conditions are checked in declared order and no
///     further than the outcome needs (`anyOf` stops at the first that holds,
///     `allOf` at the first that fails), a fault in one that is checked refuses
///     the document whatever the others say, and `not` never turns a fault into
///     a pass, so an earlier condition guards a later one. The parser checks
///     that every path an operand reads names an integer or boolean property,
///     every path a `length` or `byteLength` measures a string property, every
///     path a `count` counts an array or byte array property, every string
///     `startsWith` or `endsWith` tests a string property (a constant tested
///     against one with an `enum` starting or ending one of its values), every
///     array a `contains` looks in a typed array of the kind it looks for (a
///     string constant in the elements' `enum` when they declare one), every
///     system time or height a rule reads one the type lists in `required`
///     (none on an indexOnly type), every path compared with identifiers an
///     identifier property, every path compared with strings a string property
///     (whose `enum`, if it declares one, lists every constant it is compared
///     with), and every path `present` or `absent` tests a property of any
///     type, none transient nor inside a transient object; that every
///     comparison and `in` reads a property or the owner; that nothing is
///     compared with itself; that strings and identifiers are only compared for
///     equality, and never with each other; that no `in` lists a value twice;
///     that an `anyOf` or `allOf` holds none directly of its own kind and a
///     `not` no `not` or `notIn`; that an indexOnly type, whose deletes carry
///     no owner, reads no `$ownerId`; and that no condition or operand nests
///     deeper than
///     `MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH` (64), on every parse. Under full
///     validation it holds the limits `SystemLimits::max_property_constraints`
///     (16 rules) and `max_property_constraint_nodes` (32 per rule, every
///     comparison, `in`, listed value, `const`, presence test and logical
///     operator counting as one), and that no `anyOf` or `allOf` lists the same
///     condition twice, and at most `max_property_constraint_aggregates` (4)
///     distinct totals per type; once every type is parsed, that a tree keeps
///     each total (`documentsCountable` or `documentsSummable`, or an index
///     whose properties are exactly the filter's keys) and that no type with a
///     contested index totals its own documents, in
///     `create_document_types_from_document_schemas` 1, in place and inert
///     before this version. `DataContract::validate_document_properties` 0
///     (extended in place, inert before this version, and taking the document's
///     owner for `$ownerId`) calls `validate_property_constraints`
///     (`validate_property_constraints` 0) after the schema validation, so
///     document create and replace, and any client validating a document,
///     refuse a broken rule with `DocumentPropertyConstraintViolatedError`
///     (10422), naming the rule and why. A transfer and a purchase, which give
///     the document a new owner, are judged against the rules reading
///     `$ownerId` or the transfer's time and heights, with the new values, and
///     a price update, which sets the update's time and heights, against the
///     rules reading those (`validate_property_constraints_for_system_change`,
///     in their structure validation, in place and inert before this version;
///     the price update's call is new there). `validate_document_properties`
///     takes the document version's system values (`DocumentSystemValues`):
///     consensus gives the writer and the block's time and heights on create,
///     and on replace the stored creation and transfer values with the block's
///     as the update. The rules change nothing stored and read no state but
///     their totals. They
///     are fixed when the document type is created: a changed
///     `propertyConstraints` is an incompatible schema change on update. The
///     moderation charters contract declares its first one: a
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
///     new Drive method slots, `0` at every version. The counts are read with
///     the `getContractModerationActionCounts` query (an elected contract
///     only), whose proof reads the whole counts tree; it adds a fifth contract
///     method slot (`prove_contract_moderation_action_counts`), a verify slot
///     (`verify_contract_moderation_action_counts`) and the query's bounds
///     (`contract_moderation_action_counts`), `0` at every version as well.
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
/// 43. **The end-date cleanup of ended contested vote polls**: a block ends at
///     most `maximum_vote_polls_to_process` vote polls, the earliest end date
///     first, and removes their entries from the end-date queries.
///     `remove_contested_resource_vote_poll_end_date_query_operations` 2
///     (`DRIVE_VOTE_METHOD_VERSIONS_V3`) removes an end date only once none of
///     its vote polls remain: it reads the entries under the date and keeps the
///     date while any of them is not removed in the same batch, so the polls
///     left end in a later block.
///
/// 44. **The total supply ceiling bounds every mint and direct purchase**: a
///     token's total supply is stored in a sum item, so it can never pass
///     `i64::MAX`. Token mint and token direct purchase state validation 1
///     treat `i64::MAX` as the max supply when the token configures none (and
///     cap a configured one there), refusing a mint or purchase past it with
///     `TokenMintPastMaxSupplyError`, as a paid consensus error. State
///     validation 0 checked only a configured max supply, so such a transition
///     failed in execution as an internal error instead. Both now read the
///     total supply on every mint and purchase, and pay for that read.
///
/// 45. **A masternode vote towards an identity names a contender**: a
///     `ResourceVoteChoice::TowardsIdentity` vote for an identity that is not a
///     contender of the poll is refused, unpaid, with
///     `VoteChoiceNotAllowedForVotePollError` (40307) by `validate_state` 1 of
///     the masternode vote, which reads the contender's document reference
///     under the poll. The reserved keys of the poll's stored info, abstain
///     tree and lock tree are refused before that read. Before, a vote for an
///     unknown identity failed with an internal error, and a vote towards a
///     reserved key was counted as Lock or Abstain. `check_for_ended_vote_polls`
///     1 also ignores the lock tally of a contest resolved without locking. No
///     table moves: both generations are selected by this version alone.
///
/// 46. **A raw state transition is exactly one encoded transition**:
///     `decode_raw_state_transitions` 1 (`DRIVE_ABCI_METHOD_VERSIONS_V10`)
///     decodes with `StateTransition::deserialize_from_bytes_untrusted_exact_in_version`,
///     so bytes left over after the transition are an invalid encoding
///     (`SerializedObjectParsingError`, 10002), refused unpaid in `check_tx` and
///     in block processing. Version 0 ignored them, so the transition with
///     anything appended executed as the original under another transaction
///     hash. Version 1 also reports a transition whose version is outside its
///     active range as `StateTransitionNotActiveError` (10603) instead of a
///     decode failure, naming whichever boundary of that range was missed: its
///     start for a version not active yet, its end for one already superseded.
///
/// 47. **A storage refund is clawed back from the epochs it was priced for**:
///     removing data in epoch E refunds its owner the shares of epochs E+1
///     onward, and the refund waits for the next epoch change to be taken out of
///     the epoch storage pools. `add_distribute_storage_fee_to_epochs_operations`
///     1 (`DRIVE_ABCI_METHOD_VERSIONS_V10`) restores and subtracts it from the
///     epoch after the previous block's epoch, the one every pending refund was
///     priced in, so each of those epochs gives back its own share. The shares
///     of epochs that closed before the current one, skipped by a halt, and the
///     rounding leftovers come out of the current epoch. Version 0 started from
///     the epoch after the current one, so the current epoch kept most of its
///     refunded share and the later epochs gave back more than theirs. The
///     total taken out equals the refund in both.
///
/// 48. **An evonode's token claim covers only the epochs it read**: an
///     `EvonodesByParticipation` perpetual distribution weighs each cycle by the
///     claimant's share of the blocks proposed in the epochs it spans, from their
///     finalized epoch infos. Up to v13 the claim read at most
///     `drive_abci.query.max_returned_elements` (100) of them but evaluated its
///     whole range, up to 128 cycles (32,767 for a fixed amount), from the last
///     paid moment or, on a first claim, from the start of the distribution. An
///     evonode more than 100 epochs behind (about 2.5 years on mainnet, 4 days on
///     testnet) had every claim of a function other than a fixed amount fail as an
///     internal error, with its last paid moment never advancing, while a fixed
///     amount applied the share of the epochs read to the whole range. A claim
///     reaching an epoch whose info the fee distribution of the block had not
///     written yet (the previous epoch, in the first block of an epoch) failed or
///     was weighed the same way. `evonode_participation_rewards` 1
///     (`DRIVE_TOKEN_METHOD_VERSIONS_V2`) reads the epochs after the cycle start of
///     the last paid moment, at most `SYSTEM_LIMITS_V4.max_evonode_reward_claim_epochs`
///     (100, backfilled into the earlier tables) or one whole cycle when a cycle is
///     longer, and pays through the last whole cycle it read, which the claim stores
///     as the last paid moment, so an evonode that is behind is paid over several
///     claims. An epoch without finalized info before the last one read, one in which
///     no block was produced, counts as an epoch without blocks, and a claim that
///     read no whole cycle is refused, paid, with `InvalidTokenClaimNoCurrentRewards`.
///     `get_finalized_epoch_infos` now takes its limit from the caller; every other
///     caller passes the query bound it read before.
///
/// 49. **Documents with a time to live**: the doctype-level `ttl` keyword (meta-schema v3,
///     document type parser generation 3) makes the platform delete each document of the
///     type `ttl` seconds after its `$createdAt`, at most
///     `max_document_expirations_per_block` (128, `SYSTEM_LIMITS_V4`) weighing at most
///     `max_document_expiration_weight_per_block` (1,024 in documents plus their index levels)
///     per block after the block's state transitions (`expire_documents` 0 in
///     `DRIVE_ABCI_METHOD_VERSIONS_V10`). The keyword requires `$createdAt`, is refused
///     with `documentsKeepHistory`, `indexOnly` and a contested index, is at least
///     `min_document_ttl_seconds` (one hour) and at most `max_document_ttl_seconds` (one
///     year) at registration, and is fixed on update (`validate_update` 1). References treat such a type as deletable. Its
///     documents are stored without storage flags, indexed in the documents expirations
///     tree under `Misc` (created by `create_initial_state_structure` 4 and
///     `transition_to_version_14`), and pay the `document_ttl` group of `FEE_VERSION3`: a
///     price per byte for the time they live (tiers up to seven days, then per 9.125 days),
///     paid out to the epochs they live in (at most one era) through the lifetime storage fee
///     pools under `Pools` (`add_distribute_block_fees_into_pools_operations` 1),
///     plus their deletion prepaid as processing (per index level and per document byte; a
///     change that grows a document prepays its added bytes). From its expiry on, a
///     document can no longer be replaced, transferred, bought, repriced or restored by a
///     moderator (`DocumentExpiredError`, 40140), judged from its own `$createdAt`; its
///     owner may still delete it where `canBeDeleted` allows. See
///     `book/src/data-model/document-ttl.md`.
///
/// 50. **A contest accepts at most 1,000 contenders, and its end reaches every
///     one**: document create state validation 2 (`DRIVE_ABCI_VALIDATION_VERSIONS_V10`)
///     refuses, paid, a document that would add a contender to a contest holding
///     `max_contenders_per_contest` (`SYSTEM_LIMITS_V4`, 1,000) already
///     (`DocumentContestMaximumContendersReachedError`, 40141).
///     `add_contested_indices_for_contract_operations` 1
///     (`DRIVE_DOCUMENT_METHOD_VERSIONS_V4`) writes the last index value of a
///     poll started from this version as a count tree, so the join reads the
///     count in one element fetch; a poll started before keeps its plain tree
///     and has its contenders counted by a keys query of at most 1,000.
///     `maximum_contenders_to_consider` rises from 100 to 10,000, so the tally
///     of an ended poll, and the cleanup built from it, cover every contender
///     of a poll within the cap, and up to 10,000 of one that grew past it
///     before this version. `check_for_ended_vote_polls` 1 compares every tied
///     contender; version 0 compared at most 100.
///
/// 51. **The fund a contender pays doubles for every 50 contenders a contest
///     holds past 250**: the fund to join a contest is its fund doubled once
///     the contest holds `contested_document_contenders_before_fund_doubling`
///     (`FEE_VERSION3`, 250) contenders and again for every
///     `contested_document_contenders_per_fund_doubling` (50) more: 0.1 DASH
///     for the first 250 DPNS contenders, 0.2 for the next 50, up to 3,276.8
///     for the 951st to the 1,000th, so filling a contest costs 327,695 DASH
///     where it cost 100. A contender's prefunded voting balance is the most it
///     pays: document create state validation 2 refuses, paid, one stating less
///     than the fund to join (`DocumentContestNotPaidForError`, carrying that
///     fund), the first contender of a new contest included, and charges one
///     stating more only the fund to join, the rest staying with it. Document
///     create structure validation 1 leaves the amount to state validation;
///     version 0 wants exactly the contest's fund.
///
/// 52. **Property constraints judge what is stored, and read `$defs`**: to
///     `present` and `absent` (item 39), an object none of whose members is
///     present (`{}`, or `{ "inner": {} }` around one) is absent, since a
///     stored document reads it back as no object at all. A create or replace
///     carrying `meta: {}` was judged with `meta` present, and a later
///     transfer, purchase or price update, judged on the stored document, with
///     it absent. The parser (generation 3) reads the schema of a property
///     given as a `$ref` to the contract's `$defs` from the definition, as the
///     core parse does, when it checks a rule's string constants and defaults
///     against the property's `enum` and an `encryptedFor` key id's bounds; it
///     refused every such contract with a decoding error before.
///
/// 53. **Properties the platform generates (`generatedFrom`)**: the property
///     keyword (meta-schema v3, `apply_generated_from` 0, `GeneratedFrom` on
///     `DocumentProperty`) names a built-in `function` and its `params`,
///     properties of the same document type, as
///     `{ "function": "sys.stringTransformations.homographSafeASCII", "params": ["label"] }`.
///     System functions are named under `sys.`, leaving other names to
///     functions a contract may bring later. The `sys.stringTransformations`
///     functions take one string and change ASCII characters only, keeping
///     every other character, without Unicode tables: `lowercase`,
///     `uppercase`, `capitalize`, `camelCase`, `snakeCase`, and
///     `homographSafeASCII`, which lowercases, then maps `o` to `0` and `i`
///     and `l` to `1`, DPNS's label normalization over ASCII. The parser
///     checks at registration and update that `params` holds as many
///     properties as the function takes, each another string property that
///     is not generated itself, that neither the property nor a param is
///     transient or inside a transient object, and that every param sits
///     inside every object holding the property; a changed declaration is an
///     incompatible schema change, and `validate_update` 1 refuses a property
///     an update adds over params that all already existed
///     (`DocumentTypeUpdateError`, 40212); meta-schema v3 refuses the keyword
///     beside `$ref`, whose definition would replace it.
///     `fill_generated_properties` (0) writes a declared property a document
///     leaves out, from its params, in the action transformers of document
///     create, replace and index-only delete, before the contest resolution
///     and every check read the data, in `Document::try_from_create_transition`
///     and `try_from_replace_transition`, and in
///     `index_only_transition_entry_path_query`, the builder the prover and
///     the verifier share, with which proofs are built and checked. The client
///     transition builders, the SDK's contest fund lookup and the JS and FFI
///     property-constraint pre-checks call `regenerate_generated_properties`
///     (same slot) instead, which replaces a value the document holds and
///     removes it when a param is absent, so a transition built from a
///     fetched and edited document carries the value of its new params and
///     its contest is detected from it.
///     `DataContract::validate_document_properties` 0 calls
///     `validate_generated_from_properties` (`validate_generated_from` 0)
///     after the schema and `maxBytes`, and refuses a supplied value that is
///     not what the function generates, one without its params, or a document
///     repeating a key on the way to the property or a param, with
///     `DocumentPropertyNotGeneratedError` (10424). Every call site was
///     extended in place and is inert before this version, where the three
///     slots are `None` and the meta-schemas refuse the keyword.
///
/// 54. **An aggregate keyword names a top-level property**: `summable` and
///     `averageable` on an index, and `documentsSummable` and
///     `documentsAverageable` on a document type, name the integer property
///     each document adds to the sum. Drive reads its value from the top level
///     of the document, but the parser resolves the name among the flattened
///     properties and required fields, which also hold the dotted path of a
///     property nested in an object, and meta-schemas v1 and v2 bound only the
///     name's length. A contract naming `payment.amount` registered, and every
///     document create of the type then failed in Drive as an internal error.
///     Meta-schema v3 (`CONTRACT_VERSIONS_V6`) gives the four keywords the
///     property-name pattern `^[a-zA-Z0-9_]{1,64}$`, so a create or an update
///     carrying a dotted name is refused under full validation
///     (`JsonSchemaError`, 10101, paid in a block; `check_tx` does not fully
///     validate a contract). A contract stored with one still loads, since a
///     stored contract is parsed without full validation, but can no longer be
///     updated; no contract on mainnet or testnet names one.
/// 55. **A preallocated index's agreement source fits a tree key**: contract
///     create and update state validation 1 refuse, paid, a
///     `propertyAgreement` pair through which a preallocated index is keyed
///     when its referenced property can hold a value over 255 bytes
///     (`ReferencedDocumentPropertyAgreementInvalidError`, 40126): creating a
///     referenced document writes that value as a tree key, which failed with
///     an internal error for a value over 255 bytes, and for any value once
///     the property's midway size, which sized the estimate, passed 255
///     bytes. `add_document_for_contract_operations` 1 now estimates that
///     layer from the referring property, as an entry insert does, and
///     preallocates nothing for a referenced value wider than the referring
///     property can hold, which no referring document can agree with.
///
/// 56. **A `propertyAgreement` pair compares values, not index keys**:
///     document reference validation 0 judges each pair as two single values
///     (`Value::same_scalar_data`): strings as text, byte arrays and
///     identifiers as bytes, integers as numbers at any width, floats by
///     their `f64` bits (an integer against a float read as the float it
///     converts to, as a `number` carried as an integer is stored), booleans
///     as booleans. An identifier or byte array carried as an array of
///     `U8`s is the bytes it lists, as before; any other array agrees with
///     nothing. It compared the two sides' index key encodings, under which
///     `""` agreed with `"\0"`, and two equal values over 255 bytes, which
///     an unindexed string of 64 characters or more can hold, were refused
///     (`ReferencedDocumentPropertyMismatchError`, 40127).
///
/// 57. **Moderator abilities, and fields only moderators write**: the two
///     doctype keywords of moderator deletion (19) become one object,
///     `moderatorAbilities` (meta-schema v3): `delete` for
///     `canBeDeletedByModerators`, `deleteWithin` for
///     `canBeDeletedByModeratorsFor`, and `changeFields`, the top-level
///     properties only the contract's moderators write. The whole object is
///     fixed with the type (40212). `deleteKeepsRecord` (default true) says
///     whether a moderator's deletion leaves its removal record: without one the
///     type has no records tree, the query refuses it, a restore is refused
///     (41119) and the deletion is proved by the document's absence, which the
///     verifier learns from the contract. `deleteRefundsOwner` (default false)
///     says whether the owner is refunded its storage instead of forfeiting it
///     (the batch then carries no `ForfeitStorageRefunds`). `deleteKeepsFields`
///     lists property paths at any depth, and the timestamps and block heights
///     the type requires, whose values the removal record keeps, copied from
///     the document as it was deleted: what stays public once it is gone. It
///     needs a record, and is fixed with the type. A record keeping any
///     carries them behind bit 1 of its tag byte, encoded as the document
///     encodes its properties (a presence byte and the value per kept path,
///     the paths themselves not written) and read under the document's type,
///     and a record keeping none is written as before; the removals
///     response carries the bytes as `kept_fields` (field 8), and a
///     `moderatedDocument` reference's `where` pair on a kept property is
///     checked against the record's value. A property `changeFields` lists must be declared,
///     optional, stored, not immutable, neither a reference nor read by one,
///     neither generated nor a generation parameter, and in no contested index,
///     on a type that is not indexOnly; a type listing any keeps `$revision` even
///     when `documentsMutable` is false, and a lookup key or a list element's
///     list may not read such a field of the type it refers to.
///     `ContractUserModeration` gains the `ChangeDocumentFields` action: a
///     moderator (for a seated team, holding the new `changeDocumentFields`
///     ability, appended to `ModerationAbility`, on the type, and citing a
///     listed reason) sets or removes those fields on any document of the type,
///     whoever owns it. The changed document is judged as a replace judges one
///     (schema, `propertyConstraints` with their totals, `distinctFrom`,
///     `encryptedFor` shapes, unique indexes through
///     `validate_moderated_document_uniqueness`, the restore's check
///     generalized and renamed), its references are not checked again, and it
///     is stored with the replace's update, `$revision` one higher and
///     `$updatedAt` untouched; the moderator pays, refunds of what the change
///     frees stay the owner's. A change that changes nothing is refused (10905),
///     and a seated team's change does not count toward its action share. An
///     elected declaration must give its team `changeDocumentFields` on every
///     type that lists fields. The proof
///     is the document as it now stands. The batch transformer (in place,
///     inert before 14) refuses a document's owner who sets, changes or removes
///     such a field without moderating the contract, in the mempool too. New errors:
///     `InvalidContractModerationDocumentFieldsError` (10905),
///     `DocumentFieldNotChangeableByModeratorsError` (41123) and
///     `DocumentModeratorFieldNotWritableError` (41124), appended.
/// 58. **The last moderator's stamp (`$moderatedAt`, `$moderatedBy`)**: two
///     system properties of a document whose type keeps fields for its
///     moderators (57), the block time and the identity of the last moderator
///     to write them. A `changeDocumentFields` sets both, and so does the batch
///     transformer (in place, inert before 14) for a create or replace whose
///     signer moderates the contract and writes such a field; a replace that
///     leaves the fields alone carries them over, and transfers, purchases,
///     price updates and restores keep them. Document serialization format 3
///     (this version's) records them behind bits 512 and 1024 of its time
///     field flags, so a document without them is written as before. Parser
///     generation 3 lets an index name either on such a type, never in a
///     unique index (10231); the shipped index key, query value and size
///     arms for the two names (`get_raw_for_document_type` v0,
///     `serialize_value_for_key` v0, Drive's estimated key sizes) are reached
///     only through such an index.
///
/// 59. **`skipIfAbsent` at any position, `true` or an array, on every type**:
///     document meta-schema v3 and the generation-3 parser take
///     `skipIfAbsent: true` (skip on every optional property of the index)
///     or an array naming the skip set, and a skip property may sit at any
///     position of the index, under a `timeRange` window too. A stored type
///     may declare it (the array may then leave some optional properties on
///     the null key), except on a contested index or next to
///     `nullSearchable: false`; on an indexOnly type the skip set is every
///     optional property of the index and each optional property needs a
///     skip index of its own, without a `timeRange`. No `rankedCountable`
///     `at` level may sit above a skip property; on a stored type a ranking
///     at a skip property may not share its level with an index keeping the
///     null layout for it, and a skip property that is a byte array needs
///     `minItems` of at least 1. The v2 insert and delete walkers write an index's
///     entry only for a document carrying its skip set and build a level
///     only when an entry of the document sits at or below it; update 1
///     moves a replaced document into or out of a skip index; every index
///     picker (server and verifier) admits a skip index only for a query
///     binding each skip property, on a stored type with a constraint no
///     missing value can meet. A contract valid before keeps its layout and
///     queries: it could only skip on an indexOnly index's first property,
///     where both rules agree.
///
/// 60. **Integer-range indexes**: an index can declare an `integerRange`
///     transform (`on`, `range`, `step`, optional `phase < step`) that
///     buckets a required user integer property of at most 64 bits into
///     windows starting at `phase + k * step`; a start below the lowest
///     value of the property's integer type is clamped to it, so every
///     value is in at least one window. It shares the time-range machinery:
///     the grid-qualified level key, the walkers' fan-out (the insert, delete
///     and update walkers read one `IndexBucketing`), the overlap cap
///     (`SystemLimits::max_time_range_overlap_factor`) and the resolution
///     provenance that keeps raw queries off a bucketed index. The v1
///     `getDocuments` handler resolves the new `IN_INTEGER_RANGE` operator,
///     a typed `IntegerRangeSelection` naming one window by its start, to a
///     window-start equality from the query alone. `unique: true` needs
///     non-overlapping windows; the uniqueness probe (v1) looks in the
///     candidate's window and lets a document change its value within its
///     own window. An indexOnly type cannot declare one (its entries would
///     collide across rows that share a window); neither kind of bucketed
///     index can be a `refersTo` lookup target or a `propertyConstraints`
///     answering index; a nested source must sit in required objects and
///     the grid-qualified level key fits 255 bytes; and a document `ttl`
///     prices every window an integer-range index writes.
///
/// 61. **A `refersTo` may find its document by a hash the document reveals**:
///     a `findBy` entry may be a function, `"<referenced property>": {
///     "function": "sys.hash.sha256d", "params": [...] }` (meta-schema v3
///     `findByFunction`, parser generation 3, `apply_property_reference` 0):
///     the document is found by that property holding the SHA-256 of the
///     SHA-256 of the params' bytes joined in order, a property path of the
///     document (the property carrying the reference included),
///     `{ "const": text }`, or `"."` for a value without a path (each element
///     of a typed array, the writer, the creator), a string counting as its
///     UTF-8, a byte array as its bytes, an identifier as its 32 bytes. The
///     parser holds the function as the lookup's computed key
///     (`LookupKeySource::Hash`). The function is `SystemFunction::Hash`, a
///     `sys.hash` namespace beside the string transformations of
///     `generatedFrom`, which refuses it. A string or byte array property may
///     now carry a `refersTo` whose function reads its value
///     (`DocumentProperty::revealed_reference`, `PropertyReference::Revealed`);
///     the property keeps its type. The document such a key finds is a
///     commitment made earlier, so the reference is judged when the document is
///     created only: its params may be transient or optional, every stored value
///     it reads must be fixed once written, and a replace leaves it alone.
///     Document create structure validation 1 refuses a create missing a param,
///     repeating a key on the way to one, or whose variable-length param holds
///     the one-byte separator that must follow it
///     (`DocumentReferencePreimageInvalidError`, 10423). Beside a `findBy`
///     function the reference may declare `minimumAgeBlocks`, judged by
///     document create state validation 2 against the found document's
///     `$createdAtBlockHeight` (`ReferencedDocumentRequirementNotMetError`,
///     40142), and `consume`, which deletes the found document with the create
///     (`DocumentCreateTransitionAction` `consumed_documents`, a batch touching
///     it elsewhere refused with 40120). The hash is computed once per key and
///     billed as `ValidationOperation::DoubleSha256` by the blocks it hashes,
///     beside the document fetch. Such a `deletableDocument` reference, judged
///     on the create alone, may sit on an `immutable` property, which
///     `validate_no_immutable_deletable_element_references` otherwise refuses.
///     A `where` entry `{"$ownerId": "$ownerId"}` makes the commitment the
///     writer's own, and `consume` requires it, into the declaring contract,
///     on a type whose owners may delete, that keeps no history, declares no
///     delete token cost or delete action fee and requires no stricter
///     signature security level than the declaring type; batch advanced
///     structure 1 refuses a contract-bound key whose bounds leave out a type
///     the created type may consume (`ContractBoundedKeyOutOfBoundsError`,
///     20014). The consumed deletes are converted with the create's own
///     operations pending, so a type may consume its own documents. The
///     `where` entries beside a function are judged with it, on the create
///     alone, so the properties they read must be fixed once written or
///     transient; on a mutable type such a reference may not be an `anyOf`
///     operand; and no property a function reads may be listed under
///     `moderatorAbilities.changeFields` (57). `creatorRefersTo` takes a
///     `deletableDocument` target through a function, and the `findBy` of an
///     `ownerRefersTo` or `creatorRefersTo` may leave the value out beside
///     one. The declaration reproduces the DPNS preorder hash of a name under
///     a parent byte for byte; the DPNS contract and its create trigger are
///     unchanged. See `book/src/data-model/documents.md`.
///
/// 62. **Null flags follow each index's own path**: the v2 index-level
///     insert and delete walkers give each sub-level the null flags of its
///     parent and its own value. They carried the flags from one sibling
///     sub-level into the next, so a unique index could store its entry in
///     the `[0]` tree meant for a missing value, and a `nullSearchable:
///     false` index an entry for a document missing all of its values,
///     because of another index's values; update 1 and the document cost
///     model already judged each index alone. Entries written before
///     (by that carrying, or by update 0, which laid out a unique index
///     with some values missing as the bare reference and never skipped a
///     `nullSearchable: false` entry) are found where they are stored: where
///     an earlier writer could disagree with the rule, the v2 delete walker
///     and update 1 read the stored `[0]`, unbilled, and remove or refresh
///     the entry there; a type with a `ttl` skips the read.
///
/// 63. **`refersTo` finds its document with `findBy` and checks it with
///     `where`**: the reference keywords the notes above describe are spelled
///     anew in meta-schema v3 and parser generation 3 (`apply_property_reference`
///     0), in place, before this version reaches a network. `lookup: { index,
///     keys }` is `findBy`, the key parts directly (`{ "<index property>":
///     <source> }`, a function entry included, see 61), without the index
///     name: the index is the unique one of the referenced document type whose
///     properties are exactly those `findBy` names, not bucketing its first
///     property (`DocumentReferenceLookup::resolve_index`, run at contract
///     parse or registration and when the document is fetched; a type's
///     indexes never change after it is registered). A plain `propertyAgreement`
///     pair `{ "<referring>": "<referenced>" }` is a `where` entry keyed the
///     other way, `{ "<referenced>": "<referring>" }`, so every map of a
///     `refersTo` is keyed by the referenced document's property; the parsed
///     model keeps `property_agreement` keyed by the referring property, and a
///     referring value may be compared once. `listElement` is a
///     `permanentDocument` with `inList`, found by `findBy { "$id":
///     "<property>" }`, the one `findBy` entry that names `$id`. Without
///     `findBy` the value is the document's `$id`, as before. The parser refuses
///     `lookup`, `propertyAgreement` and the `listElement` type on every parse,
///     naming what replaced each, so a contract written with them never loads
///     with another meaning. `ReferencedDocumentLookupInvalidError` (40137)
///     carries the properties `findBy` names in place of an index name. The
///     moderation charters system contract is written in the new keywords;
///     the parsed declarations, and so validation and execution, are
///     unchanged.
/// 64. **`refersTo: moderatedDocument`**: a third kind of document reference,
///     between `permanentDocument` and `deletableDocument` and disjoint from
///     both (`DocumentReferenceKind`, `DocumentTypeV2Getters::document_reference_kind`),
///     in place in meta-schema v3, parser generation 3 and the generation 0
///     reference validators. Its target is a document type whose documents
///     leave state only when the contract's moderators remove them, each
///     removal on the record: `canBeDeleted: false`, no `ttl`, and
///     `moderatorAbilities.delete` with `deleteKeepsRecord` not false. Such a
///     type is no longer a `deletableDocument` target
///     (`ReferencedDocumentTypeModeratedError`, 40144), and a
///     `moderatedDocument` reference to any other type is refused
///     (`ReferencedDocumentTypeNotModeratedError`, 40143), at registration
///     and at write time. The id form only: no `findBy`, `inList` or operand
///     of an expression. The document must exist when the reference is
///     written; a replace re-validates it as a permanent one (the value or a
///     property its `where` reads changed, or always for a writer gate), and a
///     value the stored document held whose document a moderator removed
///     resolves to the removal record, read and billed: a `where` pair asked
///     about again compares the record's document owner for `$ownerId` and the
///     id for `$id`, and refuses any other property
///     (`ReferencedDocumentRemovedError`, 40145); a writer gate may compare
///     only those two (parser, 10231), being asked about on every replace. A
///     value is held when it is the stored document's at its path, compared
///     through the stored values the replace action carries for a changed
///     top-level property. The write-time kind check still lets a
///     `deletableDocument` reference to a moderated type through, which a
///     contract registered before this note may hold. Chained and composite joins
///     through it prove, as one more component of the merged proof, the
///     removal records of the joined ids beside their documents, and report
///     each removed document by its record (`removed_outer_documents = 4` on
///     `ChainedDocuments`, `removed = 4` on a composite `SubQueryResult`,
///     additive); a joined id with neither a document nor a record is refused
///     as a missing permanent target is. StateError discriminants 156-158.
///
/// 65. **An index may hold a value of the document a reference points at**: an
///     index property `"<reference property>.<field>"` (`DerivedIndexProperty`,
///     `DocumentTypeV2Getters::derived_index_properties`), such as a reply's
///     `postId.$ownerId`, in place in parser generation 3 (`admit_derived_index_properties`,
///     `apply_derived_index_properties`) and `create_document_types_from_document_schemas`
///     1 (`resolve_derived_index_properties`, which gives a schema field its type on the
///     referenced type on every parse). The document never stores the value: before Drive
///     keys a document of such a type, on insert (`add_document` 1), update (`update_document`
///     1, one read for both versions) and delete (`delete_read_document`, shared by owner and
///     moderator deletes and `ttl` expiry), it reads the referenced document, billed with the
///     write, or, for a `moderatedDocument` target a moderator removed, the owner and the
///     values its removal record keeps (`ContractDocumentRemoval::kept_values`, read at a
///     path by `kept_value_at`), and puts the values into the document's properties under
///     the derived names, where `get_raw_for_document_type` 0 reads them (a missing one is
///     refused, never keyed under null) and the serialization ignores them. A create reads nothing more: the
///     document reference validation 0, given a map, records the values from the documents it
///     fetched, and the create action carries them to Drive. A dry run keys the document
///     under a value of each field's type. `serialize_value_for_key` 0,
///     `deserialize_value_for_key` 0 and Drive's estimated key sizes take a derived name's type
///     from the declaration. Registration (full validation, `InvalidContractStructure`)
///     admits one only where the value can not change once written: a same-contract
///     `permanentDocument` or `moderatedDocument` reference by id, on a reference property
///     fixed once written; `$ownerId` of a type that can not change hands, `$creatorId` of a
///     type recording it, or a stored schema property fixed once written and indexable;
///     through `moderatedDocument`, `$ownerId` or a schema property the referenced type keeps
///     under `moderatorAbilities.deleteKeepsFields` (a kept path or one inside a kept object,
///     `is_path_listed`, checked in every build); not `$id`; not in a unique or contested
///     index, or as a `timeRange` or `integerRange` source; not on an indexOnly type. A
///     `skipIfAbsent` array may name one (`reads_through_reference`), which `skipIfAbsent:
///     true` leaves out; `resolve_derived_index_properties` refuses one that is never absent
///     (a required reference reading `$ownerId`, `$creatorId`, or a field required with every
///     object around it) or a byte array that may be empty. The v2 walkers, update 1 and the
///     SDK cost walker count a null skip value as absent (`document_carries`), as a derived
///     value is when its reference or field is. A `startAt` or `startAfter` cursor, placed by
///     what the named document stores, is refused on an index whose derived properties the
///     query does not fix with `==`. Every step is inert without a derived index property,
///     which only generation 3 declares.
///
/// 66. **Properties frozen under a condition**: an `immutable` entry of
///     meta-schema v3 and parser generation 3, in place, may be
///     `{ "property", "when" }` beside a property name. The condition takes
///     the grammar of a `propertyConstraints` rule, reads no `countOf` or
///     `sumOf`, and is judged on the document the replace writes (its
///     `$updatedAt` the replace's block time, so `$updatedAt - $createdAt`
///     is the document's age), with the stored document's properties read
///     through `$old.<path>` (`STORED_DOCUMENT_PREFIX`), which only such a
///     condition may read. Document replace state validation 1, extended in
///     place, refuses a replace changing, adding or removing a property whose
///     condition holds, or faults, with `DocumentImmutablePropertyChangedError`
///     (40128); the stored properties are rebuilt from the written ones and
///     `stored_changed_values`, and the replace action's `added_data_fields`
///     is gone. `immutableAllowSetting`, which `{ "present": "$old.<p>" }`
///     now says, is refused on every parse, naming its replacement. A
///     conditional property is not fixed once written
///     (`schema_property_is_fixed_once_written`), a `deletableDocument`
///     reference by id may be listed only without a condition, and on
///     contract update (document type update validation 1) a condition is
///     kept as it is or dropped for listing the property without one.
///
/// 67. **A seated team deletes a settled document together**: past a type's
///     `deleteWithin` window no moderator deletes a document alone (41116); the
///     new `moderatorAbilities.deleteSettled: { leader, approvals,
///     approversPredateDocument }` (meta-schema
///     v3, `DocumentTypeV2::moderator_settled_deletion`, fixed with the type,
///     40212) lets the members of an elected contract's seated team delete it
///     once `approvals` of them approve, the leader among them when `leader` is
///     set. It needs `deleteWithin` and an elected declaration giving the team
///     `deleteDocuments` on the type (10231, 10900), `approvals` from 1 to the
///     members the declared team can hold (its leader,
///     `SystemLimits::max_moderation_charter_elected_members` and the
///     declaration's `maxAddedModerators`), the upper bound checked at
///     registration only; a seated team whose charter elects fewer members,
///     and so holds fewer than the rule asks for, must have all it can hold
///     approve. Its `approversPredateDocument` (default `true` when `approvals`
///     is above 1, which then needs `$createdAt` in `required` at
///     registration, 10231) counts a member the leader added only for
///     documents created after its addition (the `addedModerator`'s
///     `$createdAt` earlier than the document's): a proposal or approval by a
///     later one is refused, checked before an approval already given, and an
///     approval that reads the team drops the approval of a member taken off
///     and added again too late; the leader and the elected members always
///     count.
///     `ContractUserModeration` gains two actions (appended), shaped like a
///     token group's action: `DeleteSettledDocument` proposes the deletion, kept
///     under the contract as a team action (other tree key `24`, `M` active and
///     `X` closed, each `action id -> { I: the action, S: SumTree(member ->
///     SumItem(1)) }`, created with the contract) by an id computed from the
///     contract, the proposer, its nonce, the document and the reason,
///     naming the document as it is (its last modification and `$revision`)
///     and the reason; and
///     `ApproveTeamAction { action_id }` approves it. Nothing lapses, but an
///     approval of a document changed since (its `$revision` moved, a
///     moderator's change of its fields included) is refused. Each approval is
///     its own sum item, never rewritten; when the approvals given could meet
///     the rule the team is read and the approvals of members who left are
///     dropped, refunded to them, and the one whose counted approvals meet it
///     deletes the document as `DeleteDocument` does, moves the action with the
///     approvals that counted to the closed actions (refunding each), and counts
///     toward the action share for every counted approver. The storage refund
///     forfeiture of a moderator's deletion now takes the document operations
///     alone (the document's bytes and any index subtree the deletion empties,
///     whoever paid for them), applied as a GroveDB batch of their own when the
///     batch also frees moderation storage someone is owed (a restored removal
///     record replaced, approvals moved or dropped), which is refunded as ever.
///     The proof is the signer's approval, active or closed
///     (`VerifiedContractTeamActionSignature`, appended); the new
///     `getContractTeamActions` (each action with its approval count, the sum
///     of its approvals tree) and `getContractTeamActionSigners` queries read
///     them. New errors, appended: `DocumentTypeNotDeletableOnceSettledError`
///     (41204), `ContractModerationTeamNotSeatedError` (41205),
///     `DocumentNotSettledError` (41206), `ContractTeamActionDoesNotExistError`
///     (41207), `ContractTeamActionAlreadySignedError` (41208),
///     `SettledDeletionNotRestorableError` (41209): a deletion the team approved
///     is never restored, by the leader or any member,
///     `ContractTeamActionAlreadyCompletedError` (41210),
///     `ContractTeamActionDocumentChangedError` (41211) and
///     `ContractTeamMemberAddedAfterDocumentError` (41212).
///
/// 68. **A preallocated index may be bound through `moderatedDocument`**:
///     `Index::preallocation_bindings`, in place, binds through a same-contract
///     `moderatedDocument` reference as through a `permanentDocument` one, and a
///     binding records its kind (`PreallocationBinding::kind`). Through a moderated
///     reference a binding holds only when the removal record of the referenced
///     document keeps every key it binds, the referenced `$id`, `$ownerId` or a property
///     the referenced type lists under `moderatorAbilities.deleteKeepsFields`
///     (`is_path_listed`), never `$creatorId`
///     (`PreallocationBinding::is_kept_on_removal`). `create_document_types_from_document_schemas`
///     1 and `set_document_schema` refuse, under full validation, a preallocated index
///     with no binding that holds (`validate_preallocated_indexes_kept_on_removal`,
///     `InvalidContractStructure`); the reference validation 0 checks the key width of a
///     `where` pair only through a binding that holds; and the Drive insert of a referenced document
///     (`add_document_for_contract_operations` 1) and the document cost model
///     preallocate only through one that holds
///     (`Index::preallocation_bindings_for_target`, now given the referenced type).
///     A moderator's removal leaves the trees, like its record, and a restore,
///     which puts the document back through the create path, finds them in place.
///     Inert for every contract that could be registered before: a moderated
///     reference never bound a preallocated index.
///
/// 69. **Index entries that outlive a delete (`outlivesDelete`)**: an index
///     keyword of meta-schema v3 and parser generation 3, in place
///     (`Index::outlives_delete`, `IndexLevelTypeInfo::outlives_delete`,
///     `IndexLevel::outlives_delete_at_or_below`), admitted only on a
///     `timeRange` index with a `ttl` of an indexOnly type, without a sum and
///     on a type without `entryPayload`; every schema property must also sit
///     in an index that neither skips nor outlives deletes, the proof index
///     may not outlive deletes (`index_only_proof_index`), and the flag is
///     fixed with the index (`find_first_outlives_delete_change`). A delete
///     leaves such an index's entries to expire with their window: document
///     index-only delete state validation 0 and Drive's row-integrity gate do
///     not probe it, and the delete walkers (top and index level 2) skip it
///     (`level_removes_entry`, with `IndexLevel::cleared_on_delete_at_or_below`). The row commitment leaves `$createdAt` out when
///     every index involving it outlives deletes
///     (`index_only_row_commits_created_at`, read off the index structure's
///     root, `IndexLevel::created_at_indexed_only_by_outliving`, and shared by
///     the commitment, the delete transition's construction, advanced
///     structure validation 0, which then refuses a carried `$createdAt`, and
///     Drive's indexOnly delete, which refuses one too). Document create state
///     validation 1 and the within-batch collision tracker do not probe such
///     an index, and the indexOnly terminal insert writes over an entry
///     already standing there, without reading it. Registration requires the
///     index's key (its properties but `$createdAt`, and its terminal) to hold
///     the key of an index a delete clears that skips nothing, so no two
///     documents in state share one of its entries. Inert for every contract
///     without the keyword, which every earlier grammar refuses.
/// 70. **A contested type sums only small values**: parser generation 3, in
///     place, refuses under full validation a document type with a contested
///     index and a summed property (`summable`, `averageable`,
///     `documentsSummable` or `documentsAverageable`) unless the property's
///     schema declares a `minimum` of at least -2^27 and a `maximum` of at most
///     2^27 (`SYSTEM_LIMITS_V4.max_contested_summed_value_magnitude`, `None` in
///     the earlier tables). The end of a contest writes the winner's document
///     into the type's sums with no transition to refuse, so the values must be
///     small enough that the sums stay in `i64`, which they do short of 2^36
///     documents. A stored contract still parses.
/// 71. **Documents deleted only when consumed (`canBeDeleted:
///     "onlyWhenConsumed"`)**: a third `canBeDeleted` value of meta-schema v3
///     and parser generation 3, in place
///     (`parse_can_be_deleted_only_when_consumed_keyword`, passed to the core
///     parse as `indexOnly` is, `false` for generations 1 and 2;
///     `DocumentTypeV2Getters::documents_deleted_only_when_consumed`). The
///     owner's delete reads it as `false` (document delete advanced structure
///     validation refuses it, 10404), and a `refersTo` with `consume` may
///     target the type (`DocumentReferenceLookup::referenced_side_error`,
///     which refused every type its owner can not delete). Its documents can
///     leave state, so the type is a `deletableDocument` target, never a
///     `permanentDocument` or `moderatedDocument` one
///     (`documents_can_disappear`, `document_reference_kind`). A consumed
///     document is deleted without its owner's `canBeDeleted` guard
///     (`ForceDeleteDocument` beside a contested create,
///     `force_delete_document_for_contract_operations` in
///     `AddDocumentAndDeleteConsumed`), so Drive's delete guard stays strict
///     for the owner's delete. Refused on a type that keeps history or is
///     indexOnly (10231), and fixed on update (`validate_update` v1, 40212).
///     Inert for every contract without the value, which every earlier grammar
///     refuses.
///
/// 72. **A barred author may still retract (`retractedWhen`)**: a document
///     type of meta-schema v3 and parser generation 3, in place, may declare
///     `retractedWhen`, one condition in the grammar of an `immutable` entry's
///     `when` (`$old.` reads, no `countOf` or `sumOf`), only on a mutable type
///     of a contract keeping a banlist or a suspension list (10231 on every
///     parse), and fixed on update (document type update validation 1, 40212).
///     `contract_moderation_gate` v0, in place, lets a banned or suspended
///     signer's replaces on such a type through with its bar
///     (`ContractModerationRefusal::retraction_bar`, `refused` now optional),
///     and the shared transformer, after fetching the stored document, refuses
///     with the bar (41107, 41108), paid with the nonce bumped, each whose
///     written document does not meet the condition or whose condition
///     faults. Every other rule of the type still judges the replace. So an
///     author whose documents can not be deleted can still take one back. Inert
///     before this version: the gate and the keyword exist only here.
///
/// 73. **An index that counts another index's entries
///     (`summableOffCountIndex`)**: an index keyword of meta-schema v3 and
///     parser generation 3, in place (`Index::summable_off_count_index`,
///     `IndexLevelTypeInfo::summable_off_count_index`), admitted only on an
///     indexOnly type with `rangeSummable`, naming a source index of the type
///     that holds every document once; its other properties must be fixed by
///     the source through unchanging `where` values of same-contract
///     `permanentDocument` or `moderatedDocument` references
///     (`validate_summable_off_count_indexes_lossless`, judging a value as a
///     lookup's key part is judged, `why_value_can_change`: an optional
///     `deletableDocument` reference by id a replace may clear once its
///     document is deleted is not fixed, and from this version neither is a
///     findBy key part, a findBy function's param or a `where` value beside
///     one), and one summed value per type is kept. Such an index keeps one `Element::SumItem` per group
///     in place of a value tree and entries: the index walkers (insert and
///     delete index level 2) move it by one per document, preallocation
///     creates it at zero, and document create state validation 1, document index-only delete
///     state validation 0, the within-batch collision tracker and the proof
///     index never use it. `rankedSummable` and `rankedAverageable` gain the
///     `{ "at": ... }` form on such an index only, stamped on the index
///     levels (`IndexLevel::ranked_sum_grouping`, `ranked_average_grouping`,
///     `sum_propagating`) and laid out by Drive as sum chains, count-and-sum
///     chains where an average ranking or `rangeCountable` adds counts
///     (`property_name_tree_type_and_ranked_axes_for_level`,
///     `ranked_chain_value_tree_type`); its `rankedCountable` is parsed into
///     that Sum ranking, since a document count there is its sums (no
///     `rangeCountable` needed). Sum, average and ranked queries name
///     the source index for the summed value, and a count query reads such
///     an index's sums, its document counts: a point read
///     (`document_count_of_element`), and a ranked or having-range read on
///     its Sum secondaries (`read_axis_for`), and a range read through the
///     sum surface's range forms (`counter_sums_query`). A range total
///     through any index whose path passes through a ranked level (its own,
///     or one another index ranks at a shared level) is refused cleanly
///     (`refuse_a_range_total_through_a_ranked_index`). Drive's batch methods,
///     `apply_drive_operations` and `convert_drive_operations_to_grove_operations`
///     at version 1, refuse a batch moving one document type's counters for
///     more than one document (`refuse_repeated_counter_moves`). Needs
///     grovedb's `GROVE_V4`, which admits a bare `SumItem` under a
///     `ProvableCountProvableSumIndexedTree`. Inert for every contract without
///     the keyword, which every earlier grammar refuses. For any index, the
///     range-total verifiers at version 1 (`DRIVE_VERIFY_METHOD_VERSIONS_V3`:
///     `verify_aggregate_count_proof`, `verify_carrier_aggregate_count_proof`,
///     `verify_aggregate_sum_proof`, `verify_carrier_aggregate_sum_proof`,
///     `verify_aggregate_count_and_sum_proof` and
///     `verify_carrier_aggregate_count_and_sum_proof`) verify a proof showing
///     the range holds nothing (an equality value no document holds, or an
///     empty tree of a kind the read does not aggregate), which grovedb's
///     aggregate verifiers refuse, as a zero total or no carrier branch
///     (`or_empty_range_total`), and the unproven range totals, keyed on the
///     same verifier versions, read an absent value as zero
///     (`aggregate_or_zero_when_absent`); and
///     `verify_composite_documents_proof` 1 reads the sum-bearing items of a
///     `documentsSummable` type as documents. Their
///     version 0, which every earlier protocol version selects, refuses both
///     proofs, and the unproven total fails, as released; the prover is
///     unchanged.
///
/// 74. **Withdrawals also fit a Core-anchored limit**: pooling
///     (`pool_withdrawals_into_transactions_queue` 2, which reuses version 1's
///     pooling through a shared helper) admits withdrawals up to the smaller of
///     the daily withdrawal limit (note 4) and
///     `calculate_core_anchored_withdrawal_limit`, a stricter copy of Core v24's
///     relative net unlock rule (dash#7712) read from Core's own credit pool
///     balances at chain locked heights: the pool may drop by at most
///     `core_credit_pool_unlock_limit_percent` (15; Core allows 20) of its
///     highest balance at a window start Core may use for the unlock (Core's
///     window, `core_credit_pool_window_blocks` 576 or
///     `regtest_core_credit_pool_window_blocks` 100, back from the chain locked
///     height, up to Core's asset unlock validity, `core_expiration_blocks` 48,
///     later), at least
///     `core_credit_pool_unlock_limit_floor` (1500 Dash; Core's floor is 2000),
///     less what is queued or broadcast and not completed yet. The formula is
///     `core_credit_pool_unlock_limit` 0 in `DPP_METHOD_VERSIONS_V3`. Before
///     pooling, `scan_core_blocks_for_withdrawals` reads the Core blocks the
///     chain locked height passed (at most `core_blocks_scanned_per_block_limit`,
///     32, per block) and records each one's credit pool balance, read from the
///     block's coinbase alone (`getspecialtxes`), under the withdrawals tree. The
///     Platform-side accounting can grant more than Core will mine (an asset lock published to
///     Platform after Core mined it, a whole epoch of Core rewards minted in one
///     block); over Core's limit an unlock waits unmined and is re-signed, and
///     while Core's mempool holds more than the limit Core InstantSend-locks no
///     withdrawal at all. The balance tree is created at genesis and by
///     `transition_to_version_14`, and `cleanup_expired_locks_of_withdrawal_amounts`
///     1 prunes it by Core height.
///
/// 75. **No reference by id to an indexOnly document type**: the contract
///     reference validation 0 (`validate_data_contract_references`), in place,
///     refuses a `permanentDocument`, `deletableDocument` or
///     `moderatedDocument` reference without `findBy` (or with `inList`) whose
///     referenced document type, in the declaring contract or another, is
///     indexOnly (`ReferencedDocumentTypeIndexOnlyError`, 40146, StateError
///     discriminant 171). Such a type's documents exist only as index entries,
///     and Drive refuses to fetch one by id, so every write resolving the
///     reference failed with an internal error, dropped unpaid. A `findBy`
///     into one keeps its own refusal (40137, or 10231 in the declaring
///     contract). Inert before this version: only parser generation 3 admits
///     an indexOnly document type.
///
/// 76. **Every revealed nullifier is recorded once**: each action of an
///     outputs-only Orchard bundle reveals a nullifier (that of a dummy spend,
///     which becomes the new note's `rho`). The spends already recorded and
///     checked theirs; now `Shield`, `ShieldFromAssetLock` and
///     `ShieldFromIdentity` do too. `transform_into_action` 1 of the shield and
///     the shield from asset lock (`DRIVE_ABCI_VALIDATION_VERSIONS_V10`), and
///     `transform_into_action` 0 of the shield from identity in place, refuse a
///     nullifier repeated inside the bundle or already recorded, with
///     `NullifierAlreadySpentError`: unpaid for the first two, as for the
///     spends, and a paid nonce bump for the identity-signed one. The
///     high-level operations of the shield and the shield from asset lock 1
///     (`DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4`), and of the shield from
///     identity 0 in place, record the nullifiers. Recording them is metered
///     storage for the shield and the shield from identity; the shield from
///     asset lock's flat pool fee already prices a note and a nullifier write
///     per action. The shield from identity's admission floor
///     (`compute_shielded_identity_balance_write_fee` 0, the client's estimate
///     of its complete fee) uses versioned allowances of 400 effective bytes
///     per action and 500 flat bytes, covering the complete execution-event
///     admission estimate. Actual fees remain metered. Nullifiers revealed by
///     shields before this version are not added.
///
/// 77. **Owner identities for shared and extended-address masternodes**: from
///     v24 on, Dash Core lists shared masternodes, which have no owner, payout
///     or collateral address, and extended-address masternodes, which have a
///     `payouts` list instead of a `payoutAddress`. `create_owner_identity` 1
///     needs both addresses and fails on such a masternode with
///     `DashCoreBadResponseError`, which fails the block. With
///     `create_owner_identity` 2 and `update_masternode_identities` 1
///     (`DRIVE_ABCI_METHOD_VERSIONS_V10`), a masternode without an owner
///     address gets no owner identity, only its voter and operator identities;
///     one with an owner address and either a legacy payout address or a sole
///     payout with a matching P2PKH script gets the version 1 identity,
///     TRANSFER key id 0 and OWNER key id 1, byte for byte; other payout shapes
///     get only OWNER key id 1. Legacy payout-address rotation is unchanged.
///     Payout-list changes retain, re-enable or add the sole supported P2PKH
///     TRANSFER key and disable obsolete TRANSFER keys. Split, empty or
///     unsupported lists disable all TRANSFER authority while preserving OWNER
///     and balance. Historical updaters keep their payout-list policy. This
///     version must be active on a network before its Dash Core activates V24,
///     since earlier versions keep failing on these masternodes.
///
/// 78. **Versioned Core masternode address resolution**: `update_masternode_list` 1
///     resolves nested platform addresses first, then falls back to legacy ports,
///     before storing the masternode state. Earlier protocol versions keep their
///     flat-field interpretation. The stored layout and validator construction
///     remain unchanged: new validators read the resolved stored ports, and an
///     existing validator is refreshed on a ban, service or P2P-port change.
///     Each diff starts from the old persisted representation so transient address
///     data retained before activation cannot make a running node disagree with
///     a restarted one. Payout lists remain outside the persisted representation.
///
/// 80. **A BLS12_381 signature must verify**: `verify_identity_signed_signature`
///     1 (`STATE_TRANSITION_METHOD_VERSIONS_V2`), the signature check that
///     identity-signature validation runs for every identity-signed
///     transition, refuses a signature by a BLS12_381 key that does not verify
///     (`InvalidStateTransitionSignatureError`, unpaid, as for ECDSA keys).
///     Generation 0 refused one only when the key or the signature could not be
///     read, and earlier versions replay through it. Identity-signature
///     validation v0, in place, passes the platform version to the check; the
///     tables of every earlier version select generation 0, the code it called
///     before.
///
/// 81. **Token shielded pools**: a token configuration in format version 1
///     (`TokenConfiguration::V1`, admitted by `CONTRACT_VERSIONS_V6`'s
///     `token_configuration_format` bounds) can set `hasShieldedPool`, which
///     gives the token its own Orchard pool under
///     `[Tokens, TOKEN_SHIELDED_POOLS_KEY, token_id]` laid out like the credit
///     pool. A pooled token must leave its freeze, unfreeze and destroy-frozen-
///     funds rules unassigned, since notes have no owner to freeze. Seven batch
///     token transitions (`TokenShield`, `TokenUnshield`,
///     `TokenShieldedTransfer`, `TokenMintToPool`, `TokenBurnFromPool`,
///     `TokenClaimToPool` and `TokenDirectPurchaseToPool`, validated through
///     `DRIVE_ABCI_VALIDATION_VERSIONS_V10` and gated by
///     `TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION`) move tokens between an
///     identity balance, the supply and the pool or inside it; the identity
///     signs and pays the fee in credits, and every bundle binds its pool into
///     the Orchard sighash, since all pools share the empty-tree anchor an
///     unbound bundle would verify against: a spend bundle binds the token id
///     and the batch owner (a burn binds the burner: the batch owner, or the
///     proposer of a group action), plus the recipient and amount where tokens
///     leave the pool; an outputs-only bundle (`TokenShield`,
///     `TokenMintToPool`, `TokenClaimToPool`, `TokenDirectPurchaseToPool`),
///     whose anchor is never checked against a pool, binds a per-kind tag,
///     the token id and the batch owner (for a group action mint, the
///     proposer).
///     A batch carrying any of these bundles, or a document whose token cost
///     is paid out of a pool, has to hold the compute fee the bundles will be
///     charged (`compute_shielded_verification_fee` per bundle-carrying
///     sub-transition): the batch minimum balance pre-check v1
///     (`identity_minimum_balance_pre_check`) reserves it on top of the flat
///     per-sub-transition minimum, which is orders of magnitude smaller. A
///     batch without a bundle is asked for the flat minimum, unchanged, and
///     one that asks the contract owner to pay its gas is asked for its
///     principal alone as in item 11, the compute fee being gas. The floor
///     refuses only what fee validation would refuse later, but it refuses it
///     before the Halo 2 work: `TokenClaimToPool`'s proof is skipped in check
///     tx, since its claimable amount is only known against state, so without
///     the floor a signer between the two numbers cleared the mempool with no
///     verification run and every validator then did the verification inside
///     block validation, only to refuse the batch unpaid, leaving the same
///     bytes replayable. The same holds for the bundle of a group action's
///     non-proposing signer, whose proof check tx also skips.
///     The pool balances are a term of the token conservation check
///     (`calculate_total_tokens_balance` v1 in `DRIVE_TOKEN_METHOD_VERSIONS_V2`).
///     `record_token_shielded_pool_anchors`
///     (`DRIVE_ABCI_METHOD_VERSIONS_V10`) records and prunes the anchors of the
///     pools a block touched. The pools root tree is inserted by
///     `transition_to_version_14` and by `create_initial_state_structure` v4;
///     the six shielded queries accept an optional `token_id` to target a token
///     pool.
///
/// 82. **An expiring type sums only values that keep its sums in range**:
///     parser generation 3, in place, refuses under full validation a document
///     type with a `ttl` and a summed property (`summable`, `averageable`,
///     `documentsSummable` or `documentsAverageable`) unless the property's
///     schema declares a `minimum` of at least 0, or a `minimum` of at least
///     -2^27 and a `maximum` of at most 2^27
///     (`SYSTEM_LIMITS_V4.max_expiring_signed_summed_value_magnitude`, `None` in
///     the earlier tables). The cleanup deletes expired documents at the end of
///     a block with no transition to refuse, and removing a negative value
///     raises the sums it was in; removing values that are never negative only
///     lowers them, and values within ±2^27 keep them in `i64` short of 2^36
///     documents. A stored contract still parses. No earlier version parses
///     `ttl`.
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
///   pool side is an outputs-only Orchard bundle like `Shield`, bound to the
///   funding identity (next item), and the fee is metered plus the shielded
///   compute fee, paid from the identity.
///
/// * The credit pool's outputs-only bundles bind `kind tag || owner` into their
///   Orchard sighash (`DPP_METHOD_VERSIONS_V3` sets `credit_pool_bundle_binding`
///   to `Some(0)`): `Shield` (`0x84`) the SHA-256 of its input addresses,
///   checked by `validate_shielded_proof` v1; `ShieldFromIdentity` (`0x85`) its
///   identity id; `ShieldFromAssetLock` (`0x86`) its asset lock identifier,
///   checked by the `transform_into_action` v1 that
///   `DRIVE_ABCI_VALIDATION_VERSIONS_V10` selects. A third party can no longer
///   wrap a proved bundle in a transition of their own. v13 keeps both checks
///   unbound. A sender rebuilding the same notes (Faerie Gold) is not stopped:
///   that needs the bundles' dummy nullifiers recorded and checked.
///   `ShieldFromAssetLock` also gains transition version 1
///   (`STATE_TRANSITION_SERIALIZATION_VERSIONS_V3`), the only version 14
///   admits: version 0 is refused at decode by `active_version_range`, before
///   any proof work, uncharged and with its asset lock left unspent, so one
///   still waiting when 14 activates is not burned by the bound check.
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
/// where-clause operator enum gains `IN_TIME_RANGE = 11` and
/// `IN_INTEGER_RANGE = 12`, which pre-v14 servers reject as unknown
/// operators rather than misread (the v0 wire has neither).
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
    drive: DRIVE_VERSION_V9, // changed: drive document method versions v4 — v2 index walkers (shared-prefix aggregate indexes become insertable) + the detect_ranked_mode slot; contract method versions v4: the moderation list trees, the document removal record trees and the moderation method table; apply_drive_operations 1 (a moderator's document deletion refunds nobody for what its document operations remove unless its type sets `deleteRefundsOwner`, those operations applied as a GroveDB batch of their own when the batch also frees moderation storage someone is owed, a restored removal record replaced or team action approvals moved or dropped, which is refunded to whoever its flags name; every write of one identity balance, fee pot or prefunded specialized balance in a batch merged into one; a batch writing one token balance or supply twice refused; a batch moving one document type's summableOffCountIndex counters for more than one document refused; repaid identity debt credited to the processing fee pool); convert_drive_operations_to_grove_operations 1 (refuses that counter batch too, then converts as before); index uniqueness gains validate_moderated_document_uniqueness (a moderator's document restore or field change); vote method versions v3: the end-date cleanup of ended contested vote polls removes an end date only once none of its polls remain; token method versions v2: calculate_total_tokens_balance 1 (token shielded pool balances join token conservation) and evonode_participation_rewards 1 (an evonode's token claim covers only the epochs it read); add_contested_indices_for_contract_operations 1: a poll's last index value is a count tree
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2, // changed: saved platform state structure 1 keeps masternodes and validator sets as one aux entry each
        methods: DRIVE_ABCI_METHOD_VERSIONS_V10, // changed: records the per-block total credits history for the daily withdrawal limit; record_token_shielded_pool_anchors records and prunes the anchors of the token pools a block touched; decode_raw_state_transitions, execute_event, validate_fees_of_event and add_distribute_storage_fee_to_epochs_operations each move to 1 — the table's own per-slot comments carry the full list
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V10, // changed: contested-index cross-check + refersTo document reference validation; the ContractUserModeration gates and the batch transformer's contract_moderation_gate; a contest accepts at most max_contenders_per_contest contenders and maximum_contenders_to_consider rises to 10,000; a contender's fund doubles past 250 contenders and for every 50 more; the three shielded-fee token pool transitions gain basic structure validation and document_base_transition_state_validation 1 admits a document token cost paid from a token pool; the ShieldFromAssetLock transform_into_action 1 checks its bundle against the bound preimage
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3, // changed: prune bound for the total credits history
        query: DRIVE_ABCI_QUERY_VERSIONS_V2, // changed: ranked + boolean-HAVING routing gate; the v1 handler also resolves IN_TIME_RANGE from committed block time
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5, // changed: validate_config_update 2 admits the contract moderation declaration of config V2
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3, // changed: the indexOnly delete-by-values kind (documentIndexOnlyDelete) joins the wire; ShieldFromAssetLock moves to version 1 alone; the ContractUserModeration transition
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V2, // changed: public keys in creation may carry a budget or an expiry; verify_identity_signed_signature 1: a BLS12_381 signature must verify
        state_transitions: STATE_TRANSITION_VERSIONS_V4,
        contract_versions: CONTRACT_VERSIONS_V6, // changed: token_configuration_format max_version 1 admits the shielded pool opt-in; v3 document meta-schema hosts the ranked, refersTo, requiredSince and timeRange keywords; validate_structure_interval v1 rejects a zero epoch interval; config max_version 2 (the contract moderation declaration) and validate_moderation_config
        document_versions: DOCUMENT_VERSIONS_V4, // changed: document serialization format 3 — the contract version stamp that enables `requiredSince` properties
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V3, // changed: distribution_function_evaluate v1 — deterministic libm for token reward math; reward_distribution_max_cycle_moment v1: the epoch claim cap no longer wraps; distribution_function_cycle_epochs v1: evonode cycles weighted by the epochs they span
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3, // changed: daily_withdrawal_limit v2 — a percentage of the total credits a day ago; credit_pool_bundle_binding Some(0) — the credit pool's outputs-only bundles bind a kind tag and their owner
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3, // changed: DashPay v2 adds profile payment address fields (DIP-33); withdrawals v2 admits the terminal FAILED status
    // The TTL ephemeral-bytes rate (270 credits/byte to processing) rides
    // the shared storage table; it is dead below v14 (the `ttl` grammar
    // does not parse), so no table fork is needed.
    fee_version: FEE_VERSION3, // changed: contested document contribution reduced to 0.1 DASH; masternode vote cost reduced to 0.00002 DASH; moderation election fund of 0.5 DASH; a contender's fund doubles past 250 contenders and for every 50 more; registration surcharge for once-per-identity token distributions
    system_limits: SYSTEM_LIMITS_V4, // changed: daily withdrawal limit becomes 15% of the total credits a day ago + time-range overlap-factor cap (24) + time-range TTL cap (1 week) and per-write drop cap (32); max_contract_moderators, max_contract_suspension_until, max_contract_moderation_reason_length, max_contract_warnings_per_identity, max_contract_moderation_reason_documents and contract_document_restore_window_ms (a week); max_contenders_per_contest (1,000)
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
            assert_eq!(
                (
                    fund_fees.contested_document_contenders_before_fund_doubling,
                    fund_fees.contested_document_contenders_per_fund_doubling
                ),
                (0, 0),
                "protocol {protocol_version}: every contender paid the same fund"
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
        // The fund a contender pays doubles once the contest holds 250 contenders, and again for
        // every 50 more
        expected_fees
            .vote_resolution_fund_fees
            .contested_document_contenders_before_fund_doubling = 250;
        expected_fees
            .vote_resolution_fund_fees
            .contested_document_contenders_per_fund_doubling = 50;
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

    /// The ranked index keywords are gated by the meta-schema version, so v14
    /// must select meta-schema v3 while v13 stays on v2.
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
