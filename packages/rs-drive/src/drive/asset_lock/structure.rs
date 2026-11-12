use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

/// Asset lock outpoints already used
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed(
        "spent_asset_locks",
        &[RootTree::SpentAssetLockTransactions as u8],
        "SpentAssetLockTransactions",
        "RootTree::SpentAssetLockTransactions",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .describe(
        "Every asset lock outpoint that funded an \
         identity or an address, so none is used twice.",
    )
    .child(
        StructureNode::dynamic(
            "outpoint",
            "outpoint",
            KeyMatcher::Len(36),
            KeyEncoding::Raw,
            "The transaction id followed by the output index",
        )
        .kind(ElementKind::Item)
        .source("packages/rs-drive/src/drive/asset_lock/mod.rs")
        .value(
            "serialized StoredAssetLockInfo: fully used, or \
             the credits remaining",
        )
        .describe("One asset lock and how much of it is left."),
    )
}
