use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::TokenTransferTransition;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::v0::TokenTransferTransitionActionV0;

/// Implement methods to transform a `TokenTransferTransition` into a `TokenTransferTransitionAction`.
impl TokenTransferTransitionAction {
    /// Transform a `TokenTransferTransition` into a `TokenTransferTransitionAction` using the provided data contract lookup and additional parameters.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `transaction` - The transaction context for state changes and related operations.
    /// * `value` - A `TokenTransferTransition` instance.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation.
    /// * `drive_operations` - A mutable reference to a vector that will hold the low-level drive operations performed.
    /// * `get_data_contract` - A closure that fetches data contract information based on a contract identifier.
    /// * `platform_version` - A reference to the platform version for version-specific transition logic.
    ///
    /// # Returns
    ///
    /// * `Result<TokenTransferTransitionAction, ProtocolError>` - A `TokenTransferTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn from_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: TokenTransferTransition,
        owner_id: Identifier,
        approximate_without_state_for_costs: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            TokenTransferTransition::V0(v0) => {
                let v0_action = TokenTransferTransitionActionV0::try_from_token_transfer_transition_with_contract_lookup(
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

    /// Transform a borrowed `TokenTransferTransition` into a `TokenTransferTransitionAction` using the provided data contract lookup and additional parameters.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `transaction` - The transaction context for state changes and related operations.
    /// * `value` - A reference to a `TokenTransferTransition`.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation.
    /// * `drive_operations` - A mutable reference to a vector that will hold the low-level drive operations performed.
    /// * `get_data_contract` - A closure that fetches data contract information based on a contract identifier.
    /// * `platform_version` - A reference to the platform version for version-specific transition logic.
    ///
    /// # Returns
    ///
    /// * `Result<TokenTransferTransitionAction, ProtocolError>` - A `TokenTransferTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_borrowed_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: &TokenTransferTransition,
        owner_id: Identifier,
        approximate_without_state_for_costs: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            TokenTransferTransition::V0(v0) => {
                let v0_action = TokenTransferTransitionActionV0::try_from_borrowed_token_transfer_transition_with_contract_lookup(
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
