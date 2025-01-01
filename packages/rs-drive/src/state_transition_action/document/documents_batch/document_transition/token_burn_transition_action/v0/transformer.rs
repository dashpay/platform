use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_burn_transition::v0::TokenBurnTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::v0::TokenBurnTransitionActionV0;

impl TokenBurnTransitionActionV0 {
    /// Attempts to create a `TokenBurnTransitionActionV0` from the given `TokenBurnTransitionV0` value.
    ///
    /// This function extracts the necessary data from the provided `TokenBurnTransitionV0` and
    /// delegates to the `try_from_base_transition_with_contract_lookup` function to construct the
    /// base action. It then constructs the `TokenBurnTransitionActionV0` struct by including the
    /// `burn_amount` and `public_note` values, along with the base action.
    ///
    /// # Parameters
    /// - `drive`: A reference to the `Drive` struct which provides access to the system.
    /// - `owner_id`: The identifier of the owner initiating the burn transition.
    /// - `value`: The `TokenBurnTransitionV0` containing the details for the token burn.
    /// - `approximate_without_state_for_costs`: A flag indicating whether to approximate state costs.
    /// - `transaction`: The transaction argument used for state changes.
    /// - `drive_operations`: A mutable reference to the vector of low-level operations that need to be performed.
    /// - `get_data_contract`: A closure function that looks up the data contract for a given identifier.
    /// - `platform_version`: The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    /// A `Result` containing the constructed `TokenBurnTransitionActionV0` on success, or an error
    /// if any issues occur during the process.
    ///
    /// # Errors
    /// - Returns an `Error` if any error occurs while trying to create the base action or process the burn.
    pub fn try_from_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBurnTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenBurnTransitionV0 {
            base,
            burn_amount,
            public_note,
        } = value;

        let base_action = TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
            drive,
            owner_id,
            base,
            approximate_without_state_for_costs,
            transaction,
            drive_operations,
            get_data_contract,
            platform_version,
        )?;

        Ok(TokenBurnTransitionActionV0 {
            base: base_action,
            burn_amount,
            public_note,
        })
    }

    /// Attempts to create a `TokenBurnTransitionActionV0` from the borrowed `TokenBurnTransitionV0` value.
    ///
    /// This function is similar to `try_from_token_burn_transition_with_contract_lookup`, but it
    /// operates on a borrowed `TokenBurnTransitionV0` to avoid ownership transfer. It delegates
    /// to `try_from_borrowed_base_transition_with_contract_lookup` for constructing the base action,
    /// then combines it with the `burn_amount` and `public_note` to form a `TokenBurnTransitionActionV0`.
    ///
    /// # Parameters
    /// - `drive`: A reference to the `Drive` struct which provides access to the system.
    /// - `owner_id`: The identifier of the owner initiating the burn transition.
    /// - `value`: A borrowed reference to the `TokenBurnTransitionV0` containing the details for the token burn.
    /// - `approximate_without_state_for_costs`: A flag indicating whether to approximate state costs.
    /// - `transaction`: The transaction argument used for state changes.
    /// - `drive_operations`: A mutable reference to the vector of low-level operations that need to be performed.
    /// - `get_data_contract`: A closure function that looks up the data contract for a given identifier.
    /// - `platform_version`: The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    /// A `Result` containing the constructed `TokenBurnTransitionActionV0` on success, or an error
    /// if any issues occur during the process.
    ///
    /// # Errors
    /// - Returns an `Error` if any error occurs while trying to create the base action or process the burn.
    pub fn try_from_borrowed_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenBurnTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenBurnTransitionV0 {
            base,
            burn_amount,
            public_note,
        } = value;

        let base_action =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                drive_operations,
                get_data_contract,
                platform_version,
            )?;

        Ok(TokenBurnTransitionActionV0 {
            base: base_action,
            burn_amount: *burn_amount,
            public_note: public_note.clone(),
        })
    }
}
