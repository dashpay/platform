//! The rule language of `propertyConstraints` and `deleteConstraints`.

use super::*;

/// The conditions of a property constraint rule.
const CONSTRAINT_CONDITIONS: [&str; 18] = [
    "equal",
    "notEqual",
    "lessThan",
    "lessThanOrEqual",
    "greaterThan",
    "greaterThanOrEqual",
    "in",
    "notIn",
    "startsWith",
    "endsWith",
    "contains",
    "present",
    "absent",
    "anyOf",
    "allOf",
    "not",
    "ifThen",
    "ifThenElse",
];

/// The operations of a property constraint expression.
const CONSTRAINT_OPERATIONS: [&str; 19] = [
    "referenced",
    "ifAbsent",
    "add",
    "multiply",
    "subtract",
    "divide",
    "modulo",
    "power",
    "min",
    "max",
    "abs",
    "length",
    "byteLength",
    "count",
    "countPresent",
    "byteAt",
    "countOf",
    "sumOf",
    "const",
];

impl Checker<'_> {
    /// A `propertyConstraints` or `deleteConstraints` rule: one condition.
    pub(super) fn constraint(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.property_count(map, Some(1), Some(1))?;
        self.closed(map, &CONSTRAINT_CONDITIONS)?;
        for (key, operand) in map {
            self.at(key, |c| match key.as_str() {
                "equal" | "notEqual" | "lessThan" | "lessThanOrEqual" | "greaterThan"
                | "greaterThanOrEqual" | "startsWith" | "endsWith" => c.expression_pair(operand),
                "in" | "notIn" => c.tuple(operand, 2, 2, |c, index, item| {
                    if index == 0 {
                        return c.expression(item);
                    }
                    let members = c.array(item)?;
                    let of_one_kind = members
                        .iter()
                        .all(|member| member.as_number().is_some_and(is_integer))
                        || members.iter().all(JsonValue::is_string);
                    if !of_one_kind {
                        return Err(c.no_branch(item));
                    }
                    c.item_count(members, Some(2), None)?;
                    c.unique(members)
                }),
                "contains" => c.tuple(operand, 2, 2, |c, index, item| {
                    if index == 0 {
                        c.constraint_path(item)
                    } else {
                        c.expression(item)
                    }
                }),
                "present" | "absent" => c.constraint_path(operand),
                "anyOf" | "allOf" => {
                    let items = c.array(operand)?;
                    for (index, item) in items.iter().enumerate() {
                        c.at(index, |c| {
                            c.constraint(item)?;
                            match item.as_object() {
                                Some(item) => c.absent(item, key),
                                None => Ok(()),
                            }
                        })?;
                    }
                    c.item_count(items, Some(2), None)?;
                    c.unique(items)
                }
                "not" => {
                    c.constraint(operand)?;
                    match operand.as_object() {
                        Some(negated) => {
                            c.absent(negated, "not")?;
                            c.absent(negated, "notIn")
                        }
                        None => Ok(()),
                    }
                }
                "ifThen" => c.tuple(operand, 2, 2, |c, _, item| c.constraint(item)),
                "ifThenElse" => c.tuple(operand, 3, 3, |c, _, item| c.constraint(item)),
                _ => Ok(()),
            })?;
        }
        Ok(())
    }

    /// An array of `minimum` to `maximum` items, `check` run on each with its index.
    fn tuple(
        &self,
        value: &JsonValue,
        minimum: usize,
        maximum: usize,
        check: impl Fn(&Checker<'_>, usize, &JsonValue) -> Checked,
    ) -> Checked {
        let items = self.array(value)?;
        for (index, item) in items.iter().enumerate().take(maximum) {
            self.at(index, |c| check(c, index, item))?;
        }
        if items.len() > maximum {
            return self.at(maximum, |c| {
                Err(c.fail("falseSchema", "no item is allowed here"))
            });
        }
        self.item_count(items, Some(minimum), None)
    }

    /// Exactly two expressions.
    fn expression_pair(&self, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        for (index, item) in items.iter().enumerate() {
            self.at(index, |c| c.expression(item))?;
        }
        self.item_count(items, Some(2), Some(2))
    }

    /// Two or more expressions.
    fn expressions(&self, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        for (index, item) in items.iter().enumerate() {
            self.at(index, |c| c.expression(item))?;
        }
        self.item_count(items, Some(2), None)
    }

    /// A value a rule computes: an integer, a path, or an operation.
    fn expression(&self, value: &JsonValue) -> Checked {
        match value {
            JsonValue::Number(number) if is_integer(number) => Ok(()),
            JsonValue::String(_) => self.constraint_path(value),
            JsonValue::Object(map) => {
                self.property_count(map, Some(1), Some(1))?;
                self.closed(map, &CONSTRAINT_OPERATIONS)?;
                for (key, operand) in map {
                    self.at(key, |c| match key.as_str() {
                        "referenced" => c.referenced(operand),
                        "ifAbsent" => c.tuple(operand, 2, 2, |c, index, item| {
                            if index == 0 {
                                return c.constraint_path(item);
                            }
                            if item.as_number().is_some_and(is_integer) || item.is_string() {
                                Ok(())
                            } else {
                                Err(c.type_error(item, &["integer", "string"]))
                            }
                        }),
                        "add" | "multiply" | "min" | "max" => c.expressions(operand),
                        "subtract" | "divide" | "modulo" | "power" => c.expression_pair(operand),
                        "abs" => c.expression(operand),
                        "length" | "byteLength" | "count" => c.constraint_path(operand),
                        "countPresent" => {
                            let items = c.array(operand)?;
                            for (index, item) in items.iter().enumerate() {
                                c.at(index, |c| c.constraint_path(item))?;
                            }
                            c.item_count(items, Some(2), None)?;
                            c.unique(items)
                        }
                        "byteAt" => c.tuple(operand, 2, 2, |c, index, item| {
                            if index == 0 {
                                c.constraint_path(item)
                            } else {
                                c.integer(item, Some(0), Some(65535))
                            }
                        }),
                        "countOf" => c.tuple(operand, 1, 2, |c, index, item| {
                            if index == 0 {
                                c.string_of_length(item, 1, Some(64)).map(drop)
                            } else {
                                c.aggregate_filter(item)
                            }
                        }),
                        "sumOf" => c.tuple(operand, 2, 3, |c, index, item| match index {
                            0 => c.string_of_length(item, 1, Some(64)).map(drop),
                            1 => {
                                let text = c.string(item)?;
                                if is_path(text) {
                                    Ok(())
                                } else {
                                    Err(c.pattern_error(item, PATH_PATTERN))
                                }
                            }
                            _ => c.aggregate_filter(item),
                        }),
                        "const" => c.string(operand).map(drop),
                        _ => Ok(()),
                    })?;
                }
                Ok(())
            }
            _ => Err(self.type_error(value, &["integer", "string", "object"])),
        }
    }

    /// A property a rule reads: a path of the document, maybe of its stored version, or a
    /// property of the document a reference finds.
    fn constraint_path(&self, value: &JsonValue) -> Checked {
        match value {
            JsonValue::String(text) => {
                let read = text.strip_prefix("$old.").unwrap_or(text);
                let admitted = matches!(read, "$ownerId" | "$revision")
                    || TIMESTAMP_PROPERTIES.contains(&read)
                    || is_path(read);
                if admitted {
                    Ok(())
                } else {
                    Err(self.no_branch(value))
                }
            }
            JsonValue::Object(map) => self.within("anyOf", value, |c| {
                c.closed(map, &["referenced"])?;
                c.required(map, &["referenced"])?;
                c.member(map, "referenced", |c, referenced| c.referenced(referenced))
            }),
            _ => Err(self.no_branch(value)),
        }
    }

    /// `[reference, property]`: a property of the document the reference finds.
    fn referenced(&self, value: &JsonValue) -> Checked {
        self.tuple(value, 2, 2, |c, index, item| {
            let text = c.string(item)?;
            let admitted = if index == 0 {
                is_path(text)
            } else {
                matches!(text, "$id" | "$ownerId" | "$creatorId")
                    || TIMESTAMP_PROPERTIES.contains(&text)
                    || is_path(text)
            };
            match (admitted, index) {
                (true, _) => Ok(()),
                (false, 0) => Err(c.pattern_error(item, PATH_PATTERN)),
                (false, _) => Err(c.pattern_error(item, REFERENCED_PROPERTY_PATTERN)),
            }
        })
    }

    /// The filter of a `countOf` or `sumOf`: property paths or `$ownerId`, each to a path,
    /// `$ownerId`, `$id`, an integer or a constant.
    fn aggregate_filter(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.property_count(map, Some(1), None)?;
        for (name, filter) in map {
            self.name_matching(
                name,
                OWNER_OR_PATH_PATTERN,
                name == "$ownerId" || is_path(name),
            )?;
            self.at(name, |c| match filter {
                JsonValue::Number(number) if is_integer(number) => Ok(()),
                JsonValue::String(text) => {
                    if matches!(text.as_str(), "$ownerId" | "$id") || is_path(text) {
                        Ok(())
                    } else {
                        Err(c.pattern_error(filter, OWNER_ID_OR_PATH_PATTERN))
                    }
                }
                JsonValue::Object(constant) => {
                    c.closed(constant, &["const"])?;
                    c.required(constant, &["const"])?;
                    c.member(constant, "const", |c, text| c.string(text).map(drop))
                }
                _ => Err(c.type_error(filter, &["integer", "string", "object"])),
            })?;
        }
        Ok(())
    }
}
