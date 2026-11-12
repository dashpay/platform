//! Client-side checks of the values an indexOnly delete is built from, run
//! before a transition is signed. Platform's own check of the same values is
//! [`DataContract::validate_document_properties`](crate::data_contract::DataContract)
//! in the delete's structure validation; these cover only what can be told
//! from the schema without full value-shape validation, so a client is not
//! charged for a delete Platform will refuse.

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::{
    DocumentProperty, DocumentPropertyType, DocumentTypeRef,
};
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// The path of the first value the document type does not declare, looking
/// inside declared objects; `None` when every value is declared. A `$` name
/// is never declared. A value of the wrong kind for its property (a map
/// where the schema has no object, or the other way round) is not reported.
pub fn undeclared_value_path(
    document_type: DocumentTypeRef,
    values: &BTreeMap<String, Value>,
) -> Option<String> {
    undeclared_property(
        &mut values.iter().map(|(name, value)| (name.as_str(), value)),
        &|name| document_type.properties().get(name),
    )
}

/// The first required property path the values leave out, looked up by path
/// and with every `generatedFrom` property generated the way the delete
/// transition generates it; `None` when none is missing. `$` fields are not
/// looked for.
pub fn missing_required_value_path(
    document_type: DocumentTypeRef,
    values: &BTreeMap<String, Value>,
    platform_version: &PlatformVersion,
) -> Result<Option<String>, ProtocolError> {
    let mut carried = values.clone();
    document_type.regenerate_generated_properties(&mut carried, platform_version)?;
    let carried = Value::from(carried);
    Ok(document_type
        .required_fields()
        .iter()
        .find(|path| {
            !path.starts_with('$') && matches!(carried.get_optional_value_at_path(path), Ok(None))
        })
        .cloned())
}

fn undeclared_property<'a>(
    values: &mut dyn Iterator<Item = (&'a str, &'a Value)>,
    declared: &dyn Fn(&str) -> Option<&'a DocumentProperty>,
) -> Option<String> {
    for (name, value) in values {
        let Some(property) = declared(name).filter(|_| !name.starts_with('$')) else {
            return Some(name.to_string());
        };
        let (DocumentPropertyType::Object(members), Value::Map(entries)) =
            (&property.property_type, value)
        else {
            continue;
        };
        let mut nested = entries
            .iter()
            .filter_map(|(key, value)| key.as_text().map(|key| (key, value)));
        if let Some(path) = undeclared_property(&mut nested, &|name| members.get(name)) {
            return Some(format!("{name}.{path}"));
        }
    }
    None
}

#[cfg(all(test, feature = "app-connect-contract"))]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contracts::SystemDataContract;
    use crate::system_data_contracts::load_system_data_contract;

    fn login_key_response_values() -> BTreeMap<String, Value> {
        BTreeMap::from([
            (
                "appEphemeralPubKeyHash".to_string(),
                Value::Bytes20([0x11; 20]),
            ),
            (
                "walletEphemeralPubKey".to_string(),
                Value::Bytes(vec![0x02; 33]),
            ),
            ("encryptedPayload".to_string(), Value::Bytes(vec![0x33; 60])),
        ])
    }

    #[test]
    fn should_report_the_first_undeclared_value_and_missing_required_path() {
        let contract =
            load_system_data_contract(SystemDataContract::AppConnect, PlatformVersion::latest())
                .expect("app-connect contract loads");
        let document_type = contract
            .document_type_for_name("loginKeyResponse")
            .expect("loginKeyResponse type");
        let values = login_key_response_values();

        assert_eq!(undeclared_value_path(document_type, &values), None);
        assert_eq!(
            missing_required_value_path(document_type, &values, PlatformVersion::latest())
                .expect("generation runs"),
            None
        );

        let mut extra = values.clone();
        extra.insert("contractId".to_string(), Value::Bool(true));
        assert_eq!(
            undeclared_value_path(document_type, &extra),
            Some("contractId".to_string())
        );
        let mut system = values.clone();
        system.insert("$ownerId".to_string(), Value::Identifier([7; 32]));
        assert_eq!(
            undeclared_value_path(document_type, &system),
            Some("$ownerId".to_string())
        );

        let mut partial = values;
        partial.remove("encryptedPayload");
        assert_eq!(
            missing_required_value_path(document_type, &partial, PlatformVersion::latest())
                .expect("generation runs"),
            Some("encryptedPayload".to_string())
        );
    }
}
