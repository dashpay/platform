use crate::platform_types::signature_verification_quorum_set::QuorumConfig;
use bincode::Encode;
use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::platform_serialization::de::Decode;
use std::num::{NonZeroU16, NonZeroU32};

/// bincode encodes the active signers and the window exactly as the `u16` and `u32` they hold,
/// the bytes of plain integer fields, and fails to decode a saved 0 in either, so a 0 cannot reach
/// quorum selection or chain lock verification.
#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumConfigForSavingV0 {
    quorum_type: QuorumType,
    active_signers: NonZeroU16,
    rotation: bool,
    window: NonZeroU32,
}

impl From<QuorumConfig> for QuorumConfigForSavingV0 {
    fn from(config: QuorumConfig) -> Self {
        Self {
            quorum_type: config.quorum_type,
            active_signers: config.active_signers,
            rotation: config.rotation,
            window: config.window,
        }
    }
}

impl From<QuorumConfigForSavingV0> for QuorumConfig {
    fn from(config: QuorumConfigForSavingV0) -> Self {
        Self {
            quorum_type: config.quorum_type,
            active_signers: config.active_signers,
            rotation: config.rotation,
            window: config.window,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config::{BigEndian, Configuration, NoLimit, Varint};
    use bincode::error::{DecodeError, IntegerType};

    /// The bincode configuration platform states are saved with
    const PLATFORM_BINCODE_CONFIG: Configuration<BigEndian, Varint, NoLimit> =
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

    /// The saved config as it was written while the active signers and the window were a `u16`
    /// and a `u32`
    fn saved_with_integers(active_signers: u16, window: u32) -> Vec<u8> {
        bincode::encode_to_vec(
            (QuorumType::Llmq400_60, active_signers, false, window),
            PLATFORM_BINCODE_CONFIG,
        )
        .expect("encodes")
    }

    #[test]
    fn should_save_and_restore_active_signers_and_window_with_the_bytes_of_integers() {
        // Every varint width: one byte up to 250, then the u16 and u32 markers
        let active_signers_counts = [1, 4, 250, 251, u16::MAX];
        let windows = [1, 24, 250, 251, 288, u32::from(u16::MAX) + 1, u32::MAX];
        for active_signers in active_signers_counts {
            for window in windows {
                let config = QuorumConfigForSavingV0 {
                    quorum_type: QuorumType::Llmq400_60,
                    active_signers: NonZeroU16::new(active_signers).unwrap(),
                    rotation: false,
                    window: NonZeroU32::new(window).unwrap(),
                };

                let bytes = bincode::encode_to_vec(&config, PLATFORM_BINCODE_CONFIG).unwrap();
                assert_eq!(
                    bytes,
                    saved_with_integers(active_signers, window),
                    "active signers {active_signers}, window {window}"
                );

                let (restored, read): (QuorumConfigForSavingV0, _) =
                    bincode::decode_from_slice(&bytes, PLATFORM_BINCODE_CONFIG).unwrap();
                assert_eq!(read, bytes.len());
                let restored = QuorumConfig::from(restored);
                assert_eq!(restored.active_signers.get(), active_signers);
                assert_eq!(restored.window.get(), window);
            }
        }
    }

    #[test]
    fn should_fail_to_restore_saved_active_signers_or_window_of_zero() {
        // Choosing a rotated quorum takes the logarithm of the active signers and chain lock
        // verification divides by the window, so a saved 0 in either must not decode
        for (bytes, zero_type) in [
            (saved_with_integers(0, 288), IntegerType::U16),
            (saved_with_integers(4, 0), IntegerType::U32),
        ] {
            let result = bincode::decode_from_slice::<QuorumConfigForSavingV0, _>(
                &bytes,
                PLATFORM_BINCODE_CONFIG,
            );

            assert!(
                matches!(
                    &result,
                    Err(DecodeError::NonZeroTypeIsZero { non_zero_type }) if *non_zero_type == zero_type
                ),
                "expected a zero {zero_type:?}, got {result:?}"
            );
        }
    }
}
