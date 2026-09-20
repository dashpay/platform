use crate::drive::group::paths::{
    ACTION_INFO_KEY, ACTION_SIGNERS_KEY, GROUP_ACTIVE_ACTIONS_KEY, GROUP_CLOSED_ACTIONS_KEY,
    GROUP_INFO_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/group/paths.rs";

fn action(description: &str) -> StructureNode {
    StructureNode::identifier("action", "action_id", "The action id")
        .kind(ElementKind::Tree)
        .describe(description)
        .children(vec![
            StructureNode::fixed("info", ACTION_INFO_KEY, "ActionInfo", "ACTION_INFO_KEY")
                .ascii()
                .kind(ElementKind::Item)
                .value("serialized GroupAction")
                .describe("What the action does, written by its first signer."),
            StructureNode::fixed(
                "signers",
                ACTION_SIGNERS_KEY,
                "ActionSigners",
                "ACTION_SIGNERS_KEY",
            )
            .ascii()
            .kind(ElementKind::SumTree)
            .describe("Who signed; the sum is the power gathered so far.")
            .child(
                StructureNode::identifier("signer", "identity_id", "The signer's identity id")
                    .kind(ElementKind::SumItem)
                    .value("the signer's power in the group")
                    .describe("One signature."),
            ),
        ])
}

/// Actions waiting for the members of a group to sign
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "group_actions",
        &[RootTree::GroupActions as u8],
        "GroupActions",
        "RootTree::GroupActions",
    )
    .kind(ElementKind::Tree)
    .since(9)
    .source("packages/rs-drive/src/drive/mod.rs")
    .describe("Multi party actions on tokens: proposed by one member of a contract's group, carried out once enough power has signed.")
    .child(
        StructureNode::identifier("contract", "contract_id", "The data contract id")
            .kind(ElementKind::Tree)
            .source(SOURCE)
            .describe("The groups of one contract. Created with the contract when it defines groups.")
            .child(
                StructureNode::dynamic("group", "group_contract_position", KeyMatcher::Len(2), KeyEncoding::U16Be, "The position of the group in the contract")
                    .kind(ElementKind::Tree)
                    .describe("One group.")
                    .children(vec![
                        StructureNode::fixed("info", GROUP_INFO_KEY, "GroupInfo", "GROUP_INFO_KEY")
.ascii()
                            .kind(ElementKind::Item)
                            .value("serialized Group: members, their power, the power required")
                            .describe("The group as the contract defines it."),
                        StructureNode::fixed("active", GROUP_ACTIVE_ACTIONS_KEY, "ActiveActions", "GROUP_ACTIVE_ACTIONS_KEY")
.ascii()
                            .kind(ElementKind::Tree)
                            .describe("Actions still gathering signatures. The root of the group layer.")
                            .child(action("One action in progress. Moved to the closed actions when it gathers the required power.")),
                        StructureNode::fixed("closed", GROUP_CLOSED_ACTIONS_KEY, "ClosedActions", "GROUP_CLOSED_ACTIONS_KEY")
.ascii()
                            .kind(ElementKind::Tree)
                            .describe("Actions that were carried out, kept so an action id cannot be reused.")
                            .child(action("One completed action.")),
                    ]),
            ),
    )
}
