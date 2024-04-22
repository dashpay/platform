use serde::Serialize;
#[cfg(feature = "state-transition-json-conversion")]
use serde_json::Value as JsonValue;

pub mod state_transition_helpers {
    use super::*;
    use crate::ProtocolError;
    use platform_value::Value;
    #[cfg(feature = "state-transition-json-conversion")]
    use std::convert::TryInto;

    #[cfg(feature = "state-transition-json-conversion")]
    pub fn to_json<'a, I: IntoIterator<Item = &'a str>>(
        serializable: impl Serialize,
        skip_signature_paths: I,
    ) -> Result<JsonValue, ProtocolError> {
        to_object(serializable, skip_signature_paths)
            .and_then(|v| v.try_into().map_err(ProtocolError::ValueError))
    }

    pub fn to_object<'a, I: IntoIterator<Item = &'a str>>(
        serializable: impl Serialize,
        skip_signature_paths: I,
    ) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(serializable)?;
        skip_signature_paths.into_iter().try_for_each(|path| {
            value
                .remove_values_matching_path(path)
                .map_err(ProtocolError::ValueError)
                .map(|_| ())
        })?;
        Ok(value)
    }

    pub fn to_cleaned_object<'a, I: IntoIterator<Item = &'a str>>(
        serializable: impl Serialize,
        skip_signature_paths: I,
    ) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(serializable)?;

        value = value.clean_recursive()?;

        skip_signature_paths.into_iter().try_for_each(|path| {
            value
                .remove_values_matching_path(path)
                .map_err(ProtocolError::ValueError)
                .map(|_| ())
        })?;
        Ok(value)
    }
}
