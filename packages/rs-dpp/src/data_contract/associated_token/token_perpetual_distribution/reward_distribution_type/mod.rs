mod accessors;
mod evaluate_interval;
mod max_cycle_moment;
mod validation;

use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
use crate::prelude::{BlockHeightInterval, DataContract, EpochInterval, TimestampMillisInterval};
use bincode::{Decode, Encode, DecodeUntrusted};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd, DecodeUntrusted,
)]
#[serde(tag = "$type", rename_all = "camelCase")]
pub enum RewardDistributionType {
    /// An amount of tokens is emitted every n blocks.
    /// The start and end are included if set.
    /// If start is not set then it will start at the height of the block when the data contract
    /// is registered.
    BlockBasedDistribution {
        interval: BlockHeightInterval,
        function: DistributionFunction,
    },
    /// An amount of tokens is emitted every amount of time given.
    /// The start and end are included if set.
    /// If start is not set then it will start at the time of the block when the data contract
    /// is registered.
    TimeBasedDistribution {
        interval: TimestampMillisInterval,
        function: DistributionFunction,
    },
    /// An amount of tokens is emitted every amount of epochs.
    /// The start and end are included if set.
    /// If start is not set then it will start at the epoch of the block when the data contract
    /// is registered. A distribution would happen at the start of the following epoch, even if it
    /// is just 1 block later.
    EpochBasedDistribution {
        interval: EpochInterval,
        function: DistributionFunction,
    },
}

impl RewardDistributionType {
    /// Determines the starting moment of reward distribution based on the contract creation time.
    ///
    /// This function returns the appropriate `RewardDistributionMoment`, which represents when
    /// a reward distribution should begin, based on the type of distribution and when the
    /// `DataContract` was created.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A reference to the `DataContract`, which contains details about
    ///   when the contract was created in terms of block height, timestamp, and epoch index.
    ///
    /// # Returns
    ///
    /// * `Some(RewardDistributionMoment)` if the contract's creation time can be mapped to
    ///   a valid distribution start moment.
    /// * `None` if the contract creation time is unavailable or not applicable.
    pub fn contract_creation_moment(
        &self,
        data_contract: &DataContract,
    ) -> Option<RewardDistributionMoment> {
        match self {
            RewardDistributionType::BlockBasedDistribution { .. } => data_contract
                .created_at_block_height()
                .map(RewardDistributionMoment::BlockBasedMoment),
            RewardDistributionType::TimeBasedDistribution { .. } => data_contract
                .created_at()
                .map(RewardDistributionMoment::TimeBasedMoment),
            RewardDistributionType::EpochBasedDistribution { .. } => data_contract
                .created_at_epoch()
                .map(RewardDistributionMoment::EpochBasedMoment),
        }
    }
    /// Converts a byte slice into the corresponding `RewardDistributionMoment` variant
    /// based on the type of reward distribution.
    ///
    /// This method interprets the provided bytes according to the expected type of the distribution:
    /// - `BlockBasedDistribution`: Interprets the bytes as a `BlockHeight` (`u64`).
    /// - `TimeBasedDistribution`: Interprets the bytes as a `TimestampMillis` (`u64`).
    /// - `EpochBasedDistribution`: Interprets the bytes as an `EpochIndex` (`u16`).
    ///
    /// # Parameters
    ///
    /// - `bytes`: A byte slice containing the serialized representation of the moment.
    ///
    /// # Returns
    ///
    /// - `Ok(RewardDistributionMoment)`: The successfully parsed reward distribution moment.
    /// - `Err(ProtocolError)`: If the provided bytes are of incorrect length.
    ///
    /// # Errors
    ///
    /// - `ProtocolError::DecodingError`: If the provided bytes slice does not have the expected length
    pub fn moment_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        match self {
            RewardDistributionType::BlockBasedDistribution { .. } => {
                if bytes.len() != 8 {
                    return Err(ProtocolError::DecodingError(
                        "Expected 8 bytes for BlockBasedMoment".to_string(),
                    ));
                }
                let mut array = [0u8; 8];
                array.copy_from_slice(bytes);
                Ok(RewardDistributionMoment::BlockBasedMoment(
                    u64::from_be_bytes(array),
                ))
            }
            RewardDistributionType::TimeBasedDistribution { .. } => {
                if bytes.len() != 8 {
                    return Err(ProtocolError::DecodingError(
                        "Expected 8 bytes for TimeBasedMoment".to_string(),
                    ));
                }
                let mut array = [0u8; 8];
                array.copy_from_slice(bytes);
                Ok(RewardDistributionMoment::TimeBasedMoment(
                    u64::from_be_bytes(array),
                ))
            }
            RewardDistributionType::EpochBasedDistribution { .. } => {
                if bytes.len() != 2 {
                    return Err(ProtocolError::DecodingError(
                        "Expected 2 bytes for EpochBasedMoment".to_string(),
                    ));
                }
                let mut array = [0u8; 2];
                array.copy_from_slice(bytes);
                Ok(RewardDistributionMoment::EpochBasedMoment(
                    u16::from_be_bytes(array),
                ))
            }
        }
    }
}
#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;

    fn block_based() -> RewardDistributionType {
        RewardDistributionType::BlockBasedDistribution {
            interval: 100,
            function: DistributionFunction::FixedAmount { amount: 5 },
        }
    }

    fn time_based() -> RewardDistributionType {
        RewardDistributionType::TimeBasedDistribution {
            interval: 60_000,
            function: DistributionFunction::FixedAmount { amount: 5 },
        }
    }

    fn epoch_based() -> RewardDistributionType {
        RewardDistributionType::EpochBasedDistribution {
            interval: 1,
            function: DistributionFunction::FixedAmount { amount: 5 },
        }
    }

    // ----- moment_from_bytes -----

    #[test]
    fn test_moment_from_bytes_block_ok() {
        let dt = block_based();
        let bytes = [0u8, 0, 0, 0, 0, 0, 0, 42];
        let result = dt.moment_from_bytes(&bytes).unwrap();
        assert_eq!(result, RewardDistributionMoment::BlockBasedMoment(42));
    }

    #[test]
    fn test_moment_from_bytes_block_wrong_len() {
        let dt = block_based();
        let bytes = [0u8, 0, 0, 42];
        let result = dt.moment_from_bytes(&bytes);
        assert!(matches!(result, Err(ProtocolError::DecodingError(_))));
    }

    #[test]
    fn test_moment_from_bytes_block_empty() {
        let dt = block_based();
        let result = dt.moment_from_bytes(&[]);
        assert!(matches!(result, Err(ProtocolError::DecodingError(_))));
    }

    #[test]
    fn test_moment_from_bytes_time_ok() {
        let dt = time_based();
        let bytes = [0u8, 0, 0, 0, 0, 0, 0x01, 0x00];
        let result = dt.moment_from_bytes(&bytes).unwrap();
        assert_eq!(result, RewardDistributionMoment::TimeBasedMoment(256));
    }

    #[test]
    fn test_moment_from_bytes_time_wrong_len() {
        let dt = time_based();
        let bytes = [0u8, 0, 0];
        let result = dt.moment_from_bytes(&bytes);
        assert!(matches!(result, Err(ProtocolError::DecodingError(_))));
    }

    #[test]
    fn test_moment_from_bytes_epoch_ok() {
        let dt = epoch_based();
        let bytes = [0x00, 0x07];
        let result = dt.moment_from_bytes(&bytes).unwrap();
        assert_eq!(result, RewardDistributionMoment::EpochBasedMoment(7));
    }

    #[test]
    fn test_moment_from_bytes_epoch_wrong_len_too_short() {
        let dt = epoch_based();
        let bytes = [0u8];
        let result = dt.moment_from_bytes(&bytes);
        assert!(matches!(result, Err(ProtocolError::DecodingError(_))));
    }

    #[test]
    fn test_moment_from_bytes_epoch_wrong_len_too_long() {
        let dt = epoch_based();
        let bytes = [0u8, 0, 0, 0];
        let result = dt.moment_from_bytes(&bytes);
        assert!(matches!(result, Err(ProtocolError::DecodingError(_))));
    }

    // ----- interval() / function() accessors -----

    #[test]
    fn test_interval_accessor() {
        assert_eq!(
            block_based().interval(),
            RewardDistributionMoment::BlockBasedMoment(100)
        );
        assert_eq!(
            time_based().interval(),
            RewardDistributionMoment::TimeBasedMoment(60_000)
        );
        assert_eq!(
            epoch_based().interval(),
            RewardDistributionMoment::EpochBasedMoment(1)
        );
    }

    #[test]
    fn test_function_accessor() {
        match block_based().function() {
            DistributionFunction::FixedAmount { amount } => assert_eq!(*amount, 5),
            _ => panic!("unexpected function"),
        }
    }

    // ----- Display -----

    #[test]
    fn test_display_block_based() {
        let dt = block_based();
        let s = format!("{}", dt);
        assert!(s.contains("BlockBasedDistribution"));
        assert!(s.contains("100 blocks"));
    }

    #[test]
    fn test_display_time_based() {
        let dt = time_based();
        let s = format!("{}", dt);
        assert!(s.contains("TimeBasedDistribution"));
        assert!(s.contains("60000 milliseconds"));
    }

    #[test]
    fn test_display_epoch_based() {
        let dt = epoch_based();
        let s = format!("{}", dt);
        assert!(s.contains("EpochBasedDistribution"));
        assert!(s.contains("1 epochs"));
    }

    // ----- validate_structure_interval_v0 -----

    #[test]
    fn test_validate_structure_interval_block_mainnet_too_short() {
        use dashcore::Network;
        let dt = RewardDistributionType::BlockBasedDistribution {
            interval: 50,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let result = dt.validate_structure_interval_v0(Network::Mainnet);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_structure_interval_block_mainnet_ok() {
        use dashcore::Network;
        let dt = RewardDistributionType::BlockBasedDistribution {
            interval: 100,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let result = dt.validate_structure_interval_v0(Network::Mainnet);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_structure_interval_block_testnet() {
        use dashcore::Network;
        let dt_ok = RewardDistributionType::BlockBasedDistribution {
            interval: 5,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        assert!(dt_ok
            .validate_structure_interval_v0(Network::Testnet)
            .is_valid());

        let dt_bad = RewardDistributionType::BlockBasedDistribution {
            interval: 4,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        assert!(!dt_bad
            .validate_structure_interval_v0(Network::Testnet)
            .is_valid());
    }

    #[test]
    fn test_validate_structure_interval_block_regtest() {
        use dashcore::Network;
        let dt = RewardDistributionType::BlockBasedDistribution {
            interval: 1,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        assert!(dt
            .validate_structure_interval_v0(Network::Regtest)
            .is_valid());

        let dt_zero = RewardDistributionType::BlockBasedDistribution {
            interval: 0,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        assert!(!dt_zero
            .validate_structure_interval_v0(Network::Regtest)
            .is_valid());
    }

    #[test]
    fn test_validate_structure_interval_time_mainnet_too_short() {
        use dashcore::Network;
        let dt = RewardDistributionType::TimeBasedDistribution {
            interval: 60_000,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let result = dt.validate_structure_interval_v0(Network::Mainnet);
        // Less than 1 hour = 3_600_000 ms. Should fail.
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_structure_interval_time_mainnet_not_minute_aligned() {
        use dashcore::Network;
        let dt = RewardDistributionType::TimeBasedDistribution {
            // 3_600_500 > 3_600_000 but not divisible by 60_000
            interval: 3_600_500,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let result = dt.validate_structure_interval_v0(Network::Mainnet);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_structure_interval_time_mainnet_ok() {
        use dashcore::Network;
        let dt = RewardDistributionType::TimeBasedDistribution {
            interval: 3_600_000,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let result = dt.validate_structure_interval_v0(Network::Mainnet);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_structure_interval_time_regtest_ok() {
        use dashcore::Network;
        let dt = RewardDistributionType::TimeBasedDistribution {
            interval: 60_000,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let result = dt.validate_structure_interval_v0(Network::Regtest);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_structure_interval_epoch_always_ok() {
        use dashcore::Network;
        // Epoch-based validation does no checks; even zero interval passes.
        let dt = RewardDistributionType::EpochBasedDistribution {
            interval: 0,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        assert!(dt
            .validate_structure_interval_v0(Network::Mainnet)
            .is_valid());
    }

    // ----- validate_structure_interval through the dispatcher -----

    #[test]
    fn should_reject_a_zero_epoch_interval_from_protocol_version_14() {
        use crate::consensus::basic::BasicError;
        use crate::consensus::ConsensusError;
        use dashcore::Network;
        use platform_version::version::PlatformVersion;

        let zero = RewardDistributionType::EpochBasedDistribution {
            interval: 0,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let one = RewardDistributionType::EpochBasedDistribution {
            interval: 1,
            function: DistributionFunction::FixedAmount { amount: 1 },
        };
        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let latest = PlatformVersion::latest();

        for network in [
            Network::Mainnet,
            Network::Testnet,
            Network::Devnet,
            Network::Regtest,
        ] {
            // Frozen: version 13 registers a zero interval.
            assert!(zero
                .validate_structure_interval(network, v13)
                .expect("expected v13 validation")
                .is_valid());
            let result = zero
                .validate_structure_interval(network, latest)
                .expect("expected latest validation");
            assert!(matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenDistributionEpochIntervalTooShortError(error)
                )] if error.interval() == 0
            ));
            for platform_version in [v13, latest] {
                assert!(one
                    .validate_structure_interval(network, platform_version)
                    .expect("expected validation")
                    .is_valid());
            }
        }
    }

    #[test]
    fn should_keep_block_and_time_minimums_in_every_version() {
        use dashcore::Network;
        use platform_version::version::PlatformVersion;

        let v13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let latest = PlatformVersion::latest();
        let cases = [
            (
                RewardDistributionType::BlockBasedDistribution {
                    interval: 99,
                    function: DistributionFunction::FixedAmount { amount: 1 },
                },
                Network::Mainnet,
                false,
            ),
            (
                RewardDistributionType::BlockBasedDistribution {
                    interval: 100,
                    function: DistributionFunction::FixedAmount { amount: 1 },
                },
                Network::Mainnet,
                true,
            ),
            (
                RewardDistributionType::TimeBasedDistribution {
                    interval: 3_600_500,
                    function: DistributionFunction::FixedAmount { amount: 1 },
                },
                Network::Mainnet,
                false,
            ),
            (
                RewardDistributionType::TimeBasedDistribution {
                    interval: 60_000,
                    function: DistributionFunction::FixedAmount { amount: 1 },
                },
                Network::Regtest,
                true,
            ),
        ];
        for (distribution, network, expected_valid) in cases {
            for platform_version in [v13, latest] {
                assert_eq!(
                    distribution
                        .validate_structure_interval(network, platform_version)
                        .expect("expected validation")
                        .is_valid(),
                    expected_valid,
                    "{distribution} on {network:?} at v{}",
                    platform_version.protocol_version
                );
            }
        }
    }

    #[test]
    fn should_reject_an_unknown_validate_structure_interval_version() {
        use dashcore::Network;
        use platform_version::version::PlatformVersion;

        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .token_versions
            .validate_structure_interval = 2;
        assert!(matches!(
            epoch_based().validate_structure_interval(Network::Mainnet, &platform_version),
            Err(ProtocolError::UnknownVersionMismatch { received: 2, .. })
        ));
    }
}

impl fmt::Display for RewardDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, function } => {
                write!(
                    f,
                    "BlockBasedDistribution: every {} blocks using {}",
                    interval, function
                )?;
                Ok(())
            }
            RewardDistributionType::TimeBasedDistribution { interval, function } => {
                write!(
                    f,
                    "TimeBasedDistribution: every {} milliseconds using {}",
                    interval, function
                )?;
                Ok(())
            }
            RewardDistributionType::EpochBasedDistribution { interval, function } => {
                write!(
                    f,
                    "EpochBasedDistribution: every {} epochs using {}",
                    interval, function
                )?;
                Ok(())
            }
        }
    }
}

// --- canonical conversion trait impls (unification pass 1) ---
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for RewardDistributionType {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for RewardDistributionType {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // Externally tagged enum: each variant becomes `{<VariantName>: {<fields>}}`.
    // Inner `function: DistributionFunction` is itself externally tagged.
    // Round-trip covers one variant per interval-type to lock in the typed
    // sizes (`u64` for block/timestamp, `u16` for epoch).

    #[test]
    fn json_round_trip_block_based() {
        use crate::serialization::JsonConvertible;
        let original = RewardDistributionType::BlockBasedDistribution {
            interval: 100,
            function: DistributionFunction::FixedAmount { amount: 50 },
        };
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$type": "blockBasedDistribution",
                "interval": 100,
                "function": { "$type": "fixedAmount", "amount": 50 }
            })
        );
        let recovered = RewardDistributionType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_epoch_based() {
        use crate::serialization::JsonConvertible;
        let original = RewardDistributionType::EpochBasedDistribution {
            interval: 7,
            function: DistributionFunction::FixedAmount { amount: 1_000 },
        };
        let json = original.to_json().expect("to_json");
        // `EpochInterval` is `u16` but JSON erases the size.
        assert_eq!(
            json,
            json!({
                "$type": "epochBasedDistribution",
                "interval": 7,
                "function": { "$type": "fixedAmount", "amount": 1_000 }
            })
        );
        let recovered = RewardDistributionType::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_block_based() {
        use crate::serialization::ValueConvertible;
        let original = RewardDistributionType::BlockBasedDistribution {
            interval: 100,
            function: DistributionFunction::FixedAmount { amount: 50 },
        };
        let value = original.to_object().expect("to_object");
        // `BlockHeightInterval` is `u64`. `TokenAmount` is `u64`.
        assert_eq!(
            value,
            platform_value!({
                "$type": "blockBasedDistribution",
                "interval": 100u64,
                "function": { "$type": "fixedAmount", "amount": 50u64 }
            })
        );
        let recovered = RewardDistributionType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_epoch_based() {
        use crate::serialization::ValueConvertible;
        let original = RewardDistributionType::EpochBasedDistribution {
            interval: 7,
            function: DistributionFunction::FixedAmount { amount: 1_000 },
        };
        let value = original.to_object().expect("to_object");
        // `EpochInterval` is `u16` → `Value::U16`.
        assert_eq!(
            value,
            platform_value!({
                "$type": "epochBasedDistribution",
                "interval": 7u16,
                "function": { "$type": "fixedAmount", "amount": 1_000u64 }
            })
        );
        let recovered = RewardDistributionType::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
