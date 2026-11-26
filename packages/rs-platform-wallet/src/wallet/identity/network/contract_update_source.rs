//! Splitting pasted or imported contract JSON into the sections a data
//! contract update takes (`IdentityWallet::preview_data_contract_update` and
//! `update_data_contract_with_signer`).
//!
//! Each section is returned as JSON text in source order. The order of an
//! object's members is part of a document type's schema (protocol version 15
//! refuses a reordering of a stored object's members), so a host must not
//! re-serialize a section through a JSON reader that sorts or loses keys.
//! This crate's `serde_json` keeps member order (`preserve_order`).

use crate::error::PlatformWalletError;

/// The sections of a contract update a piece of JSON supplies. A section
/// the source does not supply is `None`: the update keeps the live one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractUpdateSource {
    /// Document schemas keyed by type name, as JSON text.
    pub documents: Option<String>,
    /// Token configurations keyed by position, as JSON text.
    pub tokens: Option<String>,
    /// Groups, as JSON text (an object keyed by position, or an array).
    pub groups: Option<String>,
    /// Keywords, as a JSON array.
    pub keywords: Option<String>,
    /// The description text (not JSON).
    pub description: Option<String>,
    /// The contract configuration, as JSON text.
    pub config: Option<String>,
    /// The source has a non-empty `schemaDefs` or `$defs` object. An update
    /// does not send shared definitions: Platform keeps the live ones.
    pub carries_shared_definitions: bool,
}

/// The section names an object is read for, in the order the error lists
/// them.
const SECTION_NAMES: [&str; 8] = [
    "documents",
    "documentSchemas",
    "tokens",
    "tokenSchemas",
    "groups",
    "keywords",
    "description",
    "config",
];

/// Split `text` into the sections of a contract update.
///
/// `text` must be a JSON object. Read are `documents` or `documentSchemas`,
/// `tokens` or `tokenSchemas`, `groups`, `keywords`, `description` and
/// `config`. Every other member (`id`, `ownerId`, `version`,
/// `$formatVersion`, `schemaDefs`, ...) is ignored, so an exported contract
/// splits as is. A `null` description or config is the same as none.
///
/// An object whose members all look like document type schemas (an object
/// with `properties`, or with `type` equal to `object`) is read as a bare
/// document schemas map first: a document type may be named `documents`,
/// `description` or `groups`, and reading such a name as a section would
/// unwrap or refuse it.
///
/// Every supplied section is returned as given, whatever it holds: a
/// malformed section reaches the update and is refused there, never dropped
/// here.
pub fn split_contract_update_source(
    text: &str,
) -> Result<ContractUpdateSource, PlatformWalletError> {
    let invalid = |message: String| PlatformWalletError::InvalidParameter(message);
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| invalid("The text is not JSON.".to_string()))?;
    let serde_json::Value::Object(object) = value else {
        return Err(invalid("The JSON is not an object.".to_string()));
    };

    let carries_shared_definitions = ["schemaDefs", "$defs"].iter().any(|key| {
        object
            .get(*key)
            .and_then(serde_json::Value::as_object)
            .is_some_and(|definitions| !definitions.is_empty())
    });

    let bare_schemas = !object.is_empty() && object.values().all(looks_like_document_schema);
    if bare_schemas {
        return Ok(ContractUpdateSource {
            documents: Some(serde_json::Value::Object(object).to_string()),
            carries_shared_definitions,
            ..Default::default()
        });
    }

    let section =
        |name: &str, alias: &str| -> Result<Option<&serde_json::Value>, PlatformWalletError> {
            match (object.get(name), object.get(alias)) {
                (Some(_), Some(_)) => Err(invalid(format!(
                    "The JSON has both `{name}` and `{alias}`; keep one."
                ))),
                (value, None) | (None, value) => Ok(value),
            }
        };
    let documents = section("documents", "documentSchemas")?;
    let tokens = section("tokens", "tokenSchemas")?;
    let groups = object.get("groups");
    let keywords = object.get("keywords");
    let description = object.get("description");
    let config = object.get("config").filter(|value| !value.is_null());

    if documents.is_none()
        && tokens.is_none()
        && groups.is_none()
        && keywords.is_none()
        && description.is_none()
        && config.is_none()
    {
        if object.is_empty() {
            return Err(invalid("The JSON object is empty.".to_string()));
        }
        let mut keys: Vec<&String> = object.keys().collect();
        keys.sort();
        return Err(invalid(format!(
            "No contract sections found ({}). Top-level keys: {}.",
            SECTION_NAMES.join(", "),
            keys.iter()
                .map(|key| key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let keywords = match keywords {
        None => None,
        Some(serde_json::Value::Array(items)) => {
            Some(serde_json::Value::Array(items.clone()).to_string())
        }
        Some(_) => return Err(invalid("`keywords` must be a JSON array.".to_string())),
    };
    let description = match description {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(text)) => Some(text.clone()),
        Some(_) => return Err(invalid("`description` must be a JSON string.".to_string())),
    };

    Ok(ContractUpdateSource {
        documents: documents.map(serde_json::Value::to_string),
        tokens: tokens.map(serde_json::Value::to_string),
        groups: groups.map(serde_json::Value::to_string),
        keywords,
        description,
        config: config.map(serde_json::Value::to_string),
        carries_shared_definitions,
    })
}

/// Whether `value` looks like a document type schema: an object with
/// `properties`, or with `type` equal to `object`.
fn looks_like_document_schema(value: &serde_json::Value) -> bool {
    let Some(schema) = value.as_object() else {
        return false;
    };
    schema.contains_key("properties")
        || schema.get("type").and_then(serde_json::Value::as_str) == Some("object")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(text: &str) -> ContractUpdateSource {
        split_contract_update_source(text).expect("split")
    }

    fn refused(text: &str) -> String {
        split_contract_update_source(text)
            .expect_err("must be refused")
            .to_string()
    }

    #[test]
    fn should_keep_the_member_order_of_every_section() {
        // `z` before `a`, as positions 0 and 1 require; a sorting reader
        // would swap them.
        let documents = r#"{"note":{"type":"object","properties":{"z":{"type":"string","position":0},"a":{"type":"object","properties":{"y":{"type":"string","position":0},"b":{"type":"string","position":1}},"position":1}},"additionalProperties":false}}"#;
        let text = format!(r#"{{"id":"x","documentSchemas":{documents},"version":3}}"#);

        let source = split(&text);

        assert_eq!(source.documents.as_deref(), Some(documents));
    }

    #[test]
    fn should_split_every_section_of_an_exported_contract() {
        let text = r#"{"$formatVersion":"1","id":"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "ownerId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF","version":3,
            "config":{"$formatVersion":"1","canBeDeleted":false},"schemaDefs":null,
            "documentSchemas":{"note":{"type":"object","properties":{}}},
            "tokens":{"0":{"baseSupply":1000}},"groups":{"0":{"requiredPower":1}},
            "keywords":["alpha,beta","gamma"],"description":"  spaced  "}"#;

        let source = split(text);

        assert_eq!(
            source.documents.as_deref(),
            Some(r#"{"note":{"type":"object","properties":{}}}"#)
        );
        assert_eq!(
            source.tokens.as_deref(),
            Some(r#"{"0":{"baseSupply":1000}}"#)
        );
        assert_eq!(
            source.groups.as_deref(),
            Some(r#"{"0":{"requiredPower":1}}"#)
        );
        assert_eq!(
            source.keywords.as_deref(),
            Some(r#"["alpha,beta","gamma"]"#)
        );
        assert_eq!(source.description.as_deref(), Some("  spaced  "));
        assert_eq!(
            source.config.as_deref(),
            Some(r#"{"$formatVersion":"1","canBeDeleted":false}"#)
        );
        assert!(!source.carries_shared_definitions);
    }

    #[test]
    fn should_treat_a_null_description_and_config_as_none() {
        let source = split(
            r#"{"documentSchemas":{"note":{"type":"object"}},"description":null,"config":null}"#,
        );

        assert_eq!(source.description, None);
        assert_eq!(source.config, None);
        assert!(source.documents.is_some());
    }

    #[test]
    fn should_import_standalone_sections() {
        assert_eq!(
            split(r#"{"description":"Only this"}"#)
                .description
                .as_deref(),
            Some("Only this")
        );
        assert_eq!(
            split(r#"{"groups":[{"id":0,"requiredPower":1}]}"#)
                .groups
                .as_deref(),
            Some(r#"[{"id":0,"requiredPower":1}]"#)
        );
        assert_eq!(
            split(r#"{"config":{"readonly":true}}"#).config.as_deref(),
            Some(r#"{"readonly":true}"#)
        );
    }

    #[test]
    fn should_pass_a_malformed_section_through() {
        let source = split(r#"{"groups":{"0":null},"tokens":"not tokens"}"#);

        assert_eq!(source.groups.as_deref(), Some(r#"{"0":null}"#));
        assert_eq!(source.tokens.as_deref(), Some(r#""not tokens""#));
    }

    #[test]
    fn should_read_a_bare_document_schemas_map_even_with_section_names() {
        let text = r#"{"documents":{"type":"object","properties":{"title":{"type":"string","position":0}}},"description":{"type":"object","properties":{}}}"#;

        let source = split(text);

        assert_eq!(source.documents.as_deref(), Some(text));
        assert_eq!(source.description, None);
    }

    #[test]
    fn should_report_non_empty_shared_definitions() {
        let with = |defs: &str| {
            split(&format!(
                r#"{{{defs},"documentSchemas":{{"note":{{"type":"object","properties":{{}}}}}}}}"#
            ))
            .carries_shared_definitions
        };
        assert!(with(r#""schemaDefs":{"label":{"type":"string"}}"#));
        assert!(with(r#""$defs":{"label":{"type":"string"}}"#));
        assert!(!with(r#""schemaDefs":{}"#));
        assert!(!with(r#""schemaDefs":null"#));
    }

    #[test]
    fn should_refuse_what_cannot_be_split() {
        assert!(refused("not json").contains("not JSON"));
        assert!(refused("[1, 2]").contains("not an object"));
        assert!(refused("{}").contains("empty"));
        assert!(refused(r#"{"foo":1,"bar":{"baz":true}}"#).contains("Top-level keys: bar, foo."));
        assert!(refused(r#"{"documents":{},"documentSchemas":{}}"#)
            .contains("both `documents` and `documentSchemas`"));
        assert!(refused(r#"{"keywords":"alpha"}"#).contains("`keywords` must be a JSON array"));
        assert!(refused(r#"{"description":5}"#).contains("`description` must be a JSON string"));
    }
}
