//! Fetches execution state from grovedb storage

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

mod v0;
mod v1;

impl<C> Platform<C> {
    /// Fetches execution state from grovedb storage
    pub fn fetch_platform_state(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PlatformState>, Error> {
        match platform_version
            .drive_abci
            .methods
            .platform_state_storage
            .fetch_platform_state
        {
            0 => Platform::<C>::fetch_platform_state_v0(drive, transaction, platform_version),
            1 => Platform::<C>::fetch_platform_state_v1(drive, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_platform_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::platform_types::platform::Platform;
    use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSetV0Methods;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::fixture::platform_state::{
        PLATFORM_STATE_V3_TESTNET, PLATFORM_STATE_V8_DEVNET,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use bincode::config::{BigEndian, Configuration, NoLimit, Varint};
    use dpp::dashcore_rpc::json::QuorumType;
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;
    use std::ops::Deref;

    /// The bincode configuration platform states are saved with
    const PLATFORM_BINCODE_CONFIG: Configuration<BigEndian, Varint, NoLimit> =
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

    /// A saved quorum config: its type, active signers, rotation and window
    type SavedQuorumConfig = (QuorumType, u16, bool, u32);

    /// A testnet state in the first saved format and a devnet state in the second, each with
    /// the chain lock quorum config it was saved with
    fn fixtures() -> [(&'static str, SavedQuorumConfig); 2] {
        [
            (
                PLATFORM_STATE_V3_TESTNET.deref(),
                (QuorumType::Llmq50_60, 24, false, 24),
            ),
            (
                PLATFORM_STATE_V8_DEVNET.deref(),
                (QuorumType::LlmqDevnet, 4, false, 24),
            ),
        ]
    }

    /// The bytes of `fixture` with the window of its chain lock quorum config, saved as `saved`,
    /// set to `window`, encoded the way a `u32` window was
    fn with_chain_lock_quorum_window(
        fixture: &str,
        saved: SavedQuorumConfig,
        window: u32,
    ) -> Vec<u8> {
        let encode = |config: SavedQuorumConfig| {
            bincode::encode_to_vec(config, PLATFORM_BINCODE_CONFIG).unwrap()
        };
        let saved_bytes = encode(saved);
        let mut bytes = hex::decode(fixture).unwrap();
        let positions: Vec<usize> = bytes
            .windows(saved_bytes.len())
            .enumerate()
            .filter_map(|(position, window)| (window == saved_bytes).then_some(position))
            .collect();
        assert_eq!(
            positions.len(),
            1,
            "the chain lock quorum config is saved once"
        );
        let (quorum_type, active_signers, rotation, _) = saved;
        bytes.splice(
            positions[0]..positions[0] + saved_bytes.len(),
            encode((quorum_type, active_signers, rotation, window)),
        );
        bytes
    }

    #[test]
    fn should_save_historical_platform_states_with_unchanged_bytes() {
        for (fixture, _) in fixtures() {
            let bytes = hex::decode(fixture).unwrap();
            let (saved, read): (PlatformStateForSaving, _) =
                bincode::decode_from_slice(&bytes, PLATFORM_BINCODE_CONFIG).unwrap();
            assert_eq!(read, bytes.len());
            assert_eq!(
                bincode::encode_to_vec(&saved, PLATFORM_BINCODE_CONFIG).unwrap(),
                bytes
            );
        }
    }

    #[test]
    fn should_refuse_to_restore_saved_state_holding_a_chain_lock_quorum_window_of_0() {
        // Chain lock verification divides by the chain lock quorum window, which a restarted node
        // reads from its saved state rather than from its config. A saved window of 0 must stop the
        // node from loading that state instead of reaching chain lock verification.
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let restore = |bytes: &[u8], platform_version: &PlatformVersion| {
            let transaction = platform.drive.grove.start_transaction();
            platform
                .drive
                .store_platform_state_bytes(bytes, Some(&transaction), platform_version)
                .unwrap();
            Platform::<MockCoreRPCLike>::fetch_platform_state(
                &platform.drive,
                Some(&transaction),
                platform_version,
            )
        };

        // Protocol version 13 reads the saved state with fetch_platform_state v0, 14 with v1
        for platform_version in [PlatformVersion::get(13).unwrap(), PlatformVersion::latest()] {
            for (fixture, saved) in fixtures() {
                let state = restore(&hex::decode(fixture).unwrap(), platform_version)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    state.chain_lock_validating_quorums().config().window.get(),
                    saved.3
                );

                let result = restore(
                    &with_chain_lock_quorum_window(fixture, saved, 0),
                    platform_version,
                );
                assert!(
                    matches!(
                        &result,
                        Err(Error::Protocol(ProtocolError::PlatformDeserializationError(message)))
                            if message.contains("NonZeroTypeIsZero")
                    ),
                    "protocol version {}, {:?}: got {result:?}",
                    platform_version.protocol_version,
                    saved.0
                );
            }
        }
    }
}
