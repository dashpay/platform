use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;

#[derive(Clone, Debug)]
pub enum GroupOperationType {
    /// Adds a group action
    AddGroupAction {
        /// The contract id
        contract_id: Identifier,
        /// The group contract position
        group_contract_position: GroupContractPosition,
        /// Initialize with the action info
        initialize_with_insert_action_info: Option<GroupAction>,
        /// The action id
        action_id: Identifier,
        /// The identity that is signing
        signer_identity_id: Identifier,
        /// The signer's power in the group
        signer_power: GroupMemberPower,
    },
}
