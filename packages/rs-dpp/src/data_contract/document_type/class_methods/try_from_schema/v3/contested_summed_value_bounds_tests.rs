//! A document type with a contested index and a summed property (protocol version 14): the
//! summed property's schema must keep its values within
//! `SystemLimits::max_contested_summed_value_magnitude` (±2^27), since awarding a contest
//! adds the winner's value to the type's sums. A registration rule: a stored contract still
//! parses, and earlier generations keep registering any bounds.
use super::*;
use platform_value::platform_value;

const LIMIT: i64 = 1 << 27;

fn parse_at(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    protocol_version: u32,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "name",
        schema,
        schema_defs,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
    parse_at(
        schema,
        None,
        PlatformVersion::latest().protocol_version,
        true,
    )
}

/// An immutable `name` type of `label` and `amount`, whose `byLabel` index is unique, with
/// `contested` when given, and carries `index_keywords`. `amount` has the schema `amount`,
/// and the type carries `type_keywords`.
fn name_schema(
    amount: Value,
    contested: bool,
    type_keywords: Value,
    index_keywords: Value,
) -> Value {
    let mut index = platform_value!({
        "name": "byLabel",
        "properties": [{ "label": "asc" }],
        "unique": true,
    });
    if contested {
        index
            .insert(
                "contested".to_string(),
                platform_value!({
                    "fieldMatches": [{ "field": "label", "regexPattern": "^[a-z]{3,5}$" }],
                    "resolution": 0,
                }),
            )
            .expect("expected to set contested");
    }
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
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "label": { "type": "string", "maxLength": 20, "position": 0 },
            "amount": amount,
        },
        "required": ["label", "amount"],
        "indices": [index],
        "additionalProperties": false,
    });
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
        "sums \\\"amount\\\"",
        "contested index \\\"byLabel\\\"",
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
fn should_refuse_a_contested_type_summing_a_property_without_bounds() {
    assert_refused(parse(name_schema(
        amount(None, None),
        true,
        documents_summable(),
        platform_value!({}),
    )));
}

#[test]
fn should_refuse_a_contested_index_summing_a_property_above_the_limit() {
    assert_refused(parse(name_schema(
        amount(Some(0), Some(LIMIT + 1)),
        true,
        platform_value!({}),
        platform_value!({ "summable": "amount" }),
    )));
}

#[test]
fn should_refuse_a_contested_type_averaging_a_property_below_the_negated_limit() {
    assert_refused(parse(name_schema(
        amount(Some(-LIMIT - 1), Some(0)),
        true,
        platform_value!({ "documentsAverageable": "amount" }),
        platform_value!({}),
    )));
}

/// A missing bound admits every value on its side, whatever type the other bound infers.
#[test]
fn should_refuse_a_contested_summed_property_bounded_on_one_side_only() {
    for (minimum, maximum) in [(None, Some(1_000)), (Some(-1_000), None)] {
        assert_refused(parse(name_schema(
            amount(minimum, maximum),
            true,
            documents_summable(),
            platform_value!({}),
        )));
    }
}

#[test]
fn should_register_a_contested_summed_property_within_the_limit() {
    for (minimum, maximum) in [(0, LIMIT), (-LIMIT, LIMIT), (-5, 5)] {
        parse(name_schema(
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
    let within = BTreeMap::from([(
        "amount".to_string(),
        platform_value!({ "type": "integer", "minimum": 0, "maximum": 1_000 }),
    )]);
    let reference = platform_value!({ "$ref": "#/$defs/amount", "position": 1 });
    let latest = PlatformVersion::latest().protocol_version;
    parse_at(
        name_schema(
            reference.clone(),
            true,
            documents_summable(),
            platform_value!({}),
        ),
        Some(&within),
        latest,
        true,
    )
    .expect("a referenced amount within the limit registers");

    let unbounded =
        BTreeMap::from([("amount".to_string(), platform_value!({ "type": "integer" }))]);
    assert_refused(parse_at(
        name_schema(reference, true, documents_summable(), platform_value!({})),
        Some(&unbounded),
        latest,
        true,
    ));
}

#[test]
fn should_leave_a_summed_type_without_a_contested_index_unbounded() {
    parse(name_schema(
        amount(None, None),
        false,
        documents_summable(),
        platform_value!({}),
    ))
    .expect("an uncontested summed type registers without bounds");
}

#[test]
fn should_leave_a_contested_type_without_a_sum_unbounded() {
    parse(name_schema(
        amount(None, None),
        true,
        platform_value!({}),
        platform_value!({}),
    ))
    .expect("a contested type without a sum registers without bounds");
}

/// A contract registered before the rule is read back without full validation.
#[test]
fn should_read_back_a_stored_contested_type_summing_a_property_without_bounds() {
    parse_at(
        name_schema(
            amount(None, None),
            true,
            documents_summable(),
            platform_value!({}),
        ),
        None,
        PlatformVersion::latest().protocol_version,
        false,
    )
    .expect("a stored contract parses");
}

#[test]
fn should_keep_registering_a_contested_type_summing_a_property_without_bounds_at_protocol_version_13(
) {
    parse_at(
        name_schema(
            amount(None, None),
            true,
            documents_summable(),
            platform_value!({}),
        ),
        None,
        13,
        true,
    )
    .expect("protocol version 13 registers any bounds");
}
