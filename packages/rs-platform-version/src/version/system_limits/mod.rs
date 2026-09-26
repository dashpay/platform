pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

#[derive(Clone, Debug, Default)]
pub struct SystemLimits {
    pub estimated_contract_max_serialized_size: u16,
    pub max_field_value_size: u32,
    /// Maximum number of nested map/array containers in document properties.
    ///
    /// `None` preserves the behavior of protocol versions that predate this limit.
    pub max_document_value_depth: Option<u16>,
    /// Max size of a state transition in bytes.
    ///
    /// The cap of every ordinary state transition family, compared with the raw length before
    /// any decode (`decode_raw_state_transitions` v0 for every family; v1 for every family
    /// except the contract-code capable ones, which read
    /// `max_contract_code_state_transition_size` instead).
    ///
    /// NOTE: The Tenderdash `max-tx-bytes` in the node config must be at least the largest
    /// family cap of the active protocol version, so a transaction Drive would accept is never
    /// dropped by the mempool first. Up to protocol version 16 every family shares this value;
    /// from 17 the contract-code cap is the larger one.
    pub max_state_transition_size: u64,
    /// Max raw size in bytes of a contract create or update transition in a generation that
    /// can carry a code bundle (`DataContractCreateTransition::V1` and
    /// `DataContractUpdateTransition::V1`), detected from the wire prefix by
    /// `StateTransition::peek_envelope_kind` before the bytes are decoded. Read by
    /// `decode_raw_state_transitions` v1, the `getProofs` v1 query and the DAPI broadcast
    /// pre-filter; `None` for the protocol versions that predate those generations, where the
    /// contract families are bounded by `max_state_transition_size` like every other one.
    /// Versioned: see `max_contract_code_state_transition_size` in each `SYSTEM_LIMITS_V*`.
    pub max_contract_code_state_transition_size: Option<u64>,
    /// The bincode decode budget (`with_limit`) used to decode a contract-code capable
    /// envelope. Distinct from the wire cap above: bincode charges the allocation claims of the
    /// `Value` containers a contract schema decodes into against this budget as well as the
    /// encoded bytes, so it must leave headroom above the wire cap. Every ordinary family keeps
    /// the shipped decode of `StateTransition::deserialize_from_bytes`, which applies no
    /// bincode budget (see `StateTransitionDecodeBudget::Historical` in `dpp`). Must be one of
    /// the budgets `StateTransition::deserialize_from_bytes_with_budget` supports; `None`
    /// before the contract-code generations exist.
    pub max_contract_code_state_transition_decode_budget: Option<u64>,
    /// Sum of the canonical module bytes one code bundle may carry, validated at basic
    /// structure once the contract-code generations exist. `None` before them.
    pub max_contract_code_bundle_bytes: Option<u64>,
    /// Number of modules one code bundle may carry, validated at basic structure once the
    /// contract-code generations exist. `None` before them.
    pub max_contract_code_modules_per_bundle: Option<u16>,
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
    /// reader raising it needs to know what the other two are:
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
    /// Upper bound (in credits) of the relative daily withdrawal limit from protocol version 14:
    /// Core's credit-pool unlock capacity per day, `LimitAmountV24` = 4000 Dash per 576-block
    /// window (Core v24). Platform cannot usefully pool more than Core will mine — the excess
    /// only cycles through expiry and re-signing — so the limit never exceeds this whatever the
    /// total credits are; raise it together with Core. Must be at least `max_withdrawal_amount`.
    /// `None` for the protocol versions that predate the relative rule.
    pub max_daily_withdrawal_amount: Option<u64>,
    /// Minimum net amount (in credits) a withdrawal may send to Core, shared by the
    /// transparent (identity + address) and shielded withdrawal paths. The dust floor that
    /// keeps Core from rejecting the resulting `TxOut`. Versioned: see `min_withdrawal_amount`
    /// in each `SYSTEM_LIMITS_V*`.
    pub min_withdrawal_amount: u64,
    pub max_contract_group_size: u16,
    // This the max redemption cycles we can process if we don't use a constant distribution
    // For a constant perpetual distribution this is very cheap since it's just a multiplication
    // For other distributions we much calculate at each cycle the rewards, so we don't want to
    // do this that much
    pub max_token_redemption_cycles: u32,
    pub max_shielded_transition_actions: u16,
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
                 see the documentation on SystemLimits::max_transitions_in_documents_batch \
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
                 batch; see SystemLimits::max_transitions_in_documents_batch",
                platform_version.protocol_version
            );
        }
    }

    /// The v1 decoder picks the family cap from these fields: `None` means a contract
    /// transition is bounded like every other family, `Some` means the larger contract-code
    /// cap applies. A shipped version that gained a `Some` by a copy-paste into a new table
    /// would silently raise the cap validators on that version agree on, so the absence is
    /// pinned here for every version below 17 and the presence for 17 and above.
    #[test]
    fn contract_code_limits_are_absent_before_protocol_version_17() {
        assert_eq!(PLATFORM_VERSIONS.len(), LATEST_VERSION as usize);
        for platform_version in PLATFORM_VERSIONS {
            let limits = &platform_version.system_limits;
            let contract_code_limits = (
                limits.max_contract_code_state_transition_size,
                limits.max_contract_code_state_transition_decode_budget,
                limits.max_contract_code_bundle_bytes,
                limits.max_contract_code_modules_per_bundle,
            );
            if platform_version.protocol_version < 17 {
                assert_eq!(
                    contract_code_limits,
                    (None, None, None, None),
                    "protocol version {} predates contract code bundles and must not bound them",
                    platform_version.protocol_version
                );
            } else {
                assert!(
                    limits.max_contract_code_state_transition_size.is_some()
                        && limits
                            .max_contract_code_state_transition_decode_budget
                            .is_some()
                        && limits.max_contract_code_bundle_bytes.is_some()
                        && limits.max_contract_code_modules_per_bundle.is_some(),
                    "protocol version {} admits contract code bundles and must bound them",
                    platform_version.protocol_version
                );
            }
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
}
