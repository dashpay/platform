pub mod v1;
pub mod v2;
pub mod v3;
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
    /// Maximum time-to-live (in seconds) a `timeRange` index transform may
    /// declare, enforced at contract registration.
    ///
    /// The cap is what makes the TTL fee model safe: entries under a TTL'd
    /// index bill their bytes as processing (the ephemeral-bytes rate)
    /// instead of storage, and a flat rate is only an honest price while
    /// the lifetime it covers is bounded. One week in v5.
    /// See `book/src/drive/time-range-ttl.md`.
    ///
    /// `None` preserves the behavior of protocol versions that predate the
    /// `ttl` key (nothing to bound: the key does not parse there).
    pub max_time_range_ttl_seconds: Option<u64>,
    /// Maximum number of O(1) drop operations one write into a TTL'd
    /// `timeRange` index may spend draining expired buckets — per index
    /// level: indexes sharing a level share one drain per transition.
    ///
    /// A bucket drains deepest-first through flat-subtree drops (one per
    /// `[0]` reference tree, per emptied value tree, per property-name
    /// tree, plus the bucket itself), so the operation count scales with
    /// the window's distinct groups while each operation is O(1). Every
    /// write continues wherever the previous budget ran out; write volume
    /// scales with group volume, so drainage keeps pace roughly one window
    /// behind. `None` for the protocol versions that predate the `ttl`
    /// key.
    pub max_time_range_ttl_drop_operations_per_write: Option<u16>,
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
