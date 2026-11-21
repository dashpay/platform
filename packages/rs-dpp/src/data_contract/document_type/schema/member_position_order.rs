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
use platform_version::version::PlatformVersion;
use std::cell::{OnceCell, RefCell};
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
/// contract stores and the `$defs` the registration or update writes, which the `$ref`s it keeps
/// and gains point into.
pub struct MemberPositionOrder<'a> {
    old_definitions: Option<&'a BTreeMap<String, Value>>,
    new_definitions: Option<&'a BTreeMap<String, Value>>,
    /// How deep the walk goes, through members and `$ref`s together: the schema depth the parse
    /// admits, which counts a schema at least as deep. A pair of schemas cut off there is judged
    /// as accepted, and the answer is kept, so the walk stays linear: the parse refuses a type
    /// nested that deep, and a definition no type reads is judged again when a type gains a
    /// `$ref` to it.
    max_depth: usize,
    /// `{ "$defs": <definitions> }` for each side, the root a `$ref` is resolved against as the
    /// parse resolves it, built the first time a `$ref` is resolved.
    old_root: OnceCell<Value>,
    new_root: OnceCell<Value>,
    /// The refusal, if any, of each pair of schemas a `$ref` judged so far points at (the stored
    /// one, null for a `$ref` the contract gains, and the new one), by their addresses: the parse
    /// reads `/` and `.` in a pointer alike, so one schema is reached by many spellings, and each
    /// pair is walked once. A pair a kept `$ref` reaches is keyed with the document type schemas
    /// its pointers resolve in too. A pair being walked holds `None`, which ends a cycle.
    judged_references: RefCell<BTreeMap<JudgedPair, Option<MemberPositionOrderError>>>,
}

/// The addresses of a stored and a new schema a `$ref` points at, and of the stored and new
/// document type schemas a kept pointer outside `$defs` resolves in (null for a `$ref` the
/// contract gains, whose pointers all lead into `$defs`).
type JudgedPair = [*const Value; 4];

/// The stored and the new schema of the document type being walked, where the parse resolves a
/// `$ref` that does not point into `$defs` (it resolves against the type's schema with the
/// contract's `$defs` added). Neither while the `$defs` are walked on their own.
#[derive(Clone, Copy, Default)]
struct TypeSchemas<'v> {
    old: Option<&'v Value>,
    new: Option<&'v Value>,
}

impl<'a> MemberPositionOrder<'a> {
    /// With the `$defs` the contract stores (`None` at registration) and those the registration
    /// or update writes.
    pub fn new(
        old_definitions: Option<&'a BTreeMap<String, Value>>,
        new_definitions: Option<&'a BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> Self {
        Self {
            old_definitions,
            new_definitions,
            max_depth: platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .max_depth as usize,
            old_root: OnceCell::new(),
            new_root: OnceCell::new(),
            judged_references: RefCell::new(BTreeMap::new()),
        }
    }

    /// The first object of a contract being registered, in its document schemas or its `$defs`,
    /// whose members' order is refused: every object is one the contract gains.
    pub fn first_error_of_registration<'s>(
        &self,
        document_schemas: impl IntoIterator<Item = &'s Value>,
    ) -> Option<MemberPositionOrderError> {
        document_schemas
            .into_iter()
            .find_map(|schema| self.first_error_of_document_type(None, schema))
            .or_else(|| self.first_error_of_definitions().map(|(_, error)| error))
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
        let types = TypeSchemas {
            old: old_schema,
            new: Some(new_schema),
        };
        inline_members(new_schema)?
            .iter()
            .find_map(|(name, property)| {
                let name = name.as_text()?;
                let old_property =
                    old_properties.and_then(|properties| last_value(properties, name));
                self.first_error(old_property, property, name, types, 0)
            })
    }

    /// The first `$defs` definition the registration or update writes whose members' order is
    /// refused, against the same definition the contract stores, with its name.
    pub fn first_error_of_definitions(&self) -> Option<(String, MemberPositionOrderError)> {
        self.new_definitions?.iter().find_map(|(name, definition)| {
            let old_definition = self
                .old_definitions
                .and_then(|definitions| definitions.get(name));
            self.first_error(
                old_definition,
                definition,
                &format!("$defs.{name}"),
                TypeSchemas::default(),
                0,
            )
            .map(|error| (name.clone(), error))
        })
    }

    /// The first object of the property schema `new`, or of the objects inside it or reached
    /// through its `$ref`s, at any depth, whose members' order is refused; `old` is the same
    /// property as the contract stores it, `None` when the contract gains it, and `path` names
    /// it. The `properties` written beside a `$ref` are not read for storage, so they are not
    /// walked.
    fn first_error(
        &self,
        old: Option<&Value>,
        new: &Value,
        path: &str,
        types: TypeSchemas,
        depth: usize,
    ) -> Option<MemberPositionOrderError> {
        if depth > self.max_depth {
            return None;
        }
        let new_schema = new.as_map()?;
        if let Some(reference) = new_schema.get_optional_key(REF) {
            let reference = reference.as_text()?;
            let old_reference = old
                .and_then(Value::as_map)
                .and_then(|old| old.get_optional_key(REF))
                .and_then(Value::as_text);
            return if old_reference == Some(reference) {
                self.kept_reference_error(reference, path, types, depth)
            } else {
                self.gained_reference_error(reference, path, depth)
            };
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
                types,
                depth + 1,
            )
        })
    }

    /// Why a `$ref` the contract gains at `path` is refused: it does not point into `$defs`, or
    /// the schema it points at, or one reached from it, holds an object not listed at consecutive
    /// positions. One that points at nothing is left to the parse, which refuses it.
    fn gained_reference_error(
        &self,
        reference: &str,
        path: &str,
        depth: usize,
    ) -> Option<MemberPositionOrderError> {
        let refused = |detail: String| {
            Some(MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                "'{path}' takes its schema from '{reference}': {detail}"
            )))
        };
        if !reference.starts_with("#/$defs/") {
            return refused(
                "from protocol version 15 a `$ref` a contract gains points into its `$defs`"
                    .to_string(),
            );
        }
        let target = self.resolve(reference, &self.new_root, self.new_definitions)?;
        self.judged_through_reference(None, target, reference, TypeSchemas::default(), depth)
            .map(|error| through_reference(error, path, reference))
    }

    /// Why a `$ref` the contract keeps at `path` is refused: the object it points at, as the
    /// update writes it, does not keep the rule of an object the contract has against the one
    /// it pointed at. A pointer into `$defs` is resolved in each side's `$defs`, any other in
    /// each side's document type schema, as the parse resolves it. One that points at nothing on
    /// either side is left as it is.
    fn kept_reference_error(
        &self,
        reference: &str,
        path: &str,
        types: TypeSchemas,
        depth: usize,
    ) -> Option<MemberPositionOrderError> {
        let (old_target, new_target) = if points_into_definitions(reference) {
            (
                self.resolve(reference, &self.old_root, self.old_definitions)?,
                self.resolve(reference, &self.new_root, self.new_definitions)?,
            )
        } else {
            (
                resolve_uri(types.old?, reference).ok()?,
                resolve_uri(types.new?, reference).ok()?,
            )
        };
        self.judged_through_reference(Some(old_target), new_target, reference, types, depth)
            .map(|error| through_reference(error, path, reference))
    }

    /// [`Self::first_error`] for the schemas a `$ref` points at, walked once per pair.
    fn judged_through_reference(
        &self,
        old: Option<&Value>,
        new: &Value,
        reference: &str,
        types: TypeSchemas,
        depth: usize,
    ) -> Option<MemberPositionOrderError> {
        let address = |value: Option<&Value>| value.map_or(std::ptr::null(), std::ptr::from_ref);
        // Below a `$ref` the contract gains, every `$ref` is gained and points into `$defs`, so
        // its answer does not depend on the document type
        let types = if old.is_some() {
            types
        } else {
            TypeSchemas::default()
        };
        let key = [
            address(old),
            std::ptr::from_ref(new),
            address(types.old),
            address(types.new),
        ];
        if let Some(judged) = self.judged_references.borrow().get(&key) {
            return judged.clone();
        }
        self.judged_references.borrow_mut().insert(key, None);
        let error = self.first_error(old, new, reference, types, depth + 1);
        self.judged_references
            .borrow_mut()
            .insert(key, error.clone());
        error
    }

    /// The schema `reference` points at, among `definitions`, resolved as the parse resolves it.
    fn resolve<'r>(
        &self,
        reference: &str,
        root: &'r OnceCell<Value>,
        definitions: Option<&BTreeMap<String, Value>>,
    ) -> Option<&'r Value> {
        let root = root.get_or_init(|| {
            let definitions = definitions
                .into_iter()
                .flatten()
                .map(|(name, definition)| (Value::Text(name.clone()), definition.clone()))
                .collect();
            Value::Map(vec![(
                Value::Text("$defs".to_string()),
                Value::Map(definitions),
            )])
        });
        resolve_uri(root, reference).ok()
    }
}

/// Whether the parse finds `reference` in the contract's `$defs`: its path's first segment, which
/// ends at a `/` or a `.`, is `$defs`.
fn points_into_definitions(reference: &str) -> bool {
    reference
        .strip_prefix("#/")
        .and_then(|path| path.split(['/', '.']).next())
        == Some("$defs")
}

/// `error`, found in the schema `reference` points at, as the refusal of the `$ref` at `path`.
fn through_reference(
    error: MemberPositionOrderError,
    path: &str,
    reference: &str,
) -> MemberPositionOrderError {
    match error {
        MemberPositionOrderError::ListedOutOfPositionOrder(detail) => {
            MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                "'{path}' takes its members from '{reference}': {detail}"
            ))
        }
        MemberPositionOrderError::AddedBeforeExistingMembers(detail) => {
            MemberPositionOrderError::AddedBeforeExistingMembers(format!(
                "'{path}' takes its members from '{reference}': {detail}"
            ))
        }
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
/// When a stored position reads as no integer, the first member added takes any position and
/// the others follow it; when the highest is the last integer, none can be added.
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
    // The next free position is one above the highest stored one (none when the highest is the
    // last integer). A stored position may read as no integer (a float past every integer,
    // which registration before protocol version 15 accepted): the members are stored in map
    // order whatever their positions, so the first member added to such an object takes any
    // position, and the members added after it follow it
    let stored_positions: Option<Vec<u64>> = old_members
        .iter()
        .map(|(_, member)| stored_position(member))
        .collect();
    let mut any_position = stored_positions.is_none();
    let mut next_position = stored_positions.and_then(|positions| {
        positions
            .into_iter()
            .max()
            .map_or(Some(0), |highest| highest.checked_add(1))
    });
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
        let Some(member_position) = position(member) else {
            return refused(format!(
                "the member '{name}' an update adds to the object '{path}' needs a position, an \
                 integer from 0"
            ));
        };
        if !any_position && next_position != Some(member_position) {
            return refused(match next_position {
                Some(next) => format!(
                    "the member '{name}' an update adds to the object '{path}' must take the \
                     next free position, {next}"
                ),
                None => format!(
                    "the member '{name}' an update adds to the object '{path}' has no next free \
                     position"
                ),
            });
        }
        any_position = false;
        next_position = member_position.checked_add(1);
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
        let judge = MemberPositionOrder::new(None, None, PlatformVersion::latest());
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
        let judge = MemberPositionOrder::new(None, None, PlatformVersion::latest());
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
    /// members at consecutive positions; one it keeps is judged against the schema it pointed
    /// at, here the same one, out of position order as it was stored.
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
        let judge = MemberPositionOrder::new(
            Some(&definitions),
            Some(&definitions),
            PlatformVersion::latest(),
        );
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
            MemberPositionOrder::new(Some(&stored), Some(&reordered), PlatformVersion::latest())
                .first_error_of_definitions(),
            Some((name, MemberPositionOrderError::AddedBeforeExistingMembers(_))) if name == "m"
        ));
        assert!(matches!(
            MemberPositionOrder::new(None, Some(&reordered), PlatformVersion::latest())
                .first_error_of_definitions(),
            Some((name, MemberPositionOrderError::ListedOutOfPositionOrder(_))) if name == "m"
        ));
        assert_eq!(
            MemberPositionOrder::new(Some(&stored), Some(&stored), PlatformVersion::latest())
                .first_error_of_definitions(),
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
            MemberPositionOrder::new(
                Some(&definitions),
                Some(&definitions),
                PlatformVersion::latest()
            )
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
        let judge = MemberPositionOrder::new(
            Some(&definitions),
            Some(&definitions),
            PlatformVersion::latest(),
        );
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
            MemberPositionOrder::new(None, None, PlatformVersion::latest())
                .first_error_of_document_type(None, &schema),
            None
        );
    }

    /// A position a contract stored before protocol version 15 wrote as `1.0` counts as 1 when
    /// the next free position of its object is found.
    #[test]
    fn should_count_an_integral_float_position_stored_earlier() {
        let judge = MemberPositionOrder::new(None, None, PlatformVersion::latest());
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

    /// A `$ref` a contract gains is judged with every `$ref` the schema it points at holds, and
    /// those with theirs, once each, a cycle ending the walk; one that points at nothing is left
    /// to the parse, which refuses it.
    #[test]
    fn should_judge_every_ref_a_gained_ref_reaches() {
        let definitions = BTreeMap::from([
            (
                "outer".to_string(),
                object(vec![(
                    "x",
                    platform_value!({ "$ref": "#/$defs/legacy", "position": 0 }),
                )]),
            ),
            (
                "legacy".to_string(),
                object(vec![("b", member(1)), ("a", member(0))]),
            ),
            (
                "loop".to_string(),
                object(vec![(
                    "self",
                    platform_value!({ "$ref": "#/$defs/loop", "position": 0 }),
                )]),
            ),
        ]);
        let judge = MemberPositionOrder::new(
            Some(&definitions),
            Some(&definitions),
            PlatformVersion::latest(),
        );
        let referencing = |reference: &str| {
            platform_value!({
                "type": "object",
                "properties": { "p": { "$ref": reference, "position": 0 } },
                "additionalProperties": false,
            })
        };
        listed_out_of_position_order(
            judge.first_error_of_document_type(None, &referencing("#/$defs/outer")),
            "#/$defs/legacy",
        );
        assert_eq!(
            judge.first_error_of_document_type(None, &referencing("#/$defs/loop")),
            None
        );
        assert_eq!(
            judge.first_error_of_document_type(None, &referencing("#/$defs/missing")),
            None,
            "a $ref that points at nothing is left to the parse"
        );
    }

    /// A `$ref` a contract keeps is judged through the schema it points at: the object there,
    /// as the update writes it, keeps the order of the one the contract stores and gains members
    /// only at its next free position.
    #[test]
    fn should_hold_the_object_a_kept_ref_points_at_to_its_member_order() {
        let stored = BTreeMap::from([(
            "m".to_string(),
            object(vec![("b", member(1)), ("a", member(0))]),
        )]);
        let with =
            |members: Vec<(&str, Value)>| BTreeMap::from([("m".to_string(), object(members))]);
        let schema = platform_value!({
            "type": "object",
            "properties": { "p": { "$ref": "#/$defs/m", "position": 0 } },
            "additionalProperties": false,
        });
        let judged = |updated: &BTreeMap<String, Value>| {
            MemberPositionOrder::new(Some(&stored), Some(updated), PlatformVersion::latest())
                .first_error_of_document_type(Some(&schema), &schema)
        };
        assert_eq!(judged(&stored), None, "unchanged");
        assert_eq!(
            judged(&with(vec![
                ("b", member(1)),
                ("a", member(0)),
                ("c", member(2))
            ])),
            None,
            "a member at the next free position"
        );
        for (case, members) in [
            ("reordered", vec![("a", member(0)), ("b", member(1))]),
            (
                "a member below the next free position",
                vec![("b", member(1)), ("a", member(0)), ("c", member(5))],
            ),
        ] {
            assert!(
                matches!(
                    judged(&with(members)),
                    Some(MemberPositionOrderError::AddedBeforeExistingMembers(details))
                        if details.contains("'#/$defs/m'")
                ),
                "{case}"
            );
        }
    }

    /// An object whose highest stored position is the last integer has no next free position:
    /// no member can be added to it. One whose stored position reads as no integer (a float past
    /// every integer) takes a first added member at any position, and the next ones after it.
    #[test]
    fn should_find_no_next_free_position_past_the_last_integer() {
        let judge = MemberPositionOrder::new(None, None, PlatformVersion::latest());
        let unreadable = platform_value!({
            "type": "string",
            "maxLength": 10,
            "position": 18446744073709551616.0,
        });
        for (case, last, accepted) in [
            ("the last integer", member(u64::MAX), false),
            ("a float past every integer", unreadable, true),
        ] {
            let stored = type_with_meta(object(vec![("a", last.clone())]));
            let updated = type_with_meta(object(vec![("a", last), ("b", member(0))]));
            let error = judge.first_error_of_document_type(Some(&stored), &updated);
            assert_eq!(error.is_none(), accepted, "{case}: {error:?}");
        }
    }
}
