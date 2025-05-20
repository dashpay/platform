use dpp::identifier::Identifier;
use dpp::data_contract::DataContract;
use grovedb::PathQuery;
use platform_version::version::PlatformVersion;

/// Token transition module
pub mod token_transition;
/// A trait for converting a state transition into a [`PathQuery`] for data retrieval.
///
/// This is used to express the expected changes or queries required to validate or process
/// a state transition, particularly in the context of tokens or document transitions where
/// path-based access is necessary (e.g., GroveDB queries).
///
/// # Associated Types
/// * `Error` – the error type returned when the conversion fails.
///
/// # Required Methods
///
/// ## `try_transition_into_path_query_with_contract`
/// Attempts to convert the transition into a path query based on the provided data contract,
/// owner identity, and platform version. This is often used for querying relevant token or
/// document paths associated with the transition.
///
/// ### Parameters
/// * `data_contract`: A reference to the [`DataContract`] defining the schema.
/// * `owner_id`: The identifier of the identity owning the transition.
/// * `platform_version`: The current [`PlatformVersion`] used for versioned logic.
///
/// ### Returns
/// * `Ok(PathQuery)` – a [`PathQuery`] representing the database paths needed for verification or processing.
/// * `Err(Self::Error)` – if the transition could not be converted to a path query.
///
/// # Usage
/// This trait is typically implemented for specific transition types (e.g., token minting)
/// that require prefetching or validation of state from GroveDB.
///
pub trait TryTransitionIntoPathQuery {
    /// The error type returned on conversion failure.
    type Error;

    /// Attempts to convert a transition into a corresponding [`PathQuery`] based on the contract and owner.
    fn try_transition_into_path_query_with_contract(
        &self,
        data_contract: &DataContract,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Self::Error>;
}
