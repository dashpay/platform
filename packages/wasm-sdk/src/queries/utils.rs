use dash_sdk::dpp::platform_value::{self, Value as PlatformValue};
use js_sys::Reflect;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_dpp2::serialization::conversions::{
    from_object, js_value_to_platform_value, platform_value_from_object,
};

use crate::WasmSdkError;
use dash_sdk::platform::Identifier;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::utils::IntoWasm;

/// The contract a query names, from any `IdentifierLike`.
pub(crate) fn parse_contract_id(id: IdentifierLikeJs) -> Result<Identifier, WasmSdkError> {
    id.try_into()
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract id: {err}")))
}

pub(crate) fn deserialize_required_query<T, Q>(
    query: Q,
    missing_error: &str,
    context: &str,
) -> Result<T, WasmSdkError>
where
    T: DeserializeOwned,
    Q: Into<JsValue>,
{
    let value = query.into();

    if value.is_null() || value.is_undefined() {
        return Err(WasmSdkError::invalid_argument(missing_error.to_string()));
    }

    from_object(value)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {}: {}", context, e)))
}

fn normalize_identifier(value: &mut PlatformValue, original: &JsValue) {
    if matches!(value, PlatformValue::Map(_)) {
        if let Ok(identifier) = original.to_wasm::<IdentifierWasm>("Identifier") {
            *value = PlatformValue::Identifier(identifier.to_slice());
        }
    }
}

fn normalize_identifier_path(value: &mut PlatformValue, original: &JsValue, path: &[&str]) {
    let Some((field, remaining)) = path.split_first() else {
        return;
    };
    let PlatformValue::Map(entries) = value else {
        return;
    };
    let Some((_, node)) = entries
        .iter_mut()
        .find(|(key, _)| matches!(key, PlatformValue::Text(text) if text.as_str() == *field))
    else {
        return;
    };
    let Ok(original) = Reflect::get(original, &JsValue::from_str(field)) else {
        return;
    };
    if remaining.is_empty() {
        normalize_identifier(node, &original);
    } else {
        normalize_identifier_path(node, &original, remaining);
    }
}

pub(crate) fn deserialize_required_query_with_identifiers<T, Q>(
    query: Q,
    missing_error: &str,
    context: &str,
    paths: &[&[&str]],
    array_fields: &[&str],
) -> Result<T, WasmSdkError>
where
    T: DeserializeOwned,
    Q: Into<JsValue>,
{
    let original = query.into();
    if original.is_null() || original.is_undefined() {
        return Err(WasmSdkError::invalid_argument(missing_error.to_string()));
    }

    // Preserve whole-object conversion and its errors before inspecting named fields.
    let mut value = platform_value_from_object(&original)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {context}: {e}")))?;
    for path in paths {
        normalize_identifier_path(&mut value, &original, path);
    }
    if let PlatformValue::Map(entries) = &mut value {
        for field in array_fields {
            let Some((_, PlatformValue::Array(elements))) = entries.iter_mut().find(
                |(key, _)| matches!(key, PlatformValue::Text(text) if text.as_str() == *field),
            ) else {
                continue;
            };
            let Ok(array) = Reflect::get(&original, &JsValue::from_str(field)) else {
                continue;
            };
            for (index, element) in elements.iter_mut().enumerate() {
                if let Ok(original) = Reflect::get(&array, &JsValue::from_f64(index as f64)) {
                    normalize_identifier(element, &original);
                }
            }
        }
    }
    platform_value::from_value(value)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {context}: fromObject: {e}")))
}

pub(crate) fn deserialize_query_with_default<T, Q>(
    query: Option<Q>,
    context: &str,
) -> Result<T, WasmSdkError>
where
    T: Default + DeserializeOwned,
    Q: Into<JsValue>,
{
    let value = query.map(Into::into).unwrap_or_else(|| JsValue::UNDEFINED);

    if value.is_null() || value.is_undefined() {
        return Ok(T::default());
    }

    from_object(value)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {}: {}", context, e)))
}

pub(crate) fn convert_optional_limit(
    limit: Option<u32>,
    field: &str,
) -> Result<Option<u16>, WasmSdkError> {
    match limit {
        Some(0) => Ok(None),
        Some(value) => {
            if value > u16::MAX as u32 {
                Err(WasmSdkError::invalid_argument(format!(
                    "{} {} exceeds maximum of {}",
                    field,
                    value,
                    u16::MAX
                )))
            } else {
                Ok(Some(value as u16))
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn convert_json_values_to_platform_values(
    values: Option<Vec<JsonValue>>,
    field_name: &str,
) -> Result<Vec<PlatformValue>, WasmSdkError> {
    values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            // Use json_compatible() to ensure objects become plain JS objects (not Maps)
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            let js_value = value.serialize(&serializer).map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid {} entry: {}", field_name, err))
            })?;
            js_value_to_platform_value(&js_value).map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid {} entry: {}", field_name, err))
            })
        })
        .collect()
}

#[cfg(all(test, target_arch = "wasm32"))]
mod common_wasm_instance_tests {
    use super::*;
    use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Setters};
    use js_sys::{Map, Object};
    use std::cell::Cell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use wasm_dpp2::asset_lock_proof::outpoint::OutPointWasm;
    use wasm_dpp2::data_contract::DocumentWasm;
    use wasm_dpp2::tokens::configuration::group::GroupWasm;
    use wasm_dpp2::utils::try_from_options_mut;

    fn throwing_getter(
        value: &JsValue,
        property: &str,
        reads: Rc<Cell<u32>>,
    ) -> Closure<dyn FnMut() -> JsValue> {
        let getter = Closure::wrap(Box::new(move || -> JsValue {
            reads.set(reads.get() + 1);
            wasm_bindgen::throw_str("untrusted instance data must not be read");
        }) as Box<dyn FnMut() -> JsValue>);
        let descriptor = Object::new();
        Reflect::set(&descriptor, &"get".into(), getter.as_ref()).expect("getter descriptor");
        assert!(Reflect::define_property(
            value.dyn_ref::<Object>().expect("getter target object"),
            &property.into(),
            &descriptor,
        )
        .expect("nonenumerable getter"));
        getter
    }

    #[wasm_bindgen_test]
    fn should_reject_plain_identifier_marker_before_reading_its_pointer() {
        let value: JsValue = Object::new().into();
        Reflect::set(&value, &"__type".into(), &"Identifier".into()).expect("type marker");
        let reads = Rc::new(Cell::new(0));
        let _getter = throwing_getter(&value, "__wbg_ptr", reads.clone());

        assert!(value.to_wasm::<IdentifierWasm>("Identifier").is_err());
        assert_eq!(reads.get(), 0);
    }

    #[wasm_bindgen_test]
    fn should_reject_local_constructor_impostor_before_reading_instance_data() {
        let local: JsValue = IdentifierWasm::from([0x11; 32]).into();
        let value: JsValue = Object::new().into();
        let constructor = Reflect::get(&local, &"constructor".into()).expect("local constructor");
        Reflect::set(&value, &"constructor".into(), &constructor).expect("constructor reference");
        let type_reads = Rc::new(Cell::new(0));
        let pointer_reads = Rc::new(Cell::new(0));
        let _type_getter = throwing_getter(&value, "__type", type_reads.clone());
        let _pointer_getter = throwing_getter(&value, "__wbg_ptr", pointer_reads.clone());

        assert!(value.to_wasm::<IdentifierWasm>("Identifier").is_err());
        assert_eq!(type_reads.get(), 0);
        assert_eq!(pointer_reads.get(), 0);
    }

    #[wasm_bindgen_test]
    fn should_borrow_local_identifier_repeatedly_and_recover_after_a_freed_wrapper() {
        let value: JsValue = IdentifierWasm::from([0x11; 32]).into();
        for _ in 0..2 {
            let identifier = value
                .to_wasm::<IdentifierWasm>("Identifier")
                .expect("local borrow");
            assert_eq!(identifier.to_slice(), [0x11; 32]);
        }
        let free = Reflect::get(&value, &"free".into())
            .expect("generated free")
            .dyn_into::<js_sys::Function>()
            .expect("free function");
        free.call0(&value).expect("free local wrapper");
        assert!(value.to_wasm::<IdentifierWasm>("Identifier").is_err());

        let next: JsValue = IdentifierWasm::from([0x22; 32]).into();
        assert_eq!(
            next.to_wasm::<IdentifierWasm>("Identifier")
                .expect("later local borrow")
                .to_slice(),
            [0x22; 32]
        );
    }

    #[wasm_bindgen_test]
    fn should_preserve_outpoint_and_class_only_group_conversion() {
        let outpoint: JsValue = OutPointWasm::constructor("11".repeat(32), 7)
            .expect("outpoint")
            .into();
        assert_eq!(
            OutPointWasm::try_from(&outpoint)
                .expect("outpoint conversion")
                .vout(),
            7
        );
        let group: JsValue = GroupWasm::constructor(Map::new().into(), 3)
            .expect("group")
            .into();
        assert_eq!(
            GroupWasm::try_from(&group)
                .expect("class-only conversion")
                .required_power(),
            3
        );
    }

    #[wasm_bindgen_test]
    fn should_mutate_the_original_local_document_through_the_common_gateway() {
        let document: JsValue = DocumentWasm::new(
            Document::V0(DocumentV0::default()),
            Identifier::new([0x11; 32]),
            "test".to_string(),
            None,
        )
        .into();
        let options: JsValue = Object::new().into();
        Reflect::set(&options, &"document".into(), &document).expect("document option");
        {
            let mut borrowed =
                try_from_options_mut::<DocumentWasm>(&options, "document", "Document")
                    .expect("mutable local document");
            borrowed.inner_mut().set_id(Identifier::new([0x22; 32]));
        }
        assert_eq!(
            document
                .to_wasm::<DocumentWasm>("Document")
                .expect("original document")
                .id()
                .to_slice(),
            [0x22; 32]
        );
    }
}
