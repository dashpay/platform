use crate::identity::validation::{
    PublicKeysValidator, PUBLIC_KEY_SCHEMA, PUBLIC_KEY_SCHEMA_FOR_TRANSITION,
};

pub fn get_public_keys_validator_for_transition() -> PublicKeysValidator {
    PublicKeysValidator::new_with_schema(PUBLIC_KEY_SCHEMA_FOR_TRANSITION.clone()).unwrap()
}

pub fn get_public_keys_validator() -> PublicKeysValidator {
    PublicKeysValidator::new_with_schema(PUBLIC_KEY_SCHEMA.clone()).unwrap()
}
