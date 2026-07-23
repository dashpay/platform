pub mod v1;
pub mod v2;
pub mod v3;

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
    pub max_transitions_in_documents_batch: u16,
    pub withdrawal_transactions_per_block_limit: u16,
    pub retry_signing_expired_withdrawal_documents_per_block_limit: u16,
    pub max_withdrawal_amount: u64,
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
}

#[cfg(test)]
mod tests {
    use crate::version::PlatformVersion;

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
