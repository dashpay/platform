//! Property schemas, wherever the document schema holds them (its `properties`, nested
//! objects and the contract's `$defs`), and the element schemas of typed arrays.

use super::*;

/// The regex size limit the meta-schema check read a `pattern` under, and the JSON Schema
/// validator compiled the schema's patterns under at registration.
const PATTERN_SIZE_LIMIT: usize = 5 * (1 << 20);

/// The content media type of an identifier.
pub(super) const IDENTIFIER_MEDIA_TYPE: &str = "application/x.dash.dpp.identifier";

/// The keys of a property schema.
const PROPERTY_SCHEMA_KEYS: [&str; 37] = [
    "$id",
    "$ref",
    "$comment",
    "description",
    "examples",
    "multipleOf",
    "maximum",
    "exclusiveMaximum",
    "minimum",
    "exclusiveMinimum",
    "maxLength",
    "minLength",
    "pattern",
    "maxItems",
    "minItems",
    "uniqueItems",
    "items",
    "encryptedFor",
    "refersTo",
    "contains",
    "maxProperties",
    "minProperties",
    "required",
    "additionalProperties",
    "properties",
    "dependentRequired",
    "const",
    "enum",
    "type",
    "format",
    "contentMediaType",
    "byteArray",
    "position",
    "distinctFrom",
    "maxBytes",
    "generatedFrom",
    "requiredSince",
];

/// The keys of the element schema of a typed array.
const ARRAY_ITEM_KEYS: [&str; 20] = [
    "$comment",
    "description",
    "type",
    "multipleOf",
    "maximum",
    "exclusiveMaximum",
    "minimum",
    "exclusiveMinimum",
    "maxLength",
    "minLength",
    "pattern",
    "maxItems",
    "minItems",
    "enum",
    "format",
    "contentMediaType",
    "byteArray",
    "distinctFrom",
    "maxBytes",
    "refersTo",
];

impl Checker<'_> {
    /// A map of property schemas named by words, 1 to 100 of them: a nested object's
    /// `properties`, and the contract's `$defs`.
    pub(super) fn property_map(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        for (name, schema) in map {
            self.name_matching(name, WORD_PATTERN, is_word(name))?;
            self.at(name, |c| c.property_schema(schema))?;
        }
        self.property_count(map, Some(1), Some(100))
    }

    pub(super) fn property_schema(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed_to_unevaluated(map, &PROPERTY_SCHEMA_KEYS)?;
        for (key, keyword) in map {
            self.at(key, |c| c.property_keyword(key, keyword))?;
        }
        self.property_dependencies(map)?;
        self.property_conditions(map)
    }

    fn property_keyword(&self, key: &str, value: &JsonValue) -> Checked {
        match key {
            "$id" | "$ref" => {
                let text = self.string_of_length(value, 1, None)?;
                if text.starts_with('#') {
                    Ok(())
                } else {
                    Err(self.fail_with(
                        "pattern",
                        format!("{value} does not match \"^#\""),
                        json!({ "pattern": "^#" }),
                        "",
                    ))
                }
            }
            "items" => self.array_item(value),
            "encryptedFor" => self.encrypted_for(value),
            "refersTo" => self.refers_to(value),
            "contains" => self.compiled_patterns(value),
            "const" => Ok(()),
            "additionalProperties" => {
                self.boolean(value)?;
                self.constant(value, &json!(false))
            }
            "properties" => self.property_map(value),
            "dependentRequired" => {
                let map = self.object(value)?;
                self.property_count(map, Some(1), None)?;
                for (name, names) in map {
                    self.at(name, |c| c.string_array(names))?;
                }
                Ok(())
            }
            "enum" => {
                let items = self.array(value)?;
                self.item_count(items, Some(1), None)?;
                self.unique(items)
            }
            "byteArray" => {
                self.boolean(value)?;
                self.constant(value, &json!(true))
            }
            "position" => self.integer(value, Some(0), None),
            "distinctFrom" => self.string_of_length(value, 1, Some(256)).map(drop),
            "maxBytes" => self.integer(value, Some(1), Some(65535)),
            "generatedFrom" => self.generated_from(value),
            "requiredSince" => self.integer(value, Some(1), Some(i64::from(u32::MAX))),
            "uniqueItems" => self.boolean(value),
            "required" => self.string_array(value),
            "examples" => self.array(value).map(drop),
            key => self.value_keyword(key, value),
        }
    }

    /// A schema the document meta-schema leaves unchecked, a `contains` or a system
    /// property's, whose patterns the JSON Schema validator still compiled at registration
    /// through protocol version 14, under [`PATTERN_SIZE_LIMIT`]: each `pattern` it holds must
    /// compile under that limit, down the keywords the validation schema holds a schema
    /// under (`properties`, `items` and `contains`). The validation schema's builder compiles
    /// patterns under the `regex` crate's default limit, which a contract loaded from state
    /// is compiled under, and refuses any other keyword that holds a schema. The validator
    /// compiled the schema a `$ref` reaches only when a document reached it, so a reference
    /// is not followed.
    pub(super) fn compiled_patterns(&self, value: &JsonValue) -> Checked {
        let Some(map) = value.as_object() else {
            return Ok(());
        };
        for (key, keyword) in map {
            self.at(key, |c| match key.as_str() {
                "pattern" => c.value_keyword(key, keyword),
                "items" | "contains" => c.compiled_patterns(keyword),
                "properties" => match keyword.as_object() {
                    Some(properties) => properties
                        .iter()
                        .try_for_each(|(name, schema)| c.at(name, |c| c.compiled_patterns(schema))),
                    None => Ok(()),
                },
                _ => Ok(()),
            })?;
        }
        Ok(())
    }

    /// The keywords a property schema and an array element share, as the draft 2020-12
    /// vocabularies define them.
    fn value_keyword(&self, key: &str, value: &JsonValue) -> Checked {
        match key {
            "$comment" | "description" | "format" | "contentMediaType" => {
                self.string(value).map(drop)
            }
            "multipleOf" => {
                self.number(value)?;
                let positive = value
                    .as_number()
                    .is_some_and(|number| !number_below(number, 0) && value.as_f64() != Some(0.));
                if positive {
                    Ok(())
                } else {
                    Err(self.fail_with(
                        "exclusiveMinimum",
                        format!("{value} is less than or equal to the minimum of 0"),
                        json!({ "exclusiveMinimum": 0 }),
                        "",
                    ))
                }
            }
            "maximum" | "exclusiveMaximum" | "minimum" | "exclusiveMinimum" => self.number(value),
            "maxLength" | "minLength" | "maxItems" | "minItems" | "maxProperties"
            | "minProperties" => self.non_negative_integer(value),
            "pattern" => {
                let pattern = self.string(value)?;
                if validator_regex_within(pattern, Some(PATTERN_SIZE_LIMIT)).is_some() {
                    Ok(())
                } else {
                    Err(self.fail_with(
                        "format",
                        format!("{value} is not a \"regex\""),
                        json!({ "format": "regex" }),
                        "",
                    ))
                }
            }
            "type" => self.json_type(value),
            _ => Ok(()),
        }
    }

    /// `type`: a simple type, or a list of them, no two alike.
    fn json_type(&self, value: &JsonValue) -> Checked {
        const SIMPLE_TYPES: [&str; 7] = [
            "array", "boolean", "integer", "null", "number", "object", "string",
        ];
        self.within("anyOf", value, |c| {
            if let JsonValue::Array(items) = value {
                for (index, item) in items.iter().enumerate() {
                    c.at(index, |c| c.one_of_strings(item, &SIMPLE_TYPES))?;
                }
                c.item_count(items, Some(1), None)?;
                c.unique(items)
            } else {
                c.one_of_strings(value, &SIMPLE_TYPES)
            }
        })
    }

    /// The meta-schema's `dependentSchemas` of a property schema.
    fn property_dependencies(&self, map: &JsonMap<String, JsonValue>) -> Checked {
        let has = |key: &str| map.contains_key(key);
        if has("byteArray") {
            self.member_constant(map, "type", json!("array"))?;
        }
        if has("items") {
            self.required(map, &["type"])?;
            self.member_constant(map, "type", json!("array"))?;
        }
        if map.get("contentMediaType") == Some(&json!(IDENTIFIER_MEDIA_TYPE)) {
            self.required(map, &["byteArray", "minItems", "maxItems"])?;
            self.identifier_sizes(map)?;
            self.absent(map, "uniqueItems")?;
        }
        if has("pattern") || has("format") {
            self.required(map, &["maxLength"])?;
            self.member(map, "maxLength", |c, value| {
                c.integer(value, Some(0), Some(50000))
            })?;
        }
        if has("distinctFrom") {
            self.identifier_array(map)?;
        }
        if has("maxBytes") {
            self.required(map, &["type"])?;
            self.member_constant(map, "type", json!("string"))?;
        }
        if let Some(generated_from) = map.get("generatedFrom") {
            // A `sys.tokens` function (protocol version 15, which the meta-schema predates)
            // generates an integer; every other function a string
            let generates_an_integer = generated_from
                .get("function")
                .and_then(JsonValue::as_str)
                .is_some_and(|function| function.starts_with("sys.tokens."));
            self.required(map, &["type"])?;
            self.member_constant(
                map,
                "type",
                json!(if generates_an_integer {
                    "integer"
                } else {
                    "string"
                }),
            )?;
            if has("$ref") {
                return Err(self.not_error(
                    json!({ "required": ["$ref"] }),
                    "generatedFrom is not allowed beside a $ref",
                ));
            }
        }
        if let Some(refers_to) = map.get("refersTo") {
            let refers_to = refers_to.as_object();
            // A `required` test passes on a value that is not an object
            let names = |key: &str| refers_to.is_none_or(|refers_to| refers_to.contains_key(key));
            if names("identityProperty") {
                self.required(map, &["type", "minimum", "maximum"])?;
                self.member_constant(map, "type", json!("integer"))?;
                self.member_constant(map, "minimum", json!(0))?;
                self.member_constant(map, "maximum", json!(u32::MAX))?;
            } else if names("findBy") {
                let string = map.get("type") == Some(&json!("string"));
                let byte_array = map.get("type") == Some(&json!("array"))
                    && map
                        .get("byteArray")
                        .is_some_and(|value| self.constant(value, &json!(true)).is_ok());
                if !string && !byte_array {
                    return Err(self.fail(
                        "anyOf",
                        "a refersTo with findBy is on a string or a byte array",
                    ));
                }
            } else {
                self.identifier_array(map)?;
            }
        }
        if has("encryptedFor") {
            self.required(map, &["type", "byteArray"])?;
            self.member_constant(map, "type", json!("array"))?;
            self.member_constant(map, "byteArray", json!(true))?;
            self.absent(map, "contentMediaType")?;
        }
        Ok(())
    }

    /// The meta-schema's `allOf` conditions on a property schema.
    fn property_conditions(&self, map: &JsonMap<String, JsonValue>) -> Checked {
        let json_type = map.get("type");
        if json_type == Some(&json!("object")) {
            // Each member of an object's `properties` has a `position`
            if let Some(JsonValue::Object(properties)) = map.get("properties") {
                self.at("properties", |c| {
                    for (name, member) in properties {
                        c.at(name, |c| match member.as_object() {
                            Some(member) => c.required(member, &["position"]),
                            None => Err(c.fail_with(
                                "type",
                                format!("{member} is not of type \"object\""),
                                json!({ "type": "object" }),
                                "",
                            )),
                        })?;
                    }
                    Ok(())
                })?;
            }
        }
        // As the meta-schema's condition reads, an object is also a schema without `type`
        if (json_type.is_none() || json_type == Some(&json!("object"))) && !map.contains_key("$ref")
        {
            self.required(map, &["properties", "additionalProperties"])?;
        }
        if json_type == Some(&json!("array")) {
            if map.contains_key("byteArray") {
                self.absent(map, "items")?;
            } else {
                self.absent(map, "contentMediaType")?;
                self.required(map, &["items", "maxItems"])?;
            }
        }
        Ok(())
    }

    /// `type: array`, `byteArray: true`, the identifier media type and 32 items.
    fn identifier_array(&self, map: &JsonMap<String, JsonValue>) -> Checked {
        self.required(
            map,
            &[
                "type",
                "byteArray",
                "contentMediaType",
                "minItems",
                "maxItems",
            ],
        )?;
        self.member_constant(map, "type", json!("array"))?;
        self.member_constant(map, "byteArray", json!(true))?;
        self.member_constant(map, "contentMediaType", json!(IDENTIFIER_MEDIA_TYPE))?;
        self.identifier_sizes(map)
    }

    fn identifier_sizes(&self, map: &JsonMap<String, JsonValue>) -> Checked {
        self.member_constant(map, "byteArray", json!(true))?;
        self.member_constant(map, "minItems", json!(32))?;
        self.member_constant(map, "maxItems", json!(32))
    }

    fn encrypted_for(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed(map, &["recipient", "recipientKey", "senderKey", "scheme"])?;
        self.required(map, &["recipient", "recipientKey", "senderKey", "scheme"])?;
        for name in ["recipient", "recipientKey", "senderKey"] {
            self.member(map, name, |c, value| {
                c.string_of_length(value, 1, Some(256)).map(drop)
            })?;
        }
        self.member(map, "scheme", |c, value| {
            c.one_of_strings(value, &["ecdh-secp256k1-aes256-cbc"])
        })
    }

    fn generated_from(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed(map, &["function", "params"])?;
        self.required(map, &["function", "params"])?;
        self.member(map, "function", |c, value| {
            c.string(value)?;
            c.one_of_strings(
                value,
                &[
                    "sys.stringTransformations.camelCase",
                    "sys.stringTransformations.capitalize",
                    "sys.stringTransformations.homographSafeASCII",
                    "sys.stringTransformations.lowercase",
                    "sys.stringTransformations.snakeCase",
                    "sys.stringTransformations.uppercase",
                    "sys.tokens.decimals",
                ],
            )
        })?;
        self.member(map, "params", |c, value| {
            let items = c.array(value)?;
            c.item_count(items, Some(1), Some(16))?;
            for (index, item) in items.iter().enumerate() {
                c.at(index, |c| c.string_of_length(item, 1, Some(256)).map(drop))?;
            }
            Ok(())
        })
    }

    fn array_item(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.required(map, &["type"])?;
        self.closed(map, &ARRAY_ITEM_KEYS)?;
        for (key, keyword) in map {
            self.at(key, |c| match key.as_str() {
                "type" => c.one_of_strings(
                    keyword,
                    &["integer", "number", "string", "boolean", "array"],
                ),
                "enum" => {
                    let items = c.array(keyword)?;
                    c.item_count(items, Some(1), None)?;
                    c.unique(items)
                }
                "byteArray" => {
                    c.boolean(keyword)?;
                    c.constant(keyword, &json!(true))
                }
                "distinctFrom" => c.string_of_length(keyword, 1, Some(256)).map(drop),
                "maxBytes" => c.integer(keyword, Some(1), Some(65535)),
                "refersTo" => {
                    c.refers_to(keyword)?;
                    if keyword.get("type") == Some(&json!("identityPublicKey")) {
                        return c.at("type", |c| {
                            Err(c.not_error(
                                json!({ "const": "identityPublicKey" }),
                                "an element may not refer to an identity public key",
                            ))
                        });
                    }
                    Ok(())
                }
                key => c.value_keyword(key, keyword),
            })?;
        }
        let has = |key: &str| map.contains_key(key);
        if has("refersTo") || has("distinctFrom") {
            self.identifier_array(map)?;
        }
        if has("maxBytes") {
            self.required(map, &["type"])?;
            self.member_constant(map, "type", json!("string"))?;
        }
        if has("byteArray") {
            self.member_constant(map, "type", json!("array"))?;
        }
        if map.get("contentMediaType") == Some(&json!(IDENTIFIER_MEDIA_TYPE)) {
            self.required(map, &["byteArray", "minItems", "maxItems"])?;
            self.identifier_sizes(map)?;
        }
        if has("pattern") || has("format") {
            self.required(map, &["maxLength"])?;
            self.member(map, "maxLength", |c, value| {
                c.integer(value, Some(0), Some(50000))
            })?;
        }
        let element_type = map.get("type").and_then(JsonValue::as_str);
        if element_type == Some("array") {
            self.absent(map, "enum")?;
            self.required(map, &["byteArray"])?;
        }
        if let (Some(element_type), Some(JsonValue::Array(members))) =
            (element_type, map.get("enum"))
        {
            for (index, member) in members.iter().enumerate() {
                self.at("enum", |c| {
                    c.at(index, |c| match element_type {
                        "string" => c.string(member).map(drop),
                        "integer" => c.integer(member, None, None),
                        "number" => c.number(member),
                        "boolean" => c.boolean(member),
                        _ => Ok(()),
                    })
                })?;
            }
        }
        if element_type == Some("integer") {
            for bound in ["minimum", "maximum"] {
                self.member(map, bound, |c, value| c.integer(value, None, None))?;
            }
        }
        Ok(())
    }
}
