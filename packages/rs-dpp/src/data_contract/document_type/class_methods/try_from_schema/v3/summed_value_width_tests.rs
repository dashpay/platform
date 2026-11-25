//! The width a document type keeps its sums in (protocol version 15): 64 bits when its summed
//! property is declared within `SystemLimits::max_summed_value_magnitude_in_64_bit_sums`
//! (±2^27) on both sides, 128 bits otherwise. The type carries the outcome
//! (`DocumentTypeV2Getters::uses_big_sums`) and so does every level where one of its summing
//! indexes ends (`IndexLevelTypeInfo::big_sums`). Protocol version 14 keeps every sum in 64
//! bits.
use super::summed_value_bounds_tests::{
    amount, nothing, parse, parse_at, summed_by_the_index, summed_by_the_type, tip_schema, LIMIT,
};
use super::*;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::IndexLevel;
use platform_value::platform_value;

/// Whether each level where an index ends is marked as keeping big sums, for the levels at or
/// below `level`.
fn terminator_marks(level: &IndexLevel, marks: &mut Vec<bool>) {
    if let Some(info) = level.has_index_with_type() {
        marks.push(info.big_sums);
    }
    for sub_level in level.sub_levels().values() {
        terminator_marks(sub_level, marks);
    }
}

fn index_marks(document_type: &DocumentType) -> Vec<bool> {
    let mut marks = vec![];
    terminator_marks(document_type.index_structure(), &mut marks);
    marks
}

#[test]
fn should_keep_64_bit_sums_for_a_summed_property_within_the_limit() {
    for (minimum, maximum) in [(-LIMIT, LIMIT), (0, LIMIT), (-LIMIT, 0), (1, 5)] {
        let document_type = parse(tip_schema(
            amount(Some(minimum), Some(maximum)),
            summed_by_the_type(),
            summed_by_the_index(),
        ))
        .unwrap_or_else(|error| panic!("{minimum}..{maximum} must register: {error:?}"));
        assert!(!document_type.uses_big_sums(), "{minimum}..{maximum}");
        assert_eq!(index_marks(&document_type), vec![false]);
    }
}

#[test]
fn should_keep_128_bit_sums_for_a_summed_property_past_the_limit() {
    for (minimum, maximum) in [
        (Some(0), Some(LIMIT + 1)),
        (Some(1), Some(i64::from(u32::MAX))),
    ] {
        let document_type = parse(tip_schema(
            amount(minimum, maximum),
            summed_by_the_type(),
            summed_by_the_index(),
        ))
        .unwrap_or_else(|error| panic!("{minimum:?}..{maximum:?} must register: {error:?}"));
        assert!(document_type.uses_big_sums(), "{minimum:?}..{maximum:?}");
        assert_eq!(index_marks(&document_type), vec![true]);
    }
}

/// A contract stored before the version parses without full validation, whatever its bounds:
/// its width is decided the same way.
#[test]
fn should_keep_128_bit_sums_for_a_stored_summed_property_without_bounds() {
    for (minimum, maximum) in [
        (None, None),
        (Some(-LIMIT - 1), Some(LIMIT)),
        (None, Some(5)),
    ] {
        let document_type = parse_at(
            tip_schema(amount(minimum, maximum), nothing(), summed_by_the_index()),
            None,
            false,
            PlatformVersion::latest(),
        )
        .unwrap_or_else(|error| panic!("{minimum:?}..{maximum:?} must parse: {error:?}"));
        assert!(document_type.uses_big_sums(), "{minimum:?}..{maximum:?}");
        assert_eq!(index_marks(&document_type), vec![true]);
    }
}

/// Only the levels of the indexes that sum are marked: an index of the same type that sums
/// nothing keeps no sum to widen.
#[test]
fn should_mark_only_the_levels_of_summing_indexes() {
    let mut schema = tip_schema(
        amount(Some(0), Some(i64::from(u32::MAX))),
        summed_by_the_type(),
        summed_by_the_index(),
    );
    let Value::Array(indices) = schema
        .get_mut("indices")
        .expect("expected the indices")
        .expect("expected the indices to be set")
    else {
        panic!("expected a list of indices");
    };
    indices.push(platform_value!({
        "name": "byAmount",
        "properties": [{ "amount": "asc" }],
    }));
    let document_type = parse(schema).expect("expected the type to register");
    assert!(document_type.uses_big_sums());
    let mut marks = index_marks(&document_type);
    marks.sort();
    assert_eq!(marks, vec![false, true]);
}

#[test]
fn should_keep_64_bit_sums_for_a_type_that_sums_nothing() {
    let document_type = parse(tip_schema(amount(None, None), nothing(), nothing()))
        .expect("expected the type to register");
    assert!(!document_type.uses_big_sums());
    assert_eq!(index_marks(&document_type), vec![false]);
}

#[test]
fn should_keep_64_bit_sums_whatever_the_bounds_at_protocol_version_14() {
    let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
    for (minimum, maximum) in [
        (None, None),
        (Some(-5), None),
        (Some(0), Some(i64::from(u32::MAX))),
    ] {
        let document_type = parse_at(
            tip_schema(
                amount(minimum, maximum),
                summed_by_the_type(),
                summed_by_the_index(),
            ),
            None,
            true,
            platform_version,
        )
        .unwrap_or_else(|error| panic!("{minimum:?}..{maximum:?} must register: {error:?}"));
        assert!(!document_type.uses_big_sums(), "{minimum:?}..{maximum:?}");
        assert_eq!(index_marks(&document_type), vec![false]);
    }
}

/// The `tip` type of `tip_schema` with its `label` declared up to `max_length` characters, and
/// the index ranking its groups by their sums.
fn ranked_tip_schema(amount: Value, max_length: u64) -> Value {
    let mut schema = tip_schema(
        amount,
        summed_by_the_type(),
        platform_value!({
            "summable": "amount",
            "rangeSummable": true,
            "rankedSummable": true,
        }),
    );
    let label = schema
        .get_mut("properties")
        .expect("expected the properties")
        .expect("expected the properties to be set")
        .get_mut("label")
        .expect("expected the label")
        .expect("expected the label to be set");
    label
        .insert("maxLength".to_string(), Value::U64(max_length))
        .expect("expected to set the maximum length");
    schema
}

/// A type that keeps its sums in 128 bits ranks them on the big sum axis, whose 16-byte sort
/// key leaves a group key 239 bytes: a string of at most 59 characters (4 bytes each).
#[test]
fn should_hold_a_sum_ranking_of_128_bit_sums_to_the_big_sum_axis_key_ceiling() {
    let wide = amount(Some(0), Some(i64::from(u32::MAX)));
    let document_type = parse(ranked_tip_schema(wide.clone(), 59))
        .expect("expected a 236-byte group key to register");
    assert!(document_type.uses_big_sums());

    let error = parse(ranked_tip_schema(wide, 60))
        .expect_err("expected a 240-byte group key to be refused");
    let message = format!("{error:?}");
    assert!(
        message.contains("should be less or equal 59") && message.contains("16-byte sort key"),
        "{message}"
    );
}

/// A type that keeps its sums in 64 bits ranks them on the sum axis, whose 8-byte sort key
/// leaves a group key 247 bytes, as before.
#[test]
fn should_hold_a_sum_ranking_of_64_bit_sums_to_the_sum_axis_key_ceiling() {
    let narrow = amount(Some(0), Some(1000));
    let document_type = parse(ranked_tip_schema(narrow.clone(), 61))
        .expect("expected a 244-byte group key to register");
    assert!(!document_type.uses_big_sums());
    parse(ranked_tip_schema(narrow, 62)).expect_err("expected a 248-byte group key to be refused");
}

fn width_change_message(old: &DocumentType, new: &DocumentType) -> Option<String> {
    let result = old
        .as_ref()
        .validate_update(new.as_ref(), 2, PlatformVersion::latest())
        .expect("expected the update to be validated");
    result
        .errors
        .iter()
        .map(|error| format!("{error:?}"))
        .find(|message| message.contains("width it keeps its sums in"))
}

/// An update that moves the summed property's bounds across ±2^27 would change the width of
/// trees that already hold the type's sums, and is refused, in both directions.
#[test]
fn should_refuse_an_update_that_changes_the_width_of_the_sums() {
    let narrow = parse(tip_schema(
        amount(Some(0), Some(LIMIT)),
        summed_by_the_type(),
        summed_by_the_index(),
    ))
    .expect("expected the narrow type");
    let wide = parse(tip_schema(
        amount(Some(0), Some(LIMIT + 1)),
        summed_by_the_type(),
        summed_by_the_index(),
    ))
    .expect("expected the wide type");

    let widened = width_change_message(&narrow, &wide).expect("expected widening to be refused");
    assert!(widened.contains("from 64-bit to 128-bit"), "{widened}");
    let narrowed = width_change_message(&wide, &narrow).expect("expected narrowing to be refused");
    assert!(narrowed.contains("from 128-bit to 64-bit"), "{narrowed}");

    let also_narrow = parse(tip_schema(
        amount(Some(0), Some(LIMIT - 1)),
        summed_by_the_type(),
        summed_by_the_index(),
    ))
    .expect("expected another narrow type");
    assert_eq!(width_change_message(&narrow, &also_narrow), None);
}
