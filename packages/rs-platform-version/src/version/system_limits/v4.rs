use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 14 and above. Relative to the last
/// released table (V3) this changes the withdrawal limit, adds the
/// time-range overlap-factor cap, adds the time-range TTL pair, and raises
/// the GroveDB proof envelope floor (the TTL and floor fields joined this
/// table in place while protocol version 14 was unreleased, rather than
/// spawning a new version). The table stays editable in place until 4.2
/// (protocol version 14) is live on mainnet, and is frozen after:
///
/// * `max_time_range_ttl_seconds` is set to one week: the ceiling on the
///   `ttl` a `timeRange` index transform may declare. The cap is what makes
///   the ephemeral-bytes fee model safe — a flat processing rate is only an
///   honest price for transitional storage while the lifetime it covers is
///   bounded. See `book/src/drive/time-range-ttl.md`.
/// * `min_time_range_ttl_drop_operations_per_write` is set to 32. Drive
///   raises this floor according to the merged grid's tree structure and
///   overlap, so cleanup can retire trees faster than writes create them.
///
/// The withdrawal and overlap-factor changes:
///
/// * The daily withdrawal limit becomes relative: `daily_withdrawal_limit_percent` is set to 15,
///   so Platform pools at most 15% of the total credits it held a day ago into asset unlock
///   transactions per 24 hours — never below one maximal withdrawal and never above
///   `max_daily_withdrawal_amount`, Core's 4000 Dash unlock capacity per day — instead of the
///   flat 2000 Dash that applied from v8 (matching Core v22's `LimitAmountV22`). v13 is already
///   live on networks with the flat limit, so the change gates here.
/// * `max_time_range_overlap_factor` is set: a `timeRange` index transform may declare at most
///   24 overlapping windows per timestamp (a day-long window sliding hourly). The rule cannot
///   exist before v14 because the `timeRange` keyword itself is only admitted by the v14
///   document meta-schema.
/// * `core_dust_relay_fee_per_kb` is set to Core's default 3000 duffs/kB: an expired
///   withdrawal whose whole amount is below the dust threshold of its output script (546
///   duffs for P2PKH) is marked FAILED by `rebroadcast_expired_withdrawal_documents` v2
///   instead of being re-signed every 48 Core blocks forever. Withdrawals admitted before
///   v12's 1000-duff floor can carry such amounts on live networks.
/// * `minimum_grovedb_proof_envelope_version` becomes 1: clients verifying with v14 reject
///   the legacy GroveDB V0 proof envelope, whose item binding leaves returned item bytes
///   unauthenticated. Every live network has emitted V1 envelopes since v13.
/// * Core withdrawal fee rates are capped at 6,765 duffs per byte.
/// * Contract groups (protocol version 14): a data contract create transition may declare at
///   most 16 contract group memberships, a contract group may name at most 16 admins besides
///   its owner, and a group's name and description are capped at 64 and 256 characters. The
///   `max_contract_group_size` limit was renamed `max_group_member_count` at the same time; it
///   bounds the members of a change-control `Group` inside a contract, not a contract group.
/// * Contract moderation (protocol version 14): a moderated data contract may name at most 16
///   moderator identities, its owner counted when named. A suspension runs until at most
///   2^53 - 1 milliseconds of block time, the largest value JSON clients read exactly. The
///   text of the reason a ban or a suspension carries is at most 1024 bytes.
/// * Elected moderation teams (protocol version 14): a contract that declares an elected
///   moderation team sets its join window and vote window between one day and four weeks,
///   and its challenge cool-down between two weeks and three years, all in seconds.
/// * Typed array document properties (protocol version 14): a typed array property declares
///   `maxItems`, at most 1024 elements (`max_typed_array_items`, backfilled into the
///   earlier tables, whose parsers never read it).
/// * References (protocol version 14): one document of a document type carries at most 256
///   references, counted at registration as one per property with `refersTo`, one for the
///   type's `ownerRefersTo` and `maxItems` per typed array whose elements declare one
///   (`max_references_per_document`,
///   backfilled into the earlier tables, whose parsers never read it). Each reference is a
///   billed state read when the document is written.
/// * Reference expressions (protocol version 14): a `refersTo` `anyOf` or `allOf` list holds at
///   most 4 operands (`max_reference_operands`) and they nest at most 4 combinators deep
///   (`max_reference_expression_depth`), both backfilled into the earlier tables, whose parsers
///   never read them; every leaf counts against `max_references_per_document`.
/// * Property constraints (protocol version 14): a document type declares at most 16
///   `propertyConstraints` rules (`max_property_constraints`) of at most 32 nodes each
///   (`max_property_constraint_nodes`), both backfilled into the earlier tables, whose
///   parsers never read them. The rules read no state, so these two bound the arithmetic
///   one document write causes.
/// * Moderation charters (protocol version 14): an elected moderation declaration lets a
///   seated team's leader add at most 15 members (`max_contract_moderation_added_moderators`),
///   which joined this table in place while protocol version 14 was unreleased. A charter's
///   description cap is the charter schema's own `maxBytes`, not a limit here.
pub const SYSTEM_LIMITS_V4: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120, //5 KiB
    // Use the protocol's existing data-contract schema-depth ceiling as the conservative
    // instance budget, bounding pre-schema work well above known document requirements.
    max_document_value_depth: Some(256),
    max_typed_array_items: 1024, // typed array properties (new in v14): contract registration caps their maxItems here
    max_references_per_document: 256, // refersTo (new in v14): contract registration caps the references one document carries, a typed array of references counting its maxItems
    max_reference_operands: 4, // refersTo anyOf / allOf (new in v14): contract registration caps the operands one list holds
    max_reference_expression_depth: 4, // refersTo anyOf / allOf (new in v14): contract registration caps how deep they nest
    max_property_constraints: 16, // propertyConstraints (new in v14): contract registration caps the rules one document type declares
    max_property_constraint_nodes: 32, // propertyConstraints (new in v14): contract registration caps the nodes (comparison, operators, properties, values) of one rule
    max_state_transition_size: 20480,  //20 KiB
    // Load-bearing for state correctness, not just for throughput — see
    // SystemLimits::max_transitions_in_documents_batch and SYSTEM_LIMITS_V1.
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    daily_withdrawal_limit_percent: Some(15), // 15% of the total credits a day ago (replaces the flat 2000 Dash in v14)
    max_daily_withdrawal_amount: Some(400_000_000_000_000), // 4000 Dash: Core's unlock capacity per day (LimitAmountV24)
    min_withdrawal_amount: 1_000_000,                       //1000 duffs (raised from 190 in v12)
    core_dust_relay_fee_per_kb: Some(3000), // Core's default dust relay fee: 546-duff P2PKH threshold; expired withdrawals below it fail instead of re-signing
    max_core_fee_per_byte: Some(6_765),
    max_group_member_count: 256,
    max_contract_group_memberships_per_contract: 16,
    max_contract_group_admins: 16,
    max_contract_group_name_length: 64,
    max_contract_group_description_length: 256,
    max_contract_moderators: 16,
    max_contract_suspension_until: 9_007_199_254_740_991,
    max_contract_moderation_reason_length: 1024,
    max_contract_warnings_per_identity: 16,
    max_contract_moderation_reason_documents: 16,
    min_contract_moderation_election_window_seconds: 86_400, // one day
    max_contract_moderation_election_window_seconds: 2_419_200, // four weeks
    min_contract_moderation_challenge_cool_down_seconds: 1_209_600, // two weeks
    max_contract_moderation_challenge_cool_down_seconds: 94_608_000, // three years of 365 days
    contract_document_restore_window_ms: 604_800_000,        // 7 days
    max_contract_moderation_added_moderators: 15,
    max_token_redemption_cycles: 128,
    // NOTE: the Halo 2 proof grows with the action count (~2,273 B/action on
    // top of the 408 B serialized action), so a transition's on-wire size is
    // ~2,681 B per action + ~2,930 B fixed (measured: 2 actions → 8,294 B,
    // 6 → 19,018 B). The effective per-transition action bound under the
    // 20 KiB `max_state_transition_size` is therefore 6, NOT this cap — 16
    // only becomes reachable if the size limit is raised. Pinned by dpp's
    // `seed_pool_batch_fits_max_state_transition_size` signing test.
    max_shielded_transition_actions: 16,
    max_time_range_overlap_factor: Some(24),
    max_time_range_ttl_seconds: Some(604_800), // one week
    min_time_range_ttl_drop_operations_per_write: Some(32),
    minimum_grovedb_proof_envelope_version: 1, // clients reject legacy V0 GroveDB proof envelopes from v14
};
