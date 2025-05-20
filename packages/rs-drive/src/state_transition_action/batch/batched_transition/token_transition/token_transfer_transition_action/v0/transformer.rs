use dpp::identifier::Identifier;
use dpp::state_transition::state_transitions::document::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionActionV0;

impl TokenTransferTransitionActionV0 {
    /// Converts a `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token transfer transition, looks up the necessary data contracts, performs required
    /// checks, and applies the relevant logic for token transfer. It also calculates the fees associated with the transaction.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `value` - The `TokenTransferTransitionV0` struct containing the transition data, including token amount,
    ///   recipient details, and encrypted notes.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing transaction cost calculations when full state is not needed.
    /// * `transaction` - The transaction context, which includes state details and necessary operations for the transition.
    /// * `block_info` - Information about the current block to calculate the fees for the transition.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns the associated `DataContractFetchInfo`.
    /// * `platform_version` - A reference to the platform version, ensuring that the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenTransferTransitionActionV0, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenTransferTransitionActionV0` and the calculated `FeeResult` if successful, or an error if the transition cannot
    ///   be created or an issue arises with the provided state or data.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenTransferTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = value;

        let mut drive_operations = vec![];

        // Lookup the base action using the base transition data and contract information
        // There is no change note for transfer tokens
        let (base_action, _change_note) =
            TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?
            .into_data()?;

        let fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        // Return the TokenTransferTransitionActionV0 with the relevant data
        Ok((
            TokenTransferTransitionActionV0 {
                base: base_action,
                amount,
                recipient_id,
                public_note,
                shared_encrypted_note,
                private_encrypted_note,
            },
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token transfer transition similarly to the `try_from_token_transfer_transition_with_contract_lookup`
    /// method but operates on a borrowed reference of the `TokenTransferTransitionV0`. It performs the same checks and applies
    /// the token transfer logic, while avoiding copying the `TokenTransferTransitionV0` struct.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `value` - A reference to the `TokenTransferTransitionV0` struct containing the transition data, including token amount,
    ///   recipient details, and encrypted notes.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing transaction cost calculations when full state is not needed.
    /// * `transaction` - The transaction context, which includes state details and necessary operations for the transition.
    /// * `block_info` - Information about the current block to calculate the fees for the transition.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns the associated `DataContractFetchInfo`.
    /// * `platform_version` - A reference to the platform version, ensuring that the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenTransferTransitionActionV0, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenTransferTransitionActionV0` and the calculated `FeeResult` if successful, or an error if the transition cannot
    ///   be created or an issue arises with the provided state or data.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenTransferTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = value;

        let mut drive_operations = vec![];

        // Lookup the base action using the borrowed base transition data and contract information
        // We can never change the note
        let (base_action, _change_note) =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?
            .into_data()?;

        let fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        // Return the TokenTransferTransitionActionV0 with the relevant data
        Ok((
            TokenTransferTransitionActionV0 {
                base: base_action,
                amount: *amount,
                recipient_id: *recipient_id,
                public_note: public_note.clone(),
                shared_encrypted_note: shared_encrypted_note.clone(),
                private_encrypted_note: private_encrypted_note.clone(),
            },
            fee_result,
        ))
    }
}
