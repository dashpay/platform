use crate::drive::protocol_upgrade::{VALIDATOR_DESIRED_VERSIONS, VERSIONS_COUNTER};
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};

/// Protocol version voting
pub(crate) fn structure() -> StructureNode {
    StructureNode::fixed("versions", &[RootTree::Versions as u8], "Versions", "RootTree::Versions")
        .kind(ElementKind::Tree)
        .source("packages/rs-drive/src/drive/mod.rs")
        .book("versioning/platform-version.md")
        .describe("The protocol versions proposers ask for. Written once per block at most, which is why new root trees are hung below it.")
        .child(
            StructureNode::fixed("counter", &VERSIONS_COUNTER, "VersionsCounter", "VERSIONS_COUNTER")
                .kind(ElementKind::Tree)
                .source("packages/rs-drive/src/drive/protocol_upgrade/mod.rs")
                .describe("How many proposers ask for each protocol version.")
                .child(
                    StructureNode::dynamic("version", "protocol_version", KeyMatcher::Any, KeyEncoding::VarInt, "The protocol version")
                        .kind(ElementKind::Item)
                        .value("count, varint")
                        .describe("Proposers asking for this version."),
                ),
        )
        .child(
            StructureNode::fixed("desired", &VALIDATOR_DESIRED_VERSIONS, "ValidatorDesiredVersions", "VALIDATOR_DESIRED_VERSIONS")
                .kind(ElementKind::Tree)
                .source("packages/rs-drive/src/drive/protocol_upgrade/mod.rs")
                .describe("The protocol version each proposer asked for last.")
                .child(
                    StructureNode::identifier("validator", "pro_tx_hash", "The proposer's pro tx hash")
                        .kind(ElementKind::Item)
                        .value("protocol version, varint")
                        .describe("The version this proposer asks for."),
                ),
        )
}
