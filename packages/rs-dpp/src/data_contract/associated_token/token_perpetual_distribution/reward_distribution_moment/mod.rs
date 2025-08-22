use crate::block::epoch::EpochIndex;
use crate::prelude::{BlockHeight, TimestampMillis};
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div};
use crate::block::block_info::BlockInfo;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;

#[derive(
    Serialize,
    Deserialize,
    PlatformSerialize,
    PlatformDeserialize,
    Decode,
    Encode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
)]
#[platform_serialize(unversioned)]
pub enum RewardDistributionMoment {
    /// The reward was distributed at a block height
    BlockBasedMoment(BlockHeight),
    /// The reward was distributed at a time
    TimeBasedMoment(TimestampMillis),
    /// The reward was distributed at an epoch
    EpochBasedMoment(EpochIndex),
}

impl RewardDistributionMoment {
    /// Checks if two `RewardDistributionMoment`s are of the same type.
    pub fn same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::BlockBasedMoment(_), Self::BlockBasedMoment(_))
                | (Self::TimeBasedMoment(_), Self::TimeBasedMoment(_))
                | (Self::EpochBasedMoment(_), Self::EpochBasedMoment(_))
        )
    }

    /// Converts a `RewardDistributionMoment` into a `u64` representation.
    ///
    /// # Returns
    /// - The underlying numerical value of the moment as a `u64`.
    pub fn to_u64(&self) -> u64 {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => *height,
            RewardDistributionMoment::TimeBasedMoment(timestamp) => *timestamp,
            RewardDistributionMoment::EpochBasedMoment(epoch) => *epoch as u64,
        }
    }

    /// Computes the cycle start for the given moment, aligned with the `step` boundary.
    ///
    /// "Cycle start" is defined here as the greatest multiple of `step` not greater than `self`.
    ///
    /// # Parameters
    /// - `step`: The step interval (must be of the same variant, non-zero).
    ///
    /// # Returns
    /// - `Ok(RewardDistributionMoment)`: The moment snapped down to the nearest multiple of `step`.
    /// - `Err(ProtocolError)`: If `step` is zero or if the types are mismatched.
    pub fn cycle_start(
        &self,
        step: RewardDistributionMoment,
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        match (self, step) {
            (
                RewardDistributionMoment::BlockBasedMoment(start),
                RewardDistributionMoment::BlockBasedMoment(step_size),
            ) => {
                if step_size == 0 {
                    return Err(ProtocolError::InvalidDistributionStep(
                        "Step value cannot be zero",
                    ));
                }
                // Greatest multiple of step_size <= start
                let remainder = start % step_size;
                let cycle_start = start.saturating_sub(remainder);
                Ok(RewardDistributionMoment::BlockBasedMoment(cycle_start))
            }
            (
                RewardDistributionMoment::TimeBasedMoment(timestamp),
                RewardDistributionMoment::TimeBasedMoment(step_size),
            ) => {
                if step_size == 0 {
                    return Err(ProtocolError::InvalidDistributionStep(
                        "Step value cannot be zero",
                    ));
                }
                // Greatest multiple of step_size <= timestamp
                let remainder = timestamp % step_size;
                let cycle_start = timestamp.saturating_sub(remainder);
                Ok(RewardDistributionMoment::TimeBasedMoment(cycle_start))
            }
            (
                RewardDistributionMoment::EpochBasedMoment(epoch),
                RewardDistributionMoment::EpochBasedMoment(step_size),
            ) => {
                if step_size == 0 {
                    return Err(ProtocolError::InvalidDistributionStep(
                        "Step value cannot be zero",
                    ));
                }
                // Greatest multiple of step_size <= epoch
                let remainder = epoch % step_size;
                let cycle_start = epoch.saturating_sub(remainder);
                Ok(RewardDistributionMoment::EpochBasedMoment(cycle_start))
            }
            // Fallback for completeness—should not occur because we already did a type check
            _ => Err(ProtocolError::AddingDifferentTypes(
                "Cannot compute cycle_start with mismatched types".into(),
            )),
        }
    }

    /// Calculates the number of steps from `self` to `other`, using `step` as the increment.
    ///
    /// This function computes how many `step` intervals are needed to go from `self`
    /// to `other`. If `self >= other`, it returns `0` since no steps are needed.
    ///
    /// # Parameters
    ///
    /// - `other`: The target moment.
    /// - `step`: The step interval.
    /// - `start_included`: Whether the starting boundary is included.
    /// - `end_included`: Whether the ending boundary is included.
    ///
    /// # Returns
    ///
    /// - `Ok(u64)`: The number of steps needed.
    /// - `Err(ProtocolError)`: If `step` is zero or types are mismatched.
    pub fn steps_till(
        &self,
        other: &Self,
        step: &Self,
        start_included: bool,
        end_included: bool,
    ) -> Result<u64, ProtocolError> {
        // Depending on the variant, calculate the needed steps the same way,
        // but adjust for inclusivity at the end.
        match (self, other, step) {
            (
                RewardDistributionMoment::BlockBasedMoment(start),
                RewardDistributionMoment::BlockBasedMoment(end),
                RewardDistributionMoment::BlockBasedMoment(step_size),
            )
            | (
                RewardDistributionMoment::TimeBasedMoment(start),
                RewardDistributionMoment::TimeBasedMoment(end),
                RewardDistributionMoment::TimeBasedMoment(step_size),
            ) => {
                // If start >= end, by spec we return 0
                if *start >= *end {
                    return Ok(0);
                }

                if *step_size == 0 {
                    return Err(ProtocolError::InvalidDistributionStep(
                        "Step value cannot be zero",
                    ));
                }

                // Convert to "indexes"
                let start_index = start / step_size;
                let end_index = end / step_size;

                // Base count is the difference of those indexes
                let mut steps = end_index.saturating_sub(start_index);

                // Adjust if we're *not* including the start and the start is exactly on a boundary
                if !start_included && (start % step_size == 0) {
                    steps = steps.saturating_sub(1);
                }

                // Adjust if we *are* including the end and the end is exactly on a boundary
                if end_included && (end % step_size == 0) {
                    steps = steps.saturating_add(1);
                }

                Ok(steps)
            }
            (
                RewardDistributionMoment::EpochBasedMoment(start),
                RewardDistributionMoment::EpochBasedMoment(end),
                RewardDistributionMoment::EpochBasedMoment(step_size),
            ) => {
                // If start >= end, by spec we return 0
                if *start >= *end {
                    return Ok(0);
                }

                if *step_size == 0 {
                    return Err(ProtocolError::InvalidDistributionStep(
                        "Step value cannot be zero",
                    ));
                }

                let start_index = *start / *step_size;
                let end_index = *end / *step_size;

                let mut steps = end_index.saturating_sub(start_index) as u64;

                // Adjust if not including start but it's on a boundary
                if !start_included && (start % step_size == 0) {
                    steps = steps.saturating_sub(1);
                }

                // Adjust if end is included and exactly on a boundary
                if end_included && (end % step_size == 0) {
                    steps = steps.saturating_add(1);
                }

                Ok(steps)
            }
            _ => Err(ProtocolError::AddingDifferentTypes(
                "Cannot compute steps with mismatched types".to_string(),
            )),
        }
    }
}

impl From<RewardDistributionMoment> for u64 {
    /// Converts a `RewardDistributionMoment` into a `u64`.
    ///
    /// This conversion preserves the underlying numerical value.
    fn from(moment: RewardDistributionMoment) -> Self {
        moment.to_u64()
    }
}
impl Add<u64> for RewardDistributionMoment {
    type Output = Result<RewardDistributionMoment, ProtocolError>;

    fn add(self, rhs: u64) -> Self::Output {
        match self {
            RewardDistributionMoment::BlockBasedMoment(a) => a
                .checked_add(rhs)
                .map(RewardDistributionMoment::BlockBasedMoment)
                .ok_or(ProtocolError::Overflow("Block height addition overflow")),

            RewardDistributionMoment::TimeBasedMoment(a) => a
                .checked_add(rhs)
                .map(RewardDistributionMoment::TimeBasedMoment)
                .ok_or(ProtocolError::Overflow("Timestamp addition overflow")),

            RewardDistributionMoment::EpochBasedMoment(a) => {
                // Ensure `rhs` fits within `u16` before performing addition
                if rhs > u16::MAX as u64 {
                    return Err(ProtocolError::Overflow(
                        "Epoch index addition overflow: value exceeds u16 max",
                    ));
                }

                a.checked_add(rhs as u16)
                    .map(RewardDistributionMoment::EpochBasedMoment)
                    .ok_or(ProtocolError::Overflow("Epoch index addition overflow"))
            }
        }
    }
}

impl Add for RewardDistributionMoment {
    type Output = Result<RewardDistributionMoment, ProtocolError>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (
                RewardDistributionMoment::BlockBasedMoment(a),
                RewardDistributionMoment::BlockBasedMoment(b),
            ) => a
                .checked_add(b)
                .map(RewardDistributionMoment::BlockBasedMoment)
                .ok_or(ProtocolError::Overflow("Block height addition overflow")),
            (
                RewardDistributionMoment::TimeBasedMoment(a),
                RewardDistributionMoment::TimeBasedMoment(b),
            ) => a
                .checked_add(b)
                .map(RewardDistributionMoment::TimeBasedMoment)
                .ok_or(ProtocolError::Overflow("Timestamp addition overflow")),
            (
                RewardDistributionMoment::EpochBasedMoment(a),
                RewardDistributionMoment::EpochBasedMoment(b),
            ) => a
                .checked_add(b)
                .map(RewardDistributionMoment::EpochBasedMoment)
                .ok_or(ProtocolError::Overflow("Epoch index addition overflow")),
            _ => Err(ProtocolError::AddingDifferentTypes(
                "Cannot add different types of RewardDistributionMoment".to_string(),
            )),
        }
    }
}

impl Div<u64> for RewardDistributionMoment {
    type Output = Result<RewardDistributionMoment, ProtocolError>;

    fn div(self, rhs: u64) -> Self::Output {
        if rhs == 0 {
            return Err(ProtocolError::DivideByZero(
                "Cannot divide RewardDistributionMoment by zero",
            ));
        }

        match self {
            RewardDistributionMoment::BlockBasedMoment(a) => {
                Ok(RewardDistributionMoment::BlockBasedMoment(a / rhs))
            }
            RewardDistributionMoment::TimeBasedMoment(a) => {
                Ok(RewardDistributionMoment::TimeBasedMoment(a / rhs))
            }
            RewardDistributionMoment::EpochBasedMoment(a) => {
                // Ensure `rhs` fits within `u16` before performing addition
                if rhs > u16::MAX as u64 {
                    return Err(ProtocolError::Overflow(
                        "Epoch index addition overflow: value exceeds u16 max",
                    ));
                }
                Ok(RewardDistributionMoment::EpochBasedMoment(a / rhs as u16))
            }
        }
    }
}

impl Div for RewardDistributionMoment {
    type Output = Result<RewardDistributionMoment, ProtocolError>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (
                RewardDistributionMoment::BlockBasedMoment(a),
                RewardDistributionMoment::BlockBasedMoment(b),
            ) => {
                if b == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Cannot divide by zero block height",
                    ));
                }
                Ok(RewardDistributionMoment::BlockBasedMoment(a / b))
            }
            (
                RewardDistributionMoment::TimeBasedMoment(a),
                RewardDistributionMoment::TimeBasedMoment(b),
            ) => {
                if b == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Cannot divide by zero timestamp",
                    ));
                }
                Ok(RewardDistributionMoment::TimeBasedMoment(a / b))
            }
            (
                RewardDistributionMoment::EpochBasedMoment(a),
                RewardDistributionMoment::EpochBasedMoment(b),
            ) => {
                if b == 0 {
                    return Err(ProtocolError::DivideByZero(
                        "Cannot divide by zero epoch index",
                    ));
                }
                Ok(RewardDistributionMoment::EpochBasedMoment(a / b))
            }
            _ => Err(ProtocolError::AddingDifferentTypes(
                "Cannot divide different types of RewardDistributionMoment".to_string(),
            )),
        }
    }
}

impl PartialEq<&u64> for RewardDistributionMoment {
    fn eq(&self, other: &&u64) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => value == *other,
            RewardDistributionMoment::TimeBasedMoment(value) => value == *other,
            RewardDistributionMoment::EpochBasedMoment(value) => {
                if **other > u16::MAX as u64 {
                    false
                } else {
                    value == &(**other as u16)
                }
            }
        }
    }
}

impl PartialEq<u64> for RewardDistributionMoment {
    #[allow(clippy::op_ref)]
    fn eq(&self, other: &u64) -> bool {
        self == &other
    }
}

impl PartialEq<&u32> for RewardDistributionMoment {
    fn eq(&self, other: &&u32) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => *value as u32 == **other,
            RewardDistributionMoment::TimeBasedMoment(value) => *value as u32 == **other,
            RewardDistributionMoment::EpochBasedMoment(value) => *value as u32 == **other,
        }
    }
}

impl PartialEq<u32> for RewardDistributionMoment {
    #[allow(clippy::op_ref)]
    fn eq(&self, other: &u32) -> bool {
        self == &other
    }
}

impl PartialEq<&u16> for RewardDistributionMoment {
    fn eq(&self, other: &&u16) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => *value as u16 == **other,
            RewardDistributionMoment::TimeBasedMoment(value) => *value as u16 == **other,
            RewardDistributionMoment::EpochBasedMoment(value) => *value == **other,
        }
    }
}

impl PartialEq<u16> for RewardDistributionMoment {
    #[allow(clippy::op_ref)]
    fn eq(&self, other: &u16) -> bool {
        self == &other
    }
}

impl PartialEq<&usize> for RewardDistributionMoment {
    fn eq(&self, other: &&usize) -> bool {
        match self {
            RewardDistributionMoment::BlockBasedMoment(value) => *value as usize == **other,
            RewardDistributionMoment::TimeBasedMoment(value) => *value as usize == **other,
            RewardDistributionMoment::EpochBasedMoment(value) => *value as usize == **other,
        }
    }
}

impl PartialEq<usize> for RewardDistributionMoment {
    #[allow(clippy::op_ref)]
    fn eq(&self, other: &usize) -> bool {
        self == &other
    }
}

impl RewardDistributionMoment {
    /// Converts a reference to `BlockInfo` and a `RewardDistributionType` into a `RewardDistributionMoment`.
    ///
    /// This determines the appropriate `RewardDistributionMoment` based on the type of
    /// `RewardDistributionType`. The function selects:
    /// - **Block height** for block-based distributions.
    /// - **Timestamp (milliseconds)** for time-based distributions.
    /// - **Epoch index** for epoch-based distributions.
    ///
    /// # Arguments
    ///
    /// * `block_info` - A reference to the `BlockInfo` struct containing blockchain state details.
    /// * `distribution_type` - The `RewardDistributionType` to determine which moment should be used.
    ///
    /// # Returns
    ///
    /// Returns a `RewardDistributionMoment` corresponding to the type of distribution.
    pub fn from_block_info(
        block_info: &BlockInfo,
        distribution_type: &RewardDistributionType,
    ) -> Self {
        match distribution_type {
            RewardDistributionType::BlockBasedDistribution { .. } => {
                RewardDistributionMoment::BlockBasedMoment(block_info.height)
            }
            RewardDistributionType::TimeBasedDistribution { .. } => {
                RewardDistributionMoment::TimeBasedMoment(block_info.time_ms)
            }
            RewardDistributionType::EpochBasedDistribution { .. } => {
                RewardDistributionMoment::EpochBasedMoment(block_info.epoch.index)
            }
        }
    }
}

impl RewardDistributionMoment {
    pub fn to_be_bytes_vec(&self) -> Vec<u8> {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => height.to_be_bytes().to_vec(),
            RewardDistributionMoment::TimeBasedMoment(time) => time.to_be_bytes().to_vec(),
            RewardDistributionMoment::EpochBasedMoment(epoch) => epoch.to_be_bytes().to_vec(),
        }
    }
}

/// Implements `Display` for `RewardDistributionMoment`
impl fmt::Display for RewardDistributionMoment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => {
                write!(f, "BlockBasedMoment({})", height)
            }
            RewardDistributionMoment::TimeBasedMoment(timestamp) => {
                write!(f, "TimeBasedMoment({})", timestamp)
            }
            RewardDistributionMoment::EpochBasedMoment(epoch) => {
                write!(f, "EpochBasedMoment({})", epoch)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mismatch_block_vs_time() {
        let start = RewardDistributionMoment::BlockBasedMoment(10);
        let end = RewardDistributionMoment::TimeBasedMoment(50);
        let step = RewardDistributionMoment::BlockBasedMoment(5);

        // Mismatched type => Err(ProtocolError::AddingDifferentTypes)
        let result = start.steps_till(&end, &step, true, true);
        assert!(
            matches!(result, Err(ProtocolError::AddingDifferentTypes(_))),
            "Expected Err(AddingDifferentTypes), got: {:?}",
            result
        );
    }

    #[test]
    fn test_type_mismatch_block_vs_epoch() {
        let start = RewardDistributionMoment::BlockBasedMoment(10);
        let end = RewardDistributionMoment::EpochBasedMoment(50);
        let step = RewardDistributionMoment::BlockBasedMoment(5);

        let result = start.steps_till(&end, &step, true, true);
        assert!(
            matches!(result, Err(ProtocolError::AddingDifferentTypes(_))),
            "Expected Err(AddingDifferentTypes), got: {:?}",
            result
        );
    }

    #[test]
    fn test_zero_step_block_based() {
        let start = RewardDistributionMoment::BlockBasedMoment(10);
        let end = RewardDistributionMoment::BlockBasedMoment(50);
        let step = RewardDistributionMoment::BlockBasedMoment(0);

        let result = start.steps_till(&end, &step, true, true);
        assert!(
            matches!(result, Err(ProtocolError::InvalidDistributionStep(_))),
            "Expected Err(InvalidDistributionStep), got: {:?}",
            result
        );
    }

    #[test]
    fn test_start_greater_than_end_returns_zero() {
        let start = RewardDistributionMoment::TimeBasedMoment(100);
        let end = RewardDistributionMoment::TimeBasedMoment(50);
        let step = RewardDistributionMoment::TimeBasedMoment(10);

        // By spec, start >= end => 0
        let result = start.steps_till(&end, &step, true, true).unwrap();
        assert_eq!(result, 0, "Expected 0 steps when start >= end");
    }

    #[test]
    fn test_block_basic_inclusive() {
        let start = RewardDistributionMoment::BlockBasedMoment(0);
        let end = RewardDistributionMoment::BlockBasedMoment(100);
        let step = RewardDistributionMoment::BlockBasedMoment(10);

        // start_included = true, end_included = true
        // The multiples in [0..=100] are 0,10,20,30,40,50,60,70,80,90,100
        // We expect 11 intervals if we are counting from 0 to 100 inclusively by 10s.
        let result = start.steps_till(&end, &step, true, true).unwrap();
        assert_eq!(
            result, 11,
            "Expected 11 steps for [0..=100] in increments of 10"
        );
    }

    #[test]
    fn test_block_basic_exclusive() {
        let start = RewardDistributionMoment::BlockBasedMoment(0);
        let end = RewardDistributionMoment::BlockBasedMoment(100);
        let step = RewardDistributionMoment::BlockBasedMoment(10);

        // start_included = false, end_included = false
        // The multiples from 0..=100 by 10 are: 0,10,20,30,40,50,60,70,80,90,100
        // Excluding the start boundary (0) => skip that one
        // Excluding the end boundary (100) => skip that one
        // That leaves: 10,20,30,40,50,60,70,80,90 => 9 total
        let result = start.steps_till(&end, &step, false, false).unwrap();
        assert_eq!(
            result, 9,
            "Expected 9 steps for (0..100) in increments of 10"
        );
    }

    #[test]
    fn test_block_mixed_inclusive() {
        let start = RewardDistributionMoment::BlockBasedMoment(0);
        let end = RewardDistributionMoment::BlockBasedMoment(100);
        let step = RewardDistributionMoment::BlockBasedMoment(10);

        // start_included = false, end_included = true
        // Multiples are 0,10,20,30,40,50,60,70,80,90,100
        // Excluding start=0 => skip that boundary
        // Including end=100 => keep that boundary
        // That leaves: 10,20,30,40,50,60,70,80,90,100 => 10 total
        let result = start.steps_till(&end, &step, false, true).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_time_mixed_inclusive_with_non_multiple_bounds() {
        // Start and end are not multiples of the step
        let start = RewardDistributionMoment::TimeBasedMoment(3);
        let end = RewardDistributionMoment::TimeBasedMoment(27);
        let step = RewardDistributionMoment::TimeBasedMoment(5);

        // Multiples of 5 in the range 0..=27 are: 0,5,10,15,20,25
        // Our actual range is start=3 to end=27.
        //  - The multiples in [3..=27] are 5,10,15,20,25.
        //
        // start_included = true => but 3 is not a multiple, so that doesn't add a step
        // end_included = true => 27 is not a multiple, so that doesn't add a step
        //
        // So we only have the steps at 5,10,15,20,25 => that's 5 steps.
        let result = start.steps_till(&end, &step, true, true).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_epoch_inclusive_boundaries() {
        // Now test an epoch-based moment
        // Start=1, End=10, Step=1
        let start = RewardDistributionMoment::EpochBasedMoment(1);
        let end = RewardDistributionMoment::EpochBasedMoment(10);
        let step = RewardDistributionMoment::EpochBasedMoment(1);

        // start_included=true, end_included=true
        // If we’re counting steps at each integer from 1..=10, that’s 10 steps
        // Because each integer point is a boundary.
        let result = start.steps_till(&end, &step, true, true).unwrap();
        assert_eq!(result, 10, "Expected 10 steps for [1..=10] with step=1");
    }

    #[test]
    fn test_epoch_exclusive_boundaries() {
        // Start=1, End=10, Step=1
        let start = RewardDistributionMoment::EpochBasedMoment(1);
        let end = RewardDistributionMoment::EpochBasedMoment(10);
        let step = RewardDistributionMoment::EpochBasedMoment(1);

        // start_included=false, end_included=false
        // If we exclude the start boundary=1 and the end boundary=10,
        // we are left with steps at 2,3,4,5,6,7,8,9 => total 8
        let result = start.steps_till(&end, &step, false, false).unwrap();
        assert_eq!(result, 8, "Expected 8 steps for (1..10) with step=1");
    }

    #[test]
    fn test_epoch_start_between_boundaries() {
        // Start=2, End=10, Step=3
        let start = RewardDistributionMoment::EpochBasedMoment(2);
        let end = RewardDistributionMoment::EpochBasedMoment(10);
        let step = RewardDistributionMoment::EpochBasedMoment(3);

        // Multiples of 3 up to 10 are: 0,3,6,9 (12 is beyond 10).
        // In the range [2..10], the valid multiples are: 3,6,9
        //
        // start_included = true => 2 is not a multiple, so it doesn’t add a boundary
        // end_included = true => 10 is not a multiple, so it doesn’t add a boundary
        //
        // So steps are at 3,6,9 => 3 total
        let result = start.steps_till(&end, &step, true, true).unwrap();
        assert_eq!(result, 3);
    }
}
