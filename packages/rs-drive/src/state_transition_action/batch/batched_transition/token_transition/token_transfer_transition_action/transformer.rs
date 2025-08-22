use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::TokenTransferTransition;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::v0::TokenTransferTransitionActionV0;

/// Implement methods to transform a `TokenTransferTransition` into a `TokenTransferTransitionAction`.
impl TokenTransferTransitionAction {
    /// Converts a `TokenTransferTransition` into a `TokenTransferTransitionAction` using the provided contract lookup.
    ///
    /// This function processes a `TokenTransferTransition` (which may contain multiple versions), looks up the necessary data
    /// contracts, and calculates the associated fees for the transaction. Currently, only the `V0` variant of the transition is
    /// supported. The result is a `TokenTransferTransitionAction` along with the fee result.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance for handling data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the token transfer.
    /// * `value` - The `TokenTransferTransition` containing the transition data (currently only the `V0` variant is supported).
    /// * `approximate_without_state_for_costs` - A flag to approximate transaction costs without full state consideration.
    /// * `transaction` - The transaction context, which provides the necessary state for processing the transition.
    /// * `block_info` - Information about the current block used to calculate fees for the transition.
    /// * `get_data_contract` - A closure that takes an identifier and returns the associated `DataContractFetchInfo`.
    /// * `platform_version` - The platform version to ensure the transition is compatible with the current version logic.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenTransferTransitionAction, FeeResult), Error>` - A result containing the constructed `TokenTransferTransitionAction`
    ///   and the calculated `FeeResult`, or an error if the transition cannot be processed.
    #[allow(clippy::too_many_arguments)]
    pub fn from_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenTransferTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        match value {
            TokenTransferTransition::V0(v0) => {
                let (v0, fee) =  TokenTransferTransitionActionV0::try_from_token_transfer_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    block_info,
                    get_data_contract,
                    platform_version,
                )?;
                Ok((v0.into(), fee))
            }
        }
    }

    /// Converts a borrowed reference of a `TokenTransferTransition` into a `TokenTransferTransitionAction` using the provided contract lookup.
    ///
    /// This function is similar to `from_token_transfer_transition_with_contract_lookup` but operates on a borrowed reference of the
    /// `TokenTransferTransition`, which avoids copying the data. It processes the `TokenTransferTransition`, looks up the necessary
    /// data contracts, and calculates the associated fees for the transaction. Only the `V0` variant of the transition is supported.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance for handling data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the token transfer.
    /// * `value` - A borrowed reference to the `TokenTransferTransition` containing the transition data (currently only the `V0` variant is supported).
    /// * `approximate_without_state_for_costs` - A flag to approximate transaction costs without full state consideration.
    /// * `transaction` - The transaction context, which provides the necessary state for processing the transition.
    /// * `block_info` - Information about the current block used to calculate fees for the transition.
    /// * `get_data_contract` - A closure that takes an identifier and returns the associated `DataContractFetchInfo`.
    /// * `platform_version` - The platform version to ensure the transition is compatible with the current version logic.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenTransferTransitionAction, FeeResult), Error>` - A result containing the constructed `TokenTransferTransitionAction`
    #[allow(clippy::too_many_arguments)]
    ///   and the calculated `FeeResult`, or an error if the transition cannot be processed.
    pub fn try_from_borrowed_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenTransferTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        match value {
            TokenTransferTransition::V0(v0) => {
                let (v0, fee) =  TokenTransferTransitionActionV0::try_from_borrowed_token_transfer_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    block_info,
                    get_data_contract,
                    platform_version,
                )?;
                Ok((v0.into(), fee))
            }
        }
    }
}
