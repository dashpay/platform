//! `integerRange` in the index grammar: what [`Index::try_from_value_map`]
//! accepts and refuses on its own. The rules that need the document schema
//! (an integer source, listed in `required`) or the platform version (the
//! overlap cap) are pinned in the `try_from_schema` tests.

use super::{Index, IndexGrammarAdmissions, IntegerRangeKeyType, IntegerRangeTransform};
use crate::data_contract::errors::DataContractError;
use platform_value::{Value, ValueMap};
use serde_json::json;

fn admissions(integer_range: bool) -> IndexGrammarAdmissions {
    IndexGrammarAdmissions {
        ranked: true,
        time_range: true,
        integer_range,
        terminal: false,
        preallocated: false,
        outlives_delete: false,
        skip_if_absent: false,
        summable_off_count_index: false,
        range_countable_implies_countable: true,
        no_locking_resolution: false,
    }
}

fn parse_with(index: serde_json::Value, integer_range: bool) -> Result<Index, DataContractError> {
    let value = platform_value::to_value(index).expect("the index should convert");
    let map = value.to_map().expect("the index should be a map").clone();
    Index::try_from_value_map(map.as_slice(), admissions(integer_range))
}

fn parse(index: serde_json::Value) -> Result<Index, DataContractError> {
    parse_with(index, true)
}

fn refusal(index: serde_json::Value) -> String {
    parse(index)
        .expect_err("the index should be refused")
        .to_string()
}

#[test]
fn should_parse_an_integer_range_with_the_phase_defaulting_to_zero() {
    let index = parse(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }, { "category": "asc" }],
        "integerRange": { "on": "price", "range": 300, "step": 100 }
    }))
    .expect("the index should parse");
    let transform = index
        .integer_range
        .clone()
        .expect("the transform should be set");
    assert_eq!(transform.source, "price");
    assert_eq!(transform.range, 300);
    assert_eq!(transform.step, 100);
    assert_eq!(transform.phase, 0);
    assert_eq!(transform.overlap_factor(), 3);
    assert!(index.time_range.is_none());
    assert!(index.is_bucketed());
}

#[test]
fn should_parse_a_phase_below_the_step() {
    let index = parse(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 100, "phase": 99 }
    }))
    .expect("the index should parse");
    assert_eq!(index.integer_range.expect("set").phase, 99);
}

#[test]
fn should_refuse_a_phase_of_a_whole_step_or_more() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 100, "phase": 100 }
    }));
    assert!(message.contains("phase"), "{message}");
}

#[test]
fn should_refuse_a_range_that_is_not_a_multiple_of_the_step() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 250, "step": 100 }
    }));
    assert!(message.contains("exact multiple"), "{message}");
}

#[test]
fn should_refuse_a_zero_step_or_range() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 0 }
    }));
    assert!(
        message.contains("step must be greater than zero"),
        "{message}"
    );
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 0, "step": 100 }
    }));
    assert!(
        message.contains("range must be greater than zero"),
        "{message}"
    );
}

#[test]
fn should_refuse_a_negative_parameter() {
    let error = parse(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 100, "phase": -1 }
    }))
    .expect_err("a negative phase should be refused");
    assert!(error.to_string().contains("non-negative"), "{error}");
}

#[test]
fn should_refuse_a_source_that_is_not_the_first_property() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "category": "asc" }, { "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }));
    assert!(message.contains("first index property"), "{message}");
}

#[test]
fn should_refuse_missing_and_unknown_fields() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100 }
    }));
    assert!(message.contains("`step`"), "{message}");
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 100, "ttl": 100 }
    }));
    assert!(
        message.contains("unexpected integerRange field: ttl"),
        "{message}"
    );
}

#[test]
fn should_allow_uniqueness_only_over_non_overlapping_windows() {
    let index = parse(json!({
        "name": "onePerBand",
        "properties": [{ "price": "asc" }, { "$ownerId": "asc" }],
        "unique": true,
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }))
    .expect("non-overlapping windows may be unique");
    assert!(index.unique);

    let message = refusal(json!({
        "name": "onePerBand",
        "properties": [{ "price": "asc" }, { "$ownerId": "asc" }],
        "unique": true,
        "integerRange": { "on": "price", "range": 200, "step": 100 }
    }));
    assert!(message.contains("cannot be unique"), "{message}");
}

#[test]
fn should_refuse_null_searchable_false() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "nullSearchable": false,
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }));
    assert!(message.contains("nullSearchable"), "{message}");
}

#[test]
fn should_refuse_ranking_the_bucketed_level() {
    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "countable": "countable",
        "rangeCountable": true,
        "rankedCountable": true,
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }));
    assert!(
        message.contains("single-property integerRange"),
        "{message}"
    );

    let message = refusal(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }, { "category": "asc" }],
        "countable": "countable",
        "rangeCountable": true,
        "rankedCountable": { "at": "price" },
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }));
    assert!(message.contains("rankedCountable.at"), "{message}");

    // Ranking below the bucketed level is one leaderboard per window.
    parse(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }, { "category": "asc" }],
        "countable": "countable",
        "rangeCountable": true,
        "rankedCountable": true,
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }))
    .expect("a ranking below the bucketed level should parse");
}

#[test]
fn should_refuse_time_and_integer_ranges_together() {
    let message = refusal(json!({
        "name": "both",
        "properties": [{ "$createdAt": "asc" }],
        "timeRange": { "on": "$createdAt", "range": 3_600, "step": 3_600 },
        "integerRange": { "on": "$createdAt", "range": 100, "step": 100 }
    }));
    assert!(
        message.contains("both timeRange and integerRange"),
        "{message}"
    );
}

#[test]
fn should_treat_the_keyword_as_unknown_before_its_generation() {
    let error = parse_with(
        json!({
            "name": "byPrice",
            "properties": [{ "price": "asc" }],
            "integerRange": { "on": "price", "range": 100, "step": 100 }
        }),
        false,
    )
    .expect_err("the keyword should be unknown");
    assert!(
        error.to_string().contains("unexpected property name"),
        "{error}"
    );
}

#[test]
fn should_key_the_bucketed_level_by_the_grid() {
    let index = parse(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }, { "category": "asc" }],
        "integerRange": { "on": "price", "range": 300, "step": 100, "phase": 5 }
    }))
    .expect("the index should parse");
    assert_eq!(index.level_key(0, "price"), "price#300#100#5");
    assert_eq!(index.level_key(1, "category"), "category");
    assert_eq!(index.level_key_for_property("price"), "price#300#100#5");
    assert_eq!(index.level_key_for_property("category"), "category");
}

#[test]
fn should_conflict_only_within_one_window() {
    let mut index = parse(json!({
        "name": "onePerBand",
        "properties": [{ "price": "asc" }, { "$ownerId": "asc" }],
        "unique": true,
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }))
    .expect("the index should parse");
    if let Some(transform) = index.integer_range.as_mut() {
        transform.key_type = IntegerRangeKeyType::U32;
    }
    let document = |price: u64| -> ValueMap {
        vec![
            (Value::Text("price".to_string()), Value::U64(price)),
            (Value::Text("$ownerId".to_string()), Value::U64(7)),
        ]
    };
    assert!(index.objects_are_conflicting(&document(120), &document(199)));
    assert!(!index.objects_are_conflicting(&document(199), &document(200)));
}

#[test]
fn should_describe_the_grid_through_the_bucketing_view() {
    let index = parse(json!({
        "name": "byPrice",
        "properties": [{ "price": "asc" }],
        "integerRange": { "on": "price", "range": 100, "step": 100 }
    }))
    .expect("the index should parse");
    let bucketing = index.bucketing().expect("the index is bucketed");
    assert_eq!(bucketing.keyword(), "integerRange");
    assert_eq!(bucketing.source(), "price");
    assert!(bucketing.time_range().is_none());
    assert!(index.is_bucketed_by(&bucketing));
    let other_grid = IntegerRangeTransform {
        step: 50,
        ..bucketing.integer_range().expect("integer grid").clone()
    };
    assert!(!index.is_bucketed_by(&other_grid.into()));
}
