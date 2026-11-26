use crate::Error;
use serde_json::Value;

/// The names v3 adds; every other name is v1's ([`crate::v1::document_types`]).
pub mod document_types {
    pub mod tombstone {
        /// A name its owner gave up for good.
        pub const NAME: &str = "tombstone";

        pub mod properties {
            /// The normalized label of the tombstoned name, which finds the creator's own
            /// `domain` of that name: the create deletes it.
            pub const NORMALIZED_LABEL: &str = "normalizedLabel";
            /// The normalized parent domain name of the tombstoned name.
            pub const NORMALIZED_PARENT_DOMAIN_NAME: &str = "normalizedParentDomainName";
        }
    }

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
/// preorder counting when it holds the same hash with its owner's id first), unless the
/// create opens or joins the name's contest, which the masternode vote decides (held to the
/// order, an identity holding preorders of a name easy to guess older than a rival's could
/// delay the rival's reveal; each contender still reveals its own preorder, a block old at
/// least), and the
/// `recordsIdentityIsOwner` rule holds a new domain's identity record to its owner. A
/// preorder holds both hashes (`nameHash`, unique per owner and indexed with the
/// creation height, and `ownerNameHash`) and lives an hour (`ttl`). The protocol
/// upgrade deletes the v2 preorders and rebuilds the type.
/// The parent domain checks stay in the create trigger.
///
/// A name's owner can give the name up, two ways. A delete of the `domain` frees the
/// name: anybody may register it again (`canBeDeleted: true`; the delete trigger refuses
/// a contested name, which a masternode vote awarded and which is never contested
/// again). A `tombstone` create deletes the creator's own `domain` of that name with it
/// (its `normalizedLabel` carries a `refersTo` with `consume`, found through the domain's
/// unique index and held to the creator's own by the `where` entry) and keeps the name
/// from being registered while the tombstone exists: the `domain` rule
/// `nameIsNotTombstoned` counts the tombstones of the name through the tombstone's
/// countable index and refuses a create, one opening a contest included, unless there is
/// none. Any name can be tombstoned, a contested one too. A tombstone can not be
/// replaced, deleted or transferred; the type's description says a tombstoned name may
/// later be brought back and redistributed through a direct or indirect will of the DAO.
pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v3/dpns-contract-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
