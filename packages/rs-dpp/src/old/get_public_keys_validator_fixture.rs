use crate::identity::validation::{
    PublicKeysValidator, PUBLIC_KEY_SCHEMA, PUBLIC_KEY_SCHEMA_FOR_TRANSITION,
};
use crate::NativeBlsModule;

pub fn get_public_keys_validator_for_transition() -> PublicKeysValidator<NativeBlsModule> {
    PublicKeysValidator::new_with_schema(
        PUBLIC_KEY_SCHEMA_FOR_TRANSITION.clone(),
        NativeBlsModule::default(),
    )
    .unwrap()
}

pub fn get_public_keys_validator() -> PublicKeysValidator<NativeBlsModule> {
    PublicKeysValidator::new_with_schema(PUBLIC_KEY_SCHEMA.clone(), NativeBlsModule::default())
        .unwrap()
}
