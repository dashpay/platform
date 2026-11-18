//! A summed property (protocol version 15): its values stay within
//! `SystemLimits::max_summed_value_magnitude_without_type_total` (±2^27), or they are never
//! negative and the type keeps the total of the property over all its documents, which state
//! validation reads before a create or a replace. A registration rule: a stored contract
//! still parses, and protocol version 14 registers what it registered.
use super::*;
use crate::data_contract::config::v1::DataContractConfigSettersV1;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use platform_value::platform_value;

pub(super) const LIMIT: i64 = 1 << 27;

pub(super) fn parse_at(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "tip",
        schema,
        schema_defs,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

pub(super) fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
    parse_at(schema, None, true, PlatformVersion::latest())
}

pub(super) fn amount(minimum: Option<i64>, maximum: Option<i64>) -> Value {
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

/// A `tip` type of `label` and `amount`, indexed `byLabel`, carrying `type_keywords` and, on
/// its index, `index_keywords`. `amount` has the schema `amount`.
pub(super) fn tip_schema(amount: Value, type_keywords: Value, index_keywords: Value) -> Value {
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

pub(super) fn summed_by_the_index() -> Value {
    platform_value!({ "summable": "amount" })
}

pub(super) fn summed_by_the_type() -> Value {
    platform_value!({ "documentsSummable": "amount" })
}

pub(super) fn nothing() -> Value {
    platform_value!({})
}

fn assert_refused(result: Result<DocumentType, ProtocolError>, remedy: &str) {
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
        &format!("whose values may pass {LIMIT}"),
        remedy,
    ] {
        assert!(
            message.contains(fragment),
            "error must name {fragment}, got {message}"
        );
    }
}

/// What a property that may be negative is told.
fn signed_remedy() -> String {
    format!("a minimum of at least -{LIMIT} and a maximum of at most {LIMIT}, or a minimum of at least 0")
}

/// What a property that is never negative is told.
const NEVER_NEGATIVE_REMEDY: &str =
    "the type must declare `documentsSummable` or `documentsAverageable` on it";

#[test]
fn should_register_a_summed_property_within_the_limit_on_a_type_keeping_no_total() {
    for (minimum, maximum) in [(-LIMIT, LIMIT), (0, LIMIT), (-LIMIT, 0), (1, 5)] {
        parse(tip_schema(
            amount(Some(minimum), Some(maximum)),
            nothing(),
            summed_by_the_index(),
        ))
        .unwrap_or_else(|error| panic!("{minimum}..{maximum} must register: {error:?}"));
    }
}

#[test]
fn should_refuse_a_summed_property_without_bounds() {
    for (type_keywords, index_keywords) in [
        (nothing(), summed_by_the_index()),
        (summed_by_the_type(), nothing()),
        (
            platform_value!({ "documentsAverageable": "amount" }),
            platform_value!({ "averageable": "amount" }),
        ),
    ] {
        assert_refused(
            parse(tip_schema(
                amount(None, None),
                type_keywords,
                index_keywords,
            )),
            &signed_remedy(),
        );
    }
}

/// Values of both signs past the limit are refused whether or not the type keeps a total: a
/// branch of a sum tree can leave `i64` while the total stays in it.
#[test]
fn should_refuse_a_signed_summed_property_past_the_limit_even_with_the_types_total() {
    for (minimum, maximum) in [
        (Some(-LIMIT - 1), Some(LIMIT)),
        (Some(-LIMIT), Some(LIMIT + 1)),
        (Some(-1), None),
        (None, Some(1_000)),
    ] {
        assert_refused(
            parse(tip_schema(
                amount(minimum, maximum),
                summed_by_the_type(),
                summed_by_the_index(),
            )),
            &signed_remedy(),
        );
    }
}

#[test]
fn should_refuse_a_never_negative_summed_property_past_the_limit_on_a_type_keeping_no_total() {
    for (minimum, maximum) in [(Some(0), Some(LIMIT + 1)), (Some(1), Some(4_294_967_295))] {
        assert_refused(
            parse(tip_schema(
                amount(minimum, maximum),
                nothing(),
                summed_by_the_index(),
            )),
            NEVER_NEGATIVE_REMEDY,
        );
    }
}

#[test]
fn should_register_a_never_negative_summed_property_past_the_limit_on_a_type_keeping_its_total() {
    for type_keywords in [
        summed_by_the_type(),
        platform_value!({ "documentsAverageable": "amount" }),
    ] {
        parse(tip_schema(
            amount(Some(0), Some(4_294_967_295)),
            type_keywords,
            summed_by_the_index(),
        ))
        .expect("a never negative property on a type keeping its total must register");
    }
}

/// Without `sizedIntegerTypes` a `minimum` of 0 with no `maximum` is an `i64` that is never
/// negative.
#[test]
fn should_register_a_never_negative_summed_property_without_a_maximum_on_a_type_keeping_its_total()
{
    let mut config = DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available");
    config.set_sized_integer_types_enabled(false);
    let parse_unsized = |type_keywords: Value| {
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "tip",
            tip_schema(amount(Some(0), None), type_keywords, summed_by_the_index()),
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            PlatformVersion::latest(),
        )
    };
    parse_unsized(summed_by_the_type())
        .expect("a never negative i64 on a type keeping its total must register");
    assert_refused(parse_unsized(nothing()), NEVER_NEGATIVE_REMEDY);
}

#[test]
fn should_read_the_bounds_through_a_ref() {
    let schema_defs = |maximum: i64| {
        BTreeMap::from([(
            "tipAmount".to_string(),
            platform_value!({ "type": "integer", "minimum": -5, "maximum": maximum }),
        )])
    };
    let schema = || {
        tip_schema(
            platform_value!({ "$ref": "#/$defs/tipAmount", "position": 1 }),
            nothing(),
            summed_by_the_index(),
        )
    };
    parse_at(
        schema(),
        Some(&schema_defs(LIMIT)),
        true,
        PlatformVersion::latest(),
    )
    .expect("bounds within the limit behind a $ref must register");
    assert_refused(
        parse_at(
            schema(),
            Some(&schema_defs(LIMIT + 1)),
            true,
            PlatformVersion::latest(),
        ),
        &signed_remedy(),
    );
}

/// Only the protocol writes the documents of a type that allows no creation, as the system
/// contracts' history and order types do.
#[test]
fn should_leave_a_type_no_transition_creates_documents_of_unbounded() {
    parse(tip_schema(
        amount(None, None),
        platform_value!({ "creationRestrictionMode": 2, "documentsSummable": "amount" }),
        summed_by_the_index(),
    ))
    .expect("a type that allows no creation must register whatever it sums");
}

#[test]
fn should_leave_a_type_that_sums_nothing_unbounded() {
    parse(tip_schema(amount(None, None), nothing(), nothing()))
        .expect("a type summing nothing must register whatever its integers admit");
}

#[test]
fn should_read_back_a_stored_type_summing_a_property_without_bounds() {
    parse_at(
        tip_schema(amount(None, None), nothing(), summed_by_the_index()),
        None,
        false,
        PlatformVersion::latest(),
    )
    .expect("a stored contract must keep parsing");
}

#[test]
fn should_keep_registering_a_summed_property_without_bounds_at_protocol_version_14() {
    let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
    parse_at(
        tip_schema(amount(None, None), nothing(), summed_by_the_index()),
        None,
        true,
        platform_version,
    )
    .expect("protocol version 14 registers a summed property whatever its bounds");
}

/// One document adds one entry to an index of single values, so the type's total bounds
/// every sum as it is; each element of a typed array an index keys the document by is an
/// entry of its own.
#[test]
fn should_count_the_entries_one_document_adds_to_a_summed_index() {
    let single = parse(tip_schema(
        amount(Some(0), Some(LIMIT)),
        nothing(),
        summed_by_the_index(),
    ))
    .expect("the type must register");
    assert_eq!(
        single.index_structure().max_summed_entries_per_document(),
        1
    );

    let unsummed = parse(tip_schema(
        amount(Some(0), Some(LIMIT)),
        nothing(),
        nothing(),
    ))
    .expect("the type must register");
    assert_eq!(
        unsummed.index_structure().max_summed_entries_per_document(),
        1
    );

    let by_tags = |index_keywords: Value| {
        let mut index = platform_value!({
            "name": "byTag",
            "properties": [{ "tags": "asc" }],
        });
        let Value::Map(index_keywords) = index_keywords else {
            panic!("expected the index keywords to be a map");
        };
        for (key, value) in index_keywords {
            index
                .insert(key.as_text().expect("a text key").to_string(), value)
                .expect("expected to set the index keyword");
        }
        parse(platform_value!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": { "type": "string", "maxLength": 20 },
                    "minItems": 1,
                    "maxItems": 7,
                    "uniqueItems": true,
                    "position": 0,
                },
                "amount": amount(Some(0), Some(LIMIT)),
            },
            "required": ["tags", "amount"],
            "indices": [index],
            "additionalProperties": false,
        }))
        .expect("the type must register")
    };
    assert_eq!(
        by_tags(summed_by_the_index())
            .index_structure()
            .max_summed_entries_per_document(),
        7
    );
    assert_eq!(
        by_tags(nothing())
            .index_structure()
            .max_summed_entries_per_document(),
        1
    );
}
