mod accessors;
mod evaluate_interval;
mod validation;

use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_CYCLES_PARAM};
use crate::prelude::{BlockHeightInterval, DataContract, EpochInterval, TimestampMillisInterval};
use bincode::{Decode, Encode};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::ProtocolError;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
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

    /// Determines the maximum cycle moment allowed based on the last paid moment,
    /// the current cycle moment, and the maximum allowed token redemption cycles.
    ///
    /// This function calculates a capped distribution moment (`RewardDistributionMoment`) by limiting
    /// the range between the `last_paid_moment` (or start) and the `current_cycle_moment` to the
    /// maximum allowed number of redemption cycles (`max_cycles`).
    ///
    /// # Arguments
    /// - `last_paid_moment`: Optional last moment at which tokens were claimed.
    /// - `current_cycle_moment`: The current cycle moment as of the current block.
    /// - `max_cycles`: The maximum number of redemption cycles permitted per claim.
    ///
    /// # Returns
    /// - `RewardDistributionMoment`: The maximum allowed cycle moment capped by `max_cycles`.
    pub fn max_cycle_moment(
        &self,
        start_moment: RewardDistributionMoment,
        current_cycle_moment: RewardDistributionMoment,
        max_non_fixed_amount_cycles: u32,
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        let max_cycles = if matches!(self.function(), DistributionFunction::FixedAmount { .. }) {
            // This is much easier to calculate as it's always fixed, so we can have a near unlimited amount of cycles
            //
            MAX_DISTRIBUTION_CYCLES_PARAM
        } else {
            max_non_fixed_amount_cycles as u64
        };
        let interval = self.interval();

        // Calculate maximum allowed moment based on distribution type
        match (start_moment, interval, current_cycle_moment) {
            (
                RewardDistributionMoment::BlockBasedMoment(start),
                RewardDistributionMoment::BlockBasedMoment(step),
                RewardDistributionMoment::BlockBasedMoment(current),
            ) => Ok(RewardDistributionMoment::BlockBasedMoment(
                (start + step.saturating_mul(max_cycles)).min(current),
            )),
            (
                RewardDistributionMoment::TimeBasedMoment(start),
                RewardDistributionMoment::TimeBasedMoment(step),
                RewardDistributionMoment::TimeBasedMoment(current),
            ) => Ok(RewardDistributionMoment::TimeBasedMoment(
                (start + step.saturating_mul(max_cycles)).min(current),
            )),
            (
                RewardDistributionMoment::EpochBasedMoment(start),
                RewardDistributionMoment::EpochBasedMoment(step),
                RewardDistributionMoment::EpochBasedMoment(current),
            ) => Ok(RewardDistributionMoment::EpochBasedMoment(
                // For an epoch reward, if you are in epoch 3 you can't get rewarded for epoch 3, but only epoch 2
                (start + step.saturating_mul(max_cycles as u16)).min(current.saturating_sub(1)),
            )),
            _ => Err(ProtocolError::CorruptedCodeExecution(
                "Mismatch moment types".to_string(),
            )),
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

    // ----- max_cycle_moment -----

    #[test]
    fn test_max_cycle_moment_block_capped_by_current() {
        let dt = block_based();
        let start = RewardDistributionMoment::BlockBasedMoment(1000);
        let current = RewardDistributionMoment::BlockBasedMoment(1500);
        // max_cycles*interval = 10*100=1000. start+1000=2000 > current=1500 so capped at current
        let result = dt.max_cycle_moment(start, current, 10).unwrap();
        assert_eq!(result, RewardDistributionMoment::BlockBasedMoment(1500));
    }

    #[test]
    fn test_max_cycle_moment_block_capped_by_max_cycles_non_fixed() {
        use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
        let dt = RewardDistributionType::BlockBasedDistribution {
            interval: 100,
            function: DistributionFunction::Random { min: 1, max: 10 },
        };
        let start = RewardDistributionMoment::BlockBasedMoment(0);
        let current = RewardDistributionMoment::BlockBasedMoment(u64::MAX);
        // max_cycles = 3 (non-fixed) => 0 + 100*3 = 300
        let result = dt.max_cycle_moment(start, current, 3).unwrap();
        assert_eq!(result, RewardDistributionMoment::BlockBasedMoment(300));
    }

    #[test]
    fn test_max_cycle_moment_time_capped_by_current() {
        let dt = time_based();
        let start = RewardDistributionMoment::TimeBasedMoment(0);
        let current = RewardDistributionMoment::TimeBasedMoment(100_000);
        // max_cycles*interval could exceed current; should cap
        let result = dt.max_cycle_moment(start, current, 5).unwrap();
        // fixed amount cycles = MAX_DISTRIBUTION_CYCLES_PARAM => huge, cap by current
        assert_eq!(result, RewardDistributionMoment::TimeBasedMoment(100_000));
    }

    #[test]
    fn test_max_cycle_moment_epoch_subtracts_one() {
        let dt = epoch_based();
        let start = RewardDistributionMoment::EpochBasedMoment(0);
        let current = RewardDistributionMoment::EpochBasedMoment(3);
        let result = dt.max_cycle_moment(start, current, 10).unwrap();
        // Since fixed amount: huge max_cycles, saturating_mul on u16 -> u16::MAX;
        // min(start + step*max, current - 1) = current - 1 = 2
        assert_eq!(result, RewardDistributionMoment::EpochBasedMoment(2));
    }

    #[test]
    fn test_max_cycle_moment_epoch_current_zero_saturates() {
        let dt = epoch_based();
        let start = RewardDistributionMoment::EpochBasedMoment(0);
        let current = RewardDistributionMoment::EpochBasedMoment(0);
        // current - 1 saturates to 0
        let result = dt.max_cycle_moment(start, current, 10).unwrap();
        assert_eq!(result, RewardDistributionMoment::EpochBasedMoment(0));
    }

    #[test]
    fn test_max_cycle_moment_type_mismatch() {
        let dt = block_based();
        let start = RewardDistributionMoment::BlockBasedMoment(0);
        let current = RewardDistributionMoment::TimeBasedMoment(50);
        let result = dt.max_cycle_moment(start, current, 10);
        assert!(matches!(
            result,
            Err(ProtocolError::CorruptedCodeExecution(_))
        ));
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
