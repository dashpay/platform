use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::{GroupContractPosition, TokenContractPosition};
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::BTreeMap;

pub trait DataContractV1Getters: DataContractV0Getters {
    /// Gets a group at a certain position
    fn group(&self, position: GroupContractPosition) -> Result<&Group, ProtocolError>;
    /// Returns a reference to the groups map.
    fn groups(&self) -> &BTreeMap<GroupContractPosition, Group>;

    /// Returns a mutable reference to the groups map.
    fn groups_mut(&mut self) -> Option<&mut BTreeMap<GroupContractPosition, Group>>;
    /// Returns a reference to a group or an error.
    /// Returns an Error for V0 since it doesn't have groups.
    fn expected_group(&self, position: GroupContractPosition) -> Result<&Group, ProtocolError>;

    /// Returns a reference to the tokens map.
    fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration>;

    /// Returns a mutable reference to the tokens map.
    fn tokens_mut(&mut self) -> Option<&mut BTreeMap<TokenContractPosition, TokenConfiguration>>;

    /// Returns a mutable reference to a token configuration or an error.
    /// Returns an Error for V0 since it doesn't have tokens.
    fn expected_token_configuration(
        &self,
        position: TokenContractPosition,
    ) -> Result<&TokenConfiguration, ProtocolError>;

    /// Returns a mutable reference to a token configuration
    /// Returns `None` for V0 since it doesn't have tokens.
    fn token_configuration_mut(
        &mut self,
        position: TokenContractPosition,
    ) -> Option<&mut TokenConfiguration>;

    /// Returns the token id at a certain position
    fn token_id(&self, position: TokenContractPosition) -> Option<Identifier>;
}

pub trait DataContractV1Setters: DataContractV0Setters {
    /// Sets the groups map for the data contract.
    fn set_groups(&mut self, groups: BTreeMap<GroupContractPosition, Group>);

    /// Sets the tokens map for the data contract.
    fn set_tokens(&mut self, tokens: BTreeMap<TokenContractPosition, TokenConfiguration>);

    /// Adds or updates a single group in the groups map.
    fn add_group(&mut self, pos: GroupContractPosition, group: Group);

    /// Adds or updates a single token configuration in the tokens map.
    fn add_token(&mut self, pos: TokenContractPosition, token: TokenConfiguration);
}
