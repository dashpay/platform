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

use super::{last_value, DefinitionsRoot};
use crate::data_contract::document_type::property_names::{POSITION, PROPERTIES, REF, TYPE};
use platform_value::{Value, ValueMapHelper};
use platform_version::version::PlatformVersion;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    old_definitions: DefinitionsRoot<'a>,
    new_definitions: DefinitionsRoot<'a>,
    /// How deep a walk goes, through members and `$ref`s together: the schema depth the parse
    /// admits, which counts a schema at least as deep, so no document type the parse admits
    /// reaches it. A schema the contract gains nested deeper, reached through any `$ref`, is
    /// refused, as is a `$ref` it gains that reaches itself, that the document type parser can
    /// not follow, or whose value is not text: a walk that stopped there could not tell what
    /// lies below, and no document type could read what it reaches (the parse refuses each in
    /// a document type, and no parse reads the `$defs` of a contract with only tokens).
    ///
    /// What a contract keeps was stored with it, and is never refused for its depth: the parse
    /// bounded it in a contract with a document type, but no parse reads the `$defs` of a
    /// contract with only tokens, which may hold a cycle or a chain of any length. A pair of
    /// schemas a `$ref` the contract keeps points at is walked on its own, from a queue of the
    /// call, not inside the walk that reached it: each is walked once a call, from depth 0,
    /// whatever reaches it, so its answer is the same for every path, and no chain of `$ref`s
    /// deepens the walk. Inside it, a schema the contract keeps past this depth is left as it
    /// is when the update leaves it as it was, and refused when the update changes it, so the
    /// walk never goes deeper; what the contract gains is refused past it.
    max_depth: usize,
    /// The deepest point the walk has reached, which the walk of a `$ref` the contract gains
    /// reads as how deep the schema it points at goes.
    deepest: Cell<usize>,
    /// Whether the walk under way went deeper than `max_depth` since it was last cleared.
    cut_short: Cell<bool>,
    /// How many walks of a schema a `$ref` the contract gains points at are under way, one
    /// inside another: a refusal found inside one names the outermost `$ref` only, so a long
    /// chain does not repeat its context at every link.
    reference_nesting: Cell<usize>,
    /// What each schema a `$ref` the contract gains points at was judged, by its address in the
    /// `$defs` the judge holds: the parse reads `/` and `.` in a pointer alike, so one schema is
    /// reached by many spellings, and each is walked once. Kept as long as the judge, but for
    /// the answers of walks the depth cut short, which a call reaching the schema from less deep
    /// would answer otherwise: those are dropped at the start of each call
    /// (`cut_short_references`).
    judged_gained_references: RefCell<BTreeMap<*const Value, Judged>>,
    /// The schemas of `judged_gained_references` whose walk the depth cut short.
    cut_short_references: RefCell<Vec<*const Value>>,
}

/// How far the walk of a schema a `$ref` the contract gains points at got.
#[derive(Clone)]
enum Judged {
    /// Being walked: reaching it again is a cycle.
    Walking,
    /// Walked: its refusal, if any, and how many levels below the schema the walk went, so a
    /// `$ref` reaching it at any depth is held to the depth the walk would reach.
    Done {
        error: Option<MemberPositionOrderError>,
        height: usize,
    },
}

/// One call's walk: the stored and the new schema of the document type it walks, where the parse
/// resolves a `$ref` that does not point into `$defs` (it resolves against the type's schema
/// with the contract's `$defs` added; neither while the `$defs` are walked on their own), and the
/// pairs of schemas a `$ref` the contract keeps points at: those the call reached, by their
/// addresses, so each is walked once, and those waiting to be walked.
struct Call<'v> {
    old_type: Option<&'v Value>,
    new_type: Option<&'v Value>,
    reached_kept: RefCell<BTreeSet<[*const Value; 2]>>,
    kept: RefCell<VecDeque<KeptWork<'v>>>,
}

/// A pair of schemas a `$ref` the contract keeps points at, waiting to be walked, with the first
/// `$ref` that reached it, which a refusal found in it names.
struct KeptWork<'v> {
    old: &'v Value,
    new: &'v Value,
    path: String,
    reference: String,
}

impl<'v> Call<'v> {
    fn new(old_type: Option<&'v Value>, new_type: Option<&'v Value>) -> Self {
        Self {
            old_type,
            new_type,
            reached_kept: RefCell::new(BTreeSet::new()),
            kept: RefCell::new(VecDeque::new()),
        }
    }
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
            old_definitions: DefinitionsRoot::new(old_definitions),
            new_definitions: DefinitionsRoot::new(new_definitions),
            max_depth: platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .max_depth as usize,
            deepest: Cell::new(0),
            cut_short: Cell::new(false),
            reference_nesting: Cell::new(0),
            judged_gained_references: RefCell::new(BTreeMap::new()),
            cut_short_references: RefCell::new(Vec::new()),
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
        self.forget_answers_cut_short();
        let call = Call::new(old_schema, Some(new_schema));
        let old_properties = old_schema.and_then(inline_members);
        inline_members(new_schema)?
            .iter()
            .find_map(|(name, property)| {
                let name = name.as_text()?;
                let old_property =
                    old_properties.and_then(|properties| last_value(properties, name));
                self.first_error(old_property, property, name, &call, 0)
            })
            .or_else(|| self.first_error_of_kept_references(&call))
    }

    /// The first `$defs` definition the registration or update writes whose members' order is
    /// refused, against the same definition the contract stores, with its name.
    pub fn first_error_of_definitions(&self) -> Option<(String, MemberPositionOrderError)> {
        self.forget_answers_cut_short();
        let call = Call::new(None, None);
        self.new_definitions
            .definitions()?
            .iter()
            .find_map(|(name, definition)| {
                let old_definition = self
                    .old_definitions
                    .definitions()
                    .and_then(|definitions| definitions.get(name));
                self.first_error(
                    old_definition,
                    definition,
                    &format!("$defs.{name}"),
                    &call,
                    0,
                )
                .or_else(|| self.first_error_of_kept_references(&call))
                .map(|error| (name.clone(), error))
            })
    }

    /// The first object of the property schema `new`, or of the objects inside it or reached
    /// through the `$ref`s it gains, whose members' order is refused; `old` is the same property
    /// as the contract stores it, `None` when the contract gains it, and `path` names it. The
    /// `properties` written beside a `$ref` are not read for storage, so they are not walked. A
    /// pair a `$ref` the contract keeps points at is queued on `call`, to be walked on its own.
    fn first_error<'v>(
        &'v self,
        old: Option<&'v Value>,
        new: &'v Value,
        path: &str,
        call: &Call<'v>,
        depth: usize,
    ) -> Option<MemberPositionOrderError> {
        if old.is_none() {
            if let Some(refusal) = self.reached(depth, path) {
                return Some(refusal);
            }
        } else if depth > self.max_depth {
            // Stored with the contract deeper than any document type may be, so only a contract
            // with only tokens, whose `$defs` no parse reads, holds it: left as it is when the
            // update leaves it so, refused when the update changes it
            return if old == Some(new) {
                None
            } else {
                self.reached(depth, path)
            };
        }
        let new_schema = new.as_map()?;
        if let Some(reference) = new_schema.get_optional_key(REF) {
            let Some(reference) = reference.as_text() else {
                // A `$ref` that is not text points at nothing; one stored so stays as it is
                let stored = old
                    .and_then(Value::as_map)
                    .and_then(|old| old.get_optional_key(REF));
                return (stored != Some(reference)).then(|| {
                    MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                        "'{path}' takes its schema from a `$ref` that is not text, which the \
                         document type parser can not follow"
                    ))
                });
            };
            let old_reference = old
                .and_then(Value::as_map)
                .and_then(|old| old.get_optional_key(REF))
                .and_then(Value::as_text);
            return if old_reference == Some(reference) {
                self.queue_kept_reference(reference, path, call)
            } else {
                self.gained_reference_error(reference, path, call, depth)
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
                call,
                depth + 1,
            )
        })
    }

    /// Why a `$ref` the contract gains at `path` is refused: it does not point into `$defs`, the
    /// document type parser can not follow it there (so no document type could read what it
    /// reaches: the parse refuses it in a document type, and no parse reads the `$defs` of a
    /// contract with only tokens), or the schema it points at, or one reached from it, holds an
    /// object not listed at consecutive positions.
    fn gained_reference_error<'v>(
        &'v self,
        reference: &str,
        path: &str,
        call: &Call<'v>,
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
        let Some(target) = self.new_definitions.resolve_as_parsed(reference, None) else {
            return refused(
                "the document type parser follows it to nothing in the contract's `$defs`"
                    .to_string(),
            );
        };
        let error = self.judged_gained_reference(target, reference, call, depth);
        if self.reference_nesting.get() > 0 {
            error
        } else {
            error.map(|error| through_reference(error, path, reference))
        }
    }

    /// Queues the pair of schemas the `$ref` the contract keeps at `path` points at, to be walked
    /// on its own, unless the call reached it before. A pointer into `$defs` is resolved in each
    /// side's `$defs`, any other in each side's document type schema, as the parse resolves it.
    /// One that points at nothing on either side is left as it is.
    fn queue_kept_reference<'v>(
        &'v self,
        reference: &str,
        path: &str,
        call: &Call<'v>,
    ) -> Option<MemberPositionOrderError> {
        let old = self
            .old_definitions
            .resolve_as_parsed(reference, call.old_type)?;
        let new = self
            .new_definitions
            .resolve_as_parsed(reference, call.new_type)?;
        if !call
            .reached_kept
            .borrow_mut()
            .insert([std::ptr::from_ref(old), std::ptr::from_ref(new)])
        {
            return None;
        }
        call.kept.borrow_mut().push_back(KeptWork {
            old,
            new,
            path: path.to_string(),
            reference: reference.to_string(),
        });
        None
    }

    /// Walks the pairs queued on `call`, each on its own from depth 0, until the queue is empty
    /// or one is refused: the refusal names the first `$ref` that reached the pair.
    fn first_error_of_kept_references<'v>(
        &'v self,
        call: &Call<'v>,
    ) -> Option<MemberPositionOrderError> {
        loop {
            // The queue is borrowed for the `pop_front` only: the walk below queues on it
            let work = call.kept.borrow_mut().pop_front()?;
            if let Some(error) =
                self.first_error(Some(work.old), work.new, &work.reference, call, 0)
            {
                return Some(through_reference(error, &work.path, &work.reference));
            }
        }
    }

    /// [`Self::first_error`] for the schema a `$ref` the contract gains points at, walked once:
    /// reached again, it is held to the depth its walk went to from where it is reached now.
    fn judged_gained_reference<'v>(
        &'v self,
        new: &'v Value,
        reference: &str,
        call: &Call<'v>,
        depth: usize,
    ) -> Option<MemberPositionOrderError> {
        let key = std::ptr::from_ref(new);
        let judged = self.judged_gained_references.borrow().get(&key).cloned();
        match judged {
            Some(Judged::Walking) => {
                return Some(MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                    "the `$ref` '{reference}' reaches the schema it is written in, which no \
                     document type may"
                )))
            }
            Some(Judged::Done { error, height }) => {
                return self.reached(depth + 1 + height, reference).or(error);
            }
            None => {}
        }
        self.judged_gained_references
            .borrow_mut()
            .insert(key, Judged::Walking);
        let deepest_outside = self.deepest.replace(depth + 1);
        let cut_short_outside = self.cut_short.replace(false);
        self.reference_nesting.set(self.reference_nesting.get() + 1);
        let error = self.first_error(None, new, reference, call, depth + 1);
        self.reference_nesting.set(self.reference_nesting.get() - 1);
        let deepest_inside = self.deepest.get();
        self.deepest.set(deepest_outside.max(deepest_inside));
        let cut_short = self.cut_short.get();
        self.cut_short.set(cut_short_outside || cut_short);
        self.judged_gained_references.borrow_mut().insert(
            key,
            Judged::Done {
                error: error.clone(),
                height: deepest_inside - (depth + 1),
            },
        );
        if cut_short {
            self.cut_short_references.borrow_mut().push(key);
        }
        error
    }

    /// Records that the walk reached `depth`, refused when that is deeper than the parse admits.
    fn reached(&self, depth: usize, path: &str) -> Option<MemberPositionOrderError> {
        self.deepest.set(self.deepest.get().max(depth));
        (depth > self.max_depth).then(|| {
            self.cut_short.set(true);
            MemberPositionOrderError::ListedOutOfPositionOrder(format!(
                "the schemas reached from '{path}' are nested deeper than {} members and `$ref`s, \
                 deeper than any document type may be",
                self.max_depth
            ))
        })
    }

    /// Drops the answers of walks the depth cut short, which hold for the call that made them
    /// only: a later call reaching the schema from less deep answers otherwise.
    fn forget_answers_cut_short(&self) {
        let mut judged_gained_references = self.judged_gained_references.borrow_mut();
        for target in self.cut_short_references.borrow_mut().drain(..) {
            judged_gained_references.remove(&target);
        }
    }
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
        assert_eq!(judge.judged_gained_references.borrow().len(), 1);
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
    /// those with theirs, once each; one that reaches the schema it is written in, or points at
    /// nothing, is refused, as the parse refuses either in a document type.
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
        listed_out_of_position_order(
            judge.first_error_of_document_type(None, &referencing("#/$defs/loop")),
            "reaches the schema it is written in",
        );
        listed_out_of_position_order(
            judge.first_error_of_document_type(None, &referencing("#/$defs/missing")),
            "follows it to nothing",
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

    /// A walk that would go deeper than the parse admits is refused, so no answer the depth cut
    /// short is kept as one that passed: a schema reached that deep refuses the contract
    /// whichever `$ref` reaches it first, and the same schema reached only shallowly is judged
    /// by its members' order.
    #[test]
    fn should_refuse_a_walk_past_the_depth_whichever_ref_reaches_it_first() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .max_depth = 3;
        let definitions = BTreeMap::from([(
            "in_order".to_string(),
            object(vec![("a", member(0)), ("b", member(1))]),
        )]);
        let reference = || platform_value!({ "$ref": "#/$defs/in_order", "position": 0 });
        let holding = |inner: Value| with_position(object(vec![("x", inner)]), 0);
        // meta, meta.x, meta.x.x, then the `$ref` at depth 3, whose schema is past the depth
        let deep = type_with_meta(object(vec![("x", holding(holding(reference())))]));
        let shallow = platform_value!({
            "type": "object",
            "properties": { "p": reference() },
            "additionalProperties": false,
        });
        let judge = |schemas: &[&Value]| {
            let judge = MemberPositionOrder::new(None, Some(&definitions), &platform_version);
            schemas
                .iter()
                .find_map(|schema| judge.first_error_of_document_type(None, schema))
        };
        assert_eq!(judge(&[&shallow]), None, "reached shallowly only");
        for (case, order) in [
            ("the deep $ref first", [&deep, &shallow]),
            ("the shallow $ref first", [&shallow, &deep]),
        ] {
            assert!(
                matches!(
                    judge(&order),
                    Some(MemberPositionOrderError::ListedOutOfPositionOrder(details))
                        if details.contains("nested deeper than 3")
                ),
                "{case}"
            );
        }
    }

    /// A `$ref` cycle a contract stored in `$defs` no parse reads (a contract with only tokens)
    /// ends the walk of an update that keeps it, which still judges the members the update gives
    /// the schemas on the cycle; a cycle a registration or an update gains is refused.
    #[test]
    fn should_end_a_cycle_a_contract_keeps_and_refuse_one_it_gains() {
        let cyclic = |extra: Vec<(&str, Value)>| {
            let mut members = vec![(
                "next",
                platform_value!({ "$ref": "#/$defs/loop", "position": 0 }),
            )];
            members.extend(extra);
            BTreeMap::from([("loop".to_string(), object(members))])
        };
        let stored = cyclic(vec![]);
        let latest = PlatformVersion::latest();
        assert_eq!(
            MemberPositionOrder::new(Some(&stored), Some(&stored), latest)
                .first_error_of_definitions(),
            None,
            "kept unchanged"
        );
        assert_eq!(
            MemberPositionOrder::new(Some(&stored), Some(&cyclic(vec![("b", member(1))])), latest)
                .first_error_of_definitions(),
            None,
            "a member gained at the next free position"
        );
        assert!(
            matches!(
                MemberPositionOrder::new(
                    Some(&stored),
                    Some(&cyclic(vec![("b", member(5))])),
                    latest
                )
                .first_error_of_definitions(),
                Some((_, MemberPositionOrderError::AddedBeforeExistingMembers(_)))
            ),
            "a member gained at another position"
        );
        assert!(
            matches!(
                MemberPositionOrder::new(None, Some(&stored), latest).first_error_of_definitions(),
                Some((_, MemberPositionOrderError::ListedOutOfPositionOrder(details)))
                    if details.contains("reaches the schema it is written in")
            ),
            "registered"
        );
    }

    /// An answer a walk the depth cut short gave holds for that call only: a later call on the
    /// same judge reaching the schema from less deep answers as a judge of its own would.
    #[test]
    fn should_answer_a_later_call_as_a_fresh_judge_after_a_walk_the_depth_cut_short() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .max_depth = 3;
        let definitions = BTreeMap::from([(
            "in_order".to_string(),
            object(vec![("a", member(0)), ("b", member(1))]),
        )]);
        let reference = || platform_value!({ "$ref": "#/$defs/in_order", "position": 0 });
        let holding = |inner: Value| with_position(object(vec![("x", inner)]), 0);
        let deep = type_with_meta(object(vec![("x", holding(holding(reference())))]));
        let shallow = platform_value!({
            "type": "object",
            "properties": { "p": reference() },
            "additionalProperties": false,
        });
        let judge = MemberPositionOrder::new(None, Some(&definitions), &platform_version);
        assert!(judge.first_error_of_document_type(None, &deep).is_some());
        assert_eq!(judge.first_error_of_document_type(None, &shallow), None);
    }

    /// What a contract keeps was stored with it and is not held to the depth, so a cycle it
    /// keeps is judged alike whatever its definitions are named, however deep the chains it
    /// reaches run.
    #[test]
    fn should_judge_a_kept_cycle_alike_whatever_its_definitions_are_named() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .max_depth = 3;
        let to = |name: &str, position: u64| platform_value!({ "$ref": format!("#/$defs/{name}"), "position": position });
        let definitions = |first: &str| {
            BTreeMap::from([
                (first.to_string(), object(vec![("toB", to("b", 0))])),
                (
                    "b".to_string(),
                    object(vec![("toA", to(first, 0)), ("deep", to("c0", 1))]),
                ),
                ("c0".to_string(), object(vec![("x", to("c1", 0))])),
                ("c1".to_string(), object(vec![("x", to("c2", 0))])),
                ("c2".to_string(), object(vec![("x", to("c3", 0))])),
                ("c3".to_string(), object(vec![("s", member(0))])),
            ])
        };
        for first in ["a", "z"] {
            let stored = definitions(first);
            assert_eq!(
                MemberPositionOrder::new(Some(&stored), Some(&stored), &platform_version)
                    .first_error_of_definitions(),
                None,
                "the cycle through '{first}'"
            );
        }
    }

    /// A chain of `$ref`s a contract keeps, which a contract with only tokens may hold in
    /// `$defs` no parse reads, is walked one pair at a time from a queue, not one inside
    /// another: however long, it does not deepen the walk, and an update that keeps it is
    /// accepted. A registration gaining the same chain is refused once it runs deeper than the
    /// parse admits.
    #[test]
    fn should_walk_a_kept_chain_of_any_length_without_deepening_the_walk() {
        let links = 4096;
        let mut chain: BTreeMap<String, Value> = (0..links)
            .map(|link| {
                (
                    format!("d{link:05}"),
                    object(vec![(
                        "next",
                        platform_value!({
                            "$ref": format!("#/$defs/d{:05}", link + 1),
                            "position": 0,
                        }),
                    )]),
                )
            })
            .collect();
        chain.insert(format!("d{links:05}"), object(vec![("a", member(0))]));
        let latest = PlatformVersion::latest();
        assert_eq!(
            MemberPositionOrder::new(Some(&chain), Some(&chain), latest)
                .first_error_of_definitions(),
            None,
            "kept"
        );
        assert!(
            matches!(
                MemberPositionOrder::new(None, Some(&chain), latest).first_error_of_definitions(),
                Some((_, MemberPositionOrderError::ListedOutOfPositionOrder(details)))
                    if details.contains("nested deeper than")
            ),
            "gained"
        );
    }

    /// A schema a contract keeps is judged from where its own walk starts, whatever path of
    /// `$ref`s reaches it: a member it gains is held to the depth below it, so the verdict is the
    /// same whichever path the walk takes first.
    #[test]
    fn should_judge_a_kept_schema_alike_whichever_path_reaches_it_first() {
        for max_depth in [3, 5] {
            judge_a_kept_schema_alike_whichever_path_reaches_it_first(max_depth);
        }
    }

    fn judge_a_kept_schema_alike_whichever_path_reaches_it_first(max_depth: u16) {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .max_depth = max_depth;
        let to = |name: &str| platform_value!({ "$ref": format!("#/$defs/{name}"), "position": 0 });
        let definitions = |long: &str, gained: bool| {
            let mut shared = vec![("a", member(0))];
            if gained {
                shared.push((
                    "x",
                    with_position(
                        object(vec![(
                            "y",
                            with_position(object(vec![("z", member(0))]), 0),
                        )]),
                        1,
                    ),
                ));
            }
            BTreeMap::from([
                (
                    format!("{long}0"),
                    object(vec![("next", to(&format!("{long}1")))]),
                ),
                (
                    format!("{long}1"),
                    object(vec![("next", to(&format!("{long}2")))]),
                ),
                (format!("{long}2"), object(vec![("next", to("shared"))])),
                ("m".to_string(), object(vec![("next", to("shared"))])),
                ("shared".to_string(), object(shared)),
            ])
        };
        for long in ["a", "z"] {
            let stored = definitions(long, false);
            let updated = definitions(long, true);
            assert_eq!(
                MemberPositionOrder::new(Some(&stored), Some(&updated), &platform_version)
                    .first_error_of_definitions(),
                None,
                "the long path named '{long}', max_depth {max_depth}"
            );
        }
    }

    /// A `$ref` a contract gains that points at nothing is refused: in a contract with only
    /// tokens no parse reads its `$defs`, and no document type could reach the definition.
    #[test]
    fn should_refuse_a_gained_ref_that_points_at_nothing() {
        let definitions = BTreeMap::from([(
            "added".to_string(),
            object(vec![(
                "x",
                platform_value!({ "$ref": "#/$defs/missing", "position": 0 }),
            )]),
        )]);
        listed_out_of_position_order(
            MemberPositionOrder::new(None, Some(&definitions), PlatformVersion::latest())
                .first_error_of_definitions()
                .map(|(_, error)| error),
            "follows it to nothing",
        );
    }

    /// A `$ref` whose value is not text points at nothing a document type could read: one a
    /// contract gains is refused, one it stores stays as it is.
    #[test]
    fn should_refuse_a_gained_ref_that_is_not_text_and_keep_a_stored_one() {
        let definitions = BTreeMap::from([(
            "added".to_string(),
            object(vec![("x", platform_value!({ "$ref": 7, "position": 0 }))]),
        )]);
        let latest = PlatformVersion::latest();
        listed_out_of_position_order(
            MemberPositionOrder::new(None, Some(&definitions), latest)
                .first_error_of_definitions()
                .map(|(_, error)| error),
            "not text",
        );
        assert_eq!(
            MemberPositionOrder::new(Some(&definitions), Some(&definitions), latest)
                .first_error_of_definitions(),
            None
        );
    }

    /// A schema a contract keeps past the depth (which only a contract with only tokens can
    /// hold) is left as it is when the update leaves it so, and refused when the update changes
    /// it, so the walk never goes deeper and no change below it goes unjudged.
    #[test]
    fn should_keep_a_kept_schema_past_the_depth_only_as_it_was() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .max_depth = 3;
        let nested = |innermost: Vec<(&str, Value)>| {
            let mut schema = object(innermost);
            for _ in 0..5 {
                schema = object(vec![("a", with_position(schema, 0))]);
            }
            BTreeMap::from([("d".to_string(), schema)])
        };
        let stored = nested(vec![("s", member(0))]);
        assert_eq!(
            MemberPositionOrder::new(Some(&stored), Some(&stored), &platform_version)
                .first_error_of_definitions(),
            None,
            "unchanged"
        );
        for (case, updated) in [
            (
                "a member at its next free position",
                nested(vec![("s", member(0)), ("t", member(1))]),
            ),
            (
                "a member at another position",
                nested(vec![("s", member(0)), ("t", member(7))]),
            ),
        ] {
            assert!(
                matches!(
                    MemberPositionOrder::new(Some(&stored), Some(&updated), &platform_version)
                        .first_error_of_definitions(),
                    Some((_, MemberPositionOrderError::ListedOutOfPositionOrder(details)))
                        if details.contains("nested deeper than 3")
                ),
                "{case}"
            );
        }
    }
}
