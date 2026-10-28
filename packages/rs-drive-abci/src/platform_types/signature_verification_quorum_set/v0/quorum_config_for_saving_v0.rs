use crate::platform_types::signature_verification_quorum_set::QuorumConfig;
use bincode::Encode;
use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::platform_serialization::de::Decode;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumConfigForSavingV0 {
    quorum_type: QuorumType,
    active_signers: u16,
    rotation: bool,
    /// Encoded exactly as a `u32`, so states saved while this field was a `u32` still decode;
    /// a saved window of 0 fails to decode instead of reaching chain lock verification.
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

    /// The saved config as it was written while the window was a `u32`
    fn saved_with_u32_window(window: u32) -> Vec<u8> {
        bincode::encode_to_vec(
            (QuorumType::Llmq400_60, 4u16, false, window),
            PLATFORM_BINCODE_CONFIG,
        )
        .expect("encodes")
    }

    #[test]
    fn should_save_and_restore_window_with_the_bytes_of_a_u32() {
        // Every varint width: one byte up to 250, then u16, u32 markers
        for window in [1, 24, 250, 251, 288, u32::from(u16::MAX) + 1, u32::MAX] {
            let config = QuorumConfigForSavingV0 {
                quorum_type: QuorumType::Llmq400_60,
                active_signers: 4,
                rotation: false,
                window: NonZeroU32::new(window).unwrap(),
            };

            let bytes = bincode::encode_to_vec(&config, PLATFORM_BINCODE_CONFIG).unwrap();
            assert_eq!(bytes, saved_with_u32_window(window), "window {window}");

            let (restored, read): (QuorumConfigForSavingV0, _) =
                bincode::decode_from_slice(&bytes, PLATFORM_BINCODE_CONFIG).unwrap();
            assert_eq!(read, bytes.len());
            assert_eq!(QuorumConfig::from(restored).window.get(), window);
        }
    }

    #[test]
    fn should_fail_to_restore_saved_window_of_zero() {
        // Chain lock verification divides by the window, so a saved 0 must not decode
        let result = bincode::decode_from_slice::<QuorumConfigForSavingV0, _>(
            &saved_with_u32_window(0),
            PLATFORM_BINCODE_CONFIG,
        );

        assert!(
            matches!(
                result,
                Err(DecodeError::NonZeroTypeIsZero {
                    non_zero_type: IntegerType::U32
                })
            ),
            "got {result:?}"
        );
    }
}
