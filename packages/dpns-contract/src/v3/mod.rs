use crate::Error;
use serde_json::Value;

/// The names v3 adds; every other name is v1's ([`crate::v1::document_types`]).
pub mod document_types {
    pub mod preorder {
        pub mod properties {
            /// The double SHA-256 of the owner's id, the normalized label, "." and the
            /// normalized parent domain name: a preorder holding it counts (`validWhen`).
            pub const OWNER_NAME_HASH: &str = "ownerNameHash";
            /// The same hash without the owner's id, shared by every preorder of a name,
            /// which the domain's `creatorRefersTo` finds.
            pub const NAME_HASH: &str = "nameHash";
        }
    }
}

/// The document type names are unchanged from v1 (see [`crate::v1::document_types`]),
/// and so are every `domain` property's position and the transient list, so domains
/// written under v1 or v2 decode unchanged: `preorderSalt` stays declared, transient
/// and no longer required, but nothing reads it. v3 replaces most of the `domain`
/// create data trigger with schema keywords: `normalizedLabel` and
/// `normalizedParentDomainName` are generated from their counterparts
/// (`generatedFrom`), `creatorRefersTo` finds the creator's own preorder of the name
/// from an earlier block and deletes it (a `findBy` over the creator and the double
/// SHA-256 of the normalized label, "." and the normalized parent, with
/// `minimumAgeBlocks` and `consume`, which needs the preorder to record
/// `$createdAtBlockHeight`), and refuses it while another identity holds a valid
/// preorder of the name from an earlier block (`chooseFirst` by creation height, a
/// preorder counting when it holds the same hash with its owner's id first), and the
/// `recordsIdentityIsOwner` rule holds a new domain's identity record to its owner. A
/// preorder holds both hashes (`nameHash`, unique per owner and indexed with the
/// creation height, and `ownerNameHash`) and lives an hour (`ttl`). The protocol
/// upgrade deletes the v2 preorders and rebuilds the type. Domains can no longer be
/// deleted (`canBeDeleted: false`).
/// The parent domain checks stay in the create trigger.
pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v3/dpns-contract-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
