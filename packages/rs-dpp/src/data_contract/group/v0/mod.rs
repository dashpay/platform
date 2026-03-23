use crate::consensus::basic::data_contract::{
    GroupExceedsMaxMembersError, GroupHasTooFewMembersError, GroupMemberHasPowerOfZeroError,
    GroupMemberHasPowerOverLimitError, GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError,
    GroupRequiredPowerIsInvalidError, GroupTotalPowerLessThanRequiredError,
};
use crate::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use crate::data_contract::group::methods::v0::GroupMethodsV0;
use crate::data_contract::group::{GroupMemberPower, GroupRequiredPower};
use crate::data_contract::GroupContractPosition;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[serde(rename_all = "camelCase")]
#[platform_serialize(unversioned)]
pub struct GroupV0 {
    pub members: BTreeMap<Identifier, GroupMemberPower>,
    pub required_power: GroupRequiredPower,
}

impl GroupV0Getters for GroupV0 {
    fn member_power(&self, member_id: Identifier) -> Result<u32, ProtocolError> {
        self.members
            .get(&member_id)
            .cloned()
            .ok_or(ProtocolError::GroupMemberNotFound(format!(
                "Group member {} not found",
                member_id
            )))
    }

    fn members(&self) -> &BTreeMap<Identifier, u32> {
        &self.members
    }

    fn members_mut(&mut self) -> &mut BTreeMap<Identifier, u32> {
        &mut self.members
    }

    fn required_power(&self) -> GroupRequiredPower {
        self.required_power
    }
}

impl GroupV0Setters for GroupV0 {
    fn set_members(&mut self, members: BTreeMap<Identifier, u32>) {
        self.members = members;
    }

    fn set_member_power(&mut self, member_id: Identifier, power: u32) {
        self.members.insert(member_id, power);
    }

    fn remove_member(&mut self, member_id: &Identifier) -> bool {
        self.members.remove(member_id).is_some()
    }

    fn set_required_power(&mut self, required_power: GroupRequiredPower) {
        self.required_power = required_power;
    }
}

impl GroupMethodsV0 for GroupV0 {
    /// Validates the group to ensure:
    /// - The sum of all group member powers is equal to or greater than the required power.
    /// - No group member has a power of 0.
    /// - The group does not exceed the maximum allowed members (256).
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)` if the group is valid.
    /// - `Err(ProtocolError)` if validation fails due to an invalid group configuration.
    fn validate(
        &self,
        group_contract_position: Option<GroupContractPosition>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let max_group_members = platform_version.system_limits.max_contract_group_size as u32;
        const GROUP_POWER_LIMIT: GroupMemberPower = u16::MAX as GroupMemberPower;

        // Check the number of members does not exceed the maximum allowed
        if self.members.len() as u32 > max_group_members {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                GroupExceedsMaxMembersError::new(max_group_members).into(),
            ));
        }

        if self.members.len() < 2 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                GroupHasTooFewMembersError::new(group_contract_position).into(),
            ));
        }

        let mut total_power: GroupMemberPower = 0;

        let mut total_power_without_unilateral_members: GroupMemberPower = 0;

        // Iterate over members to validate their power and calculate the total power
        for (&member, &power) in &self.members {
            if power == 0 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    GroupMemberHasPowerOfZeroError::new(member).into(),
                ));
            }
            if power > GROUP_POWER_LIMIT {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    GroupMemberHasPowerOverLimitError::new(member, power, GROUP_POWER_LIMIT).into(),
                ));
            }
            if power > self.required_power {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    GroupMemberHasPowerOverLimitError::new(member, power, self.required_power)
                        .into(),
                ));
            }
            total_power = total_power
                .checked_add(power)
                .ok_or_else(|| ProtocolError::Overflow("Total power overflowed"))?;

            if power < self.required_power {
                total_power_without_unilateral_members = total_power_without_unilateral_members
                    .checked_add(power)
                    .ok_or_else(|| ProtocolError::Overflow("Total power overflowed"))?;
            }
        }

        // Check if the total power meets the required power
        if total_power < self.required_power {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                GroupTotalPowerLessThanRequiredError::new(total_power, self.required_power).into(),
            ));
        }

        // Check if the total power without unilateral members meets the required power
        if total_power_without_unilateral_members < self.required_power
            && total_power_without_unilateral_members > 0
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError::new(
                    total_power_without_unilateral_members,
                    self.required_power,
                )
                .into(),
            ));
        }

        if self.required_power == 0 || self.required_power() > GROUP_POWER_LIMIT {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                GroupRequiredPowerIsInvalidError::new(self.required_power, GROUP_POWER_LIMIT)
                    .into(),
            ));
        }

        // If all validations pass, return an empty validation result
        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validate {
        use super::*;
        use crate::consensus::basic::BasicError;
        use crate::consensus::ConsensusError;

        #[test]
        fn test_group_with_all_unilateral_members() {
            let member1 = Identifier::random();
            let member2 = Identifier::random();

            let group = GroupV0 {
                members: [(member1, 1), (member2, 1)].into(),
                required_power: 1,
            };

            let platform_version = PlatformVersion::latest();

            let result = group
                .validate(None, platform_version)
                .expect("group should be valid");

            assert!(result.is_valid());
        }

        #[test]
        fn test_group_exceeds_max_members() {
            let platform_version = PlatformVersion::latest();
            let max = platform_version.system_limits.max_contract_group_size as u32;

            let mut members = BTreeMap::new();
            for i in 0..=max {
                let mut id_bytes = [0u8; 32];
                id_bytes[0..4].copy_from_slice(&i.to_le_bytes());
                members.insert(Identifier::new(id_bytes), 1);
            }

            let group = GroupV0 {
                members,
                required_power: 1,
            };

            let result = group
                .validate(None, platform_version)
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupExceedsMaxMembersError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupExceedsMaxMembersError");
            };
        }

        #[test]
        fn test_group_too_few_members_one() {
            let group = GroupV0 {
                members: [(Identifier::random(), 1)].into(),
                required_power: 1,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupHasTooFewMembersError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupHasTooFewMembersError");
            };
        }

        #[test]
        fn test_group_member_has_power_of_zero() {
            let group = GroupV0 {
                members: [(Identifier::random(), 0), (Identifier::random(), 1)].into(),
                required_power: 1,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupMemberHasPowerOfZeroError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupMemberHasPowerOfZeroError");
            };
        }

        #[test]
        fn test_group_member_power_over_limit() {
            let group = GroupV0 {
                members: [(Identifier::random(), 65_536), (Identifier::random(), 1)].into(),
                required_power: 65_536,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupMemberHasPowerOverLimitError");
            };
        }

        #[test]
        fn test_group_member_power_exceeds_required() {
            let group = GroupV0 {
                members: [(Identifier::random(), 6), (Identifier::random(), 5)].into(),
                required_power: 5,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupMemberHasPowerOverLimitError");
            };
        }

        #[test]
        fn test_group_total_power_less_than_required() {
            let group = GroupV0 {
                members: [(Identifier::random(), 2), (Identifier::random(), 2)].into(),
                required_power: 5,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupTotalPowerLessThanRequiredError(
                _,
            ))) = result.errors.first()
            else {
                panic!("expected GroupTotalPowerLessThanRequiredError");
            };
        }

        #[test]
        fn test_group_non_unilateral_member_power_less_than_required() {
            let group = GroupV0 {
                members: [(Identifier::random(), 10), (Identifier::random(), 5)].into(),
                required_power: 10,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(
                BasicError::GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(_),
            )) = result.errors.first()
            else {
                panic!("expected GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError");
            };
        }

        #[test]
        fn test_group_required_power_zero() {
            let group = GroupV0 {
                members: [(Identifier::random(), 1), (Identifier::random(), 1)].into(),
                required_power: 0,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            // Required power of zero is currently intercepted by the per-member `power > required_power`
            // check before `GroupRequiredPowerIsInvalidError` is evaluated.
            let Some(ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupMemberHasPowerOverLimitError");
            };
        }

        #[test]
        fn test_group_required_power_over_limit() {
            let group = GroupV0 {
                members: [
                    (Identifier::random(), 65_535),
                    (Identifier::random(), 65_535),
                    (Identifier::random(), 65_535),
                ]
                .into(),
                required_power: 65_536,
            };

            let result = group
                .validate(None, PlatformVersion::latest())
                .expect("should not error");

            let Some(ConsensusError::BasicError(BasicError::GroupRequiredPowerIsInvalidError(_))) =
                result.errors.first()
            else {
                panic!("expected GroupRequiredPowerIsInvalidError");
            };
        }
    }
}
