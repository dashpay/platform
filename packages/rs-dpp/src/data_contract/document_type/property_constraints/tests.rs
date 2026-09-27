use super::*;
use platform_value::platform_value;
use platform_version::version::PLATFORM_VERSIONS;

/// The rules of a schema whose `propertyConstraints` is `declaration`.
fn parse(declaration: Value) -> Result<BTreeMap<String, PropertyConstraint>, DataContractError> {
    let schema = platform_value!({ "type": "object", "propertyConstraints": declaration });
    parse_property_constraints(&schema, "order")
}

/// The one rule of a declaration naming it `rule`.
fn parse_rule_value(rule: Value) -> PropertyConstraint {
    parse(platform_value!({ "rule": rule }))
        .unwrap_or_else(|error| panic!("the rule should parse: {error}"))
        .remove("rule")
        .expect("the rule is parsed under its name")
}

fn expect_refusal(declaration: Value, needle: &str) {
    match parse(declaration.clone()) {
        Err(DataContractError::InvalidContractStructure(message)) => assert!(
            message.contains(needle),
            "{declaration:?}: expected a refusal containing {needle:?}, got: {message}"
        ),
        other => panic!("{declaration:?}: expected a refusal containing {needle:?}, got {other:?}"),
    }
}

fn property(path: &str) -> ConstraintExpression {
    ConstraintExpression::Property {
        path: path.to_string(),
        if_absent: 0,
    }
}

/// A document's properties, as `validate_document_properties` hands them over.
fn data(entries: &[(&str, Value)]) -> Value {
    Value::from(
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect::<BTreeMap<String, Value>>(),
    )
}

/// The value of `expression`, parsed as the left side of an `equal`, for `data`.
fn evaluate(expression: Value, data: &Value) -> Result<i128, PropertyConstraintViolation> {
    parse_rule_value(platform_value!({ "equal": [expression, "anchor"] }))
        .left
        .evaluate(data)
}

// ── parsing ─────────────────────────────────────────────────────────────

#[test]
fn should_parse_every_operator_comparison_and_the_if_absent_operand() {
    let rules = parse(platform_value!({
        "depositCoversOrder": {
            "lessThanOrEqual": [
                { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
                "deposit"
            ]
        },
        "wholeLots": { "equal": [{ "modulo": ["quantity", 10] }, 0] },
        "minimumOrder": {
            "greaterThanOrEqual": [
                { "multiply": ["price", { "ifAbsent": ["quantity", 1] }] },
                100
            ]
        },
        "rest": {
            "notEqual": [
                { "subtract": [{ "divide": ["meta.total", 2] }, { "power": ["fee", 3] }] },
                -5
            ]
        },
        "less": { "lessThan": ["fee", "price"] },
        "more": { "greaterThan": ["price", "fee"] }
    }))
    .expect("the declaration parses");

    // In name order, the order a document is checked against them
    assert_eq!(
        rules.keys().collect::<Vec<_>>(),
        [
            "depositCoversOrder",
            "less",
            "minimumOrder",
            "more",
            "rest",
            "wholeLots"
        ]
    );
    assert_eq!(
        rules["depositCoversOrder"],
        PropertyConstraint {
            comparison: ConstraintComparison::LessThanOrEqual,
            left: ConstraintExpression::Multiply(vec![
                ConstraintExpression::Add(vec![property("price"), property("fee")]),
                property("quantity"),
            ]),
            right: property("deposit"),
        }
    );
    assert_eq!(
        rules["wholeLots"],
        PropertyConstraint {
            comparison: ConstraintComparison::Equal,
            left: ConstraintExpression::Modulo(
                Box::new(property("quantity")),
                Box::new(ConstraintExpression::Value(10))
            ),
            right: ConstraintExpression::Value(0),
        }
    );
    assert_eq!(
        rules["minimumOrder"],
        PropertyConstraint {
            comparison: ConstraintComparison::GreaterThanOrEqual,
            left: ConstraintExpression::Multiply(vec![
                property("price"),
                ConstraintExpression::Property {
                    path: "quantity".to_string(),
                    if_absent: 1
                },
            ]),
            right: ConstraintExpression::Value(100),
        }
    );
    assert_eq!(
        rules["rest"],
        PropertyConstraint {
            comparison: ConstraintComparison::NotEqual,
            left: ConstraintExpression::Subtract(
                Box::new(ConstraintExpression::Divide(
                    Box::new(property("meta.total")),
                    Box::new(ConstraintExpression::Value(2))
                )),
                Box::new(ConstraintExpression::Power(
                    Box::new(property("fee")),
                    Box::new(ConstraintExpression::Value(3))
                )),
            ),
            right: ConstraintExpression::Value(-5),
        }
    );
    assert_eq!(rules["less"].comparison, ConstraintComparison::LessThan);
    assert_eq!(rules["more"].comparison, ConstraintComparison::GreaterThan);
}

#[test]
fn should_read_nothing_from_a_schema_without_the_keyword() {
    let schema = platform_value!({ "type": "object" });
    assert!(parse_property_constraints(&schema, "order")
        .expect("parses")
        .is_empty());
    // A schema that is not an object is the core parser's to refuse
    assert!(
        parse_property_constraints(&Value::Text("x".to_string()), "order")
            .expect("parses")
            .is_empty()
    );
}

#[test]
fn should_refuse_a_malformed_declaration() {
    let cases = [
        (platform_value!(["x"]), "must be an object of rules by name"),
        (platform_value!({}), "must declare at least one rule"),
        (
            platform_value!({ "bad-name": { "equal": ["price", 1] } }),
            "but a rule name is 1 to 64 letters, digits or underscores",
        ),
        (
            platform_value!({ "rule": ["price", 1] }),
            "rule \"rule\" must be an object with one key, its comparison",
        ),
        (
            platform_value!({ "rule": { "equal": ["price", 1], "lessThan": ["price", 1] } }),
            "rule \"rule\" must be an object with one key, its comparison",
        ),
        (
            platform_value!({ "rule": { "atMost": ["price", 1] } }),
            "compares with \"atMost\", which is not a comparison",
        ),
        (
            platform_value!({ "rule": { "equal": ["price"] } }),
            "at equal must list exactly two operands",
        ),
        (
            platform_value!({ "rule": { "equal": ["price", 1, 2] } }),
            "at equal must list exactly two operands",
        ),
        (
            platform_value!({ "rule": { "equal": ["price", true] } }),
            "at equal[1] must be an integer, a property path or an object with one key",
        ),
        (
            platform_value!({ "rule": { "equal": ["price", 1.5] } }),
            "at equal[1] holds 1.5, which is not an integer in the range of a 128-bit signed \
             integer",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "ifAbsent": ["price", 0.5] }, 1] } }),
            "at equal[0].ifAbsent[1] holds 0.5, which is not an integer",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "sum": ["price", 1] }, 1] } }),
            "at equal[0] names \"sum\", which is not one of add, subtract",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "add": ["price"] }, 1] } }),
            "at equal[0].add must list two or more operands",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "subtract": ["price", 1, 2] }, 1] } }),
            "at equal[0].subtract must list exactly two operands",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "divide": ["price", 0] }, 1] } }),
            "at equal[0].divide divides by 0",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "modulo": ["price", 0] }, 1] } }),
            "at equal[0].modulo divides by 0",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "power": ["price", -2] }, 1] } }),
            "at equal[0].power raises to the negative power -2",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "ifAbsent": ["price"] }, 1] } }),
            "at equal[0].ifAbsent must list a property path and the integer value",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "ifAbsent": [1, "price"] }, 1] } }),
            "at equal[0].ifAbsent must name a property path first",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "ifAbsent": ["price", "fee"] }, 1] } }),
            "at equal[0].ifAbsent must give an integer value second",
        ),
        (
            platform_value!({ "rule": { "equal": [{ "add": [1, 2] }, 3] } }),
            "rule \"rule\" reads no property",
        ),
    ];
    for (declaration, needle) in cases {
        expect_refusal(declaration, needle);
    }

    // A literal must fit the arithmetic
    let too_large = Value::Map(vec![(
        Value::Text("rule".to_string()),
        Value::Map(vec![(
            Value::Text("equal".to_string()),
            Value::Array(vec![
                Value::Text("price".to_string()),
                Value::U128(u128::MAX),
            ]),
        )]),
    )]);
    expect_refusal(
        too_large,
        "which is not an integer in the range of a 128-bit signed integer",
    );
}

/// JSON does not tell `100` from `100.0`, and the meta-schema's `integer` type admits
/// both, so a literal written as a float with no fractional part is that integer.
#[test]
fn should_read_a_float_literal_without_a_fractional_part_as_an_integer() {
    let rule = parse_rule_value(platform_value!({
        "equal": [{ "ifAbsent": ["price", 5.0] }, 100.0]
    }));
    assert_eq!(
        rule,
        PropertyConstraint {
            comparison: ConstraintComparison::Equal,
            left: ConstraintExpression::Property {
                path: "price".to_string(),
                if_absent: 5
            },
            right: ConstraintExpression::Value(100),
        }
    );
}

/// No rule may nest deeper than the constant cap, on any parse: it keeps a crafted
/// declaration from driving the parser into unbounded recursion.
#[test]
fn should_refuse_a_rule_nested_deeper_than_the_parse_depth_cap() {
    let nested = |levels: usize| {
        let mut expression = platform_value!("price");
        for _ in 0..levels {
            expression = platform_value!({ "subtract": [expression, 1] });
        }
        platform_value!({ "rule": { "equal": [expression, 0] } })
    };
    // The comparison's side is at depth 1, each subtract one deeper
    parse(nested(MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH - 1)).expect("at the cap");
    expect_refusal(
        nested(MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH),
        &format!("nests deeper than {MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH} levels"),
    );
}

/// Every rule a contract can register parses: a rule is never deeper than its node
/// count, which the registration limit caps below the parse depth cap at every protocol
/// version.
#[test]
fn should_keep_every_registrable_rule_within_the_parse_depth_cap() {
    for platform_version in PLATFORM_VERSIONS {
        assert!(
            usize::from(platform_version.system_limits.max_property_constraint_nodes)
                <= MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH,
            "protocol version {} registers rules deeper than every parse accepts",
            platform_version.protocol_version
        );
    }
}

/// Nodes are the comparison, every operator and every operand; paths are listed in the
/// order they are read, a path read twice listed twice.
#[test]
fn should_count_nodes_and_list_the_paths_a_rule_reads() {
    let rule = parse_rule_value(platform_value!({
        "lessThanOrEqual": [
            { "multiply": [{ "add": ["price", { "ifAbsent": ["fee", 3] }] }, "price"] },
            7
        ]
    }));
    // lessThanOrEqual, multiply, add, price, ifAbsent fee, price, 7
    assert_eq!(rule.node_count(), 7);
    assert_eq!(rule.property_paths(), ["price", "fee", "price"]);
}

// ── evaluation ──────────────────────────────────────────────────────────

#[test]
fn should_evaluate_every_operator() {
    let values = data(&[
        ("a", Value::U64(7)),
        ("b", Value::I64(-3)),
        ("c", Value::U8(2)),
    ]);
    for (expression, expected) in [
        (platform_value!("a"), 7),
        (platform_value!(12), 12),
        (platform_value!({ "add": ["a", "b", "c"] }), 6),
        (platform_value!({ "multiply": ["a", "b", "c"] }), -42),
        (platform_value!({ "subtract": ["a", "b"] }), 10),
        (platform_value!({ "divide": ["a", "c"] }), 3),
        (platform_value!({ "modulo": ["a", "c"] }), 1),
        (platform_value!({ "power": ["b", 3] }), -27),
        (platform_value!({ "power": ["c", 0] }), 1),
        (platform_value!({ "power": [0, 0] }), 1),
        // ((a + c) * b) - a
        (
            platform_value!({ "subtract": [{ "multiply": [{ "add": ["a", "c"] }, "b"] }, "a"] }),
            -34,
        ),
    ] {
        assert_eq!(
            evaluate(expression.clone(), &values),
            Ok(expected),
            "{expression:?}"
        );
    }
}

/// Euclidean division: the remainder is never negative, and the quotient is the one that
/// goes with it. For operands that are not negative it is ordinary integer division.
#[test]
fn should_divide_and_take_remainders_the_euclidean_way() {
    for (dividend, divisor, quotient, remainder) in [
        (7, 2, 3, 1),
        (-7, 2, -4, 1),
        (7, -2, -3, 1),
        (-7, -2, 4, 1),
        (6, 3, 2, 0),
        (-6, 3, -2, 0),
    ] {
        let values = data(&[("x", Value::I64(dividend)), ("y", Value::I64(divisor))]);
        assert_eq!(
            evaluate(platform_value!({ "divide": ["x", "y"] }), &values),
            Ok(i128::from(quotient)),
            "{dividend} / {divisor}"
        );
        assert_eq!(
            evaluate(platform_value!({ "modulo": ["x", "y"] }), &values),
            Ok(i128::from(remainder)),
            "{dividend} mod {divisor}"
        );
    }
}

#[test]
fn should_take_zero_or_the_if_absent_value_for_an_absent_property() {
    let values = data(&[
        ("present", Value::U32(4)),
        ("empty", Value::Null),
        ("meta", platform_value!({ "count": 9 })),
    ]);
    for (expression, expected) in [
        (platform_value!("missing"), 0),
        (platform_value!("empty"), 0),
        (platform_value!({ "ifAbsent": ["missing", 5] }), 5),
        (platform_value!({ "ifAbsent": ["empty", -5] }), -5),
        (platform_value!({ "ifAbsent": ["present", 5] }), 4),
        (platform_value!("meta.count"), 9),
        (platform_value!("meta.missing"), 0),
        (platform_value!({ "ifAbsent": ["other.count", 2] }), 2),
        // An intermediate that is not an object reads as absent: the schema validation
        // that runs first refuses such a document
        (platform_value!({ "ifAbsent": ["present.count", 3] }), 3),
    ] {
        assert_eq!(
            evaluate(expression.clone(), &values),
            Ok(expected),
            "{expression:?}"
        );
    }
}

#[test]
fn should_refuse_what_does_not_fit_or_has_no_integer_result() {
    let values = data(&[
        ("max", Value::I128(i128::MAX)),
        ("min", Value::I128(i128::MIN)),
        ("zero", Value::U8(0)),
        ("minusOne", Value::I8(-1)),
        ("negative", Value::I8(-2)),
        ("huge", Value::U128(u128::MAX)),
        ("float", Value::Float(1.0)),
        ("two", Value::U8(2)),
    ]);
    for (expression, expected) in [
        (
            platform_value!({ "add": ["max", 1] }),
            PropertyConstraintViolation::Overflow,
        ),
        // An intermediate overflow is a fault, even when a later operand would bring the
        // result back in range
        (
            platform_value!({ "add": ["max", 1, -1] }),
            PropertyConstraintViolation::Overflow,
        ),
        (
            platform_value!({ "subtract": ["min", 1] }),
            PropertyConstraintViolation::Overflow,
        ),
        (
            platform_value!({ "multiply": ["max", 2] }),
            PropertyConstraintViolation::Overflow,
        ),
        (
            platform_value!({ "divide": ["min", "minusOne"] }),
            PropertyConstraintViolation::Overflow,
        ),
        (
            platform_value!({ "power": ["two", 127] }),
            PropertyConstraintViolation::Overflow,
        ),
        (
            platform_value!({ "divide": [1, "zero"] }),
            PropertyConstraintViolation::DivisionByZero,
        ),
        (
            platform_value!({ "modulo": [1, "zero"] }),
            PropertyConstraintViolation::DivisionByZero,
        ),
        // Also for an absent divisor, which counts as 0
        (
            platform_value!({ "divide": [1, "missing"] }),
            PropertyConstraintViolation::DivisionByZero,
        ),
        (
            platform_value!({ "power": [2, "negative"] }),
            PropertyConstraintViolation::NegativeExponent,
        ),
        // A value the arithmetic cannot hold
        (
            platform_value!("huge"),
            PropertyConstraintViolation::Overflow,
        ),
        // A float the schema's `integer` type admits, which no integer property stores
        (
            platform_value!("float"),
            PropertyConstraintViolation::NotAnInteger,
        ),
    ] {
        assert_eq!(
            evaluate(expression.clone(), &values),
            Err(expected),
            "{expression:?}"
        );
    }

    // The remainder by -1 is 0 for every dividend, `i128::MIN` included, whose quotient
    // does not fit
    assert_eq!(
        evaluate(platform_value!({ "modulo": ["min", "minusOne"] }), &values),
        Ok(0)
    );
    assert_eq!(
        evaluate(platform_value!({ "power": ["two", 126] }), &values),
        Ok(1i128 << 126)
    );
}

/// An exponent too large for `checked_pow` still has an exact result for the bases 0, 1
/// and -1; every other base overflows.
#[test]
fn should_raise_the_bases_that_stay_in_range_to_any_power() {
    let exponent = i128::from(u32::MAX) + 1;
    let odd_exponent = exponent + 1;
    for (base, exponent, expected) in [
        (0, exponent, Ok(0)),
        (1, exponent, Ok(1)),
        (-1, exponent, Ok(1)),
        (-1, odd_exponent, Ok(-1)),
        (2, exponent, Err(PropertyConstraintViolation::Overflow)),
        (-2, odd_exponent, Err(PropertyConstraintViolation::Overflow)),
    ] {
        let values = data(&[
            ("base", Value::I128(base)),
            ("exponent", Value::I128(exponent)),
        ]);
        assert_eq!(
            evaluate(platform_value!({ "power": ["base", "exponent"] }), &values),
            expected,
            "{base} to the power {exponent}"
        );
    }
}

#[test]
fn should_report_whether_a_rule_holds_and_the_left_fault_first() {
    let rule = parse_rule_value(platform_value!({
        "lessThanOrEqual": [
            { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
            "deposit"
        ]
    }));
    let order = |price: u64, fee: u64, quantity: u64, deposit: u64| {
        data(&[
            ("price", Value::U64(price)),
            ("fee", Value::U64(fee)),
            ("quantity", Value::U64(quantity)),
            ("deposit", Value::U64(deposit)),
        ])
    };
    // (10 + 2) * 3 = 36
    assert_eq!(rule.violation(&order(10, 2, 3, 36)), None);
    assert_eq!(rule.violation(&order(10, 2, 3, 100)), None);
    assert_eq!(
        rule.violation(&order(10, 2, 3, 35)),
        Some(PropertyConstraintViolation::NotMet)
    );

    let both_sides_fail = parse_rule_value(platform_value!({
        "equal": [{ "divide": ["price", "zero"] }, { "power": ["price", "negative"] }]
    }));
    let values = data(&[
        ("price", Value::U64(1)),
        ("zero", Value::U64(0)),
        ("negative", Value::I64(-1)),
    ]);
    assert_eq!(
        both_sides_fail.violation(&values),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    for comparison in ConstraintComparison::ALL {
        let expected = match comparison {
            ConstraintComparison::Equal => [false, true, false],
            ConstraintComparison::NotEqual => [true, false, true],
            ConstraintComparison::LessThan => [true, false, false],
            ConstraintComparison::LessThanOrEqual => [true, true, false],
            ConstraintComparison::GreaterThan => [false, false, true],
            ConstraintComparison::GreaterThanOrEqual => [false, true, true],
        };
        assert_eq!(
            [
                comparison.holds(1, 2),
                comparison.holds(2, 2),
                comparison.holds(3, 2)
            ],
            expected,
            "{}",
            comparison.wire_name()
        );
    }
}
