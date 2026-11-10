use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::UnsupportedVersionError;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

/// The refusal of a delta update that reaches a transformer of updates carrying the whole
/// contract. A delta becomes an action only once it is merged onto the stored contract, which
/// its validation does from protocol version 15
/// ([`DataContractUpdateTransitionAction::into_delta_action`]), so one that gets here was not
/// merged. It is refused as an unsupported version, a consensus error, rather than with an
/// error that would stop block execution.
fn delta_not_merged(
    transition: &DataContractUpdateTransition,
    platform_version: &PlatformVersion,
) -> ProtocolError {
    let bounds = &platform_version
        .dpp
        .state_transition_serialization_versions
        .contract_update_state_transition;
    ProtocolError::ConsensusError(Box::new(
        UnsupportedVersionError::new(
            transition.feature_version(),
            bounds.min_version,
            bounds.max_version,
        )
        .into(),
    ))
}

impl DataContractUpdateTransitionAction {
    /// tries to transform the DataContractUpdateTransition into a DataContractUpdateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid
    ///
    /// A delta update, which embeds no contract, is refused with a consensus error: see
    /// [`Self::into_delta_action`].
    pub fn try_from_transition(
        value: DataContractUpdateTransition,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                Ok(DataContractUpdateTransitionActionV0::try_from_transition(
                    v0,
                    block_info,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?
                .into())
            }
            DataContractUpdateTransition::V1(_) => Err(delta_not_merged(&value, platform_version)),
        }
    }

    /// tries to transform the borrowed DataContractUpdateTransition into a DataContractUpdateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid
    ///
    /// A delta update, which embeds no contract, is refused with a consensus error: see
    /// [`Self::into_delta_action`].
    pub fn try_from_borrowed_transition(
        value: &DataContractUpdateTransition,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractUpdateTransition::V0(v0) => Ok(
                DataContractUpdateTransitionActionV0::try_from_borrowed_transition(
                    v0,
                    block_info,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?
                .into(),
            ),
            DataContractUpdateTransition::V1(_) => Err(delta_not_merged(value, platform_version)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
    use dpp::tests::fixtures::get_data_contract_fixture;

    /// A delta is merged onto the stored contract before it becomes an action. One that reaches
    /// a transformer of updates carrying the whole contract is refused with a consensus error,
    /// never an error that would stop block execution.
    #[test]
    fn should_refuse_a_delta_that_was_not_merged_with_a_consensus_error() {
        let platform_version = PlatformVersion::latest();
        let old_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        let delta: DataContractUpdateTransition =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("expected a delta")
                .into();

        for result in [
            DataContractUpdateTransitionAction::try_from_borrowed_transition(
                &delta,
                &BlockInfo::default(),
                true,
                &mut vec![],
                platform_version,
            ),
            DataContractUpdateTransitionAction::try_from_transition(
                delta.clone(),
                &BlockInfo::default(),
                true,
                &mut vec![],
                platform_version,
            ),
        ] {
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(error)) if matches!(
                    *error,
                    ConsensusError::BasicError(BasicError::UnsupportedVersionError(_))
                )
            );
        }
    }
}
