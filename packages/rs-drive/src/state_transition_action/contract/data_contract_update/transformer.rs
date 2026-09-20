use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::v1::DataContractUpdateTransitionActionV1;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::UnsupportedVersionError;
use dpp::data_contract::DataContract;
use dpp::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV1,
};
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::validation::ConsensusValidationResult;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

/// A delta-based (V1) update can only become an action once merged onto the
/// stored contract (see `try_from_borrowed_v1_transition`). Reaching the
/// full-contract transformers with one means the node runs a protocol version
/// that does not know the form, so it is rejected as an unsupported version:
/// a consensus error, so validation degrades to a nonce bump rather than an
/// execution error.
fn unsupported_v1(
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
    /// A delta-based (V1) transition needs the stored contract; use
    /// [`try_from_borrowed_v1_transition`](Self::try_from_borrowed_v1_transition).
    /// Here it is an unsupported version, a consensus error.
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
            DataContractUpdateTransition::V1(_) => Err(unsupported_v1(&value, platform_version)),
        }
    }

    /// tries to transform the borrowed DataContractUpdateTransition into a DataContractUpdateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid
    ///
    /// A delta-based (V1) transition needs the stored contract; use
    /// [`try_from_borrowed_v1_transition`](Self::try_from_borrowed_v1_transition).
    /// Here it is an unsupported version, a consensus error.
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
            DataContractUpdateTransition::V1(_) => Err(unsupported_v1(value, platform_version)),
        }
    }

    /// Merges a delta-based (V1) update transition onto the stored contract
    /// it targets and returns the action that applies the result.
    ///
    /// The caller fetched `old_data_contract` from state (and charged for
    /// the fetch). The delta's own checks (ownership, updated entries exist,
    /// new entries do not) come back as consensus errors in the result; the
    /// merged contract still has to pass the same update validation a
    /// full-contract transition gets.
    pub fn try_from_borrowed_v1_transition(
        value: &DataContractUpdateTransitionV1,
        old_data_contract: &DataContract,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError> {
        let registration_cost = value.registration_cost(platform_version)?;

        let validation_result = old_data_contract.apply_update(
            value.into(),
            block_info,
            full_validation,
            validation_operations,
            platform_version,
        )?;

        Ok(validation_result.map(|data_contract| {
            DataContractUpdateTransitionActionV1 {
                data_contract,
                identity_contract_nonce: value.identity_contract_nonce,
                user_fee_increase: value.user_fee_increase,
                registration_cost,
            }
            .into()
        }))
    }
}
