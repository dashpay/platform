use platform_value::Value;
use platform_version::version::PlatformVersion;

use crate::consensus::basic::value_error::ValueError;
use crate::consensus::basic::BasicError;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub type SubValidator = fn(
    path: &str,
    key: &str,
    parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError>;

#[inline(always)]
pub(super) fn traversal_validator_v0(
    raw_data_contract: &Value,
    validators: &[SubValidator],
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();
    let mut values_queue: Vec<(&Value, String)> = vec![(raw_data_contract, String::from(""))];

    while let Some((value, path)) = values_queue.pop() {
        match value {
            Value::Map(current_map) => {
                for (key, current_value) in current_map.iter() {
                    if current_value.is_map() || current_value.is_array() {
                        let new_path =
                            format!("{}/{}", path, key.non_qualified_string_representation());
                        values_queue.push((current_value, new_path))
                    }
                    match key
                        .to_str()
                        .map_err(|err| BasicError::ValueError(ValueError::new(err)))
                    {
                        Ok(key) => {
                            for validator in validators {
                                validator(
                                    &path,
                                    key,
                                    value,
                                    current_value,
                                    &mut result,
                                    platform_version,
                                )?;
                            }
                        }
                        Err(err) => result.add_error(err),
                    }
                }
            }
            Value::Array(arr) => {
                for (i, value) in arr.iter().enumerate() {
                    if value.is_map() {
                        let new_path = format!("{}/[{}]", path, i);
                        values_queue.push((value, new_path))
                    }
                }
            }
            _ => {}
        };
    }
    Ok(result)
}
