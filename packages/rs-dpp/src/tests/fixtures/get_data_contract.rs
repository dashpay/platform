use std::sync::Arc;

use serde_json::json;

use crate::prelude::*;
use crate::{
    data_contract::validation::data_contract_validator::DataContractValidator,
    data_contract::DataContractFactory,
    identifier,
    tests::utils::generate_random_identifier_struct,
    version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
};

pub fn get_data_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let documents = json!(
    {
        "niceDocument": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                }
            },
            "required": [
                "$createdAt"
            ],
            "additionalProperties": false
        },
        "prettyDocument": {
            "type": "object",
            "properties": {
                "lastName": {
                    "$ref": "#/$defs/lastName"
                }
            },
            "required": [
                "lastName",
                "$updatedAt"
            ],
            "additionalProperties": false
        },
        "indexedDocument": {
            "type": "object",
            "indices": [
                {
                    "name": "index1",
                    "properties": [
                        {
                            "$ownerId": "asc"
                        },
                        {
                            "firstName": "asc"
                        }
                    ],
                    "unique": true
                },
                {
                    "name": "index2",
                    "properties": [
                        {
                            "$ownerId": "asc"
                        },
                        {
                            "lastName": "asc"
                        }
                    ],
                    "unique": true
                },
                {
                    "name": "index3",
                    "properties": [
                        {
                            "lastName": "asc"
                        }
                    ]
                },
                {
                    "name": "index4",
                    "properties": [
                        {
                            "$createdAt": "asc"
                        },
                        {
                            "$updatedAt": "asc"
                        }
                    ]
                },
                {
                    "name": "index5",
                    "properties": [
                        {
                            "$updatedAt": "asc"
                        }
                    ]
                },
                {
                    "name": "index6",
                    "properties": [
                        {
                            "$createdAt": "asc"
                        }
                    ]
                }
            ],
            "properties": {
                "firstName": {
                    "type": "string",
                    "maxLength": 63
                },
                "lastName": {
                    "type": "string",
                    "maxLength": 63
                }
            },
            "required": [
                "firstName",
                "$createdAt",
                "$updatedAt",
                "lastName"
            ],
            "additionalProperties": false
        },
        "noTimeDocument": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
                "array": {
                    "type": "array",
                    "items": {
                        "type": "number"
                    }
                }
            },
            "additionalProperties": false
        },
        "uniqueDates": {
            "type": "object",
            "indices": [
                {
                    "name": "index1",
                    "properties": [
                        {
                            "$createdAt": "asc"
                        },
                        {
                            "$updatedAt": "asc"
                        }
                    ],
                    "unique": true
                },
                {
                    "name": "index2",
                    "properties": [
                        {
                            "$updatedAt": "asc"
                        }
                    ]
                }
            ],
            "properties": {
                "firstName": {
                    "type": "string"
                },
                "lastName": {
                    "type": "string"
                }
            },
            "required": [
                "firstName",
                "$createdAt",
                "$updatedAt"
            ],
            "additionalProperties": false
        },
        "withByteArrays": {
            "type": "object",
            "indices": [
                {
                    "name": "index1",
                    "properties": [
                        {
                            "byteArrayField": "asc"
                        }
                    ]
                }
            ],
            "properties": {
                "byteArrayField": {
                    "type": "array",
                    "byteArray": true,
                    "maxItems": 16,
                },
                "identifierField": {
                    "type": "array",
                    "byteArray": true,
                    "contentMediaType": identifier::MEDIA_TYPE,
                    "minItems": 32,
                    "maxItems": 32
                }
            },
            "required": [
                "byteArrayField"
            ],
            "additionalProperties": false
        },
        "optionalUniqueIndexedDocument": {
            "type": "object",
            "properties": {
                "firstName": {
                    "type": "string",
                    "maxLength": 63
                },
                "lastName": {
                    "type": "string",
                    "maxLength": 63
                },
                "country": {
                    "type": "string",
                    "maxLength": 63
                },
                "city": {
                    "type": "string",
                    "maxLength": 63
                }
            },
            "indices": [
                {
                    "name": "index1",
                    "properties": [
                        {
                            "firstName": "asc"
                        }
                    ],
                    "unique": true
                },
                {
                    "name": "index2",
                    "properties": [
                        {
                            "$ownerId": "asc"
                        },
                        {
                            "firstName": "asc"
                        },
                        {
                            "lastName": "asc"
                        }
                    ],
                    "unique": true
                },
                {
                    "name": "index3",
                    "properties": [
                        {
                            "country": "asc"
                        },
                        {
                            "city": "asc"
                        }
                    ],
                    "unique": true
                }
            ],
            "required": [
                "firstName",
                "lastName"
            ],
            "additionalProperties": false
        }
    });

    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());
    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));
    let factory = DataContractFactory::new(1, data_contract_validator);

    let owner_id = owner_id.unwrap_or_else(generate_random_identifier_struct);

    let mut data_contract = factory
        .create(owner_id, documents)
        .expect("data in fixture should be correct");

    data_contract
        .defs
        .insert(String::from("lastName"), json!({ "type" : "string"}));

    data_contract
}
