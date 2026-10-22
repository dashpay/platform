//! The `indices` of a document schema. An index property name is held to the 255 bytes of a
//! GroveDB key, where the meta-schema admitted 256 characters.

use super::*;

/// The longest key GroveDB stores, and so the longest index property name.
const MAX_INDEX_PROPERTY_NAME_BYTES: usize = 255;

/// The keys of an index.
const INDEX_KEYS: [&str; 21] = [
    "name",
    "properties",
    "unique",
    "nullSearchable",
    "contested",
    "countable",
    "rangeCountable",
    "summable",
    "rangeSummable",
    "averageable",
    "rangeAverageable",
    "rankedCountable",
    "rankedSummable",
    "rankedAverageable",
    "timeRange",
    "integerRange",
    "terminal",
    "preallocated",
    "outlivesDelete",
    "skipIfAbsent",
    "summableOffCountIndex",
];

impl Checker<'_> {
    pub(super) fn indices(&self, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        self.item_count(items, Some(1), Some(10))?;
        for (index, item) in items.iter().enumerate() {
            self.at(index, |c| c.index(item))?;
        }
        Ok(())
    }

    fn index(&self, value: &JsonValue) -> Checked {
        let map = self.object(value)?;
        self.required(map, &["name"])?;
        self.closed(map, &INDEX_KEYS)?;
        for (key, keyword) in map {
            self.at(key, |c| c.index_keyword(key, keyword))?;
        }
        let has = |key: &str| map.contains_key(key);
        if has("rangeAverageable") {
            self.required(map, &["averageable"])?;
        }
        if has("rangeSummable") && !has("averageable") && !has("summableOffCountIndex") {
            self.required(map, &["summable"])?;
        }
        let ranked = |key: &str| {
            map.get(key)
                .is_some_and(|value| value == &json!(true) || value.is_object())
        };
        if ranked("rankedCountable") && !has("rangeAverageable") && !has("summableOffCountIndex") {
            self.required(map, &["rangeCountable"])?;
        }
        if ranked("rankedSummable") && !has("rangeAverageable") {
            self.required(map, &["rangeSummable"])?;
        }
        if ranked("rankedAverageable") && !(has("rangeCountable") && has("rangeSummable")) {
            self.required(map, &["rangeAverageable"])?;
        }
        let ranked_object = |key: &str| map.get(key).is_some_and(JsonValue::is_object);
        if ranked_object("rankedSummable") || ranked_object("rankedAverageable") {
            self.required(map, &["summableOffCountIndex"])?;
        }
        if has("summableOffCountIndex") {
            self.required(map, &["rangeSummable"])?;
            self.member_constant(map, "rangeSummable", json!(true))?;
            if has("terminal") || has("summable") || has("averageable") {
                return Err(self.fail(
                    "not",
                    "a summableOffCountIndex index has no terminal, summable or averageable",
                ));
            }
        }
        Ok(())
    }

    fn index_keyword(&self, key: &str, value: &JsonValue) -> Checked {
        match key {
            "name" => self.string_of_length(value, 1, Some(32)).map(drop),
            "properties" => {
                let items = self.array(value)?;
                self.item_count(items, Some(1), Some(10))?;
                for (index, item) in items.iter().enumerate() {
                    self.at(index, |c| {
                        let property = c.object(item)?;
                        c.property_count(property, Some(1), Some(1))?;
                        for (name, order) in property {
                            if name.len() > MAX_INDEX_PROPERTY_NAME_BYTES {
                                return Err(c.name_error(
                                    name,
                                    "maxBytes",
                                    json!({ "maxBytes": MAX_INDEX_PROPERTY_NAME_BYTES }),
                                    format!(
                                        "the index property name \"{name}\" is longer than \
                                         {MAX_INDEX_PROPERTY_NAME_BYTES} bytes"
                                    ),
                                ));
                            }
                            c.at(name, |c| {
                                c.string(order)?;
                                c.one_of_strings(order, &["asc"])
                            })?;
                        }
                        Ok(())
                    })?;
                }
                Ok(())
            }
            "unique" | "nullSearchable" | "rangeCountable" | "rangeSummable"
            | "rangeAverageable" | "preallocated" | "outlivesDelete" => self.boolean(value),
            "contested" => {
                let map = self.object(value)?;
                self.closed(map, &["fieldMatches", "resolution", "description"])?;
                self.required(map, &["resolution"])?;
                self.member(map, "fieldMatches", |c, value| {
                    let items = c.array(value)?;
                    c.item_count(items, Some(1), None)?;
                    for (index, item) in items.iter().enumerate() {
                        c.at(index, |c| {
                            let field_match = c.object(item)?;
                            c.closed(field_match, &["field", "regexPattern"])?;
                            c.required(field_match, &["field", "regexPattern"])?;
                            for name in ["field", "regexPattern"] {
                                c.member(field_match, name, |c, value| {
                                    c.string_of_length(value, 1, Some(256)).map(drop)
                                })?;
                            }
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
                self.member(map, "resolution", |c, value| {
                    c.integer(value, None, None)?;
                    c.one_of_integers(value, &[0, 1])
                })?;
                self.member(map, "description", |c, value| {
                    c.string_of_length(value, 1, Some(256)).map(drop)
                })
            }
            "countable" => {
                if value.is_boolean() {
                    return Ok(());
                }
                self.within("oneOfNotValid", value, |c| {
                    c.string(value)?;
                    c.one_of_strings(
                        value,
                        &["notCountable", "countable", "countableAllowingOffset"],
                    )
                })
            }
            "summable" | "averageable" => {
                self.string_of_length(value, 1, Some(64))?;
                self.word(value)
            }
            "rankedCountable" | "rankedSummable" | "rankedAverageable" => {
                if value.is_boolean() {
                    return Ok(());
                }
                self.within("oneOfNotValid", value, |c| {
                    let map = c.object(value)?;
                    c.closed(map, &["at"])?;
                    c.required(map, &["at"])?;
                    c.member(map, "at", |c, levels| c.names_or_one(levels))
                })
            }
            "timeRange" | "integerRange" => {
                let map = self.object(value)?;
                if key == "timeRange" {
                    self.closed(map, &["on", "range", "step", "phase", "ttl"])?;
                } else {
                    self.closed(map, &["on", "range", "step", "phase"])?;
                }
                self.required(map, &["on", "range", "step"])?;
                self.member(map, "on", |c, on| {
                    c.string_of_length(on, 1, Some(256)).map(drop)
                })?;
                for name in ["range", "step", "ttl"] {
                    self.member(map, name, |c, value| c.integer(value, Some(1), None))?;
                }
                self.member(map, "phase", |c, phase| c.integer(phase, Some(0), None))
            }
            "terminal" => self.within("oneOfNotValid", value, |c| c.names_or_one(value)),
            "skipIfAbsent" => {
                if value.is_boolean() {
                    return Ok(());
                }
                self.within("oneOfNotValid", value, |c| c.names(value))
            }
            "summableOffCountIndex" => self.string_of_length(value, 1, Some(32)).map(drop),
            _ => Ok(()),
        }
    }

    /// A string of 1 to 256 characters, or a list of 1 to 10 such strings, no two alike.
    fn names_or_one(&self, value: &JsonValue) -> Checked {
        if value.is_string() {
            return self.string_of_length(value, 1, Some(256)).map(drop);
        }
        self.names(value)
    }

    /// A list of 1 to 10 strings of 1 to 256 characters, no two alike.
    fn names(&self, value: &JsonValue) -> Checked {
        let items = self.array(value)?;
        for (index, item) in items.iter().enumerate() {
            self.at(index, |c| c.string_of_length(item, 1, Some(256)).map(drop))?;
        }
        self.item_count(items, Some(1), Some(10))?;
        self.unique(items)
    }
}
