use std::process::id;
use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::paths::PathChunk;
use jsonschema::primitive_type::PrimitiveType::Integer;
use serde_json::Value;
use crate::errors::consensus::ConsensusError;
use crate::identity::validation::IdentityValidator;

#[test]
pub fn balance_should_be_an_integer() {
    let mut identity = crate::tests::fixtures::identity_fixture_json();
    let map = identity.as_object_mut().expect("Expected value to be an JSON object");
    map.insert("balance".parse().unwrap(), Value::from(1.2));

    let identity_validator = IdentityValidator::new().unwrap();
    let result = identity_validator.validate_identity(&identity);

    // expectJsonSchemaError(result);

    let error = result.errors().first().expect("Expected to be at least one validation error");

    match error {
        ConsensusError::JsonSchemaError(error) => {
            //assert_eq!(error.to_string(), "something");
            //let expected_kind = ValidationErrorKind::Type { kind: TypeKind::Single(Integer) };
            //assert_eq!(error.kind(), &expected_kind);
            let keyword = error.keyword().expect("Expected to have a keyword");

            assert_eq!(keyword, "type");
            assert_eq!(error.instance_path().to_string(), "/balance");
        }
        _ => panic!("Expected JsonSchemaError")
    }
}