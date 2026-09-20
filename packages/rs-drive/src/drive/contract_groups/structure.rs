use crate::drive::contract_groups::paths::{
    CONTRACT_GROUPS_GROUPS_KEY, CONTRACT_GROUPS_MEMBERS_KEY, CONTRACT_GROUP_CONTRACTS_KEY,
    CONTRACT_GROUP_DOCUMENT_TYPES_KEY, CONTRACT_GROUP_INFO_KEY, CONTRACT_GROUP_TOKENS_KEY,
    CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY, CONTRACT_MEMBERSHIPS_GROUPS_KEY,
    CONTRACT_MEMBERSHIPS_TOKENS_KEY,
};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

const SOURCE: &str = "packages/rs-drive/src/drive/contract_groups/paths.rs";

fn group_reference(target: &str, description: &str) -> StructureNode {
    StructureNode::identifier("group", "contract_group_id", "The contract group id")
        .kind(ElementKind::Reference)
        .reference(target)
        .describe(description)
}

/// Contract groups: named sets of contracts, document types and tokens
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "contract_groups",
        &[RootTree::ContractGroups as u8],
        "ContractGroups",
        "RootTree::ContractGroups",
    )
    .kind(ElementKind::Tree)
    .since(14)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("data-model/contract-groups.md")
    .describe("Groups of contracts, document types and tokens an identity key can be bound to. Key 124 hangs below Versions so no fee bearing transition pays for the extra root node.")
    .children(vec![
        StructureNode::fixed("groups", CONTRACT_GROUPS_GROUPS_KEY, "Groups", "CONTRACT_GROUPS_GROUPS_KEY")
            .kind(ElementKind::Tree)
            .source(SOURCE)
            .describe("The forward store: everything about a group under its id.")
            .child(
                StructureNode::identifier("group", "contract_group_id", "The contract group id")
                    .kind(ElementKind::Tree)
                    .describe("One contract group.")
                    .children(vec![
                        StructureNode::fixed("info", CONTRACT_GROUP_INFO_KEY, "Info", "CONTRACT_GROUP_INFO_KEY")
                            .kind(ElementKind::Item)
                            .value("bincode ContractGroupInfo: owner, optional name and description")
                            .describe("Who owns the group and what it is called."),
                        StructureNode::fixed("contracts", CONTRACT_GROUP_CONTRACTS_KEY, "Contracts", "CONTRACT_GROUP_CONTRACTS_KEY")
                            .kind(ElementKind::Tree)
                            .describe("Whole contracts in the group.")
                            .child(
                                StructureNode::identifier("contract", "contract_id", "The data contract id")
                                    .kind(ElementKind::Item)
                                    .value("empty; the key carries the information")
                                    .describe("One member contract."),
                            ),
                        StructureNode::fixed("document_types", CONTRACT_GROUP_DOCUMENT_TYPES_KEY, "DocumentTypes", "CONTRACT_GROUP_DOCUMENT_TYPES_KEY")
                            .kind(ElementKind::Tree)
                            .describe("Single document types in the group, on one flat level so paging is a plain range.")
                            .child(
                                StructureNode::dynamic("member", "contract_id_and_document_type", KeyMatcher::Any, KeyEncoding::Composite, "The contract id followed by the document type name")
                                    .kind(ElementKind::Item)
                                    .value("empty; the key carries the information")
                                    .describe("One member document type."),
                            ),
                        StructureNode::fixed("tokens", CONTRACT_GROUP_TOKENS_KEY, "Tokens", "CONTRACT_GROUP_TOKENS_KEY")
                            .kind(ElementKind::Tree)
                            .describe("Single tokens in the group.")
                            .child(
                                StructureNode::dynamic("member", "contract_id_and_token_position", KeyMatcher::Len(34), KeyEncoding::Composite, "The contract id followed by the token position, u16 big endian")
                                    .kind(ElementKind::Item)
                                    .value("empty; the key carries the information")
                                    .describe("One member token."),
                            ),
                    ]),
            ),
        StructureNode::fixed("members", CONTRACT_GROUPS_MEMBERS_KEY, "Members", "CONTRACT_GROUPS_MEMBERS_KEY")
            .kind(ElementKind::Tree)
            .source(SOURCE)
            .describe("The backwards index: every group a contract belongs to, under the contract id.")
            .child(
                StructureNode::identifier("contract", "contract_id", "The data contract id")
                    .kind(ElementKind::Tree)
                    .describe("The memberships of one contract.")
                    .children(vec![
                        StructureNode::fixed("groups", CONTRACT_MEMBERSHIPS_GROUPS_KEY, "Groups", "CONTRACT_MEMBERSHIPS_GROUPS_KEY")
                            .kind(ElementKind::Tree)
                            .lazy()
                            .describe("Groups the whole contract belongs to.")
                            .child(group_reference("contract_groups.groups.group.contracts.contract", "A group holding the contract.")),
                        StructureNode::fixed("document_types", CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY, "DocumentTypes", "CONTRACT_MEMBERSHIPS_DOCUMENT_TYPES_KEY")
                            .kind(ElementKind::Tree)
                            .lazy()
                            .describe("Groups single document types of the contract belong to.")
                            .child(
                                StructureNode::dynamic("document_type", "document_type_name", KeyMatcher::Any, KeyEncoding::Utf8, "The document type name")
                                    .kind(ElementKind::Tree)
                                    .describe("The groups holding this document type.")
                                    .child(group_reference("contract_groups.groups.group.document_types.member", "A group holding the document type.")),
                            ),
                        StructureNode::fixed("tokens", CONTRACT_MEMBERSHIPS_TOKENS_KEY, "Tokens", "CONTRACT_MEMBERSHIPS_TOKENS_KEY")
                            .kind(ElementKind::Tree)
                            .lazy()
                            .describe("Groups single tokens of the contract belong to.")
                            .child(
                                StructureNode::dynamic("token", "token_position", KeyMatcher::Len(2), KeyEncoding::U16Be, "The token position in the contract")
                                    .kind(ElementKind::Tree)
                                    .describe("The groups holding this token.")
                                    .child(group_reference("contract_groups.groups.group.tokens.member", "A group holding the token.")),
                            ),
                    ]),
            ),
    ])
}
