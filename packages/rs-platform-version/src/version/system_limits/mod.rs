pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

#[derive(Clone, Debug, Default)]
pub struct SystemLimits {
    pub estimated_contract_max_serialized_size: u16,
    pub max_field_value_size: u32,
    /// Maximum number of nested map/array containers in document properties.
    ///
    /// `None` preserves the behavior of protocol versions that predate this limit.
    pub max_document_value_depth: Option<u16>,
    /// Maximum `maxItems` a typed array document property (`type: "array"` with an `items`
    /// element schema) may declare, enforced when a contract is registered or updated (every
    /// parse requires `maxItems`; full validation refuses one above this). The bound keeps an
    /// array's worst-case encoded size, which fee estimation charges by, small. Read by
    /// document type parser generation 3 (protocol version 14), the only generation that
    /// parses typed arrays, and never reached before.
    pub max_typed_array_items: u16,
    /// Maximum number of references one document of a document type may carry, counted at
    /// contract registration or update from the type's `refersTo` declarations: one for each
    /// property that declares one (an identifier, or a key id carrying a key reference), one
    /// for the type's `ownerRefersTo`, and `maxItems` for each typed array whose identifier
    /// elements declare one. Every reference is checked against state when the
    /// document is created or replaced, each check a billed read, so this bounds the reads one
    /// document write can cause; without it a type could declare many typed arrays of
    /// `max_typed_array_items` references each. Refused under full validation only, like
    /// `max_typed_array_items`. Read by document type parser generation 3 (protocol version
    /// 14), the only generation that parses `refersTo`, and never reached before.
    pub max_references_per_document: u16,
    /// Maximum number of operands one `anyOf` or `allOf` list of a `refersTo` reference
    /// expression may hold (it holds at least two). Every leaf may be read for each value the
    /// declaration covers when the document is written, and each counts against
    /// `max_references_per_document`; this keeps one list from spending the whole budget on
    /// alternatives. Refused under full validation only, like `max_typed_array_items`. Read by
    /// document type parser generation 3 (protocol version 14), the only generation that parses
    /// `refersTo`, and never reached before.
    pub max_reference_operands: u16,
    /// Maximum number of `anyOf` / `allOf` combinators on any path from a `refersTo` reference
    /// expression to one of its leaves (a flat `anyOf` is 1). Refused under full validation
    /// only, like `max_reference_operands`. Must stay at most
    /// `dpp`'s `MAX_REFERENCE_EXPRESSION_DECODE_DEPTH` (16), the nesting a decoder of a
    /// consensus error carrying the declaration accepts; a test there holds every version to
    /// it. Read by document type parser generation 3 (protocol version 14) and never reached
    /// before.
    pub max_reference_expression_depth: u16,
    /// Maximum number of named rules one document type's `propertyConstraints` may
    /// declare. Every rule is evaluated on each create and replace of a document of the
    /// type, so this and `max_property_constraint_nodes` are what bound the arithmetic one
    /// document write causes, and `max_property_constraint_aggregates` the state it reads. Refused under full validation only,
    /// like `max_typed_array_items`. Read by document type parser generation 3 (protocol
    /// version 14), the only generation that parses `propertyConstraints`, and never
    /// reached before.
    pub max_property_constraints: u16,
    /// Maximum number of nodes in one `propertyConstraints` rule: every comparison, every
    /// `in` and each value it lists (a `notIn` costing what its `in` costs), every
    /// `contains`, `startsWith`, `endsWith`, `present` or `absent`, every `anyOf`, `allOf`,
    /// `not`, `ifThen` or `ifThenElse`, every arithmetic operator (`min`, `max` and `abs`
    /// included) and every operand: an integer value, a `const`, a property, a size
    /// (`length`, `byteLength`, `count`) or a system time or height. An `ifAbsent` operand
    /// is one node, the default it gives included.
    /// Refused under full validation only, like `max_property_constraints`. Read by document
    /// type parser generation 3 (protocol version 14) and never reached before.
    pub max_property_constraint_nodes: u16,
    /// Maximum number of distinct `countOf` and `sumOf` totals the `propertyConstraints`
    /// rules of one document type read. Each is a billed read of a count or sum tree on
    /// every create or replace of a document of the type, and on a transfer, a purchase or
    /// a price update judged against a rule reading it, so this bounds the state one
    /// document write reads for its rules. A total two rules read alike counts once. Refused under full validation
    /// only, like `max_property_constraints`. Read by document type parser generation 3
    /// (protocol version 14) and never reached before.
    pub max_property_constraint_aggregates: u16,
    /// Max size of a state transition in bytes.
    ///
    /// NOTE: This must be equal to the `max-tx-bytes` in the Tenderdash config
    pub max_state_transition_size: u64,
    /// Maximum number of batched transitions (document and token transitions counted together)
    /// one batch state transition may carry.
    ///
    /// This cap is load-bearing for state correctness, not merely a size or throughput limit.
    /// `BatchTransitionAction::into_high_level_drive_operations` flattens every transition of a
    /// batch into one `Vec<DriveOperation>`, and `apply_drive_operations` turns that vector into
    /// a single GroveDB batch. Within one such batch the ordinary document Add/Update/Delete
    /// conversions are blind to each other: the check that decides whether an index group tree
    /// has become empty and should be removed sees committed state plus only the operations of
    /// its own conversion.
    ///
    /// While the cap is 1, no two document operations can share a GroveDB batch *by way of a
    /// batch state transition*, so on that route the blindness has nothing to act on. Raise it
    /// and two operations that jointly empty a group each observe the other's document still
    /// committed, each conclude the group is not yet empty, and the group tree survives with no
    /// documents behind it. On a ranked index that leftover tree is mirrored into the aggregate
    /// secondary, so the group keeps ranking with a zero aggregate — sorting ahead of every
    /// group with a positive one — and, because primary and secondary agree that the empty
    /// group exists, the state is internally consistent: integrity verification passes and
    /// proofs attest the wrong ranking against the live root hash.
    ///
    /// The cap is not the only thing standing between that machinery and a live path, and a
    /// reader raising it needs to know what the others are:
    ///
    /// * `Drive::update_contract_keywords_operations` puts N blind document deletes and M adds
    ///   in one batch over a single shared index group. Every batch it actually emits refills
    ///   the group it empties, but only because its caller skips it outright when the new
    ///   keyword set is empty — and that skip is a shield, not a fix. Called directly with an
    ///   empty set it does strand the group, and the skip leaves the old keyword documents in
    ///   place, so a contract that clears its keywords keeps being found under them. Both
    ///   halves are pinned by tests; see the call site in `update_contract_v1`.
    /// * `DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType` threads
    ///   the accumulated operations through, so document operations in *that* variant do see
    ///   their siblings — which is why the withdrawal paths batch many documents safely. It is
    ///   not a drop-in for batch transitions: it carries no delete variant.
    /// * A `summableOffCountIndex` counter is read and rewritten by each document conversion
    ///   (`Drive::add_summable_off_count_counter_operations`). Two documents of one batch in one
    ///   counter group, which a source keyed by more than its owner admits (a terminal such as
    ///   `["$ownerId", "emoji"]`), would each read the stored count and write the same next
    ///   value, losing one move; the group's last delete would then find the counter at zero
    ///   and fail. Protocol version 14's batch methods refuse such a batch instead
    ///   (`Drive::refuse_repeated_counter_moves`), but per document type, not per group, and
    ///   as an internal error: with the cap raised, two likes of different posts, or two posts
    ///   each preallocating their own counters, would pass validation (the within-batch entry
    ///   tracker does not claim counter groups) and then fail as an internal error, dropping a
    ///   valid transition unpaid. Raising the cap therefore needs the counter moves folded
    ///   across the documents of a batch (one read and one write per counter) in place of
    ///   that refusal.
    ///
    /// * A token shielded pool leans on the cap twice, and neither is visible from the pool's
    ///   own code. Its balance write is absolute rather than a delta, so two pool operations in
    ///   one batch would silently discard the first — value lost on every node, no disagreement
    ///   to notice. And an outputs-only bundle's sighash binds the owner, but nothing that
    ///   tells one of that owner's transitions from another, while its anchor is never checked
    ///   against a pool at all — so the same authorized bytes can sit in two shields of one
    ///   batch: state validation runs per transition against the transaction before any
    ///   operation applies, so the second cannot see the first's pending insert, and the
    ///   within-bundle check is scoped to one action set. The two inserts are then byte-identical
    ///   in path, key and value, which a node running the shipped batching default folds in
    ///   silence while a node verifying batch
    ///   consistency refuses — the two disagree on one block and neither shows why. Raising the
    ///   cap means batch-scoped nullifier deduplication and a delta-based pool balance write,
    ///   not just making the ignored cases above pass.
    ///
    /// * The batch minimum balance pre-check reserves the compute fee of every shielded pool
    ///   bundle a batch carries, on top of `document_batch_sub_transition` per sub-transition.
    ///   It refuses nobody who could have paid only because a bundle-carrying sub-transition's
    ///   metered fee is itself far above that flat minimum: the band the floor newly refuses is
    ///   `metered_fee < flat_minimum` wide, and one pool action's ~550 metered storage bytes
    ///   price it two orders of magnitude above the 100,000 flat minimum, so the band is empty.
    ///   A cap above one does not by itself change that — the floor and the charge are both
    ///   per sub-transition — but a later change that lets a batch carry a bundle alongside
    ///   sub-transitions cheaper than the flat minimum would reopen it, and the floor would
    ///   then start refusing batches that fee validation would have executed. Recheck the
    ///   inequality rather than assuming it.
    ///
    /// Five cases in `rs-drive`'s `batched_group_drain` suite are `#[ignore]`d for exactly this
    /// reason; the rest of that suite runs. Anyone raising this cap should un-ignore those five
    /// first and make them pass.
    pub max_transitions_in_documents_batch: u16,
    pub withdrawal_transactions_per_block_limit: u16,
    pub retry_signing_expired_withdrawal_documents_per_block_limit: u16,
    pub max_withdrawal_amount: u64,
    /// Daily withdrawal limit as a percentage of the total credits Platform held a day ago.
    /// From protocol version 14 Platform pools at most this share of the total credits recorded
    /// at the latest block at least 24 hours before the current one into asset unlock
    /// transactions per 24 hours (`daily_withdrawal_limit` method version 2; the history is
    /// kept by `record_total_credits_history_for_withdrawals`). `None` for the protocol versions
    /// that predate the rule: method version 0 derived the limit from the current total, method
    /// version 1 applied a flat 2000 Dash. Versioned: see `daily_withdrawal_limit_percent` in
    /// each `SYSTEM_LIMITS_V*`.
    pub daily_withdrawal_limit_percent: Option<u8>,
    /// Allowed drop of Core's credit pool per window, as a percentage of its balance at the
    /// window start, in the Core-anchored withdrawal limit of protocol version 14, read by
    /// `core_credit_pool_unlock_limit` method version 0. Platform pools a withdrawal only while
    /// it also fits this limit, a stricter copy of Core v24's own unlock rule (20%, at least
    /// 2000 Dash), so it does not pool more than Core will mine. `None` for the protocol
    /// versions that predate the Core-anchored limit.
    pub core_credit_pool_unlock_limit_percent: Option<u8>,
    /// Smallest allowed drop (in credits) of Core's credit pool per window in the Core-anchored
    /// withdrawal limit, applied when `core_credit_pool_unlock_limit_percent` of the window start
    /// balance is less; read by `core_credit_pool_unlock_limit` method version 0. Below Core
    /// v24's own 2000 Dash floor, so small pools keep a margin too, and at least
    /// `max_withdrawal_amount` so a queued withdrawal always fits eventually. `None` for the
    /// protocol versions that predate the Core-anchored limit.
    pub core_credit_pool_unlock_limit_floor: Option<u64>,
    /// Core's credit pool window on mainnet, testnet and devnets (`CreditPoolPeriodBlocks` in
    /// Dash Core's chain parameters): how many Core blocks before an asset unlock's block lies
    /// the balance Core v24 measures the unlock limit from. The Core-anchored withdrawal limit
    /// reads its window starts this far back, up to Core's asset unlock validity
    /// (`withdrawal_constants.core_expiration_blocks`) later, so the window must be at least
    /// that long, and recorded balances older than it are pruned. Read through
    /// `core_credit_pool_window_blocks` in dpp. `None` for the protocol versions that predate
    /// the Core-anchored limit.
    pub core_credit_pool_window_blocks: Option<u32>,
    /// Core's credit pool window on regtest, which Dash Core shortens; see
    /// `core_credit_pool_window_blocks`. `None` for the protocol versions that predate the
    /// Core-anchored limit.
    pub regtest_core_credit_pool_window_blocks: Option<u32>,
    /// Minimum net amount (in credits) a withdrawal may send to Core, shared by the
    /// transparent (identity + address) and shielded withdrawal paths. The dust floor that
    /// keeps Core from rejecting the resulting `TxOut`. Versioned: see `min_withdrawal_amount`
    /// in each `SYSTEM_LIMITS_V*`.
    pub min_withdrawal_amount: u64,
    /// Core's dust relay fee rate in duffs per kilobyte, from which the per-output dust
    /// threshold Core's mempool enforces is derived (Core's `GetDustThreshold`: the fee at
    /// this rate of the serialized output plus the input that would spend it, 546 duffs for
    /// a P2PKH output at the default 3000 duffs/kB). From protocol version 14 an expired
    /// withdrawal whose whole amount is below the threshold of its output script is marked
    /// FAILED instead of being re-signed forever (`rebroadcast_expired_withdrawal_documents`
    /// method version 2). `None` for the protocol versions that predate the rule.
    pub core_dust_relay_fee_per_kb: Option<u64>,
    /// Maximum Core transaction fee rate, in duffs per byte, accepted for a withdrawal.
    /// `None` preserves the behavior of protocol versions that predate this limit.
    pub max_core_fee_per_byte: Option<u32>,
    /// Maximum number of members a change-control `Group` declared inside a data contract may
    /// have (the groups token change-control rules delegate to). Not to be confused with
    /// contract groups, the identity-owned sets of contracts below.
    pub max_group_member_count: u16,
    /// Maximum number of contract group memberships one data contract create transition may
    /// declare. Contract groups exist from protocol version 14; earlier versions never reach
    /// the check.
    pub max_contract_group_memberships_per_contract: u16,
    /// Maximum number of admins a contract group may name besides its owner.
    pub max_contract_group_admins: u16,
    /// Maximum length, in characters, of a contract group name.
    pub max_contract_group_name_length: u16,
    /// Maximum length, in characters, of a contract group description.
    pub max_contract_group_description_length: u16,
    /// Maximum number of moderator identities a moderated data contract may name
    /// (`DataContractConfigV2::moderation`); the owner counts when it is named, and moderates
    /// without being named. Contract moderation exists from protocol
    /// version 14; read by the contract's `validate_moderation_config` v0 and never reached
    /// before.
    pub max_contract_moderators: u16,
    /// Latest block time, in milliseconds, a contract suspension may run until: 2^53 - 1, the
    /// largest integer JSON and JavaScript numbers hold exactly, which is how `until` travels
    /// to clients. Read by the `ContractUserModeration` basic structure validation v0
    /// (protocol version 14) and never reached before.
    pub max_contract_suspension_until: u64,
    /// Maximum length, in bytes of UTF-8, of the text of the reason a ban, a suspension, a
    /// warning or a moderator's document deletion carries (`ContractModerationReason::text`). Read by the `ContractUserModeration` basic
    /// structure validation v0 (protocol version 14) and never reached before.
    pub max_contract_moderation_reason_length: u16,
    /// Maximum number of warnings one identity may carry on a contract's warning list at a
    /// time: a warn that would exceed it is refused until the warnings are cleared. Read by
    /// the `ContractUserModeration` state validation v0 (protocol version 14) and never
    /// reached before.
    pub max_contract_warnings_per_identity: u16,
    /// Maximum number of documents a contract moderation reason may cite
    /// (`ContractModerationReason::documents`). Read by the reason's validation (protocol
    /// version 14) and never reached before.
    pub max_contract_moderation_reason_documents: u16,
    /// Shortest join window and vote window, in seconds, an elected moderation team
    /// declaration (`ContractModerators::Elected`) may set on mainnet: one day. Every other
    /// network has no floor, a window of 0 included, so test elections resolve at once. Read
    /// by the contract's `validate_moderation_config` v0 (protocol version 14) and never
    /// reached before.
    pub min_mainnet_contract_moderation_election_window_seconds: u32,
    /// Longest join window and vote window, in seconds, such a declaration may set: four
    /// weeks.
    pub max_contract_moderation_election_window_seconds: u32,
    /// Shortest challenge cool-down, in seconds, such a declaration may set: two weeks. The
    /// cool-down is how long a seated team is safe from a challenge after a seat change.
    pub min_contract_moderation_challenge_cool_down_seconds: u32,
    /// Longest challenge cool-down, in seconds, such a declaration may set: three years.
    pub max_contract_moderation_challenge_cool_down_seconds: u32,
    /// How long after a moderator's deletion of a document, in milliseconds of block time,
    /// the contract's moderators may restore it (`ContractUserModeration`'s `RestoreDocument`
    /// action): a week. Read by the `ContractUserModeration` state validation v0 (protocol
    /// version 14) and never reached before.
    pub contract_document_restore_window_ms: u64,
    /// Most members an elected moderation declaration may let a seated team's leader add
    /// after the election (`maxAddedModerators`). Read by the declaration's validation
    /// (protocol version 14) and never reached before.
    pub max_contract_moderation_added_moderators: u16,
    /// Most members a moderation charter elects beside its leader: the `maxItems` of the
    /// moderation charters contract's `electedCharter.members`, which must stay equal to it.
    /// With the leader and the members an elected declaration lets the leader add
    /// (`maxAddedModerators`), it bounds how many members of a seated team a document type's
    /// `moderatorAbilities.deleteSettled` may require to approve the deletion of a settled
    /// document. Refused under full validation only, so a stored contract stays readable if it
    /// ever shrinks. Read by the document type parser (protocol version 14) and never reached
    /// before.
    pub max_moderation_charter_elected_members: u16,
    /// Most contenders one contested document resource vote poll accepts: a document that
    /// would add one more is refused. The end of a poll tallies, and cleans up, every
    /// contender in one block, so this bounds that work; `maximum_contenders_to_consider`
    /// must stay at least this where it is read. Read by the contested document create
    /// state validation v2 (protocol version 14) and never reached before.
    pub max_contenders_per_contest: u16,
    // This the max redemption cycles we can process if we don't use a constant distribution
    // For a constant perpetual distribution this is very cheap since it's just a multiplication
    // For other distributions we much calculate at each cycle the rewards, so we don't want to
    // do this that much
    pub max_token_redemption_cycles: u32,
    /// Most finalized epochs one `EvonodesByParticipation` perpetual distribution claim reads
    /// to weigh the claimant's participation, unless one cycle of the distribution spans more
    /// epochs, in which case the claim reads that one whole cycle. The claim pays only through
    /// the last whole cycle it read, so an evonode further behind is paid over several claims.
    /// Read by `evonode_participation_rewards` v1 (protocol version 14); 100 in every table,
    /// the bound the read was held to before (`drive_abci.query.max_returned_elements`), which
    /// v0 keeps reading.
    pub max_evonode_reward_claim_epochs: u16,
    pub max_shielded_transition_actions: u16,
    /// Highest `minimumPoolNotesForOutgoing` a token's configuration may set. The threshold
    /// refuses outflows from the token's shielded pool while the pool holds fewer notes, so
    /// without an upper bound an issuer could set one no pool ever reaches and strand every
    /// holder's shielded balance, irreversibly on a readonly contract. Read by the token
    /// configuration validation of contract create and update and by `TokenConfigUpdate`
    /// (protocol version 14), which never reach it before.
    pub max_token_pool_notes_for_outgoing: u64,
    /// Maximum overlap factor (`range / step`) a `timeRange` index transform
    /// may declare, enforced at contract registration.
    ///
    /// The overlap factor is the number of buckets that contain any given
    /// timestamp — i.e. the write amplification of the index: every document
    /// insert, delete, and (on a bucket-set change) update fans out into that
    /// many index entries. The bound of 24 covers the natural worst case, a
    /// day-long window sliding hourly, without letting a contract buy a
    /// 256-entry fan-out per document.
    ///
    /// `None` preserves the behavior of protocol versions that predate
    /// time-range indexes (nothing to bound: the `timeRange` keyword does not
    /// parse there).
    pub max_time_range_overlap_factor: Option<u64>,
    /// Maximum time-to-live (in seconds) a `timeRange` index transform may
    /// declare, enforced at contract registration.
    ///
    /// The cap is what makes the TTL fee model safe: entries under a TTL'd
    /// index bill their bytes as processing (the ephemeral-bytes rate)
    /// instead of storage, and a flat rate is only an honest price while
    /// the lifetime it covers is bounded. One week in V4.
    /// See `book/src/drive/time-range-ttl.md`.
    ///
    /// `None` preserves the behavior of protocol versions that predate the
    /// `ttl` key (nothing to bound: the key does not parse there).
    pub max_time_range_ttl_seconds: Option<u64>,
    /// Minimum per-write drainage budget for a TTL'd time-range grid.
    /// Drive raises this floor to twice the maximum trees one document
    /// can create in the grid's merged index structure, times its overlap
    /// factor. This gives cleanup capacity above the tree creation rate,
    /// including shared grids and deep suffixes. Each drop is O(1).
    /// `None` disables cleanup on versions predating the `ttl` key.
    pub min_time_range_ttl_drop_operations_per_write: Option<u16>,
    /// Minimum time to live, in seconds, a document type may declare with its `ttl`
    /// keyword, enforced when a contract is registered or updated (full validation only,
    /// like `max_document_ttl_seconds`). A document the cleanup deletes before its writer
    /// has fetched the proof of its create would fail that proof's verification (it proves
    /// the document present); the floor keeps every document well past that point. Read by
    /// document type parser generation 3 (protocol version 14).
    ///
    /// `None` preserves the behavior of protocol versions that predate the keyword.
    pub min_document_ttl_seconds: Option<u32>,
    /// Maximum time to live, in seconds, a document type may declare with its `ttl`
    /// keyword, enforced when a contract is registered or updated (full validation only,
    /// like `max_typed_array_items`). Documents of a type with a `ttl` are deleted by the
    /// platform once `$createdAt + ttl` has passed; the cap bounds how long the flagless,
    /// prepaid storage of such a document can live, which is what the per-period price of
    /// the fee schedule's `document_ttl` group is calibrated for. Read by document type
    /// parser generation 3 (protocol version 14), the only generation that parses `ttl`.
    ///
    /// `None` preserves the behavior of protocol versions that predate the keyword
    /// (nothing to bound: it does not parse there).
    pub max_document_ttl_seconds: Option<u32>,
    /// Maximum number of expired documents the platform deletes in one block, after the
    /// block's state transitions (`expire_documents` v0). Expirations beyond it wait for
    /// the next block, oldest first. Bounds the unbilled work the cleanup adds to a block.
    ///
    /// 0 on protocol versions that predate document expiry, where the event does not run
    /// (`expire_documents` is `None` in their method tables).
    pub max_document_expirations_per_block: u16,
    /// The most work the document expiry cleanup does in one block, beside
    /// `max_document_expirations_per_block`: each deleted document weighs 1 plus the weighted
    /// index levels of its type (every index counts its properties, times the overlapping
    /// windows of a `timeRange` index), the measure its prepaid deletion fee is sized by.
    /// The cleanup stops before a document that would pass it, except the block's first, so
    /// the backlog always drains.
    ///
    /// 0 on protocol versions that predate document expiry, where the event does not run.
    pub max_document_expiration_weight_per_block: u32,
    /// The largest magnitude a summed property may admit on a document type with a
    /// contested index, enforced when a contract is registered or updated (full validation
    /// only, like `max_document_ttl_seconds`): the property's schema must declare a
    /// `maximum` of at most this and a `minimum` of at least its negation. The end of a
    /// contest writes the winner's document into the type's sums with no transition to
    /// refuse, so the values must be small enough that the sums stay in `i64`: with every
    /// value this small, a sum of fewer than 2^36 documents does. Read by document type
    /// parser generation 3 (protocol version 14).
    ///
    /// `None` preserves the behavior of protocol versions whose parsers do not read it.
    pub max_contested_summed_value_magnitude: Option<u64>,
    /// The largest magnitude a summed property whose values may be negative may admit on a
    /// document type with a `ttl`, enforced when a contract is registered or updated (full
    /// validation only, like `max_document_ttl_seconds`): unless the property's schema
    /// declares a `minimum` of at least 0, it must declare a `maximum` of at most this and a
    /// `minimum` of at least its negation. The platform deletes expired documents at the end
    /// of a block with no transition to refuse, and removing a negative value raises every
    /// sum it was in, so the values must be small enough that the sums stay in `i64`: with
    /// every value this small, a sum of fewer than 2^36 documents does. Removing values that
    /// are never negative only lowers sums. Read by document type parser generation 3
    /// (protocol version 14).
    ///
    /// `None` preserves the behavior of protocol versions whose parsers do not read it.
    pub max_expiring_signed_summed_value_magnitude: Option<u64>,
}

#[cfg(test)]
mod tests {
    use crate::version::protocol_version::PLATFORM_VERSIONS;
    use crate::version::{PlatformVersion, LATEST_VERSION};

    /// The cap is what keeps two document operations out of a shared GroveDB batch, and with
    /// them the phantom index groups described on `max_transitions_in_documents_batch`. It has
    /// been 1 since the first mainnet release; a version that relaxes it must be a deliberate,
    /// reviewed decision rather than a copy-paste into a new `SYSTEM_LIMITS_V*`.
    #[test]
    fn documents_batch_is_capped_at_one_transition_at_every_protocol_version() {
        // The loop below only inspects what the registry holds, so the registry
        // has to be known complete first. `LATEST_VERSION` is declared
        // independently of `PLATFORM_VERSIONS`, which is what makes it a usable
        // reference point: a version that is declared but never added to the
        // registry leaves the count short and fails here, and so does a registry
        // that loses entries. Deriving the expectation from the registry itself
        // — `PlatformVersion::latest()` is `PLATFORM_VERSIONS.last()` — would
        // pass in both cases.
        assert_eq!(
            PLATFORM_VERSIONS.len(),
            LATEST_VERSION as usize,
            "the protocol version registry does not hold every declared version, so the cap \
             would go unchecked on the ones it is missing"
        );
        for platform_version in PLATFORM_VERSIONS {
            assert_eq!(
                platform_version
                    .system_limits
                    .max_transitions_in_documents_batch,
                1,
                "protocol version {} allows more than one transition per documents batch; \
                 token shielded pools rely on this cap for two separate properties, so read \
                 the documentation on SystemLimits::max_transitions_in_documents_batch \
                 for what that exposes",
                platform_version.protocol_version
            );
        }
    }

    /// The mock versions are never live, but they do execute state transitions
    /// in drive-abci's protocol-upgrade suite, and one of them hand-writes its
    /// `SystemLimits` rather than reusing a `SYSTEM_LIMITS_V*` — so it is the
    /// one place the loop above cannot reach. A mock at a raised cap would
    /// surface the phantom-group defect there as an unexplained failure.
    ///
    /// `PLATFORM_TEST_VERSIONS` is a process-global `OnceLock`, so if another
    /// test in this binary initialised it first this asserts over whatever is
    /// actually in use rather than over the defaults named here. That is the
    /// more useful of the two, and deliberate.
    #[cfg(feature = "mock-versions")]
    #[test]
    fn mock_platform_versions_carry_the_same_documents_batch_cap() {
        use crate::version::mocks::v2_test::TEST_PLATFORM_V2;
        use crate::version::mocks::v3_test::TEST_PLATFORM_V3;
        use crate::version::protocol_version::PLATFORM_TEST_VERSIONS;

        let versions =
            PLATFORM_TEST_VERSIONS.get_or_init(|| vec![TEST_PLATFORM_V2, TEST_PLATFORM_V3]);
        assert!(
            !versions.is_empty(),
            "the mock version registry is empty; this test would assert nothing"
        );
        for platform_version in versions {
            assert_eq!(
                platform_version
                    .system_limits
                    .max_transitions_in_documents_batch,
                1,
                "mock platform version {} allows more than one transition per documents \
                 batch; token shielded pools rely on this cap for two separate properties, so \
                 read SystemLimits::max_transitions_in_documents_batch",
                platform_version.protocol_version
            );
        }
    }

    #[test]
    fn document_value_depth_limit_starts_at_protocol_version_13() {
        // v12 is already active on live networks, so the limit must not apply there.
        assert_eq!(
            PlatformVersion::get(12)
                .expect("protocol version 12 should exist")
                .system_limits
                .max_document_value_depth,
            None
        );
        assert_eq!(
            PlatformVersion::get(13)
                .expect("protocol version 13 should exist")
                .system_limits
                .max_document_value_depth,
            Some(256)
        );
    }

    /// The Core-anchored withdrawal limit never drops below its floor, and pooling stops at the
    /// first queued withdrawal that does not fit: a floor below one maximal withdrawal would let
    /// a maximal withdrawal wait forever on a small credit pool, with everything queued behind
    /// it.
    #[test]
    fn should_keep_the_core_credit_pool_floor_at_least_one_maximal_withdrawal() {
        let with_a_floor: Vec<_> = PLATFORM_VERSIONS
            .iter()
            .filter_map(|platform_version| {
                platform_version
                    .system_limits
                    .core_credit_pool_unlock_limit_floor
                    .map(|floor| (platform_version, floor))
            })
            .collect();
        assert!(
            !with_a_floor.is_empty(),
            "no protocol version sets a Core credit pool floor; this test would assert nothing"
        );
        for (platform_version, floor) in with_a_floor {
            assert!(
                floor >= platform_version.system_limits.max_withdrawal_amount,
                "protocol version {} sets a Core credit pool floor of {} credits, below one \
                 maximal withdrawal ({} credits)",
                platform_version.protocol_version,
                floor,
                platform_version.system_limits.max_withdrawal_amount
            );
        }
    }

    /// The Core-anchored withdrawal limit reads window starts from the chain locked height back
    /// by Core's credit pool window up to Core's asset unlock validity later. A window shorter
    /// than that validity would put the nearest window start above the chain locked height,
    /// whose balance is not final and so not the same on every node.
    #[test]
    fn should_keep_every_core_credit_pool_window_at_least_the_unlock_validity() {
        let with_a_window: Vec<_> = PLATFORM_VERSIONS
            .iter()
            .flat_map(|platform_version| {
                let system_limits = &platform_version.system_limits;
                [
                    system_limits.core_credit_pool_window_blocks,
                    system_limits.regtest_core_credit_pool_window_blocks,
                ]
                .into_iter()
                .flatten()
                .map(move |window_blocks| (platform_version, window_blocks))
            })
            .collect();
        assert!(
            !with_a_window.is_empty(),
            "no protocol version sets a Core credit pool window; this test would assert nothing"
        );
        for (platform_version, window_blocks) in with_a_window {
            let unlock_validity_blocks = platform_version
                .drive_abci
                .withdrawal_constants
                .core_expiration_blocks;
            assert!(
                window_blocks >= unlock_validity_blocks,
                "protocol version {} sets a Core credit pool window of {} blocks, shorter than \
                 Core's asset unlock validity ({} blocks)",
                platform_version.protocol_version,
                window_blocks,
                unlock_validity_blocks
            );
        }
    }

    /// The withdrawal structure generations selected from protocol version 14 read the cap
    /// through `dpp::withdrawal::validate_core_fee_per_byte_cap`, which treats `None` as "no
    /// cap" per the field's contract. A table that selected one of those generations without a
    /// cap would drop the limit silently, so that combination has to be a deliberate edit here.
    #[test]
    fn should_carry_a_core_fee_cap_wherever_the_capped_withdrawal_rules_are_selected() {
        let selecting_capped_rules: Vec<_> = PLATFORM_VERSIONS
            .iter()
            .filter(|platform_version| {
                let dpp_transitions = &platform_version.dpp.state_transitions;
                let identity_structure = platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_credit_withdrawal_state_transition
                    .basic_structure;
                dpp_transitions
                    .address_funds
                    .validate_credit_withdrawal_structure
                    >= 1
                    || dpp_transitions.shielded.validate_withdrawal_structure >= 1
                    || identity_structure.is_some_and(|version| version >= 2)
            })
            .collect();
        assert!(
            !selecting_capped_rules.is_empty(),
            "no protocol version selects the fee-capped withdrawal rules; this test would \
             assert nothing"
        );
        for platform_version in selecting_capped_rules {
            assert!(
                platform_version
                    .system_limits
                    .max_core_fee_per_byte
                    .is_some(),
                "protocol version {} selects the fee-capped withdrawal structure rules without \
                 a Core fee-rate cap; see SystemLimits::max_core_fee_per_byte",
                platform_version.protocol_version
            );
        }
    }

    #[test]
    fn core_fee_per_byte_limit_starts_at_protocol_version_14() {
        // v13 is already active on live networks, so the limit must not apply there.
        assert_eq!(
            PlatformVersion::get(13)
                .expect("protocol version 13 should exist")
                .system_limits
                .max_core_fee_per_byte,
            None
        );
        assert_eq!(
            PlatformVersion::get(14)
                .expect("protocol version 14 should exist")
                .system_limits
                .max_core_fee_per_byte,
            Some(6_765)
        );
    }
}
