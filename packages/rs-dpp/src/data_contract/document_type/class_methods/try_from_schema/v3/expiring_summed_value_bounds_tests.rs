//! A document type with a `ttl` and a summed property (protocol version 14): unless the
//! summed property's schema keeps its values at 0 or above, it must keep them within
//! `SystemLimits::max_expiring_signed_summed_value_magnitude` (±2^27), since deleting an
//! expired document takes its value out of the type's sums. A registration rule: a stored
//! contract still parses.
use super::*;
use crate::data_contract::config::v1::DataContractConfigSettersV1;
use platform_value::platform_value;

const LIMIT: i64 = 1 << 27;

fn parse_at(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available");
    parse_with_config(schema, schema_defs, &config, full_validation)
}

fn parse_with_config(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    config: &DataContractConfig,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "story",
        schema,
        schema_defs,
        &BTreeMap::new(),
        config,
        full_validation,
        &mut vec![],
        PlatformVersion::latest(),
    )
}

fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
    parse_at(schema, None, true)
}

/// Parsed in a contract that does not size its integers, where `amount` is an `i64` whatever
/// its bounds are.
fn parse_unsized(schema: Value) -> Result<DocumentType, ProtocolError> {
    let mut config = DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available");
    config.set_sized_integer_types_enabled(false);
    parse_with_config(schema, None, &config, true)
}

/// An integer `amount` with the schema bounds `minimum` and `maximum`, as written
fn amount_bounded_by(minimum: Value, maximum: Value) -> Value {
    platform_value!({
        "type": "integer",
        "position": 1,
        "minimum": minimum,
        "maximum": maximum,
    })
}

/// A `story` type of `label` and `amount`, indexed `byLabel`, with a `ttl` when `expiring`,
/// carrying `type_keywords` and, on its index, `index_keywords`. `amount` has the schema
/// `amount`.
fn story_schema(
    amount: Value,
    expiring: bool,
    type_keywords: Value,
    index_keywords: Value,
) -> Value {
    let mut index = platform_value!({
        "name": "byLabel",
        "properties": [{ "label": "asc" }],
    });
    let Value::Map(index_keywords) = index_keywords else {
        panic!("expected the index keywords to be a map");
    };
    for (key, value) in index_keywords {
        index
            .insert(key.as_text().expect("a text key").to_string(), value)
            .expect("expected to set the index keyword");
    }
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "label": { "type": "string", "maxLength": 20, "position": 0 },
            "amount": amount,
        },
        "required": ["$createdAt", "label", "amount"],
        "indices": [index],
        "additionalProperties": false,
    });
    if expiring {
        schema
            .insert("ttl".to_string(), Value::U32(86_400))
            .expect("expected to set the ttl");
    }
    let Value::Map(type_keywords) = type_keywords else {
        panic!("expected the type keywords to be a map");
    };
    for (key, value) in type_keywords {
        schema
            .insert(key.as_text().expect("a text key").to_string(), value)
            .expect("expected to set the type keyword");
    }
    schema
}

fn amount(minimum: Option<i64>, maximum: Option<i64>) -> Value {
    let mut amount = platform_value!({ "type": "integer", "position": 1 });
    if let Some(minimum) = minimum {
        amount
            .insert("minimum".to_string(), Value::I64(minimum))
            .expect("expected to set the minimum");
    }
    if let Some(maximum) = maximum {
        amount
            .insert("maximum".to_string(), Value::I64(maximum))
            .expect("expected to set the maximum");
    }
    amount
}

fn documents_summable() -> Value {
    platform_value!({ "documentsSummable": "amount" })
}

fn assert_refused(result: Result<DocumentType, ProtocolError>) {
    let error = result.expect_err("the document type must be refused");
    // A paid refusal needs the consensus variant: the data contract error variant would
    // surface as an internal error in a block.
    assert!(
        matches!(error, ProtocolError::ConsensusError(_)),
        "expected a consensus error, got {error:?}"
    );
    let message = format!("{error:?}");
    for fragment in [
        "sets `ttl` and sums \\\"amount\\\"",
        "a minimum of at least 0",
        &format!("at least -{LIMIT}"),
        &format!("at most {LIMIT}"),
    ] {
        assert!(
            message.contains(fragment),
            "error must name {fragment}, got {message}"
        );
    }
}

#[test]
fn should_refuse_an_expiring_type_summing_a_property_without_bounds() {
    assert_refused(parse(story_schema(
        amount(None, None),
        true,
        documents_summable(),
        platform_value!({}),
    )));
}

#[test]
fn should_refuse_an_expiring_index_summing_a_negative_property_above_the_limit() {
    assert_refused(parse(story_schema(
        amount(Some(-1), Some(LIMIT + 1)),
        true,
        platform_value!({}),
        platform_value!({ "summable": "amount" }),
    )));
}

#[test]
fn should_refuse_an_expiring_type_averaging_a_property_below_the_negated_limit() {
    assert_refused(parse(story_schema(
        amount(Some(-LIMIT - 1), Some(0)),
        true,
        platform_value!({ "documentsAverageable": "amount" }),
        platform_value!({}),
    )));
}

/// Without a `minimum` the values may be as negative as an `i64` allows, whatever the
/// `maximum`.
#[test]
fn should_refuse_an_expiring_summed_property_bounded_above_only() {
    assert_refused(parse(story_schema(
        amount(None, Some(1_000)),
        true,
        documents_summable(),
        platform_value!({}),
    )));
}

/// A negative `minimum` within the limit still needs a `maximum` within it: the sums the
/// negative values leave can be as large as the positive ones make them.
#[test]
fn should_refuse_an_expiring_signed_summed_property_without_a_maximum() {
    assert_refused(parse(story_schema(
        amount(Some(-1), None),
        true,
        documents_summable(),
        platform_value!({}),
    )));
}

/// Removing values that are never negative only lowers the sums, so they need no other
/// bound than the one the sum itself requires.
#[test]
fn should_register_an_expiring_summed_property_that_is_never_negative() {
    for (minimum, maximum) in [(0, i64::from(u32::MAX)), (1, LIMIT + 1), (0, 1)] {
        parse(story_schema(
            amount(Some(minimum), Some(maximum)),
            true,
            documents_summable(),
            platform_value!({}),
        ))
        .unwrap_or_else(|error| panic!("[{minimum}, {maximum}] registers: {error:?}"));
    }
}

#[test]
fn should_register_an_expiring_signed_summed_property_within_the_limit() {
    for (minimum, maximum) in [(-LIMIT, LIMIT), (-1, 1), (-LIMIT, 0)] {
        parse(story_schema(
            amount(Some(minimum), Some(maximum)),
            true,
            documents_summable(),
            platform_value!({}),
        ))
        .unwrap_or_else(|error| panic!("[{minimum}, {maximum}] registers: {error:?}"));
    }
}

/// The bounds are read where the property's `$ref` points.
#[test]
fn should_read_the_bounds_through_a_ref() {
    let reference = platform_value!({ "$ref": "#/$defs/amount", "position": 1 });
    let never_negative = BTreeMap::from([(
        "amount".to_string(),
        platform_value!({ "type": "integer", "minimum": 0, "maximum": 1_000_000_000 }),
    )]);
    parse_at(
        story_schema(
            reference.clone(),
            true,
            documents_summable(),
            platform_value!({}),
        ),
        Some(&never_negative),
        true,
    )
    .expect("a referenced amount that is never negative registers");

    let unbounded =
        BTreeMap::from([("amount".to_string(), platform_value!({ "type": "integer" }))]);
    assert_refused(parse_at(
        story_schema(reference, true, documents_summable(), platform_value!({})),
        Some(&unbounded),
        true,
    ));
}

#[test]
fn should_leave_a_summed_type_without_a_ttl_unbounded() {
    parse(story_schema(
        amount(None, None),
        false,
        documents_summable(),
        platform_value!({}),
    ))
    .expect("a summed type without a ttl registers without bounds");
}

#[test]
fn should_leave_an_expiring_type_without_a_sum_unbounded() {
    parse(story_schema(
        amount(None, None),
        true,
        platform_value!({}),
        platform_value!({}),
    ))
    .expect("an expiring type without a sum registers without bounds");
}

/// A contract registered before the rule is read back without full validation.
#[test]
fn should_read_back_a_stored_expiring_type_summing_a_property_without_bounds() {
    parse_at(
        story_schema(
            amount(None, None),
            true,
            documents_summable(),
            platform_value!({}),
        ),
        None,
        false,
    )
    .expect("a stored contract parses");
}

/// The meta-schema admits any number as a bound. A `minimum` of at least 0 settles the rule
/// however the `maximum` is written: a float, or an integer past `i64::MAX`.
#[test]
fn should_register_a_never_negative_property_whatever_number_bounds_it_above() {
    for maximum in [Value::Float(1.5), Value::U64(u64::MAX)] {
        parse_unsized(story_schema(
            amount_bounded_by(Value::U64(0), maximum.clone()),
            true,
            documents_summable(),
            platform_value!({}),
        ))
        .unwrap_or_else(|error| panic!("a maximum of {maximum:?} registers: {error:?}"));
    }
}

/// Bounds written as floats are compared as numbers, on both sides of the limit.
#[test]
fn should_compare_float_bounds_of_a_signed_property_with_the_limit() {
    parse_unsized(story_schema(
        amount_bounded_by(Value::Float(-1.5), Value::Float(1.5)),
        true,
        documents_summable(),
        platform_value!({}),
    ))
    .expect("float bounds within the limit register");

    let past_the_limit = -(LIMIT as f64) - 0.5;
    assert_refused(parse_unsized(story_schema(
        amount_bounded_by(Value::Float(past_the_limit), Value::Float(1.5)),
        true,
        documents_summable(),
        platform_value!({}),
    )));
}
