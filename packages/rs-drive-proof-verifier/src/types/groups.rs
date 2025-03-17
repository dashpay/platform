use crate::types::RetrievedObjects;
use dpp::data_contract::group::{Group, GroupMemberPower};
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;

/// Groups
pub type Groups = RetrievedObjects<GroupContractPosition, Group>;

/// Group actions
pub type GroupActions = RetrievedObjects<Identifier, GroupAction>;

/// Group action signers
pub type GroupActionSigners = RetrievedObjects<Identifier, GroupMemberPower>;
