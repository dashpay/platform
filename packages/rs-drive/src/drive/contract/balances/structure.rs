use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

/// Contract credits: one sum subtree per contract holding its credit buckets
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "contract_credits",
        &[RootTree::ContractCredits as u8],
        "ContractCredits",
        "RootTree::ContractCredits",
    )
    .kind(ElementKind::SumTree)
    .since(17)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("drive/contract-credit-buckets.md")
    .describe(
        "The credits every contract holds, in buckets. An \
         ordinary sum tree, so its aggregate is the total \
         of live contract credits and is a term of the \
         credit conservation equation. Key 100 is \
         provisional and hangs below Misc.",
    )
    .child(
        StructureNode::identifier("contract", "contract_id", "The data contract id")
            .kind(ElementKind::SumTree)
            .source("packages/rs-drive/src/drive/contract/balances/mod.rs")
            .describe(
                "One contract's credit buckets. A wiped contract's \
                 tree is wrapped in a not-summed element, so its \
                 retained credits leave the root aggregate.",
            )
            .child(
                StructureNode::dynamic(
                    "bucket",
                    "bucket_position",
                    KeyMatcher::Len(2),
                    KeyEncoding::U16Be,
                    "The bucket position",
                )
                .kind(ElementKind::SumItem)
                .value("credits")
                .describe("The credits in one bucket."),
            ),
    )
}
