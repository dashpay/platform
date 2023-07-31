fn enrich_with_base_schema(
    schema: &JsonSchema,
    exclude_properties: &[&str], // TODO: Do we need this?
) -> Result<JsonSchema, ProtocolError> {
    let cloned_schema = schema.clone();

    let base_properties = BASE_DOCUMENT_SCHEMA
        .get_schema_properties()?
        .as_object()
        .ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::JSONSchema(
                JSONSchemaError::CreateSchemaError("'properties' is not a map"),
            ))
        })?;

    let base_required = BASE_DOCUMENT_SCHEMA.get_schema_required_fields()?;

    for (_, cloned_document) in cloned_schema.documents.iter_mut() {
        if let Some(JsonValue::Object(ref mut properties)) =
            cloned_document.get_mut(PROPERTY_PROPERTIES)
        {
            properties.extend(
                base_properties
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
            );
        }

        if let Some(JsonValue::Array(ref mut required)) = cloned_document.get_mut(PROPERTY_REQUIRED)
        {
            required.extend(
                base_required
                    .iter()
                    .map(|v| JsonValue::String(v.to_string())),
            );
            required.retain(|p| {
                if let JsonValue::String(v) = p {
                    return !exclude_properties.contains(&v.as_str());
                }
                true
            });
        }
    }

    Ok(cloned_schema)
}
