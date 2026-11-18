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
