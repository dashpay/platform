use versioned_feature_core::FeatureVersion;

/// The Tenderdash consensus parameters Drive owns.
///
/// Tenderdash takes consensus parameter updates from Drive's `prepare_proposal`,
/// `process_proposal` and `init_chain` responses (`consensus_params_update`), which is the only
/// way every validator adopts a new value at the same height. Each field here is therefore
/// protocol state: it changes only with a protocol version, and `consensus_params_update`
/// compares the old and new version's values to decide what to push.
#[derive(Clone, Debug, Default)]
pub struct ConsensusVersions {
    pub tenderdash_consensus_version: FeatureVersion,
    /// `ConsensusParams.block.max_bytes`: the largest block Tenderdash will build or accept,
    /// in bytes. Pushed by `consensus_params_update` v2 when the value differs from the previous
    /// protocol version's. Must exceed the largest state transition family cap of the version
    /// (`SystemLimits::max_contract_code_state_transition_size` from protocol version 17) by
    /// enough for the header, commit and evidence. `None` for the protocol versions that never
    /// pushed block parameters and left the genesis value in place.
    pub block_max_bytes: Option<u64>,
    /// `ConsensusParams.block.max_gas`, pushed together with `block_max_bytes`: Tenderdash
    /// copies both fields whenever a `Block` update is present, so a version that sets the byte
    /// cap has to carry the gas cap it intends to keep. `None` wherever `block_max_bytes` is.
    pub block_max_gas: Option<i64>,
}

#[cfg(test)]
mod tests {
    use crate::version::protocol_version::PLATFORM_VERSIONS;
    use crate::version::LATEST_VERSION;

    /// A `Some` on a shipped version would make `consensus_params_update` v2 push a block
    /// parameter change at a boundary the network already crossed with the genesis value in
    /// place, so the absence is pinned for every version below 17. From 17 the two fields must
    /// travel together: Tenderdash overwrites both whenever either is pushed.
    #[test]
    fn block_params_are_absent_before_protocol_version_17_and_paired_after() {
        assert_eq!(PLATFORM_VERSIONS.len(), LATEST_VERSION as usize);
        for platform_version in PLATFORM_VERSIONS {
            let consensus = &platform_version.consensus;
            if platform_version.protocol_version < 17 {
                assert_eq!(
                    (consensus.block_max_bytes, consensus.block_max_gas),
                    (None, None),
                    "protocol version {} must not push block parameters",
                    platform_version.protocol_version
                );
            } else {
                assert!(
                    consensus.block_max_bytes.is_some() && consensus.block_max_gas.is_some(),
                    "protocol version {} must carry both block parameters",
                    platform_version.protocol_version
                );
            }
        }
    }
}
