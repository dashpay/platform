use std::borrow::Cow;

use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::paths::{JSONPointer, JsonPointerNode};
use jsonschema::primitive_type::PrimitiveType;
use jsonschema::{ErrorIterator, Keyword, ValidationError};
use serde_json::Value as JsonValue;
use std::iter::once;

pub struct ByteArrayKeyword;

impl Keyword for ByteArrayKeyword {
    fn validate<'instance>(
        &self,
        instance: &'instance JsonValue,
        instance_path: &JsonPointerNode,
    ) -> ErrorIterator<'instance> {
        // Make sure it's an array
        if !instance.is_array() {
            let error = ValidationError {
                instance_path: instance_path.into(),
                schema_path: JSONPointer::default(),
                kind: ValidationErrorKind::Type {
                    kind: TypeKind::Single(PrimitiveType::Array),
                },
                instance: Cow::Borrowed(instance),
            };

            return Box::new(once(error));
        }

        // Make sure it's an array of bytes
        let bytes = instance
            .as_array()
            .expect("instance must be array and verified above");

        for (i, value) in bytes.iter().enumerate() {
            match value.as_u64() {
                Some(byte) if byte > u8::MAX as u64 => {
                    let error = ValidationError {
                        instance_path: instance_path.push(i).into(),
                        schema_path: JSONPointer::default(),
                        kind: ValidationErrorKind::Maximum {
                            limit: u8::MAX.into(),
                        },
                        instance: Cow::Borrowed(value),
                    };

                    return Box::new(once(error));
                }
                None => {
                    let error = ValidationError {
                        instance_path: instance_path.push(i).into(),
                        schema_path: JSONPointer::default(),
                        kind: ValidationErrorKind::Type {
                            kind: TypeKind::Single(PrimitiveType::Integer),
                        },
                        instance: Cow::Borrowed(value),
                    };

                    return Box::new(once(error));
                }
                Some(_) => {}
            }
        }

        Box::new(None.into_iter())
    }
    fn is_valid(&self, _instance: &JsonValue) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod validate {
        use super::*;
        use assert_matches::assert_matches;
        use jsonschema::error::ValidationErrorKind;
        use jsonschema::paths::JSONPointer;
        use std::borrow::Cow;

        #[test]
        fn should_return_error_when_instance_is_not_an_array() {
            let instance: JsonValue = json!({});
            let instance_path = JsonPointerNode::default();

            let keyword = ByteArrayKeyword {};

            let errors = keyword.validate(&instance, &instance_path);

            assert_matches!(
                errors.collect::<Vec<_>>().as_slice(),
                [ValidationError {
                    kind: ValidationErrorKind::Type {
                        kind: TypeKind::Single(PrimitiveType::Array)
                    },
                    instance_path: actual_instance_path,
                    schema_path: actual_schema_path,
                    instance: actual_instance,
                    ..
                }] if *actual_instance_path == instance_path.into()
                    && actual_schema_path == &JSONPointer::default()
                    && actual_instance == &Cow::Borrowed(&instance)
            );
        }

        #[test]
        fn should_return_error_when_array_item_is_not_an_integer() {
            let instance: JsonValue = json!([1, "a"]);
            let instance_path = JsonPointerNode::default();

            let keyword = ByteArrayKeyword {};

            let errors = keyword.validate(&instance, &instance_path);

            assert_matches!(
                errors.collect::<Vec<_>>().as_slice(),
                [ValidationError {
                    kind: ValidationErrorKind::Type {
                        kind: TypeKind::Single(PrimitiveType::Integer)
                    },
                    instance_path: actual_instance_path,
                    schema_path: actual_schema_path,
                    instance: actual_instance,
                    ..
                }] if *actual_instance_path == instance_path.push(1).into()
                    && actual_schema_path == &JSONPointer::default()
                    && actual_instance == &Cow::<JsonValue>::Owned(JsonValue::from("a"))
            );
        }

        #[test]
        fn should_return_error_when_array_item_is_bigger_than_255() {
            let instance: JsonValue = json!([1, 500]);
            let instance_path = JsonPointerNode::default();

            let keyword = ByteArrayKeyword {};

            let errors = keyword.validate(&instance, &instance_path);

            assert_matches!(
                errors.collect::<Vec<_>>().as_slice(),
                [ValidationError {
                    kind: ValidationErrorKind::Maximum { limit },
                    instance_path: actual_instance_path,
                    schema_path: actual_schema_path,
                    instance: actual_instance,
                    ..
                }] if *actual_instance_path == instance_path.push(1).into()
                    && actual_schema_path == &JSONPointer::default()
                    && actual_instance == &Cow::<JsonValue>::Owned(JsonValue::from(500))
                    && limit == &JsonValue::from(u8::MAX)
            );
        }
    }
}
