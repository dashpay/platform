use crate::Error;
use serde_json::Value;

/// The document type names and property names are unchanged from v1 (see
/// [`crate::v1::document_types`]), and so are every `domain` property's position and
/// the transient list, so domains written under v1 or v2 decode unchanged. v3 replaces
/// most of the `domain` create data trigger with schema keywords: `normalizedLabel`
/// and `normalizedParentDomainName` are generated from their counterparts
/// (`generatedFrom`), `preorderSalt` reveals the writer's own preorder from an
/// earlier block and deletes it (`refersTo` with a `findBy` over the writer and the
/// hash of the writer's id, the salt, the normalized label, "." and the parent,
/// `minimumAgeBlocks` and `consume`, which needs the preorder to record
/// `$createdAtBlockHeight`), and the `recordsIdentityIsOwner` rule holds a new
/// domain's identity record to its owner. Preorders are unique per owner
/// (`ownerAndSaltedHash`), so the protocol upgrade deletes the v2 preorders and
/// rebuilds the type. Domains can no longer be deleted (`canBeDeleted: false`). The
/// parent domain checks stay in the create trigger.
pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v3/dpns-contract-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
