//! The order an object lists its members in, read against their `position`s.
//!
//! An object stores its members in the order its `properties` map lists them, at every protocol
//! version, while a document type's own properties are stored in `position` order. From protocol
//! version 15 an object a contract gains (at registration, or in an update that adds it) lists
//! its members at consecutive positions in map order, so for every such object the two orders
//! agree and its members are stored as a document type's own properties are; a `$ref` it gains
//! points into its `$defs`, at a schema whose objects are listed the same way. An object a
//! contract already has keeps the order it was registered with, which before protocol version 15
//! could differ from its positions, or leave gaps in them: its members keep their order, and a
//! member an update adds to it comes after its existing ones, at its next free positions, so
//! every document stored before reads the added member as absent.
//!
//! Reads the schemas as written, and every read is infallible: a schema of another shape is the
//! business of the checks that parse it.

use super::last_value;
use crate::data_contract::document_type::property_names::{POSITION, PROPERTIES, REF, TYPE};
use crate::util::json_schema::resolve_uri;
use platform_value::{Value, ValueMapHelper};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

/// Why an object's members are refused by their order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberPositionOrderError {
    /// An object the contract gains does not list its members at consecutive positions in map
    /// order, or a `$ref` it gains does not point at a schema of `$defs` whose objects do.
    ListedOutOfPositionOrder(String),
    /// An object the contract has changes the order of its members, or a member an update adds
    /// to it is not listed after them at its next free position.
    AddedBeforeExistingMembers(String),
}

/// Judges the objects of a contract's schemas by their members' order, with the `$defs` the
/// registration or update writes, which the `$ref`s it gains point into.
pub struct MemberPositionOrder {
    /// `{ "$defs": <definitions> }`, the root a `$ref` into `$defs` is resolved against, as the
    /// parse resolves it.
    root: Value,
    /// The detail of the refusal, if any, of each schema a `$ref` judged so far points at, by
    /// the schema's address in `root`: the parse reads `/` and `.` in a pointer alike, so one
    /// schema is reached by many spellings, and each is walked once.
    judged_references: RefCell<BTreeMap<*const Value, Option<String>>>,
}

impl MemberPositionOrder {
    /// With the `$defs` of the contract as the registration or update writes them.
    pub fn new(definitions: Option<&BTreeMap<String, Value>>) -> Self {
        let definitions = definitions
            .into_iter()
            .flatten()
            .map(|(name, definition)| (Value::Text(name.clone()), definition.clone()))
            .collect();
        Self {
            root: Value::Map(vec![(
                Value::Text("$defs".to_string()),
                Value::Map(definitions),
            )]),
            judged_references: RefCell::new(BTreeMap::new()),
        }
    }

    /// The first object of the document type schema `new_schema` whose members' order is
    /// refused: each of the type's own properties, whose order is their `position`s whatever the
    /// map says, is judged against the same property of `old_schema`, the type as the contract
    /// stores it (`None` for a type the contract gains).
    pub fn first_error_of_document_type(
        &self,
        old_schema: Option<&Value>,
        new_schema: &Value,
    ) -> Option<MemberPositionOrderError> {
        let old_properties = old_schema.and_then(inline_members);
        inline_members(new_schema)?
            .iter()
            .find_map(|(name, property)| {
                let name = name.as_text()?;
                let old_property =
                    old_properties.and_then(|properties| last_value(properties, name));
                self.first_error(old_property, property, name, true)
            })
    }

    /// The first `$defs` definition, of those this was made with, whose members' order is
    /// refused, against the same definition of `old_definitions` (the contract's stored `$defs`,
    /// `None` at registration), with its name.
    pub fn first_error_of_definitions(
        &self,
        old_definitions: Option<&BTreeMap<String, Value>>,
    ) -> Option<(String, MemberPositionOrderError)> {
        let definitions = self
            .root
            .as_map()
            .and_then(|root| root.get_optional_key("$defs"))
            .and_then(Value::as_map)?;
        definitions.iter().find_map(|(name, definition)| {
            let name = name.as_text()?;
            let old_definition = old_definitions.and_then(|old| old.get(name));
            self.first_error(old_definition, definition, &format!("$defs.{name}"), true)
                .map(|error| (name.to_string(), error))
        })
    }

    /// The first object of the property schema `new`, or of the objects inside it at any depth,
    /// whose members' order is refused; `old` is the same property as the contract stores it,
    /// `None` when the contract gains it, and `path` names it. A `$ref` the contract gains is
    /// judged when `judge_references` is set; the `properties` written beside a `$ref` are not
    /// read for storage, so they are not walked.
    fn first_error(
        &self,
        old: Option<&Value>,
        new: &Value,
        path: &str,
        judge_references: bool,
    ) -> Option<MemberPositionOrderError> {
        let new_schema = new.as_map()?;
        if let Some(reference) = new_schema.get_optional_key(REF) {
            let kept = old
                .and_then(Value::as_map)
                .and_then(|old| old.get_optional_key(REF))
                == Some(reference);
            if !judge_references || kept {
                return None;
            }
            return self.gained_reference_error(reference.as_text()?, path);
        }
        if !is_object(new_schema) {
            return None;
        }
        let Some(Value::Map(new_members)) = new_schema.get_optional_key(PROPERTIES) else {
            return None;
        };
        let old_members = old.and_then(inline_members);
        let error = match old_members {
            None => not_at_consecutive_positions(new_members, path),
            Some(old_members) => not_kept_or_appended(old_members, new_members, path),
        };
        if error.is_some() {
            return error;
        }
        new_members.iter().find_map(|(name, member)| {
            let name = name.as_text()?;
            let old_member = old_members.and_then(|old_members| last_value(old_members, name));
            self.first_error(
                old_member,
                member,
                &format!("{path}.{name}"),
                judge_references,
            )
        })
    }

    /// Why a `$ref` the contract gains at `path` is refused: it does not point into `$defs`, or
    /// the schema it points at holds an object not listed at consecutive positions. That
    /// schema's own `$ref`s are judged where they are written. One the parse could not resolve
    /// is left to it.
    fn gained_reference_error(
        &self,
        reference: &str,
        path: &str,
    ) -> Option<MemberPositionOrderError> {
        if !reference.starts_with("#/$defs/") {
            return Some(MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                "'{path}' takes its schema from '{reference}': from protocol version 15 a `$ref` \
                 a contract gains points into its `$defs`"
            )));
        }
        let target = resolve_uri(&self.root, reference).ok()?;
        let address = target as *const Value;
        let cached = self.judged_references.borrow().get(&address).cloned();
        let detail = match cached {
            Some(detail) => detail,
            None => {
                let detail = self
                    .first_error(None, target, reference, false)
                    .map(|error| match error {
                        MemberPositionOrderError::ListedOutOfPositionOrder(detail)
                        | MemberPositionOrderError::AddedBeforeExistingMembers(detail) => detail,
                    });
                self.judged_references
                    .borrow_mut()
                    .insert(address, detail.clone());
                detail
            }
        };
        detail.map(|detail| {
            MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                "'{path}' takes its members from '{reference}', which does not list them at \
                 consecutive positions: {detail}"
            ))
        })
    }
}

/// Whether `schema` is an object (`"type": "object"`), the only schema whose `properties` the
/// parse reads as members.
fn is_object(schema: &[(Value, Value)]) -> bool {
    last_value(schema, TYPE).and_then(Value::as_text) == Some("object")
}

/// The members of `schema` when it is an object written inline: its `properties` map.
fn inline_members(schema: &Value) -> Option<&[(Value, Value)]> {
    let schema = schema.as_map()?;
    if schema.get_optional_key(REF).is_some() || !is_object(schema) {
        return None;
    }
    match schema.get_optional_key(PROPERTIES) {
        Some(Value::Map(members)) => Some(members.as_slice()),
        _ => None,
    }
}

fn position(member: &Value) -> Option<u64> {
    member.get_integer::<u64>(POSITION).ok()
}

/// The position a member stored before protocol version 15 takes, a value such as `1.0` (which
/// registration then accepted) read as the integer it stands for.
fn stored_position(member: &Value) -> Option<u64> {
    position(member).or_else(|| match member.as_map()?.get_optional_key(POSITION)? {
        Value::Float(value)
            if value.fract() == 0.0 && *value >= 0.0 && *value < u64::MAX as f64 =>
        {
            Some(*value as u64)
        }
        _ => None,
    })
}

/// `None` when every member of `members` takes an integer position, one above the position of
/// the member listed before it.
fn not_at_consecutive_positions(
    members: &[(Value, Value)],
    path: &str,
) -> Option<MemberPositionOrderError> {
    let refused = |detail: String| {
        Some(MemberPositionOrderError::ListedOutOfPositionOrder(format!(
            "the object '{path}' must list its members at consecutive positions: {detail}"
        )))
    };
    let mut previous: Option<(&str, u64)> = None;
    for (name, member) in members {
        let Some(name) = name.as_text() else {
            continue;
        };
        let Some(member_position) = position(member) else {
            return refused(format!("'{name}' needs a position, an integer from 0"));
        };
        if let Some((previous_name, previous_position)) = previous {
            if member_position == previous_position {
                return refused(format!(
                    "'{name}' takes position {member_position}, as '{previous_name}' does"
                ));
            }
            if member_position < previous_position {
                return refused(format!(
                    "'{previous_name}' at position {previous_position} is listed before \
                     '{name}' at position {member_position}"
                ));
            }
            if Some(member_position) != previous_position.checked_add(1) {
                return refused(format!(
                    "'{name}' takes position {member_position} after '{previous_name}' at \
                     {previous_position}, leaving a gap"
                ));
            }
        }
        previous = Some((name, member_position));
    }
    None
}

/// `None` when the members of `new_members` that `old_members` holds keep their order, and every
/// other one is listed after them at the next free position: one above the highest
/// position of the old members (0 when they have none), then one above each added before it.
fn not_kept_or_appended(
    old_members: &[(Value, Value)],
    new_members: &[(Value, Value)],
    path: &str,
) -> Option<MemberPositionOrderError> {
    let refused =
        |detail: String| Some(MemberPositionOrderError::AddedBeforeExistingMembers(detail));
    let old_order: Vec<&str> = old_members
        .iter()
        .filter_map(|(name, _)| name.as_text())
        .collect();
    let existing: BTreeSet<&str> = old_order.iter().copied().collect();
    let new_names: BTreeSet<&str> = new_members
        .iter()
        .filter_map(|(name, _)| name.as_text())
        .collect();
    let mut kept_in_old_order = old_order.iter().filter(|name| new_names.contains(*name));
    let mut next_position = old_members
        .iter()
        .filter_map(|(_, member)| stored_position(member))
        .max()
        .map_or(Some(0), |highest| highest.checked_add(1));
    let mut first_added: Option<&str> = None;
    for (name, member) in new_members {
        let Some(name) = name.as_text() else {
            continue;
        };
        if existing.contains(name) {
            if let Some(added) = first_added {
                return refused(format!(
                    "the member '{added}' an update adds to the object '{path}' is listed \
                     before its existing member '{name}'"
                ));
            }
            if kept_in_old_order.next() != Some(&name) {
                return refused(format!(
                    "the object '{path}' would list its existing members in another order than \
                     the one they are stored in ({})",
                    old_order.join(", ")
                ));
            }
            continue;
        }
        first_added.get_or_insert(name);
        let member_position = position(member);
        if member_position.is_none() || member_position != next_position {
            return refused(format!(
                "the member '{name}' an update adds to the object '{path}' must take the next \
                 free position{}",
                next_position.map_or_else(String::new, |next| format!(", {next}"))
            ));
        }
        next_position = next_position.and_then(|next| next.checked_add(1));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn object(members: Vec<(&str, Value)>) -> Value {
        let members = members
            .into_iter()
            .map(|(name, member)| (Value::Text(name.to_string()), member))
            .collect();
        platform_value!({
            "type": "object",
            "properties": Value::Map(members),
            "additionalProperties": false,
        })
    }

    fn member(position: u64) -> Value {
        platform_value!({ "type": "string", "maxLength": 10, "position": position })
    }

    fn with_position(mut schema: Value, position: u64) -> Value {
        schema
            .insert("position".to_string(), Value::U64(position))
            .expect("expected a map");
        schema
    }

    /// A document type with the single property `meta`.
    fn type_with_meta(meta: Value) -> Value {
        platform_value!({
            "type": "object",
            "properties": { "meta": with_position(meta, 0) },
            "additionalProperties": false,
        })
    }

    fn listed_out_of_position_order(error: Option<MemberPositionOrderError>, fragment: &str) {
        assert!(
            matches!(
                &error,
                Some(MemberPositionOrderError::ListedOutOfPositionOrder(details))
                    if details.contains(fragment)
            ),
            "expected a refusal containing {fragment}, got {error:?}"
        );
    }

    #[test]
    fn should_hold_an_object_the_contract_gains_to_consecutive_positions_at_every_depth() {
        let judge = MemberPositionOrder::new(None);
        for (case, meta) in [
            ("from 0", object(vec![("a", member(0)), ("b", member(1))])),
            (
                "from 1, as DPNS records",
                object(vec![("identity", member(1))]),
            ),
        ] {
            assert_eq!(
                judge.first_error_of_document_type(None, &type_with_meta(meta)),
                None,
                "{case}"
            );
        }
        for (case, meta, fragment) in [
            (
                "out of position order",
                object(vec![("b", member(1)), ("a", member(0))]),
                "'b' at position 1 is listed before 'a'",
            ),
            (
                "a gap",
                object(vec![("a", member(0)), ("b", member(2))]),
                "leaving a gap",
            ),
            (
                "a repeat",
                object(vec![("a", member(1)), ("b", member(1))]),
                "as 'a' does",
            ),
            (
                "a float",
                object(vec![
                    ("a", member(0)),
                    ("b", platform_value!({ "type": "boolean", "position": 1.0 })),
                ]),
                "needs a position",
            ),
            (
                "one level down",
                object(vec![
                    ("a", member(0)),
                    (
                        "inner",
                        with_position(object(vec![("q", member(1)), ("p", member(0))]), 1),
                    ),
                ]),
                "'meta.inner'",
            ),
        ] {
            let error = judge.first_error_of_document_type(None, &type_with_meta(meta));
            assert!(
                matches!(
                    &error,
                    Some(MemberPositionOrderError::ListedOutOfPositionOrder(details))
                        if details.contains(fragment)
                ),
                "{case}: {error:?}"
            );
        }
    }

    /// An object stored before protocol version 15 may list its members out of position order
    /// or with a gap: it stays as it is, and a member it gains comes after its existing ones at
    /// its next free position.
    #[test]
    fn should_let_an_existing_object_gain_members_at_its_next_free_positions() {
        let judge = MemberPositionOrder::new(None);
        let stored = type_with_meta(object(vec![
            ("slug", member(2)),
            ("title", member(0)),
            ("description", member(1)),
        ]));
        let with = |added: Vec<(&str, Value)>| {
            let mut members = vec![
                ("slug", member(2)),
                ("title", member(0)),
                ("description", member(1)),
            ];
            members.extend(added);
            type_with_meta(object(members))
        };
        for (case, new) in [
            ("unchanged", stored.clone()),
            (
                "one member at the next position",
                with(vec![("layout", member(3))]),
            ),
            (
                "two members at the next positions",
                with(vec![("layout", member(3)), ("theme", member(4))]),
            ),
        ] {
            assert_eq!(
                judge.first_error_of_document_type(Some(&stored), &new),
                None,
                "{case}"
            );
        }
        let gap = type_with_meta(object(vec![("a", member(0)), ("b", member(2))]));
        assert_eq!(
            judge.first_error_of_document_type(Some(&gap), &gap),
            None,
            "a gap stored earlier is kept"
        );

        let listed_first = type_with_meta(object(vec![
            ("layout", member(3)),
            ("slug", member(2)),
            ("title", member(0)),
            ("description", member(1)),
        ]));
        let reordered = type_with_meta(object(vec![
            ("title", member(0)),
            ("slug", member(2)),
            ("description", member(1)),
        ]));
        for (case, new, fragment) in [
            (
                "listed before the existing members",
                listed_first,
                "listed before",
            ),
            ("the existing members reordered", reordered, "another order"),
            (
                "a position below the next free one",
                with(vec![("layout", member(1))]),
                "next free position, 3",
            ),
            (
                "a position past the next free one",
                with(vec![("layout", member(5))]),
                "next free position, 3",
            ),
            (
                "added members out of position order",
                with(vec![("theme", member(4)), ("layout", member(3))]),
                "next free position, 3",
            ),
        ] {
            let error = judge.first_error_of_document_type(Some(&stored), &new);
            assert!(
                matches!(
                    &error,
                    Some(MemberPositionOrderError::AddedBeforeExistingMembers(details))
                        if details.contains(fragment)
                ),
                "{case}: {error:?}"
            );
        }
    }

    /// A `$ref` the contract gains points into `$defs`, at a schema whose objects list their
    /// members at consecutive positions; one it keeps is not judged again.
    #[test]
    fn should_hold_a_gained_ref_to_a_definition_listed_at_consecutive_positions() {
        let definitions = BTreeMap::from([
            (
                "in_order".to_string(),
                object(vec![("a", member(0)), ("b", member(1))]),
            ),
            (
                "out_of_order".to_string(),
                object(vec![("b", member(1)), ("a", member(0))]),
            ),
            (
                "keys".to_string(),
                object(vec![(
                    "recipient",
                    platform_value!({ "type": "array", "byteArray": true, "position": 0 }),
                )]),
            ),
        ]);
        let judge = MemberPositionOrder::new(Some(&definitions));
        let referencing = |reference: &str| {
            platform_value!({
                "type": "object",
                "properties": { "p": { "$ref": reference, "position": 0 } },
                "additionalProperties": false,
            })
        };
        for reference in ["#/$defs/in_order", "#/$defs/keys/properties/recipient"] {
            assert_eq!(
                judge.first_error_of_document_type(None, &referencing(reference)),
                None,
                "{reference}"
            );
        }
        listed_out_of_position_order(
            judge.first_error_of_document_type(None, &referencing("#/$defs/out_of_order")),
            "'#/$defs/out_of_order'",
        );
        listed_out_of_position_order(
            judge.first_error_of_document_type(None, &referencing("#/properties/q")),
            "points into its `$defs`",
        );
        let stored = referencing("#/$defs/out_of_order");
        assert_eq!(
            judge.first_error_of_document_type(Some(&stored), &stored),
            None,
            "a $ref the contract keeps"
        );
    }

    /// Every definition is judged, whether a type references it or not: its members keep their
    /// order and are appended at the next free positions.
    #[test]
    fn should_hold_every_definition_to_its_member_order() {
        let stored = BTreeMap::from([(
            "m".to_string(),
            object(vec![("a", member(0)), ("b", member(1))]),
        )]);
        let reordered = BTreeMap::from([(
            "m".to_string(),
            object(vec![("b", member(1)), ("a", member(0))]),
        )]);
        assert!(matches!(
            MemberPositionOrder::new(Some(&reordered)).first_error_of_definitions(Some(&stored)),
            Some((name, MemberPositionOrderError::AddedBeforeExistingMembers(_))) if name == "m"
        ));
        assert!(matches!(
            MemberPositionOrder::new(Some(&reordered)).first_error_of_definitions(None),
            Some((name, MemberPositionOrderError::ListedOutOfPositionOrder(_))) if name == "m"
        ));
        assert_eq!(
            MemberPositionOrder::new(Some(&stored)).first_error_of_definitions(Some(&stored)),
            None
        );
    }

    /// A member that takes its schema from a `$ref` is placed by the `position` written beside
    /// the `$ref`, never by one its definition carries: the definition's would leave a gap.
    #[test]
    fn should_read_the_position_written_beside_a_members_ref() {
        let definitions = BTreeMap::from([(
            "flag".to_string(),
            platform_value!({ "type": "boolean", "position": 5 }),
        )]);
        let meta = object(vec![
            (
                "x",
                platform_value!({ "$ref": "#/$defs/flag", "position": 0 }),
            ),
            ("y", member(1)),
        ]);
        assert_eq!(
            MemberPositionOrder::new(Some(&definitions))
                .first_error_of_document_type(None, &type_with_meta(meta)),
            None
        );
    }

    /// The parse reads `/` and `.` in a pointer alike: every spelling of one schema is judged
    /// with a single walk.
    #[test]
    fn should_walk_each_schema_a_ref_points_at_once_however_it_is_spelled() {
        let definitions = BTreeMap::from([(
            "a".to_string(),
            platform_value!({
                "type": "object",
                "properties": { "b": with_position(object(vec![("c", member(0))]), 0) },
                "additionalProperties": false,
            }),
        )]);
        let judge = MemberPositionOrder::new(Some(&definitions));
        for reference in [
            "#/$defs/a/properties/b",
            "#/$defs/a.properties/b",
            "#/$defs/a.properties.b",
        ] {
            let schema = platform_value!({
                "type": "object",
                "properties": { "p": { "$ref": reference, "position": 0 } },
                "additionalProperties": false,
            });
            assert_eq!(
                judge.first_error_of_document_type(None, &schema),
                None,
                "{reference}"
            );
        }
        assert_eq!(judge.judged_references.borrow().len(), 1);
    }

    /// Only an object's `properties` are members: the parse ignores a `properties` map written
    /// on another schema, and so does this.
    #[test]
    fn should_ignore_properties_written_on_a_schema_that_is_no_object() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "note": {
                    "type": "string",
                    "maxLength": 10,
                    "position": 0,
                    "properties": { "x": { "type": "string" } },
                },
            },
            "additionalProperties": false,
        });
        assert_eq!(
            MemberPositionOrder::new(None).first_error_of_document_type(None, &schema),
            None
        );
    }

    /// A position a contract stored before protocol version 15 wrote as `1.0` counts as 1 when
    /// the next free position of its object is found.
    #[test]
    fn should_count_an_integral_float_position_stored_earlier() {
        let judge = MemberPositionOrder::new(None);
        let float_member = platform_value!({ "type": "string", "maxLength": 10, "position": 1.0 });
        let stored = type_with_meta(object(vec![("a", member(0)), ("b", float_member.clone())]));
        let with_c = |position: u64| {
            type_with_meta(object(vec![
                ("a", member(0)),
                ("b", float_member.clone()),
                ("c", member(position)),
            ]))
        };
        assert_eq!(
            judge.first_error_of_document_type(Some(&stored), &with_c(2)),
            None
        );
        assert!(matches!(
            judge.first_error_of_document_type(Some(&stored), &with_c(1)),
            Some(MemberPositionOrderError::AddedBeforeExistingMembers(details))
                if details.contains("next free position, 2")
        ));
    }
}
