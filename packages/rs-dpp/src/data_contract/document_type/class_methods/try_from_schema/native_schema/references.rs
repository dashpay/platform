//! `refersTo` declarations, on properties, on array elements and as `ownerRefersTo` or
//! `creatorRefersTo`, with the grammars of their `findBy`, `where` and `constraints`.

use super::*;

/// The keys of a `refersTo` declaration.
const REFERS_TO_KEYS: [&str; 14] = [
    "type",
    "anyOf",
    "allOf",
    "contractId",
    "documentType",
    "contractRequirements",
    "keyIdProperty",
    "identityProperty",
    "keyRequirements",
    "findBy",
    "where",
    "inList",
    "consume",
    "constraints",
];

impl Checker<'_> {
    /// A `refersTo` declaration: its keys, their placement and the grammar of each.
    pub(super) fn refers_to(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed(map, &REFERS_TO_KEYS)?;
        let has = |key: &str| map.contains_key(key);
        if has("anyOf") || has("allOf") {
            self.property_count(map, None, Some(1))?;
        } else {
            self.required(map, &["type"])?;
        }
        for (key, keyword) in map {
            self.at(key, |c| c.refers_to_keyword(key, keyword))?;
        }
        let target = map.get("type").and_then(JsonValue::as_str);
        if has("constraints") || has("consume") {
            self.required(map, &["findBy"])?;
            if has("inList") {
                return Err(self.not_error(
                    json!({ "required": ["inList"] }),
                    "a refersTo with constraints or consume takes no inList",
                ));
            }
        }
        let chooses_first = map
            .get("constraints")
            .and_then(JsonValue::as_object)
            .is_some_and(|constraints| constraints.contains_key("chooseFirst"));
        if has("consume") || chooses_first {
            self.required(map, &["type"])?;
            self.member_constant(map, "type", json!("deletableDocument"))?;
        }
        if matches!(
            target,
            Some("permanentDocument" | "deletableDocument" | "moderatedDocument")
        ) {
            self.required(map, &["type", "documentType"])?;
        } else {
            for name in ["contractId", "documentType", "findBy", "where"] {
                self.absent(map, name)?;
            }
        }
        if target == Some("moderatedDocument") {
            self.absent(map, "findBy")?;
        }
        if has("inList") {
            self.required(map, &["type", "findBy"])?;
            self.member_constant(map, "type", json!("permanentDocument"))?;
            self.member(map, "findBy", |c, find_by| {
                if let Some(find_by) = find_by.as_object() {
                    c.required(find_by, &["$id"])?;
                    c.property_count(find_by, None, Some(1))?;
                }
                Ok(())
            })?;
        }
        let finds_by_id = map
            .get("findBy")
            .and_then(JsonValue::as_object)
            .is_some_and(|find_by| find_by.contains_key("$id"));
        if finds_by_id {
            self.required(map, &["inList"])?;
        }
        if target == Some("identityPublicKey") {
            match (has("keyIdProperty"), has("identityProperty")) {
                (true, false) | (false, true) => {}
                _ => {
                    return Err(self.fail(
                        "oneOfNotValid",
                        "an identityPublicKey refersTo takes exactly one of keyIdProperty and identityProperty",
                    ))
                }
            }
        } else {
            for name in ["keyIdProperty", "identityProperty", "keyRequirements"] {
                self.absent(map, name)?;
            }
        }
        if target != Some("contract") {
            self.absent(map, "contractRequirements")?;
        }
        Ok(())
    }

    fn refers_to_keyword(&self, key: &str, value: &JsonValue) -> Checked {
        match key {
            "type" => self.one_of_strings(
                value,
                &[
                    "identity",
                    "contract",
                    "token",
                    "permanentDocument",
                    "identityPublicKey",
                    "deletableDocument",
                    "moderatedDocument",
                ],
            ),
            "anyOf" | "allOf" => self.reference_operands(key, value),
            "contractId" => {
                if let Some(text) = value.as_str() {
                    let length = text.chars().count();
                    let base58 = text
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() && !matches!(c, '0' | 'O' | 'I' | 'l'));
                    if (32..=44).contains(&length) && base58 {
                        return Ok(());
                    }
                } else if let Some(items) = value.as_array() {
                    let bytes = items.len() == 32
                        && items.iter().all(|item| {
                            item.as_number().is_some_and(|number| {
                                is_integer(number)
                                    && !number_below(number, 0)
                                    && !number_above(number, 255)
                            })
                        });
                    if bytes {
                        return Ok(());
                    }
                }
                Err(self.fail("oneOfNotValid", format!("{value} is not a contract id")))
            }
            "documentType" => {
                self.string_of_length(value, 1, Some(64))?;
                self.word(value)
            }
            "contractRequirements" => {
                let map = self.object(value)?;
                self.closed(
                    map,
                    &[
                        "moderation",
                        "minimumAgeSeconds",
                        "minimumSecondsSinceUpdate",
                        "owner",
                        "readonly",
                        "keepsHistory",
                        "ownerProtected",
                    ],
                )?;
                self.property_count(map, Some(1), None)?;
                for (name, requirement) in map {
                    self.at(name, |c| match name.as_str() {
                        "moderation" => c.one_of_strings(requirement, &["elected", "electionOpen"]),
                        "minimumAgeSeconds" | "minimumSecondsSinceUpdate" => {
                            c.integer(requirement, Some(1), Some(i64::from(u32::MAX)))
                        }
                        "owner" => c.one_of_strings(requirement, &["self", "other"]),
                        "readonly" | "keepsHistory" => c.constant(requirement, &json!(true)),
                        "ownerProtected" => c.boolean(requirement),
                        _ => Ok(()),
                    })?;
                }
                Ok(())
            }
            "keyIdProperty" | "inList" => self.path(value),
            "identityProperty" => {
                if matches!(value.as_str(), Some("$ownerId" | "$creatorId")) {
                    Ok(())
                } else {
                    self.within("anyOf", value, |c| c.path(value))
                }
            }
            "keyRequirements" => {
                let map = self.object(value)?;
                self.closed(map, &["purpose", "boundTo"])?;
                self.property_count(map, Some(1), None)?;
                self.member(map, "purpose", |c, purpose| {
                    c.one_of_strings(
                        purpose,
                        &[
                            "authentication",
                            "encryption",
                            "decryption",
                            "transfer",
                            "voting",
                            "owner",
                        ],
                    )
                })?;
                self.member(map, "boundTo", |c, bound_to| {
                    c.string_of_length(bound_to, 1, Some(64))?;
                    c.word(bound_to)
                })
            }
            "findBy" => {
                let map = self.object(value)?;
                self.property_count(map, Some(1), Some(10))?;
                for (name, source) in map {
                    self.name_of_length(name, 1, 256)?;
                    self.at(name, |c| c.find_by_source(source))?;
                }
                Ok(())
            }
            "where" => {
                let map = self.object(value)?;
                self.property_count(map, Some(1), Some(10))?;
                for (name, condition) in map {
                    if name.chars().count() > 256 {
                        return Err(self.name_error(
                            name,
                            "maxLength",
                            json!({ "maxLength": 256 }),
                            format!("\"{name}\" is longer than 256 characters"),
                        ));
                    }
                    if !(is_path(name)
                        || matches!(name.as_str(), "$ownerId" | "$creatorId" | "$id"))
                    {
                        return Err(self.name_error(
                            name,
                            "anyOf",
                            json!({}),
                            format!("\"{name}\" is not valid under any of the given schemas"),
                        ));
                    }
                    self.at(name, |c| c.where_condition(condition))?;
                }
                Ok(())
            }
            "consume" => self.constant(value, &json!(true)),
            "constraints" => self.reference_constraints(value),
            _ => Ok(()),
        }
    }

    /// The operands of a `refersTo` `anyOf` or `allOf`.
    fn reference_operands(&self, combinator: &str, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        self.item_count(items, Some(2), None)?;
        self.unique(items)?;
        for (index, operand) in items.iter().enumerate() {
            self.at(index, |c| {
                c.refers_to(operand)?;
                let map = c.object(operand)?;
                c.absent(map, combinator)?;
                c.absent(map, "identityProperty")?;
                c.member(map, "type", |c, target| {
                    c.one_of_strings(
                        target,
                        &["identity", "permanentDocument", "deletableDocument"],
                    )
                })?;
                if map.get("type") == Some(&json!("deletableDocument")) {
                    c.required(map, &["findBy"])?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// `ownerRefersTo` or `creatorRefersTo`: a `refersTo` declaration to an identity or a
    /// document found by `findBy`.
    pub(super) fn owner_refers_to(&self, value: &JsonValue) -> Checked {
        self.refers_to(value)?;
        let map = self.object(value)?;
        self.member(map, "type", |c, target| {
            c.one_of_strings(
                target,
                &["identity", "permanentDocument", "deletableDocument"],
            )
        })?;
        if matches!(
            map.get("type").and_then(JsonValue::as_str),
            Some("permanentDocument" | "deletableDocument")
        ) {
            self.required(map, &["findBy"])?;
        }
        Ok(())
    }

    /// Where a `findBy` entry takes its value from: `.`, `$ownerId`, a property path or a
    /// function.
    fn find_by_source(&self, value: &JsonValue) -> Checked {
        if value.is_object() {
            return self.within("anyOf", value, |c| c.find_by_function(value));
        }
        let admitted = matches!(value.as_str(), Some("." | "$ownerId"))
            || self.short_string_admitted(value, is_path);
        if admitted {
            Ok(())
        } else {
            Err(self.no_branch(value))
        }
    }

    fn find_by_function(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.closed(map, &["function", "params"])?;
        self.required(map, &["function", "params"])?;
        self.member(map, "function", |c, function| {
            c.one_of_strings(function, &["sys.hash.sha256d"])
        })?;
        self.member(map, "params", |c, params| {
            let items = c.array(params)?;
            c.item_count(items, Some(1), Some(16))?;
            for (index, param) in items.iter().enumerate() {
                c.at(index, |c| {
                    let admitted = match param {
                        JsonValue::Object(constant) => {
                            constant.len() == 1
                                && constant
                                    .get("const")
                                    .and_then(JsonValue::as_str)
                                    .is_some_and(|text| (1..=64).contains(&text.chars().count()))
                        }
                        _ => {
                            matches!(param.as_str(), Some("." | "$ownerId"))
                                || c.short_string_admitted(param, is_path)
                        }
                    };
                    if admitted {
                        Ok(())
                    } else {
                        Err(c.no_branch(param))
                    }
                })?;
            }
            Ok(())
        })
    }

    /// What a `where` entry compares the referenced value with: a property path,
    /// `$ownerId`, or a condition.
    fn where_condition(&self, value: &JsonValue) -> Checked {
        if value.is_object() {
            return self.within("anyOf", value, |c| c.where_operator(value));
        }
        if value.as_str() == Some("$ownerId") || self.short_string_admitted(value, is_path) {
            Ok(())
        } else {
            Err(self.no_branch(value))
        }
    }

    fn where_operator(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.property_count(map, Some(1), Some(1))?;
        self.closed(
            map,
            &[
                "const",
                "notEqual",
                "in",
                "greaterThan",
                "greaterThanOrEqual",
                "lessThan",
                "lessThanOrEqual",
                "contains",
                "containsAll",
            ],
        )?;
        for (key, operand) in map {
            self.at(key, |c| match key.as_str() {
                "const" => c.scalar(operand),
                "notEqual" => {
                    if operand.is_object() {
                        return c.within("anyOf", operand, |c| {
                            c.where_constant(operand, |c, constant| c.scalar(constant))
                        });
                    }
                    if c.short_string_admitted(operand, |text| text == "$ownerId" || is_path(text))
                    {
                        Ok(())
                    } else {
                        Err(c.no_branch(operand))
                    }
                }
                "in" => {
                    let items = c.array(operand)?;
                    let of_one_kind = items
                        .iter()
                        .all(|item| item.as_number().is_some_and(is_integer))
                        || items.iter().all(JsonValue::is_boolean)
                        || items.iter().all(JsonValue::is_string);
                    if !of_one_kind {
                        return Err(c.no_branch(operand));
                    }
                    c.item_count(items, Some(2), Some(30))?;
                    c.unique(items)
                }
                "greaterThan" | "greaterThanOrEqual" | "lessThan" | "lessThanOrEqual" => {
                    if operand.is_object() {
                        return c.within("anyOf", operand, |c| {
                            c.where_constant(operand, |c, constant| c.integer(constant, None, None))
                        });
                    }
                    let admitted = c.short_string_admitted(operand, |text| {
                        TIMESTAMP_PROPERTIES.contains(&text) || is_path(text)
                    });
                    if admitted {
                        Ok(())
                    } else {
                        Err(c.no_branch(operand))
                    }
                }
                "contains" => {
                    let text = c.string_of_length(operand, 1, Some(256))?;
                    if text == "$ownerId" || is_path(text) {
                        Ok(())
                    } else {
                        Err(c.pattern_error(operand, OWNER_OR_PATH_PATTERN))
                    }
                }
                "containsAll" => c.path(operand),
                _ => Ok(()),
            })?;
        }
        Ok(())
    }

    /// `{ "const": value }`, with `check` on the value.
    fn where_constant(
        &self,
        value: &JsonValue,
        check: impl FnOnce(&Checker<'_>, &JsonValue) -> Checked,
    ) -> Checked {
        let map = self.object(value)?;
        self.closed(map, &["const"])?;
        self.required(map, &["const"])?;
        self.member(map, "const", check)
    }

    /// An integer, a boolean or a string.
    fn scalar(&self, value: &JsonValue) -> Checked {
        let scalar =
            value.as_number().is_some_and(is_integer) || value.is_boolean() || value.is_string();
        if scalar {
            Ok(())
        } else {
            Err(self.type_error(value, &["integer", "boolean", "string"]))
        }
    }

    /// A `refersTo` `constraints`: `minimumAgeBlocks` and `chooseFirst`.
    fn reference_constraints(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.property_count(map, Some(1), None)?;
        self.closed(map, &["minimumAgeBlocks", "chooseFirst"])?;
        self.member(map, "minimumAgeBlocks", |c, blocks| {
            c.integer(blocks, Some(1), Some(i64::from(u32::MAX)))
        })?;
        self.member(map, "chooseFirst", |c, choice| {
            let choice = c.object(choice)?;
            c.closed(choice, &["among", "validWhen", "by"])?;
            c.required(choice, &["among", "validWhen", "by"])?;
            c.member(choice, "among", |c, among| c.path(among))?;
            c.member(choice, "validWhen", |c, valid_when| {
                let map = c.object(valid_when)?;
                c.property_count(map, Some(1), Some(1))?;
                for (name, function) in map {
                    c.name_of_length(name, 1, 256)?;
                    c.name_matching(name, PATH_PATTERN, is_path(name))?;
                    c.at(name, |c| c.find_by_function(function))?;
                }
                Ok(())
            })?;
            c.member(choice, "by", |c, by| {
                let items = c.array(by)?;
                c.item_count(items, Some(1), Some(10))?;
                for (index, order) in items.iter().enumerate() {
                    c.at(index, |c| {
                        let admitted = match order {
                            JsonValue::Object(map) => {
                                map.len() == 1
                                    && map.iter().all(|(name, direction)| {
                                        (1..=256).contains(&name.chars().count())
                                            && matches!(direction.as_str(), Some("asc" | "desc"))
                                    })
                            }
                            _ => order
                                .as_str()
                                .is_some_and(|text| (1..=256).contains(&text.chars().count())),
                        };
                        if admitted {
                            Ok(())
                        } else {
                            Err(c.no_branch(order))
                        }
                    })?;
                }
                Ok(())
            })
        })
    }
}
