use json_schema_compatibility_validator::{validate_schemas_compatibility, Options};
use serde_json::json;

#[test]
fn test_properties_reordering() {
    let original_schema = json!({
        "properties": {
            "prop1": {
                "type": "string"
            },
            "prop2": {
                "type": "string"
            },
            "prop3": {
                "type": "number"
            }
        }
    });

    let new_schema = json!({
        "properties": {
            "prop2": {
                "type": "string"
            },
            "prop1": {
                "type": "string"
            },
            "prop3": {
                "type": "number"
            },
        }
    });

    let result = validate_schemas_compatibility(&original_schema, &new_schema, Options::default())
        .expect("schema compatibility validation error");

    assert!(
        result.is_compatible(),
        "assertion failed: incompatible changes: {:?}",
        result.incompatible_changes()
    );
}

#[test]
fn test_reordering_from_string() {
    let original_schema_string = r#"{
        "properties": {
            "prop1": {
                "type": "string"
            },
            "prop2": {
                "type": "string"
            },
            "prop3": {
                "type": "number"
            }
        }
    }"#;

    let original_schema = serde_json::from_str(original_schema_string)
        .expect("failed to decode from string to json value");

    let new_schema_string = r#"{
        "properties": {
            "prop2": {
                "type": "string"
            },
            "prop1": {
                "type": "string"
            },
            "prop3": {
                "type": "number"
            }
        }
    }"#;

    let new_schema = serde_json::from_str(new_schema_string)
        .expect("failed to decode from string to json value");

    let result = validate_schemas_compatibility(&original_schema, &new_schema, Options::default())
        .expect("schema compatibility validation error");

    assert!(
        result.is_compatible(),
        "assertion failed: incompatible changes: {:?}",
        result.incompatible_changes()
    );
}

#[test]
fn test_keywords_reordering() {
    let original_schema = json!({
        "type": "array",
        "items": false,
        "prefixItems": [
            {
                "type": "string",
            },
            {
                "type": "number"
            }
        ]
    });

    let new_schema = json!({
        "type": "array",
        "prefixItems": [
            {
                "type": "string",
            },
            {
                "type": "number"
            }
        ],
        "items": false,
    });

    let result = validate_schemas_compatibility(&original_schema, &new_schema, Options::default())
        .expect("schema compatibility validation error");

    assert!(
        result.is_compatible(),
        "assertion failed: incompatible changes: {:?}",
        result.incompatible_changes()
    );
}

#[test]
fn test_inner_keywords_reordering() {
    let original_schema = json!({
        "type": "array",
        "items": false,
        "prefixItems": [
            {
                "type": "string",
                "minLength": 1,
                "maxLength": 20,
                "pattern": "^[a-z]$",
            },
            {
                "type": "number"
            }
        ]
    });

    let new_schema = json!({
        "type": "array",
        "items": false,
        "prefixItems": [
            {
                "type": "string",
                "maxLength": 20,
                "minLength": 1,
                "pattern": "^[a-z]$",
            },
            {
                "type": "number"
            }
        ]
    });

    let result = validate_schemas_compatibility(&original_schema, &new_schema, Options::default())
        .expect("schema compatibility validation error");

    assert!(
        result.is_compatible(),
        "assertion failed: incompatible changes: {:?}",
        result.incompatible_changes()
    );
}
