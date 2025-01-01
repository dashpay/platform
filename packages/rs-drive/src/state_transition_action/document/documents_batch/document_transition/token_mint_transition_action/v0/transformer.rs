use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_mint_transition::v0::TokenMintTransitionV0;
use dpp::ProtocolError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::tokens::errors::TokenError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_mint_transition_action::v0::TokenMintTransitionActionV0;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

impl TokenMintTransitionActionV0 {
    /// Converts a `TokenMintTransitionV0` into a `TokenMintTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token minting transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for minting.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the minting transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenMintTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `drive_operations` - A mutable reference to a vector that will hold the low-level drive operations performed
    ///   during this transition. This allows tracking the changes that need to be persisted.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<TokenMintTransitionActionV0, Error>` - Returns the constructed `TokenMintTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    pub fn try_from_token_mint_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenMintTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenMintTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
            public_note,
        } = value;

        let position = base.token_contract_position();

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

        let identity_balance_holder_id = issued_to_identity_id
            .or_else(|| {
                base_action
                    .data_contract_fetch_info_ref()
                    .contract
                    .tokens()
                    .get(&position)
                    .and_then(|token_configuration| {
                        token_configuration.new_tokens_destination_identity()
                    })
            })
            .ok_or(ProtocolError::Token(
                TokenError::DestinationIdentityForMintingNotSetError.into(),
            ))?;

        Ok(TokenMintTransitionActionV0 {
            base: base_action,
            mint_amount: amount,
            identity_balance_holder_id,
            public_note,
        })
    }

    /// Converts a borrowed `TokenMintTransitionV0` into a `TokenMintTransitionActionV0` using the provided contract lookup.
    ///
    /// This method works similarly to `try_from_token_mint_transition_with_contract_lookup`, but it borrows the
    /// `TokenMintTransitionV0` to avoid unnecessary cloning of the transition object.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance for data access.
    /// * `owner_id` - The identifier of the owner initiating the minting transition.
    /// * `transaction` - The transaction context for state changes.
    /// * `value` - A reference to a `TokenMintTransitionV0` struct containing transition data.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without full state.
    /// * `drive_operations` - A mutable reference to a vector for tracking low-level drive operations.
    /// * `get_data_contract` - A closure to fetch data contract information based on a contract identifier.
    /// * `platform_version` - A reference to the platform version for version-specific transition logic.
    ///
    /// # Returns
    ///
    /// * `Result<TokenMintTransitionActionV0, Error>` - Returns the resulting `TokenMintTransitionActionV0` if successful,
    ///   or an error if something goes wrong (e.g., missing data, invalid state).
    pub fn try_from_borrowed_token_mint_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenMintTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenMintTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
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

        let identity_balance_holder_id = issued_to_identity_id
            .or_else(|| {
                base_action
                    .data_contract_fetch_info_ref()
                    .contract
                    .tokens()
                    .get(&base.token_contract_position())
                    .and_then(|token_configuration| {
                        token_configuration.new_tokens_destination_identity()
                    })
            })
            .ok_or(ProtocolError::Token(
                TokenError::DestinationIdentityForMintingNotSetError.into(),
            ))?;

        Ok(TokenMintTransitionActionV0 {
            base: base_action,
            mint_amount: *amount,
            identity_balance_holder_id,
            public_note: public_note.clone(),
        })
    }
}
