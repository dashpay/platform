use crate::drive::address_funds::queries::CLEAR_ADDRESS_POOL;
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

/// Credits held by Platform addresses
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "address_balances",
        &[RootTree::AddressBalances as u8],
        "AddressBalances",
        "RootTree::AddressBalances",
    )
    .kind(ElementKind::SumTree)
    .since(11)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("addresses/platform-addresses.md")
    .describe(
        "Credits held by Platform addresses rather than \
         identities.",
    )
    .child(
        StructureNode::fixed(
            "clear",
            CLEAR_ADDRESS_POOL,
            "ClearAddressPool",
            "CLEAR_ADDRESS_POOL",
        )
        .ascii()
        .kind(ElementKind::ProvableCountSumTree)
        .source("packages/rs-drive/src/drive/address_funds/queries.rs")
        .describe(
            "Transparent address balances. The count is part \
             of the hashed state, so the number of funded \
             addresses can be proven.",
        )
        .child(
            StructureNode::dynamic(
                "address",
                "address",
                KeyMatcher::Any,
                KeyEncoding::Raw,
                "The serialized Platform address",
            )
            .kind(ElementKind::ItemWithSumItem)
            .value("the address nonce; the sum is its credits")
            .describe("One address: its nonce and its balance."),
        ),
    )
}
