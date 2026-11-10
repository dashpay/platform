mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    40, 81, 10, 2, 166, 107, 62, 218, 6, 184, 111, 202, 230, 218, 170, 157, 181, 119, 102, 211,
    195, 114, 92, 200, 90, 88, 205, 30, 174, 49, 203, 200,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

/// The contract's definitions for `platform_version`. Version 0, the value the tables of
/// protocol versions below 15 carry, names no schema generation and is refused like any
/// unknown version: the contract does not exist before its activation.
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.decision_proposals {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "decision_proposals_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.decision_proposals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "decision_proposals_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use base58::FromBase58;
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn should_match_the_published_system_contract_id() {
        let base58_decoded = "3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF"
            .from_base58()
            .expect("base58 id");
        assert_eq!(base58_decoded, ID_BYTES);
        assert_eq!(ID, Identifier(IdentifierBytes32(ID_BYTES)));
    }

    #[test]
    fn should_load_the_schema_at_the_latest_platform_version() {
        use v1::document_types::proposal;

        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");
        // The yes/no proposal, and the two types of a multiple choice one.
        let types: BTreeSet<&str> = schema
            .as_object()
            .expect("the schema is an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            types,
            BTreeSet::from([
                proposal::NAME,
                v1::document_types::multiple_choice_proposal::NAME,
                v1::document_types::choice::NAME,
            ])
        );
        let proposal_type = &schema[proposal::NAME];
        let properties = proposal_type["properties"]
            .as_object()
            .expect("proposal has properties");
        let declared: BTreeSet<&str> = properties.keys().map(String::as_str).collect();
        assert_eq!(
            declared,
            BTreeSet::from([
                proposal::properties::TITLE,
                proposal::properties::DESCRIPTION,
                proposal::properties::URL,
                proposal::properties::PASS_PERCENT,
                proposal::properties::MINIMUM_TURNOUT_PERCENT,
                proposal::properties::END_DATE,
            ])
        );
        // A pass threshold of 50 to 80 percent, a turnout of at most half the network
        let bounds = |property: &str| {
            (
                properties[property]["minimum"].as_u64(),
                properties[property]["maximum"].as_u64(),
            )
        };
        assert_eq!(
            bounds(proposal::properties::PASS_PERCENT),
            (Some(50), Some(80))
        );
        assert_eq!(
            bounds(proposal::properties::MINIMUM_TURNOUT_PERCENT),
            (Some(0), Some(50))
        );
        let indexes: Vec<&str> = proposal_type["indices"]
            .as_array()
            .expect("proposal has indexes")
            .iter()
            .filter_map(|index| index["name"].as_str())
            .collect();
        assert_eq!(
            indexes,
            [
                proposal::indexes::BY_END_DATE,
                proposal::indexes::BY_OWNER,
                proposal::indexes::BY_CREATED_AT
            ]
        );
        // The rule masternodes vote by never changes; the text may be edited for a while, and
        // every version is kept. A proposal is never deleted.
        assert_eq!(proposal_type["documentsMutable"], Value::Bool(true));
        assert_eq!(proposal_type["documentsKeepHistory"], Value::Bool(true));
        assert_eq!(proposal_type["canBeDeleted"], Value::Bool(false));
        let immutable = proposal_type["immutable"]
            .as_array()
            .expect("proposal lists immutable properties");
        for property in [
            proposal::properties::PASS_PERCENT,
            proposal::properties::MINIMUM_TURNOUT_PERCENT,
            proposal::properties::END_DATE,
        ] {
            assert!(
                immutable.contains(&Value::String(property.to_string())),
                "{property} never changes"
            );
        }
        for property in [
            proposal::properties::TITLE,
            proposal::properties::DESCRIPTION,
            proposal::properties::URL,
        ] {
            assert!(
                immutable
                    .iter()
                    .any(|entry| entry["property"] == property && entry["when"].is_object()),
                "{property} freezes once its edit window ends"
            );
        }
    }

    #[test]
    fn should_put_the_choices_of_a_multiple_choice_proposal_to_a_contest() {
        use v1::document_types::{choice, multiple_choice_proposal};

        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");

        // The question's title and url may be edited, and the description it points at
        // changed, for two hours, every version kept, and then they freeze. It is never
        // deleted, which is what lets a choice refer to it.
        let question = &schema[multiple_choice_proposal::NAME];
        assert_eq!(question["documentsMutable"], Value::Bool(true));
        assert_eq!(question["documentsKeepHistory"], Value::Bool(true));
        assert_eq!(question["canBeDeleted"], Value::Bool(false));
        let two_hours_after_creation = serde_json::json!({
            "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, 7_200_000]
        });
        let frozen: Vec<(&str, &Value)> = question["immutable"]
            .as_array()
            .expect("the question lists immutable properties")
            .iter()
            .filter(|entry| entry.is_object())
            .map(|entry| {
                (
                    entry["property"].as_str().expect("a conditional entry"),
                    &entry["when"],
                )
            })
            .collect();
        assert_eq!(
            frozen,
            [
                (
                    multiple_choice_proposal::properties::TITLE,
                    &two_hours_after_creation
                ),
                (
                    multiple_choice_proposal::properties::DESCRIPTION_ID,
                    &two_hours_after_creation
                ),
                (
                    multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID,
                    &two_hours_after_creation
                ),
                (
                    multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE,
                    &two_hours_after_creation
                ),
                (
                    multiple_choice_proposal::properties::URL,
                    &two_hours_after_creation
                ),
            ]
        );
        let declared: BTreeSet<&str> = question["properties"]
            .as_object()
            .expect("the question has properties")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            declared,
            BTreeSet::from([
                multiple_choice_proposal::properties::TITLE,
                multiple_choice_proposal::properties::DESCRIPTION_ID,
                multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID,
                multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE,
                multiple_choice_proposal::properties::URL,
                multiple_choice_proposal::properties::OPEN,
            ])
        );
        // The question holds no description of its own: it points at a document of a
        // moderated type, of the contract and the type it names, holding a string or bytes
        // that a moderator's removal takes away and that its author writes
        let taken_with_the_document = serde_json::json!({
            "allOf": [
                { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$self"] } },
                { "not": { "contains": ["$moderatorAbilities.changeFields", "$self"] } }
            ]
        });
        assert_eq!(
            question["properties"][multiple_choice_proposal::properties::DESCRIPTION_ID]
                ["refersTo"],
            serde_json::json!({
                "type": "moderatedDocument",
                "contractIdProperty":
                    multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID,
                "documentTypeProperty":
                    multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE,
                "documentTypeRequirements": {
                    "properties": {
                        "body": {
                            "name": "$any",
                            "type": "string",
                            "when": taken_with_the_document
                        },
                        "bodyBytes": {
                            "name": "$any",
                            "type": "array",
                            "byteArray": true,
                            "when": taken_with_the_document
                        }
                    },
                    "require": { "anyOf": ["body", "bodyBytes"] }
                }
            })
        );
        // and that contract is one whose moderators the masternodes elect
        assert_eq!(
            question["properties"][multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID]
                ["refersTo"],
            serde_json::json!({
                "type": "contract",
                "contractRequirements": { "moderation": "elected" }
            })
        );
        let required: Vec<&str> = question["required"]
            .as_array()
            .expect("the question lists required properties")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        for property in [
            multiple_choice_proposal::properties::DESCRIPTION_ID,
            multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID,
            multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE,
        ] {
            assert!(required.contains(&property), "{property} is required");
        }
        // Every question says whether it is open, for good
        assert_eq!(
            question["properties"][multiple_choice_proposal::properties::OPEN]["type"],
            "boolean"
        );
        assert!(question["required"]
            .as_array()
            .expect("the question lists required properties")
            .contains(&Value::String(
                multiple_choice_proposal::properties::OPEN.to_string()
            )));
        assert!(question["immutable"]
            .as_array()
            .expect("the question lists immutable properties")
            .contains(&Value::String(
                multiple_choice_proposal::properties::OPEN.to_string()
            )));
        let indexes: Vec<&str> = question["indices"]
            .as_array()
            .expect("the question has indexes")
            .iter()
            .filter_map(|index| index["name"].as_str())
            .collect();
        assert_eq!(
            indexes,
            [
                multiple_choice_proposal::indexes::BY_OWNER,
                multiple_choice_proposal::indexes::BY_CREATED_AT
            ]
        );

        // A choice names its question, and the choices of one question share one contest:
        // the only index is unique on the question alone, contested for every value, and
        // resolved by a vote among documents, with a lock choice.
        let choice_type = &schema[choice::NAME];
        assert_eq!(choice_type["documentsMutable"], Value::Bool(false));
        assert_eq!(choice_type["canBeDeleted"], Value::Bool(false));
        let reference = &choice_type["properties"][choice::properties::PROPOSAL_ID]["refersTo"];
        assert_eq!(reference["type"], "permanentDocument");
        assert_eq!(reference["documentType"], multiple_choice_proposal::NAME);
        let indexes = choice_type["indices"]
            .as_array()
            .expect("the choice has indexes");
        assert_eq!(indexes.len(), 1);
        let contest = &indexes[0];
        assert_eq!(contest["name"], choice::indexes::BY_PROPOSAL);
        assert_eq!(contest["unique"], Value::Bool(true));
        assert_eq!(
            contest["properties"],
            serde_json::json!([{ choice::properties::PROPOSAL_ID: "asc" }])
        );
        // Resolution 2: each choice is a contender of its own, so an identity enters several
        assert_eq!(contest["contested"]["resolution"], 2);
        assert!(contest["contested"].get("fieldMatches").is_none());

        // The choices of a question that is not open are its owner's
        assert_eq!(
            choice_type["propertyConstraints"],
            serde_json::json!({
                choice::rules::CLOSED_PROPOSAL_TAKES_ITS_OWNERS_CHOICES: {
                    "ifThen": [
                        {
                            "equal": [
                                {
                                    "referenced": [
                                        choice::properties::PROPOSAL_ID,
                                        multiple_choice_proposal::properties::OPEN
                                    ]
                                },
                                0
                            ]
                        },
                        {
                            "equal": [
                                { "referenced": [choice::properties::PROPOSAL_ID, "$ownerId"] },
                                "$ownerId"
                            ]
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn should_refuse_to_load_below_the_activation_version() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14");
        assert!(matches!(
            load_documents_schemas(platform_version),
            Err(Error::UnknownVersionMismatch { received: 0, .. })
        ));
        assert!(matches!(
            load_definitions(platform_version),
            Err(Error::UnknownVersionMismatch { received: 0, .. })
        ));
    }
}
