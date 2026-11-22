//! The order an object lists its members in, read against their `position`s.
//!
//! An object stores its members in the order its `properties` map lists them, at every protocol
//! version, while a document type's own properties are stored in `position` order. From protocol
//! version 15 an object a contract gains (at registration, or in an update that adds it) must
//! list its members in position order, so for every such object the two orders agree and its
//! members are stored as a document type's own properties are. An object a contract already has
//! keeps the order it was registered with, which before protocol version 15 could differ from
//! its position order: a member an update adds to it comes after its existing ones in both
//! orders, so every document stored before reads the added member as absent.
//!
//! Reads the schemas as written, and every read is infallible: a schema of another shape is the
//! business of the checks that parse it.

use super::last_value;
use crate::data_contract::document_type::property_names::{POSITION, PROPERTIES, REF};
use platform_value::{Value, ValueMapHelper};
use std::collections::BTreeSet;

/// Why an object's members are refused by their order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberPositionOrderError {
    /// An object the contract gains lists its members in an order other than their positions.
    ListedOutOfPositionOrder(String),
    /// A member an update adds to an existing object comes before one of the object's members,
    /// in the map or in position order, or the added members are not listed in position order.
    AddedBeforeExistingMembers(String),
}

/// The first object of the property schema `new`, or of the objects inside it at any depth,
/// whose members' order is refused; `old` is the same property as the contract stores it, `None`
/// when the update (or the registration) adds it, and `path` names it.
///
/// An object that takes its members from a `$ref` is judged where its definition is
/// (`$defs`), and the `properties` written beside a `$ref` are not read for storage, so neither
/// is walked here.
pub fn first_member_position_order_error(
    old: Option<&Value>,
    new: &Value,
    path: &str,
) -> Option<MemberPositionOrderError> {
    let new_members = inline_members(new)?;
    let old_members = old.and_then(inline_members);
    let error = match old_members {
        None => listed_out_of_position_order(new_members, path),
        Some(old_members) => added_before_existing_members(old_members, new_members, path),
    };
    if error.is_some() {
        return error;
    }
    new_members.iter().find_map(|(name, member)| {
        let name = name.as_text()?;
        let old_member = old_members.and_then(|old_members| last_value(old_members, name));
        first_member_position_order_error(old_member, member, &format!("{path}.{name}"))
    })
}

/// [`first_member_position_order_error`] for every object of the document type schema
/// `new_schema`: each of its own properties, whose order is their `position`s whatever the map
/// says, is walked against the same property of `old_schema`, the type as the contract stores
/// it (`None` for a type the contract gains).
pub fn first_member_position_order_error_of_document_type(
    old_schema: Option<&Value>,
    new_schema: &Value,
) -> Option<MemberPositionOrderError> {
    let old_properties = old_schema.and_then(inline_members);
    inline_members(new_schema)?
        .iter()
        .find_map(|(name, property)| {
            let name = name.as_text()?;
            let old_property = old_properties.and_then(|properties| last_value(properties, name));
            first_member_position_order_error(old_property, property, name)
        })
}

/// The members of `schema` when it is an object written inline: its `properties` map.
fn inline_members(schema: &Value) -> Option<&[(Value, Value)]> {
    let schema = schema.as_map()?;
    if schema.get_optional_key(REF).is_some() {
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

/// `None` when the positions of `members`, in map order, rise; also when one is not an integer,
/// which the parse refuses.
fn listed_out_of_position_order(
    members: &[(Value, Value)],
    path: &str,
) -> Option<MemberPositionOrderError> {
    let positions: Option<Vec<(&str, u64)>> = members
        .iter()
        .map(|(name, member)| Some((name.as_text()?, position(member)?)))
        .collect();
    let positions = positions?;
    let out_of_order = positions.windows(2).find(|pair| pair[0].1 >= pair[1].1)?;
    Some(MemberPositionOrderError::ListedOutOfPositionOrder(format!(
        "the object '{path}' must list its members in position order: '{}' at position {} is \
         listed before '{}' at position {}",
        out_of_order[0].0, out_of_order[0].1, out_of_order[1].0, out_of_order[1].1
    )))
}

/// `None` when every member of `new_members` that `old_members` does not hold is listed after
/// all of those it holds, takes a position above all of theirs, and the added members are listed
/// in position order.
fn added_before_existing_members(
    old_members: &[(Value, Value)],
    new_members: &[(Value, Value)],
    path: &str,
) -> Option<MemberPositionOrderError> {
    let existing: BTreeSet<&str> = old_members
        .iter()
        .filter_map(|(name, _)| name.as_text())
        .collect();
    let highest_existing = old_members
        .iter()
        .filter_map(|(_, member)| position(member))
        .max();
    let mut previous_added: Option<(&str, u64)> = None;
    let mut first_added: Option<&str> = None;
    for (name, member) in new_members {
        let Some(name) = name.as_text() else {
            continue;
        };
        if existing.contains(name) {
            if let Some(added) = first_added {
                return Some(MemberPositionOrderError::AddedBeforeExistingMembers(
                    format!(
                    "the member '{added}' an update adds to the object '{path}' is listed before \
                     its existing member '{name}'"
                ),
                ));
            }
            continue;
        }
        first_added.get_or_insert(name);
        let Some(added_position) = position(member) else {
            continue;
        };
        if let Some(highest) = highest_existing.filter(|highest| added_position <= *highest) {
            return Some(MemberPositionOrderError::AddedBeforeExistingMembers(
                format!(
                    "the member '{name}' an update adds to the object '{path}' takes position \
                 {added_position}, which is not above every existing member's (the highest is \
                 {highest})"
                ),
            ));
        }
        if let Some((previous, previous_position)) =
            previous_added.filter(|(_, previous_position)| added_position <= *previous_position)
        {
            return Some(MemberPositionOrderError::AddedBeforeExistingMembers(
                format!(
                    "the members an update adds to the object '{path}' must be listed in position \
                 order: '{previous}' at position {previous_position} is listed before '{name}' at \
                 position {added_position}"
                ),
            ));
        }
        previous_added = Some((name, added_position));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn object(members: Value) -> Value {
        platform_value!({ "type": "object", "properties": members, "additionalProperties": false })
    }

    fn member(position: u64) -> Value {
        platform_value!({ "type": "string", "maxLength": 10, "position": position })
    }

    #[test]
    fn should_hold_an_object_the_contract_gains_to_position_order_at_every_depth() {
        let in_order = object(platform_value!({ "a": member(0), "b": member(1) }));
        assert_eq!(
            first_member_position_order_error(None, &in_order, "meta"),
            None
        );
        // An object may start above 0, as DPNS records does
        let from_one = object(platform_value!({ "identity": member(1) }));
        assert_eq!(
            first_member_position_order_error(None, &from_one, "records"),
            None
        );

        let out_of_order = object(platform_value!({ "b": member(1), "a": member(0) }));
        assert!(matches!(
            first_member_position_order_error(None, &out_of_order, "meta"),
            Some(MemberPositionOrderError::ListedOutOfPositionOrder(details))
                if details.contains("'meta'") && details.contains("'b' at position 1")
        ));
        let nested = object(platform_value!({
            "a": member(0),
            "inner": with_position(out_of_order, 1),
        }));
        assert!(matches!(
            first_member_position_order_error(None, &nested, "meta"),
            Some(MemberPositionOrderError::ListedOutOfPositionOrder(details))
                if details.contains("'meta.inner'")
        ));
    }

    fn with_position(mut schema: Value, position: u64) -> Value {
        schema
            .insert("position".to_string(), Value::U64(position))
            .expect("expected a map");
        schema
    }

    /// An object stored before protocol version 15 may list its members out of position order:
    /// it stays updatable, and a member it gains comes after its existing ones in both orders.
    #[test]
    fn should_let_an_existing_object_gain_members_after_its_existing_ones_in_both_orders() {
        let stored = object(platform_value!({
            "slug": member(2),
            "title": member(0),
            "description": member(1),
        }));
        let with = |added: Value| {
            let mut members = stored
                .get_value("properties")
                .unwrap()
                .as_map()
                .unwrap()
                .clone();
            members.extend(added.as_map().unwrap().iter().cloned());
            object(Value::Map(members))
        };

        for (case, new) in [
            ("unchanged", stored.clone()),
            (
                "one member after",
                with(platform_value!({ "layout": member(3) })),
            ),
            (
                "two members after, in position order",
                with(platform_value!({ "layout": member(3), "theme": member(4) })),
            ),
        ] {
            assert_eq!(
                first_member_position_order_error(Some(&stored), &new, "metadata"),
                None,
                "{case}"
            );
        }

        let listed_first = object(platform_value!({
            "layout": member(3),
            "slug": member(2),
            "title": member(0),
            "description": member(1),
        }));
        for (case, new, fragment) in [
            (
                "listed before the existing members",
                listed_first,
                "listed before",
            ),
            (
                "a position below the highest existing one",
                with(platform_value!({ "layout": member(1) })),
                "not above",
            ),
            (
                "added members out of position order",
                with(platform_value!({ "theme": member(4), "layout": member(3) })),
                "in position order",
            ),
        ] {
            assert!(
                matches!(
                    first_member_position_order_error(Some(&stored), &new, "metadata"),
                    Some(MemberPositionOrderError::AddedBeforeExistingMembers(details))
                        if details.contains(fragment)
                ),
                "{case}"
            );
        }
    }

    /// An object an update adds inside an existing one is held to position order, as at
    /// registration; one that takes its members from a `$ref` is judged at its definition.
    #[test]
    fn should_hold_an_object_added_to_an_existing_one_to_position_order() {
        let stored = object(platform_value!({ "a": member(0) }));
        let added = object(platform_value!({
            "a": member(0),
            "inner": with_position(object(platform_value!({ "q": member(1), "p": member(0) })), 1),
        }));
        assert!(matches!(
            first_member_position_order_error(Some(&stored), &added, "meta"),
            Some(MemberPositionOrderError::ListedOutOfPositionOrder(details))
                if details.contains("'meta.inner'")
        ));

        let through_ref = platform_value!({ "$ref": "#/$defs/d", "position": 0 });
        assert_eq!(
            first_member_position_order_error(None, &through_ref, "shared"),
            None
        );
    }
}
