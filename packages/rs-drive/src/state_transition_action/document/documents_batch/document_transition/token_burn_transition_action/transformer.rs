use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{
    TokenBurnTransitionAction, TokenBurnTransitionActionV0,
};
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransition;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

/// Implement methods to transform a `TokenBurnTransition` into a `TokenBurnTransitionAction`.
impl TokenBurnTransitionAction {
    /// Transform a `TokenBurnTransition` into a `TokenBurnTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance used for accessing the system.
    /// * `owner_id` - The identifier of the owner initiating the burn transition.
    /// * `transaction` - The transaction argument used for state changes.
    /// * `value` - A `TokenBurnTransition` instance.
    /// * `approximate_without_state_for_costs` - A flag indicating whether to approximate state costs without full state.
    /// * `drive_operations` - A mutable reference to the vector of low-level operations that need to be performed.
    /// * `get_data_contract` - A closure that fetches the `DataContractFetchInfo` given a contract ID.
    /// * `platform_version` - The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    ///
    /// * `Result<TokenBurnTransitionAction, ProtocolError>` - A `TokenBurnTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBurnTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            TokenBurnTransition::V0(v0) => {
                let v0_action = TokenBurnTransitionActionV0::try_from_token_burn_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    drive_operations,
                    get_data_contract,
                    platform_version,
                )?;
                Ok(v0_action.into())
            }
        }
    }

    /// Transform a borrowed `TokenBurnTransition` into a `TokenBurnTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance used for accessing the system.
    /// * `owner_id` - The identifier of the owner initiating the burn transition.
    /// * `transaction` - The transaction argument used for state changes.
    /// * `value` - A reference to a `TokenBurnTransition`.
    /// * `approximate_without_state_for_costs` - A flag indicating whether to approximate state costs without full state.
    /// * `drive_operations` - A mutable reference to the vector of low-level operations that need to be performed.
    /// * `get_data_contract` - A closure that fetches the `DataContractFetchInfo` given a contract ID.
    /// * `platform_version` - The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    ///
    /// * `Result<TokenBurnTransitionAction, ProtocolError>` - A `TokenBurnTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_borrowed_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenBurnTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            TokenBurnTransition::V0(v0) => {
                let v0_action = TokenBurnTransitionActionV0::try_from_borrowed_token_burn_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    drive_operations,
                    get_data_contract,
                    platform_version,
                )?;
                Ok(v0_action.into())
            }
        }
    }
}
