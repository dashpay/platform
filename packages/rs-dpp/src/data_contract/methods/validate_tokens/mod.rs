use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::TokenContractPosition;
use crate::prelude::DataContract;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

impl DataContract {
    /// Validates the provided tokens to ensure they meet the requirements for data contracts.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the data contract these tokens belong to.
    /// - `tokens`: A reference to a `BTreeMap` of token contract positions (`TokenContractPosition`)
    ///   mapped to their corresponding `TokenConfiguration` objects.
    /// - `allow_offset_start`: If true, allows the tokens to start at a position other than 0.
    ///   This is useful for update transitions where new tokens continue from existing ones.
    /// - `network`: The network type for validation rules.
    /// - `platform_version`: A reference to the [`PlatformVersion`](crate::version::PlatformVersion)
    ///   object specifying the version of the platform and determining which validation method to use.
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)` if all the tokens pass validation:
    ///   - Token contract positions must be contiguous, i.e., no gaps between positions.
    ///   - Each token must meet its individual validation criteria.
    /// - `Err(ProtocolError)` if:
    ///   - An unknown or unsupported platform version is provided.
    ///   - Validation of any token fails.
    ///
    /// # Behavior
    /// - Delegates the actual validation logic to the appropriate versioned implementation
    ///   (`validate_tokens_v0`) based on the provided platform version.
    /// - If an unknown platform version is encountered, a `ProtocolError::UnknownVersionMismatch`
    ///   is returned.
    ///
    /// # Errors
    /// - Returns a `ProtocolError::UnknownVersionMismatch` if the platform version is not recognized.
    /// - Returns validation errors for:
    ///   - Non-contiguous token contract positions (`NonContiguousContractTokenPositionsError`).
    ///   - Invalid token base supply (`InvalidTokenBaseSupplyError`).
    ///   - Missing destination identity when required (`NewTokensDestinationIdentityOptionRequiredError`).
    pub fn validate_tokens(
        contract_id: Identifier,
        tokens: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        allow_offset_start: bool,
        network: dashcore::Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_tokens
        {
            0 => Self::validate_tokens_v0(
                contract_id,
                tokens,
                allow_offset_start,
                network,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_tokens".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
