use crate::drive::RootTree;
use crate::structure::{ElementKind, StructureNode};

/// Identity credit balances
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "balances",
        &[RootTree::Balances as u8],
        "Balances",
        "RootTree::Balances",
    )
    .kind(ElementKind::SumTree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("data-model/identities.md")
    .describe(
        "The credit balance of every identity. Kept out \
         of the identity tree so balance proofs stay \
         small.",
    )
    .child(
        StructureNode::identifier("identity", "identity_id", "The identity id")
            .kind(ElementKind::SumItem)
            .source("packages/rs-drive/src/drive/balances/mod.rs")
            .value("credits")
            .describe("One identity's balance."),
    )
}
