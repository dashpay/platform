use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::TokenTransferTransitionActionV0;

impl TokenTransferTransitionActionV0 {
    /// Converts a `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using the provided contract lookup and additional parameters.
    ///
    /// This method processes the token transfer transition and returns the corresponding transition action,
    /// while looking up necessary data contracts, performing the required checks, and applying the relevant logic for token transfer.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `value` - The `TokenTransferTransitionV0` struct containing the transition data, including token amount,
    ///   recipient details, and encrypted notes.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `transaction` - The transaction context for state changes and related operations.
    /// * `drive_operations` - A mutable reference to a vector that will hold the low-level drive operations performed
    ///   during this transition. This allows tracking the changes that need to be persisted.
    /// * `get_data_contract` - A closure to fetch data contract information based on a contract identifier.
    /// * `platform_version` - A reference to the platform version for version-specific transition logic.
    ///
    /// # Returns
    ///
    /// * `Result<TokenTransferTransitionActionV0, ProtocolError>` - Returns the constructed `TokenTransferTransitionActionV0`
    ///   if successful, or an error if any issue arises (e.g., missing data or an invalid state transition).
    pub fn try_from_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenTransferTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_owner_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = value;

        // Lookup the base action using the base transition data and contract information
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

        // Return the TokenTransferTransitionActionV0 with the relevant data
        Ok(TokenTransferTransitionActionV0 {
            base: base_action,
            amount,
            recipient_id: recipient_owner_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        })
    }

    /// Converts a borrowed `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using the provided contract lookup and additional parameters.
    ///
    /// This method works similarly to `try_from_token_transfer_transition_with_contract_lookup`, but it borrows the
    /// `TokenTransferTransitionV0` to avoid unnecessary cloning of the transition object.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance for data access.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `value` - A reference to the `TokenTransferTransitionV0` struct containing transition data (borrowed).
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation.
    /// * `transaction` - The transaction context for state changes and related operations.
    /// * `drive_operations` - A mutable reference to a vector that will hold the low-level drive operations performed.
    /// * `get_data_contract` - A closure to fetch data contract information based on a contract identifier.
    /// * `platform_version` - A reference to the platform version for version-specific transition logic.
    ///
    /// # Returns
    ///
    /// * `Result<TokenTransferTransitionActionV0, ProtocolError>` - Returns the resulting `TokenTransferTransitionActionV0`
    ///   if successful, or an error if something goes wrong (e.g., missing data, invalid state).
    pub fn try_from_borrowed_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenTransferTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_owner_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = value;

        // Lookup the base action using the borrowed base transition data and contract information
        let base_action =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                &base,
                approximate_without_state_for_costs,
                transaction,
                drive_operations,
                get_data_contract,
                platform_version,
            )?;

        // Return the TokenTransferTransitionActionV0 with the relevant data
        Ok(TokenTransferTransitionActionV0 {
            base: base_action.into(),
            amount: *amount,
            recipient_id: *recipient_owner_id,
            public_note: public_note.clone(),
            shared_encrypted_note: shared_encrypted_note.clone(),
            private_encrypted_note: private_encrypted_note.clone(),
        })
    }
}
