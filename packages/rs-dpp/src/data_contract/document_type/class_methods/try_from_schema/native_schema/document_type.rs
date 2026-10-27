//! The document schema's own keywords: its shape, its flags and the document-type keywords
//! (`moderatorAbilities`, `actionFees`, `tokenCost`, `immutable` and the rest).

use super::*;

/// The system properties a document schema's `properties` may name, each with any schema
/// whose patterns compile ([`Checker::compiled_patterns`]).
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

/// The document actions `actionFees` and `tokenCost` price.
const DOCUMENT_ACTIONS: [&str; 6] = [
    "create",
    "replace",
    "delete",
    "transfer",
    "update_price",
    "purchase",
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
        if map.contains_key("rangeAverageable") {
            self.required(map, &["documentsAverageable"])?;
        }
        if map.contains_key("rangeSummable") && !map.contains_key("documentsAverageable") {
            self.required(map, &["documentsSummable"])?;
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
            "indices" => self.indices(value),
            "signatureSecurityLevelRequirement" => {
                self.integer(value, None, None)?;
                self.one_of_integers(value, &[1, 2, 3])
            }
            "documentsKeepHistory"
            | "keepsTransferHistory"
            | "keepsPurchaseHistory"
            | "keepsPricingHistory"
            | "documentsMutable"
            | "documentsCountable"
            | "rangeCountable"
            | "rangeSummable"
            | "rangeAverageable"
            | "indexOnly" => self.boolean(value),
            "canBeDeleted" => self.one_of(
                value,
                &[json!(true), json!(false), json!("onlyWhenConsumed")],
            ),
            "transferable" | "tradeMode" => {
                self.integer(value, None, None)?;
                self.one_of_integers(value, &[0, 1])
            }
            "creationRestrictionMode"
            | "requiresIdentityEncryptionBoundedKey"
            | "requiresIdentityDecryptionBoundedKey" => {
                self.integer(value, None, None)?;
                self.one_of_integers(value, &[0, 1, 2])
            }
            "documentsSummable" | "documentsAverageable" => {
                self.string_of_length(value, 1, Some(64))?;
                self.word(value)
            }
            "moderatorAbilities" => self.moderator_abilities(value),
            "ttl" => self.integer(value, Some(1), Some(i64::from(u32::MAX))),
            "entryPayload" => {
                let items = self.array(value)?;
                for (index, item) in items.iter().enumerate() {
                    self.at(index, |c| c.string_of_length(item, 1, Some(64)).map(drop))?;
                }
                self.item_count(items, Some(1), Some(16))?;
                self.unique(items)
            }
            "actionFees" => self.action_fees(value),
            "tokenCost" => self.token_cost(value),
            "properties" => self.root_properties(value),
            "transient" => {
                let items = self.array(value)?;
                for (index, item) in items.iter().enumerate() {
                    self.at(index, |c| c.string(item).map(drop))?;
                }
                Ok(())
            }
            "immutable" => self.immutable(value),
            "retractedWhen" => {
                let map = self.object(value)?;
                self.property_count(map, Some(1), Some(1))
            }
            "ownerRefersTo" | "creatorRefersTo" => self.owner_refers_to(value),
            "propertyConstraints" | "deleteConstraints" => {
                let map = self.object(value)?;
                for (name, rule) in map {
                    self.name_matching(name, WORD_PATTERN, is_word(name))?;
                    self.at(name, |c| c.constraint(rule))?;
                }
                self.property_count(map, Some(1), None)
            }
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
            if SYSTEM_PROPERTIES.contains(&name.as_str()) {
                self.at(name, |c| c.compiled_patterns(schema))?;
                continue;
            }
            if !is_word(name) {
                // A word, or one of the system names
                return Err(self.name_error(
                    name,
                    "oneOfNotValid",
                    json!({}),
                    format!("\"{name}\" is not valid under any of the schemas listed in the 'oneOf' keyword"),
                ));
            }
            self.at(name, |c| c.property_schema(schema))?;
        }
        self.property_count(map, Some(1), Some(100))
    }

    fn moderator_abilities(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed(
            map,
            &[
                "delete",
                "deleteWithin",
                "deleteKeepsRecord",
                "deleteRefundsOwner",
                "deleteSettled",
                "deleteKeepsFields",
                "changeFields",
            ],
        )?;
        self.property_count(map, Some(1), None)?;
        for (name, ability) in map {
            self.at(name, |c| match name.as_str() {
                "delete" | "deleteKeepsRecord" | "deleteRefundsOwner" => c.boolean(ability),
                "deleteWithin" => c.integer(ability, Some(1), Some(i64::from(u32::MAX))),
                "deleteSettled" => {
                    let settled = c.object(ability)?;
                    c.closed(
                        settled,
                        &["leader", "approvals", "approversPredateDocument"],
                    )?;
                    c.property_count(settled, Some(1), None)?;
                    c.member(settled, "leader", |c, value| c.boolean(value))?;
                    c.member(settled, "approvals", |c, value| {
                        c.integer(value, Some(1), Some(31))
                    })?;
                    c.member(settled, "approversPredateDocument", |c, value| {
                        c.boolean(value)
                    })
                }
                "deleteKeepsFields" | "changeFields" => {
                    let items = c.array(ability)?;
                    for (index, item) in items.iter().enumerate() {
                        c.at(index, |c| c.string_of_length(item, 1, None).map(drop))?;
                    }
                    c.item_count(items, Some(1), None)?;
                    c.unique(items)
                }
                _ => Ok(()),
            })?;
        }
        Ok(())
    }

    fn action_fees(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        let mut allowed = vec!["pricing"];
        allowed.extend(DOCUMENT_ACTIONS);
        self.closed(map, &allowed)?;
        for (name, fee) in map {
            self.at(name, |c| {
                if name == "pricing" {
                    c.string(fee)?;
                    return c.one_of_strings(fee, &["feeMultiplier", "fixed"]);
                }
                let fee = c.object(fee)?;
                c.closed(fee, &["owner", "moderators"])?;
                c.property_count(fee, Some(1), None)?;
                for payer in ["owner", "moderators"] {
                    c.member(fee, payer, |c, value| {
                        c.integer(value, Some(0), Some(i64::MAX))
                    })?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    fn token_cost(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed(map, &DOCUMENT_ACTIONS)?;
        for (name, cost) in map {
            self.at(name, |c| {
                let cost = c.object(cost)?;
                c.closed(
                    cost,
                    &[
                        "contractId",
                        "tokenPosition",
                        "amount",
                        "effect",
                        "gasFeesPaidBy",
                        "optional",
                    ],
                )?;
                c.required(cost, &["tokenPosition", "amount"])?;
                c.member(cost, "contractId", |c, value| {
                    let items = c.array(value)?;
                    c.item_count(items, Some(32), Some(32))?;
                    // `byteArray`, as the meta-schema check applied it
                    for (index, item) in items.iter().enumerate() {
                        match item.as_u64() {
                            None => return c.at(index, |c| Err(c.type_error(item, &["integer"]))),
                            Some(byte) if byte > u64::from(u8::MAX) => {
                                return c.at(index, |c| {
                                    Err(c.fail_with(
                                        "maximum",
                                        format!("{item} is greater than the maximum of 255"),
                                        json!({ "maximum": u8::MAX }),
                                        "",
                                    ))
                                })
                            }
                            Some(_) => {}
                        }
                    }
                    Ok(())
                })?;
                c.member(cost, "tokenPosition", |c, value| {
                    c.integer(value, Some(0), Some(65535))
                })?;
                c.member(cost, "amount", |c, value| {
                    c.integer(value, Some(1), Some(281_474_976_710_655))
                })?;
                c.member(cost, "effect", |c, value| {
                    c.integer(value, None, None)?;
                    c.one_of_integers(value, &[0, 1])
                })?;
                c.member(cost, "gasFeesPaidBy", |c, value| {
                    c.integer(value, None, None)?;
                    c.one_of_integers(value, &[0, 1, 2])
                })?;
                c.member(cost, "optional", |c, value| c.boolean(value))
            })?;
        }
        Ok(())
    }

    fn immutable(&self, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        for (index, item) in items.iter().enumerate() {
            self.at(index, |c| {
                if item.is_string() {
                    return Ok(());
                }
                c.within("anyOf", item, |c| {
                    let entry = c.object(item)?;
                    c.closed(entry, &["property", "when"])?;
                    c.required(entry, &["property", "when"])?;
                    c.member(entry, "property", |c, value| c.string(value).map(drop))?;
                    c.member(entry, "when", |c, value| {
                        let when = c.object(value)?;
                        c.property_count(when, Some(1), Some(1))
                    })
                })
            })?;
        }
        Ok(())
    }
}
