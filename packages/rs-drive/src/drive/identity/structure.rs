use crate::drive::identity::contract_info::ContractInfoStructure;
use crate::drive::identity::IdentityRootStructure;
use crate::drive::RootTree;
use crate::structure::{ElementKind, KeyEncoding, KeyMatcher, StructureNode};
use dpp::identity::{Purpose, SecurityLevel};

const SOURCE: &str = "packages/rs-drive/src/drive/identity/mod.rs";
const KEYS: &str = "identities.identity.keys.key";

fn key_reference(description: &str) -> StructureNode {
    StructureNode::dynamic(
        "key",
        "key_id",
        KeyMatcher::Any,
        KeyEncoding::VarInt,
        "The key id",
    )
    .kind(ElementKind::Reference)
    .reference(KEYS)
    .describe(description)
}

/// Identities, without their balances
pub(crate) fn structure() -> StructureNode {
    let security_level = |segment: &str, level: SecurityLevel, label: &str| {
        StructureNode::fixed(
            segment,
            &[level as u8],
            label,
            &format!("SecurityLevel::{label}"),
        )
        .kind(ElementKind::Tree)
        .source("packages/rs-dpp/src/identity/identity_public_key/security_level.rs")
        .describe("Authentication keys of this security level.")
        .child(key_reference("One authentication key of this level."))
    };

    StructureNode::fixed(
        "identities",
        &[RootTree::Identities as u8],
        "Identities",
        "RootTree::Identities",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("data-model/identities.md")
    .describe(
        "Every identity: its keys, revision and nonces. \
         Balances live in their own root tree.",
    )
    .child(
        StructureNode::identifier("identity", "identity_id", "The identity id")
            .kind(ElementKind::Tree)
            .source(SOURCE)
            .describe("One identity.")
            .children(vec![
                StructureNode::fixed(
                    "contract_info",
                    &[IdentityRootStructure::IdentityContractInfo as u8],
                    "IdentityContractInfo",
                    "IdentityRootStructure::IdentityContractInfo",
                )
                .kind(ElementKind::Tree)
                .lazy()
                .describe(
                    "What the identity keeps per data contract: a \
                     nonce and the keys bound to the contract.",
                )
                .child(
                    StructureNode::dynamic(
                        "bound",
                        "contract_or_document_type",
                        KeyMatcher::Any,
                        KeyEncoding::Composite,
                        "A contract id, or a contract id followed by a \
                         document type name for keys bound to one \
                         document type",
                    )
                    .kind(ElementKind::Tree)
                    .source("packages/rs-drive/src/drive/identity/contract_info/mod.rs")
                    .describe(
                        "The identity's state for one contract or one of \
                         its document types.",
                    )
                    .children(vec![
                        StructureNode::fixed(
                            "nonce",
                            &[ContractInfoStructure::IdentityContractNonceKey as u8],
                            "IdentityContractNonce",
                            "ContractInfoStructure::IdentityContractNonceKey",
                        )
                        .kind(ElementKind::Item)
                        .lazy()
                        .value(
                            "u64 big endian: the nonce and a bitmap of \
                             recently missed nonces",
                        )
                        .describe(
                            "Stops replays of the identity's transitions on \
                             this contract.",
                        ),
                        StructureNode::fixed(
                            "keys",
                            &[ContractInfoStructure::ContractInfoKeysKey as u8],
                            "ContractInfoKeys",
                            "ContractInfoStructure::ContractInfoKeysKey",
                        )
                        .kind(ElementKind::Tree)
                        .lazy()
                        .describe(
                            "The identity's keys bound to this contract, by \
                             purpose.",
                        )
                        .children(vec![StructureNode::dynamic(
                            "purpose",
                            "purpose",
                            KeyMatcher::Len(1),
                            KeyEncoding::U8,
                            "The key purpose: encryption or decryption",
                        )
                        .kind(ElementKind::Tree)
                        .describe("Bound keys of one purpose.")
                        .children(vec![
                            StructureNode::fixed("current", &[], "CurrentKey", "")
                                .kind(ElementKind::Reference)
                                .lazy()
                                .reference(KEYS)
                                .describe(
                                    "The current key of the purpose, stored at the \
                                         empty key: the single key when the contract asks \
                                         for a unique bound key, otherwise a sibling \
                                         reference to the latest key.",
                                ),
                            key_reference("One bound key, when the contract allows several."),
                        ])]),
                    ]),
                ),
                StructureNode::fixed(
                    "nonce",
                    &[IdentityRootStructure::IdentityTreeNonce as u8],
                    "IdentityTreeNonce",
                    "IdentityRootStructure::IdentityTreeNonce",
                )
                .kind(ElementKind::Item)
                .value(
                    "u64 big endian: the nonce and a bitmap of \
                     recently missed nonces",
                )
                .describe(
                    "Stops replays of identity level transitions such \
                     as credit transfers.",
                ),
                StructureNode::fixed(
                    "negative_credit",
                    &[IdentityRootStructure::IdentityTreeNegativeCredit as u8],
                    "IdentityTreeNegativeCredit",
                    "IdentityRootStructure::IdentityTreeNegativeCredit",
                )
                .kind(ElementKind::Item)
                .value("credits, u64 big endian")
                .describe(
                    "Debt the identity owes, taken out of its next \
                     top up.",
                ),
                StructureNode::fixed(
                    "keys",
                    &[IdentityRootStructure::IdentityTreeKeys as u8],
                    "IdentityTreeKeys",
                    "IdentityRootStructure::IdentityTreeKeys",
                )
                .kind(ElementKind::Tree)
                .book("sdk/identity-keys.md")
                .describe("The identity's public keys.")
                .child(
                    StructureNode::dynamic(
                        "key",
                        "key_id",
                        KeyMatcher::Any,
                        KeyEncoding::VarInt,
                        "The key id",
                    )
                    .kind(ElementKind::Item)
                    .value("serialized IdentityPublicKey")
                    .describe("One public key."),
                ),
                StructureNode::fixed(
                    "key_references",
                    &[IdentityRootStructure::IdentityTreeKeyReferences as u8],
                    "IdentityTreeKeyReferences",
                    "IdentityRootStructure::IdentityTreeKeyReferences",
                )
                .kind(ElementKind::Tree)
                .book("sdk/identity-keys.md")
                .describe(
                    "Keys by purpose and security level, so a query \
                     can ask for the authentication keys of a level.",
                )
                .children(vec![
                    StructureNode::fixed(
                        "authentication",
                        &[Purpose::AUTHENTICATION as u8],
                        "AUTHENTICATION",
                        "Purpose::AUTHENTICATION",
                    )
                    .kind(ElementKind::Tree)
                    .source("packages/rs-dpp/src/identity/identity_public_key/purpose.rs")
                    .describe("Authentication keys, by security level.")
                    .children(vec![
                        security_level("master", SecurityLevel::MASTER, "MASTER"),
                        security_level("critical", SecurityLevel::CRITICAL, "CRITICAL"),
                        security_level("high", SecurityLevel::HIGH, "HIGH"),
                        security_level("medium", SecurityLevel::MEDIUM, "MEDIUM"),
                    ]),
                    StructureNode::fixed(
                        "transfer",
                        &[Purpose::TRANSFER as u8],
                        "TRANSFER",
                        "Purpose::TRANSFER",
                    )
                    .kind(ElementKind::Tree)
                    .source("packages/rs-dpp/src/identity/identity_public_key/purpose.rs")
                    .describe("Keys that can move credits out of the identity.")
                    .child(key_reference("One transfer key.")),
                    StructureNode::fixed(
                        "voting",
                        &[Purpose::VOTING as u8],
                        "VOTING",
                        "Purpose::VOTING",
                    )
                    .kind(ElementKind::Tree)
                    .source("packages/rs-dpp/src/identity/identity_public_key/purpose.rs")
                    .describe("Keys a masternode identity votes with.")
                    .child(key_reference("One voting key.")),
                ]),
                StructureNode::fixed(
                    "revision",
                    &[IdentityRootStructure::IdentityTreeRevision as u8],
                    "IdentityTreeRevision",
                    "IdentityRootStructure::IdentityTreeRevision",
                )
                .kind(ElementKind::Item)
                .value("u64 big endian")
                .describe("Incremented by every identity update."),
                StructureNode::fixed(
                    "key_budgets",
                    &[IdentityRootStructure::IdentityTreeKeyBudgets as u8],
                    "IdentityTreeKeyBudgets",
                    "IdentityRootStructure::IdentityTreeKeyBudgets",
                )
                .kind(ElementKind::Tree)
                .since(14)
                .lazy()
                .book("data-model/key-limits.md")
                .describe(
                    "What is left of each budgeted key's limit. \
                     Created with the identity's first budgeted key.",
                )
                .child(
                    StructureNode::dynamic(
                        "key",
                        "key_id",
                        KeyMatcher::Any,
                        KeyEncoding::VarInt,
                        "The key id",
                    )
                    .kind(ElementKind::Item)
                    .value("remaining budget, u64 big endian")
                    .describe("The remaining budget of one key."),
                ),
            ]),
    )
}

/// Public key hashes that belong to exactly one identity
pub(crate) fn unique_key_hashes_structure() -> StructureNode {
    StructureNode::fixed(
        "unique_key_hashes",
        &[RootTree::UniquePublicKeyHashesToIdentities as u8],
        "UniquePublicKeyHashesToIdentities",
        "RootTree::UniquePublicKeyHashesToIdentities",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("data-model/identities.md")
    .describe("Finds the identity owning a unique public key.")
    .child(
        StructureNode::dynamic(
            "key_hash",
            "public_key_hash",
            KeyMatcher::Len(20),
            KeyEncoding::Hash20,
            "The hash160 of the public key",
        )
        .kind(ElementKind::Item)
        .value("the identity id, 32 bytes")
        .describe("The identity this key belongs to."),
    )
}

/// Public key hashes several identities can share
pub(crate) fn non_unique_key_hashes_structure() -> StructureNode {
    StructureNode::fixed(
        "non_unique_key_hashes",
        &[RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8],
        "NonUniquePublicKeyKeyHashesToIdentities",
        "RootTree::NonUniquePublicKeyKeyHashesToIdentities",
    )
    .kind(ElementKind::Tree)
    .source("packages/rs-drive/src/drive/mod.rs")
    .book("data-model/identities.md")
    .describe(
        "Finds the identities sharing a public key, as \
         masternode owner and voting keys can be.",
    )
    .child(
        StructureNode::dynamic(
            "key_hash",
            "public_key_hash",
            KeyMatcher::Len(20),
            KeyEncoding::Hash20,
            "The hash160 of the public key",
        )
        .kind(ElementKind::Tree)
        .describe("The identities using this key.")
        .child(
            StructureNode::identifier("identity", "identity_id", "The identity id")
                .kind(ElementKind::Item)
                .value("empty")
                .describe("One identity using the key."),
        ),
    )
}
