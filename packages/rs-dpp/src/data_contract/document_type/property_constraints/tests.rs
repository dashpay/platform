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
    match parse_rule_value(platform_value!({ "equal": [expression, "anchor"] })) {
        PropertyConstraint::Compare { left, .. } => left.evaluate(data),
        other => panic!("an equal parses to a comparison, got {other:?}"),
    }
}

/// The comparison `rule` is, which it must be.
fn comparison_of(rule: &PropertyConstraint) -> ConstraintComparison {
    match rule {
        PropertyConstraint::Compare { comparison, .. } => *comparison,
        other => panic!("expected a comparison, got {other:?}"),
    }
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
        PropertyConstraint::Compare {
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
        PropertyConstraint::Compare {
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
        PropertyConstraint::Compare {
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
        PropertyConstraint::Compare {
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
    assert_eq!(
        comparison_of(&rules["less"]),
        ConstraintComparison::LessThan
    );
    assert_eq!(
        comparison_of(&rules["more"]),
        ConstraintComparison::GreaterThan
    );
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
            "rule \"rule\" must be an object with one key: a comparison (equal, notEqual",
        ),
        (
            platform_value!({ "rule": { "equal": ["price", 1], "lessThan": ["price", 1] } }),
            "rule \"rule\" must be an object with one key: a comparison (equal, notEqual",
        ),
        (
            platform_value!({ "rule": { "atMost": ["price", 1] } }),
            "rule \"rule\" names \"atMost\", which is not a comparison (equal",
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
        PropertyConstraint::Compare {
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

// ── anyOf, allOf and not ────────────────────────────────────────────────

fn compare(
    comparison: ConstraintComparison,
    left: ConstraintExpression,
    right: ConstraintExpression,
) -> PropertyConstraint {
    PropertyConstraint::Compare {
        comparison,
        left,
        right,
    }
}

#[test]
fn should_parse_any_of_all_of_and_not() {
    let rules = parse(platform_value!({
        "aIsZeroOrBIsFour": { "anyOf": [{ "equal": ["a", 0] }, { "equal": ["b", 4] }] },
        "notBoth": {
            "not": { "allOf": [{ "greaterThan": ["a", 0] }, { "greaterThan": ["b", 0] }] }
        },
        "nested": {
            "allOf": [
                { "anyOf": [{ "equal": ["a", 0] }, { "lessThan": ["a", "b"] }] },
                { "not": { "equal": [{ "ifAbsent": ["b", 1] }, 3] } }
            ]
        }
    }))
    .expect("the declaration parses");

    let a_is = |value| {
        compare(
            ConstraintComparison::Equal,
            property("a"),
            ConstraintExpression::Value(value),
        )
    };
    assert_eq!(
        rules["aIsZeroOrBIsFour"],
        PropertyConstraint::AnyOf(vec![
            a_is(0),
            compare(
                ConstraintComparison::Equal,
                property("b"),
                ConstraintExpression::Value(4)
            ),
        ])
    );
    assert_eq!(
        rules["notBoth"],
        PropertyConstraint::Not(Box::new(PropertyConstraint::AllOf(vec![
            compare(
                ConstraintComparison::GreaterThan,
                property("a"),
                ConstraintExpression::Value(0)
            ),
            compare(
                ConstraintComparison::GreaterThan,
                property("b"),
                ConstraintExpression::Value(0)
            ),
        ])))
    );
    assert_eq!(
        rules["nested"],
        PropertyConstraint::AllOf(vec![
            PropertyConstraint::AnyOf(vec![
                a_is(0),
                compare(ConstraintComparison::LessThan, property("a"), property("b")),
            ]),
            PropertyConstraint::Not(Box::new(compare(
                ConstraintComparison::Equal,
                ConstraintExpression::Property {
                    path: "b".to_string(),
                    if_absent: 1
                },
                ConstraintExpression::Value(3)
            ))),
        ])
    );
}

/// The errors place a fault by its path through the conditions, then through the
/// operands of the comparison it sits in.
#[test]
fn should_refuse_a_malformed_condition() {
    let one = platform_value!({ "equal": ["price", 1] });
    let two = platform_value!({ "equal": ["price", 2] });
    let fee = platform_value!({ "equal": ["fee", 1] });
    let cases = [
        (
            platform_value!({ "anyOf": [one.clone()] }),
            "rule \"rule\" at anyOf must list two or more conditions",
        ),
        (
            platform_value!({ "allOf": one.clone() }),
            "rule \"rule\" at allOf must list two or more conditions",
        ),
        (
            platform_value!({ "allOf": [] }),
            "rule \"rule\" at allOf must list two or more conditions",
        ),
        (
            platform_value!({ "not": [one.clone()] }),
            "rule \"rule\" at not must be an object with one key: a comparison",
        ),
        (
            platform_value!({ "anyOf": [one.clone(), two.clone()], "equal": ["price", 3] }),
            "rule \"rule\" must be an object with one key: a comparison",
        ),
        (
            platform_value!({ "anyOf": [one.clone(), ["price", 1]] }),
            "rule \"rule\" at anyOf[1] must be an object with one key: a comparison",
        ),
        (
            platform_value!({ "anyOf": [one.clone(), { "or": [one.clone(), two.clone()] }] }),
            "rule \"rule\" at anyOf[1] names \"or\", which is not a comparison",
        ),
        // A flat list says the same
        (
            platform_value!({ "anyOf": [{ "anyOf": [one.clone(), two.clone()] }, fee.clone()] }),
            "rule \"rule\" at anyOf[0] is an anyOf directly inside an anyOf",
        ),
        (
            platform_value!({ "allOf": [fee.clone(), { "allOf": [one.clone(), two.clone()] }] }),
            "rule \"rule\" at allOf[1] is an allOf directly inside an allOf",
        ),
        // A double negation says what the condition inside it says
        (
            platform_value!({ "not": { "not": one.clone() } }),
            "rule \"rule\" at not.not is a not directly inside a not",
        ),
        // Every comparison reads a property, not only the rule as a whole: a constant
        // one would make the anyOf hold for every document
        (
            platform_value!({ "anyOf": [one.clone(), { "equal": [1, 1] }] }),
            "rule \"rule\" at anyOf[1] reads no property",
        ),
        (
            platform_value!({ "not": { "equal": [2, { "add": [1, 1] }] } }),
            "rule \"rule\" at not reads no property",
        ),
        (
            platform_value!({
                "allOf": [one.clone(), { "not": { "lessThan": [{ "divide": ["price", 0] }, 1] } }]
            }),
            "rule \"rule\" at allOf[1].not.lessThan[0].divide divides by 0",
        ),
        (
            platform_value!({ "anyOf": [one.clone(), { "equal": ["price"] }] }),
            "rule \"rule\" at anyOf[1].equal must list exactly two operands",
        ),
    ];
    for (condition, needle) in cases {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// Conditions nest within the same cap as operands: the rule's own condition is at
/// depth 0, and whatever a condition holds one level deeper.
#[test]
fn should_refuse_conditions_nested_deeper_than_the_parse_depth_cap() {
    let nested = |levels: usize| {
        let mut condition = platform_value!({ "equal": ["price", 0] });
        for level in 0..levels {
            // Alternating, since an anyOf directly inside an anyOf is refused
            let key = if level % 2 == 0 { ANY_OF } else { ALL_OF };
            let sibling = platform_value!({ "equal": ["price", level as u64 + 1] });
            condition = Value::Map(vec![(
                Value::Text(key.to_string()),
                Value::Array(vec![condition, sibling]),
            )]);
        }
        platform_value!({ "rule": condition })
    };
    // The innermost comparison sits `levels` deep, its operands one deeper
    parse(nested(MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH - 1)).expect("at the cap");
    expect_refusal(
        nested(MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH),
        &format!("equal[0] nests deeper than {MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH} levels"),
    );
    // One level more puts the comparison itself past the cap, inside the anyOf of
    // the first level, and the condition parse refuses it before its operands
    expect_refusal(
        nested(MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH + 1),
        &format!("anyOf[0] nests deeper than {MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH} levels"),
    );
}

/// A list that repeats a condition is found where it sits, the first in declared
/// order, comparing conditions as they parse. The parse itself accepts it: the check
/// runs under full validation.
#[test]
fn should_find_a_condition_an_any_of_or_all_of_repeats() {
    let one = platform_value!({ "equal": ["price", 1] });
    let two = platform_value!({ "equal": ["price", 2] });
    let fee = platform_value!({ "equal": ["fee", 1] });
    for (condition, expected) in [
        (
            platform_value!({ "anyOf": [one.clone(), two.clone()] }),
            None,
        ),
        // The same condition in two different lists is no repeat
        (
            platform_value!({
                "allOf": [
                    { "anyOf": [one.clone(), fee.clone()] },
                    { "anyOf": [one.clone(), two.clone()] }
                ]
            }),
            None,
        ),
        (
            platform_value!({ "anyOf": [one.clone(), two.clone(), one.clone()] }),
            Some(("anyOf[2]", "anyOf[0]")),
        ),
        // Alike once parsed: JSON does not tell `1` from `1.0`, and a path on its own
        // reads as `ifAbsent` 0
        (
            platform_value!({ "allOf": [one.clone(), { "equal": ["price", 1.0] }] }),
            Some(("allOf[1]", "allOf[0]")),
        ),
        (
            platform_value!({
                "anyOf": [one.clone(), { "equal": [{ "ifAbsent": ["price", 0] }, 1] }]
            }),
            Some(("anyOf[1]", "anyOf[0]")),
        ),
        // Found through a not, inside a nested list
        (
            platform_value!({
                "allOf": [
                    fee.clone(),
                    { "not": { "anyOf": [two.clone(), one.clone(), two.clone()] } }
                ]
            }),
            Some(("allOf[1].not.anyOf[2]", "allOf[1].not.anyOf[0]")),
        ),
        // The first repeat in declared order
        (
            platform_value!({
                "anyOf": [{ "allOf": [fee.clone(), fee.clone()] }, one.clone(), one.clone()]
            }),
            Some(("anyOf[0].allOf[1]", "anyOf[0].allOf[0]")),
        ),
        (
            platform_value!({ "anyOf": [{ "present": "fee" }, one.clone(), { "present": "fee" }] }),
            Some(("anyOf[2]", "anyOf[0]")),
        ),
        // An in lists a set: the same values in another order are the same condition
        (
            platform_value!({
                "anyOf": [{ "in": ["fee", [1, 2]] }, one.clone(), { "in": ["fee", [2, 1]] }]
            }),
            Some(("anyOf[2]", "anyOf[0]")),
        ),
        // Testing the presence of a property and its absence are different conditions
        (
            platform_value!({ "anyOf": [{ "present": "fee" }, { "absent": "fee" }] }),
            None,
        ),
    ] {
        let rule = parse_rule_value(condition.clone());
        assert_eq!(
            rule.repeated_condition(),
            expected.map(|(repeat, earlier)| (repeat.to_string(), earlier.to_string())),
            "{condition:?}"
        );
    }
}

// ── in ──────────────────────────────────────────────────────────────────

#[test]
fn should_parse_in() {
    assert_eq!(
        parse_rule_value(platform_value!({ "in": ["kind", [7, 1, 3.0]] })),
        PropertyConstraint::In {
            operand: property("kind"),
            values: BTreeSet::from([1, 3, 7]),
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "in": [{ "modulo": ["quantity", 10] }, [0, 5]] })),
        PropertyConstraint::In {
            operand: ConstraintExpression::Modulo(
                Box::new(property("quantity")),
                Box::new(ConstraintExpression::Value(10))
            ),
            values: BTreeSet::from([0, 5]),
        }
    );

    for (condition, needle) in [
        (
            platform_value!({ "in": ["kind"] }),
            "rule \"rule\" at in must list an integer expression and the values it may take",
        ),
        (
            platform_value!({ "in": "kind" }),
            "rule \"rule\" at in must list an integer expression and the values it may take",
        ),
        (
            platform_value!({ "in": ["kind", [1, 2], [3]] }),
            "rule \"rule\" at in must list an integer expression and the values it may take",
        ),
        (
            platform_value!({ "in": ["kind", [1]] }),
            "rule \"rule\" at in[1] must list two or more integer values",
        ),
        (
            platform_value!({ "in": ["kind", 1] }),
            "rule \"rule\" at in[1] must list two or more integer values",
        ),
        // A listed value is a literal, never a path or an expression
        (
            platform_value!({ "in": ["kind", [1, "fee"]] }),
            "rule \"rule\" at in[1][1] must be an integer value",
        ),
        (
            platform_value!({ "in": ["kind", [1, { "add": [1, 1] }]] }),
            "rule \"rule\" at in[1][1] must be an integer value",
        ),
        (
            platform_value!({ "in": ["kind", [1, 2.5]] }),
            "rule \"rule\" at in[1][1] holds 2.5, which is not an integer",
        ),
        // Alike once parsed: JSON does not tell `1` from `1.0`
        (
            platform_value!({ "in": ["kind", [1, 2, 1.0]] }),
            "rule \"rule\" at in[1][2] repeats the value at in[1][0]",
        ),
        (
            platform_value!({ "in": [5, [1, 5]] }),
            "rule \"rule\" reads no property",
        ),
        (
            platform_value!({
                "anyOf": [{ "equal": ["fee", 1] }, { "in": [{ "add": [1, 2] }, [1, 3]] }]
            }),
            "rule \"rule\" at anyOf[1] reads no property",
        ),
        (
            platform_value!({ "in": [{ "divide": ["kind", 0] }, [1, 2]] }),
            "rule \"rule\" at in[0].divide divides by 0",
        ),
        (
            platform_value!({ "not": { "in": [{ "sum": ["kind", 1] }, [1, 2]] } }),
            "rule \"rule\" at not.in[0] names \"sum\", which is not one of add",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// An `in` holds when its operand takes a listed value, a fault in the operand breaks the
/// rule, and it is one node plus one per value.
#[test]
fn should_hold_an_in_when_its_operand_takes_a_listed_value() {
    let rule = parse_rule_value(platform_value!({ "in": ["kind", [1, 3, 7]] }));
    for (kind, holds) in [(1, true), (3, true), (7, true), (2, false), (8, false)] {
        assert_eq!(
            rule.holds(&data(&[("kind", Value::U64(kind))])),
            Ok(holds),
            "kind {kind}"
        );
    }
    // An absent operand reads as 0
    assert_eq!(rule.holds(&data(&[])), Ok(false));
    let with_zero = parse_rule_value(platform_value!({ "in": ["kind", [0, 1]] }));
    assert_eq!(with_zero.holds(&data(&[])), Ok(true));

    let divided = parse_rule_value(platform_value!({ "in": [{ "divide": [10, "kind"] }, [2, 5]] }));
    assert_eq!(divided.violation(&data(&[("kind", Value::U64(5))])), None);
    assert_eq!(
        divided.violation(&data(&[("kind", Value::U64(3))])),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(
        divided.violation(&data(&[("kind", Value::U64(0))])),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    // in, kind, and one per value
    assert_eq!(rule.node_count(), 5);
    assert_eq!(rule.property_reads(), [("kind", PropertyRead::Value)]);
}

// ── present and absent ──────────────────────────────────────────────────

#[test]
fn should_parse_present_and_absent() {
    assert_eq!(
        parse_rule_value(platform_value!({ "present": "meta.total" })),
        PropertyConstraint::Present("meta.total".to_string())
    );
    assert_eq!(
        parse_rule_value(platform_value!({
            "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
        })),
        PropertyConstraint::AnyOf(vec![
            PropertyConstraint::Absent("discount".to_string()),
            compare(
                ConstraintComparison::GreaterThan,
                property("discount"),
                ConstraintExpression::Value(0)
            ),
        ])
    );

    for (condition, needle) in [
        (
            platform_value!({ "present": 1 }),
            "rule \"rule\" at present must name a property path",
        ),
        (
            platform_value!({ "absent": ["discount"] }),
            "rule \"rule\" at absent must name a property path",
        ),
        (
            platform_value!({ "not": { "present": { "add": ["price", 1] } } }),
            "rule \"rule\" at not.present must name a property path",
        ),
        (
            platform_value!({ "exists": "discount" }),
            "rule \"rule\" names \"exists\", which is not a comparison (equal, notEqual, \
             lessThan, lessThanOrEqual, greaterThan, greaterThanOrEqual), in, present, absent, \
             anyOf, allOf or not",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// A presence test is one node, and reads its property by presence, where an operand
/// reads one by value.
#[test]
fn should_count_a_presence_test_as_one_node_reading_by_presence() {
    let rule = parse_rule_value(platform_value!({
        "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
    }));
    // anyOf, absent discount, greaterThan, discount, 0
    assert_eq!(rule.node_count(), 5);
    assert_eq!(
        rule.property_reads(),
        [
            ("discount", PropertyRead::Presence),
            ("discount", PropertyRead::Value)
        ]
    );
    assert_eq!(rule.property_paths(), ["discount", "discount"]);
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

/// `a == 0 || b == 4`, and `allOf` and `not` over the same comparisons.
#[test]
fn should_combine_conditions_with_any_of_all_of_and_not() {
    let any_of = parse_rule_value(platform_value!({
        "anyOf": [{ "equal": ["a", 0] }, { "equal": ["b", 4] }]
    }));
    let all_of = parse_rule_value(platform_value!({
        "allOf": [{ "equal": ["a", 0] }, { "equal": ["b", 4] }]
    }));
    let not = parse_rule_value(platform_value!({
        "not": { "anyOf": [{ "equal": ["a", 0] }, { "equal": ["b", 4] }] }
    }));
    for (a, b, either, both) in [
        (0, 4, true, true),
        (0, 5, true, false),
        (1, 4, true, false),
        (1, 5, false, false),
    ] {
        let values = data(&[("a", Value::U64(a)), ("b", Value::U64(b))]);
        assert_eq!(any_of.holds(&values), Ok(either), "a {a}, b {b}: anyOf");
        assert_eq!(all_of.holds(&values), Ok(both), "a {a}, b {b}: allOf");
        assert_eq!(not.holds(&values), Ok(!either), "a {a}, b {b}: not");
        assert_eq!(
            any_of.violation(&values),
            (!either).then_some(PropertyConstraintViolation::NotMet),
            "a {a}, b {b}"
        );
    }
    // An absent property still counts as 0
    assert_eq!(any_of.holds(&data(&[("b", Value::U64(5))])), Ok(true));
}

/// Conditions are checked in declared order, no further than the outcome needs, so an
/// earlier one guards a later one; a fault in a condition that is checked breaks the rule
/// whatever the others would say, and `not` does not turn it into a pass.
#[test]
fn should_stop_at_the_outcome_and_break_the_rule_on_the_first_fault() {
    let quotient_is_two = platform_value!({ "equal": [{ "divide": ["a", "b"] }, 2] });
    let guarded_any_of = parse_rule_value(platform_value!({
        "anyOf": [{ "equal": ["b", 0] }, quotient_is_two.clone()]
    }));
    let unguarded_any_of = parse_rule_value(platform_value!({
        "anyOf": [quotient_is_two.clone(), { "equal": ["b", 0] }]
    }));
    let guarded_all_of = parse_rule_value(platform_value!({
        "allOf": [{ "notEqual": ["b", 0] }, quotient_is_two.clone()]
    }));
    let negated = parse_rule_value(platform_value!({ "not": quotient_is_two }));

    let values = |a: u64, b: u64| data(&[("a", Value::U64(a)), ("b", Value::U64(b))]);
    let zero_divisor = values(6, 0);
    assert_eq!(guarded_any_of.violation(&zero_divisor), None);
    assert_eq!(
        unguarded_any_of.violation(&zero_divisor),
        Some(PropertyConstraintViolation::DivisionByZero)
    );
    assert_eq!(
        guarded_all_of.violation(&zero_divisor),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(
        negated.violation(&zero_divisor),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    // 4 / 2 = 2, 6 / 2 = 3
    for rule in [&guarded_any_of, &unguarded_any_of, &guarded_all_of] {
        assert_eq!(rule.violation(&values(4, 2)), None, "{rule:?}");
        assert_eq!(
            rule.violation(&values(6, 2)),
            Some(PropertyConstraintViolation::NotMet),
            "{rule:?}"
        );
    }
    assert_eq!(
        negated.violation(&values(4, 2)),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(negated.violation(&values(6, 2)), None);

    // An allOf stops at the first condition that fails, before a later fault
    let fails_before_the_fault = parse_rule_value(platform_value!({
        "allOf": [{ "equal": ["a", 1] }, { "equal": [{ "divide": ["a", "b"] }, 2] }]
    }));
    assert_eq!(
        fails_before_the_fault.violation(&zero_divisor),
        Some(PropertyConstraintViolation::NotMet)
    );
}

/// Every comparison and logical operator is a node, and the paths are listed in declared
/// order.
#[test]
fn should_count_the_nodes_and_list_the_paths_of_combined_conditions() {
    let rule = parse_rule_value(platform_value!({
        "anyOf": [
            { "equal": ["a", 0] },
            { "not": { "equal": [{ "ifAbsent": ["b", 1] }, "a"] } }
        ]
    }));
    // anyOf, equal, a, 0, not, equal, ifAbsent b, a
    assert_eq!(rule.node_count(), 8);
    assert_eq!(rule.property_paths(), ["a", "b", "a"]);
}

/// A property the document leaves out, or sets to null, is absent, as it is for an
/// operand; one it sets to anything else, 0 and objects included, is present.
#[test]
fn should_tell_a_property_left_out_from_one_set_to_zero() {
    let values = data(&[
        ("zero", Value::U64(0)),
        ("empty", Value::Null),
        ("note", Value::Text("hi".to_string())),
        ("meta", platform_value!({ "count": 9 })),
        ("flat", Value::U8(1)),
    ]);
    for (path, present) in [
        ("zero", true),
        ("note", true),
        ("meta", true),
        ("meta.count", true),
        ("missing", false),
        ("empty", false),
        ("meta.missing", false),
        // An intermediate that is not an object reads as absent
        ("flat.count", false),
    ] {
        let present_rule = parse_rule_value(platform_value!({ "present": path }));
        let absent_rule = parse_rule_value(platform_value!({ "absent": path }));
        assert_eq!(present_rule.holds(&values), Ok(present), "present {path}");
        assert_eq!(absent_rule.holds(&values), Ok(!present), "absent {path}");
    }

    // Optional, but above zero when given: an operand alone reads a discount left out
    // as 0, so it cannot say this
    let rule = parse_rule_value(platform_value!({
        "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
    }));
    assert_eq!(rule.violation(&data(&[])), None);
    assert_eq!(rule.violation(&data(&[("discount", Value::U64(5))])), None);
    assert_eq!(
        rule.violation(&data(&[("discount", Value::U64(0))])),
        Some(PropertyConstraintViolation::NotMet)
    );
}
