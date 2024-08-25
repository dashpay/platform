use crate::data_contract::JsonValue;
use crate::validation::JsonSchemaValidator;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl JsonSchemaValidator {
    #[inline(always)]
    pub(super) fn new_compiled_v0(
        json_schema: &JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let validator = Self::new();

        validator.compile(json_schema, platform_version)?;

        Ok(validator)
    }
}
