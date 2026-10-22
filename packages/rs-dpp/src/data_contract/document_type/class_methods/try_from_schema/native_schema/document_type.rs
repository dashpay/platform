//! The document schema's own shape: its keys, `properties` and `$defs`.

use super::*;

/// The system properties a document schema's `properties` may name, each with any schema.
const SYSTEM_PROPERTIES: [&str; 12] = [
    "$id",
    "$ownerId",
    "$revision",
    "$createdAt",
    "$updatedAt",
    "$transferredAt",
    "$createdAtBlockHeight",
    "$updatedAtBlockHeight",
    "$transferredAtBlockHeight",
    "$createdAtCoreBlockHeight",
    "$updatedAtCoreBlockHeight",
    "$transferredAtCoreBlockHeight",
];

/// The `$schema` the base schema gives every document schema.
pub(super) const DOCUMENT_SCHEMA_URI: &str =
    "https://github.com/dashpay/platform/blob/master/packages/rs-dpp/schema/meta_schemas/document/v1/document-meta.json";

/// The keys of a document schema.
const ROOT_KEYS: [&str; 43] = [
    "type",
    "$schema",
    "$defs",
    "indices",
    "signatureSecurityLevelRequirement",
    "documentsKeepHistory",
    "keepsTransferHistory",
    "keepsPurchaseHistory",
    "keepsPricingHistory",
    "documentsMutable",
    "canBeDeleted",
    "transferable",
    "tradeMode",
    "creationRestrictionMode",
    "requiresIdentityEncryptionBoundedKey",
    "requiresIdentityDecryptionBoundedKey",
    "documentsCountable",
    "rangeCountable",
    "documentsSummable",
    "rangeSummable",
    "documentsAverageable",
    "rangeAverageable",
    "moderatorAbilities",
    "ttl",
    "indexOnly",
    "entryPayload",
    "actionFees",
    "tokenCost",
    "properties",
    "transient",
    "immutable",
    "retractedWhen",
    "ownerRefersTo",
    "creatorRefersTo",
    "propertyConstraints",
    "deleteConstraints",
    "additionalProperties",
    "required",
    "$comment",
    "description",
    "minProperties",
    "maxProperties",
    "dependentRequired",
];

impl Checker<'_> {
    pub(super) fn root(&self, root: &JsonValue) -> Checked {
        let map = self.object(root)?;
        self.required(
            map,
            &["$schema", "type", "properties", "additionalProperties"],
        )?;
        self.closed(map, &ROOT_KEYS)?;
        for (key, value) in map {
            self.at(key, |c| c.root_keyword(key, value))?;
        }
        Ok(())
    }

    fn root_keyword(&self, key: &str, value: &JsonValue) -> Checked {
        match key {
            "type" => {
                self.string(value)?;
                self.constant(value, &json!("object"))
            }
            "$schema" => {
                self.string(value)?;
                self.constant(value, &json!(DOCUMENT_SCHEMA_URI))
            }
            "$defs" => self.property_map(value),
            "properties" => self.root_properties(value),
            "additionalProperties" => {
                self.boolean(value)?;
                self.constant(value, &json!(false))
            }
            "required" => self.string_array(value),
            "$comment" | "description" => self.string(value).map(drop),
            "minProperties" | "maxProperties" => self.non_negative_integer(value),
            "dependentRequired" => {
                let map = self.object(value)?;
                for (name, names) in map {
                    self.at(name, |c| c.string_array(names))?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The document schema's own `properties`: system properties with any schema, the
    /// others named by words, 1 to 100 of them.
    fn root_properties(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        for (name, schema) in map {
            self.at(name, |c| {
                if SYSTEM_PROPERTIES.contains(&name.as_str()) {
                    return Ok(());
                }
                if !is_word(name) {
                    return Err(c.fail_with(
                        "propertyNames",
                        format!("\"{name}\" is not a property name"),
                        json!({ "pattern": "^[a-zA-Z0-9_]{1,64}$" }),
                        name,
                    ));
                }
                c.property_schema(schema)
            })?;
        }
        self.property_count(map, Some(1), Some(100))
    }
}
