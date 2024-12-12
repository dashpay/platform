use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::{Group, GroupName};
use crate::data_contract::TokenName;
use std::collections::BTreeMap;

pub trait DataContractV1Getters: DataContractV0Getters {
    /// Returns a reference to the groups map.
    fn groups(&self) -> &BTreeMap<GroupName, Group>;

    /// Returns a mutable reference to the groups map.
    fn groups_mut(&mut self) -> &mut BTreeMap<GroupName, Group>;

    /// Returns a reference to the tokens map.
    fn tokens(&self) -> &BTreeMap<TokenName, TokenConfiguration>;

    /// Returns a mutable reference to the tokens map.
    fn tokens_mut(&mut self) -> &mut BTreeMap<TokenName, TokenConfiguration>;
}

pub trait DataContractV1Setters: DataContractV0Setters {
    /// Sets the groups map for the data contract.
    fn set_groups(&mut self, groups: BTreeMap<GroupName, Group>);

    /// Sets the tokens map for the data contract.
    fn set_tokens(&mut self, tokens: BTreeMap<TokenName, TokenConfiguration>);

    /// Adds or updates a single group in the groups map.
    fn add_group(&mut self, name: GroupName, group: Group);

    /// Adds or updates a single token configuration in the tokens map.
    fn add_token(&mut self, name: TokenName, token: TokenConfiguration);
}
