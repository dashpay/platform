use super::*;
use platform_value::platform_value;
use platform_version::version::PLATFORM_VERSIONS;

/// The paths the unit tests treat as string properties, `labels` an array of
/// strings, as a document type's parse reports one.
const STRING_PROPERTIES: [&str; 5] = ["status", "from", "to", "meta.state", "labels"];

/// The paths the unit tests treat as identifier properties; every path neither lists
/// is an integer one.
/// `members` is an array of identifiers.
const IDENTIFIER_PROPERTIES: [&str; 4] = ["buyerId", "sellerId", "meta.ownerRef", "members"];

fn property_kind(path: &str) -> Option<EqualityKind> {
    if STRING_PROPERTIES.contains(&path) {
        Some(EqualityKind::Text)
    } else if IDENTIFIER_PROPERTIES.contains(&path) {
        Some(EqualityKind::Identifier)
    } else {
        None
    }
}

/// The rules of a schema whose `propertyConstraints` is `declaration`.
fn parse(declaration: Value) -> Result<BTreeMap<String, PropertyConstraint>, DataContractError> {
    let schema = platform_value!({ "type": "object", "propertyConstraints": declaration });
    parse_property_constraints(&schema, "order", &property_kind)
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
        PropertyConstraint::Compare { left, .. } => {
            left.evaluate(data, &DocumentSystemValues::default())
        }
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
    assert!(parse_property_constraints(&schema, "order", &property_kind)
        .expect("parses")
        .is_empty());
    // A schema that is not an object is the core parser's to refuse
    assert!(
        parse_property_constraints(&Value::Text("x".to_string()), "order", &property_kind)
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
            platform_value!({ "rule": { "equal": [{ "ifAbsent": ["price", true] }, 1] } }),
            "at equal[0].ifAbsent must give an integer value second",
        ),
        // A string default reads a string property, which arithmetic never takes
        (
            platform_value!({
                "rule": { "equal": [{ "add": [{ "ifAbsent": ["price", "fee"] }, 1] }, 1] }
            }),
            "at equal[0].add[0].ifAbsent gives a string default, which only a comparison of \
             strings takes, never an integer expression",
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
            rule.holds(
                &data(&[("kind", Value::U64(kind))]),
                &DocumentSystemValues::default()
            ),
            Ok(holds),
            "kind {kind}"
        );
    }
    // An absent operand reads as 0
    assert_eq!(
        rule.holds(&data(&[]), &DocumentSystemValues::default()),
        Ok(false)
    );
    let with_zero = parse_rule_value(platform_value!({ "in": ["kind", [0, 1]] }));
    assert_eq!(
        with_zero.holds(&data(&[]), &DocumentSystemValues::default()),
        Ok(true)
    );

    let divided = parse_rule_value(platform_value!({ "in": [{ "divide": [10, "kind"] }, [2, 5]] }));
    assert_eq!(
        divided.violation(
            &data(&[("kind", Value::U64(5))]),
            &DocumentSystemValues::default()
        ),
        None
    );
    assert_eq!(
        divided.violation(
            &data(&[("kind", Value::U64(3))]),
            &DocumentSystemValues::default()
        ),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(
        divided.violation(
            &data(&[("kind", Value::U64(0))]),
            &DocumentSystemValues::default()
        ),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    // in, kind, and one per value
    assert_eq!(rule.node_count(), 5);
    assert_eq!(rule.property_reads(), [("kind", PropertyRead::Value)]);
}

// ── strings ─────────────────────────────────────────────────────────────

/// A string property read without a default.
fn text(path: &str) -> TextProperty {
    TextProperty {
        path: path.to_string(),
        if_absent: None,
    }
}

/// A string property read with a string default, `{ "ifAbsent": [path, default] }`.
fn text_or(path: &str, default: &str) -> TextProperty {
    TextProperty {
        path: path.to_string(),
        if_absent: Some(default.to_string()),
    }
}

fn text_compare(comparison: ConstraintComparison, path: &str, value: &str) -> PropertyConstraint {
    PropertyConstraint::TextCompare {
        comparison,
        property: text(path),
        value: value.to_string(),
    }
}

/// A string constant is a `const` object, since a string on its own is a path; a
/// comparison with one is the same whichever side it sits on.
#[test]
fn should_parse_string_comparisons() {
    assert_eq!(
        parse_rule_value(platform_value!({ "equal": ["status", { "const": "closed" }] })),
        text_compare(ConstraintComparison::Equal, "status", "closed")
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "notEqual": [{ "const": "closed" }, "meta.state"] })),
        text_compare(ConstraintComparison::NotEqual, "meta.state", "closed")
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "in": ["status", ["pending", "open"]] })),
        PropertyConstraint::TextIn {
            property: text("status"),
            values: BTreeSet::from(["open".to_string(), "pending".to_string()]),
        }
    );

    for (condition, needle) in [
        (
            platform_value!({ "lessThan": ["status", { "const": "b" }] }),
            "rule \"rule\" at lessThan compares strings, which only equal and notEqual do",
        ),
        (
            platform_value!({ "equal": ["status", { "const": 5 }] }),
            "rule \"rule\" at equal[1].const must be a string: an integer is written as itself",
        ),
        (
            platform_value!({ "equal": [{ "const": "a" }, { "const": "b" }] }),
            "rule \"rule\" reads no property",
        ),
        (
            platform_value!({
                "anyOf": [
                    { "equal": ["fee", 1] },
                    { "notEqual": [{ "const": "a" }, { "const": "a" }] }
                ]
            }),
            "rule \"rule\" at anyOf[1] reads no property",
        ),
        // The other side is a property, never an expression or a value
        (
            platform_value!({ "equal": [{ "ifAbsent": ["status", 0] }, { "const": "a" }] }),
            "rule \"rule\" at equal[0] must be the path of a string property, an ifAbsent giving \
             one a string default, or a const",
        ),
        (
            platform_value!({ "equal": [5, { "const": "a" }] }),
            "rule \"rule\" at equal[0] must be the path of a string property",
        ),
        // A constant is no integer operand
        (
            platform_value!({ "equal": [{ "add": ["price", { "const": "a" }] }, 1] }),
            "rule \"rule\" at equal[0].add[1] is a string constant, which only equal and notEqual \
             compare",
        ),
        (
            platform_value!({ "in": [{ "const": "a" }, [1, 2]] }),
            "rule \"rule\" at in[0] is a string constant",
        ),
        (
            platform_value!({ "in": [{ "add": ["status", 1] }, ["open", "closed"]] }),
            "rule \"rule\" at in[0] must be the path of a string property or an ifAbsent giving \
             one a string default: an in over strings reads a string property",
        ),
        (
            platform_value!({ "in": ["status", ["open"]] }),
            "rule \"rule\" at in[1] must list two or more string values",
        ),
        (
            platform_value!({ "in": ["status", ["open", 2]] }),
            "rule \"rule\" at in[1][1] must be a string, as the first value is",
        ),
        (
            platform_value!({ "in": ["status", ["open", "closed", "open"]] }),
            "rule \"rule\" at in[1][2] repeats the value at in[1][0]",
        ),
        (
            platform_value!({ "in": ["fee", [1, "two"]] }),
            "rule \"rule\" at in[1][1] must be an integer value",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// A string property equals a constant when it holds that string; one the document
/// leaves out, sets to null or holds as something else equals none, so `notEqual`
/// holds for it and `in` does not.
#[test]
fn should_compare_a_string_property_with_constants() {
    let equal = parse_rule_value(platform_value!({ "equal": ["status", { "const": "closed" }] }));
    let not_equal =
        parse_rule_value(platform_value!({ "notEqual": ["status", { "const": "closed" }] }));
    let in_list = parse_rule_value(platform_value!({ "in": ["status", ["open", "closed"]] }));
    for (status, is_closed, is_listed) in [
        (Some(Value::Text("closed".to_string())), true, true),
        (Some(Value::Text("open".to_string())), false, true),
        (Some(Value::Text("Closed".to_string())), false, false),
        (Some(Value::Text(String::new())), false, false),
        (Some(Value::Null), false, false),
        (Some(Value::U64(1)), false, false),
        (None, false, false),
    ] {
        let values = match &status {
            Some(value) => data(&[("status", value.clone())]),
            None => data(&[]),
        };
        assert_eq!(
            equal.holds(&values, &DocumentSystemValues::default()),
            Ok(is_closed),
            "equal, {status:?}"
        );
        assert_eq!(
            not_equal.holds(&values, &DocumentSystemValues::default()),
            Ok(!is_closed),
            "notEqual, {status:?}"
        );
        assert_eq!(
            in_list.holds(&values, &DocumentSystemValues::default()),
            Ok(is_listed),
            "in, {status:?}"
        );
    }

    // A closed order must carry closedAt
    let rule = parse_rule_value(platform_value!({
        "anyOf": [{ "notEqual": ["status", { "const": "closed" }] }, { "present": "closedAt" }]
    }));
    let closed = Value::Text("closed".to_string());
    assert_eq!(
        rule.violation(&data(&[]), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        rule.violation(
            &data(&[("status", closed.clone()), ("closedAt", Value::U64(9))]),
            &DocumentSystemValues::default()
        ),
        None
    );
    assert_eq!(
        rule.violation(
            &data(&[("status", closed)]),
            &DocumentSystemValues::default()
        ),
        Some(PropertyConstraintViolation::NotMet)
    );
}

/// A string comparison is three nodes, as a comparison of a path with a value; an in over
/// strings two plus one per value. Both read their property as text, and list their
/// constants for the enum check.
#[test]
fn should_count_and_list_what_a_string_comparison_reads() {
    let rule = parse_rule_value(platform_value!({
        "anyOf": [
            { "equal": ["status", { "const": "closed" }] },
            { "in": ["kind", ["b", "a", "c"]] }
        ]
    }));
    // anyOf, equal, status, closed, in, kind, a, b, c
    assert_eq!(rule.node_count(), 9);
    assert_eq!(
        rule.property_reads(),
        [("status", PropertyRead::Text), ("kind", PropertyRead::Text)]
    );
    assert_eq!(
        rule.text_constants(),
        [
            ("status", "closed"),
            ("kind", "a"),
            ("kind", "b"),
            ("kind", "c")
        ]
    );
    // Written either way round, the same condition
    let rule = parse_rule_value(platform_value!({
        "anyOf": [
            { "equal": ["status", { "const": "closed" }] },
            { "equal": ["fee", 1] },
            { "equal": [{ "const": "closed" }, "status"] }
        ]
    }));
    assert_eq!(
        rule.repeated_condition(),
        Some(("anyOf[2]".to_string(), "anyOf[0]".to_string()))
    );
}

/// Two bare paths that both name string properties compare the strings; anything else
/// between two expressions stays an integer comparison.
#[test]
fn should_parse_a_comparison_of_two_string_properties() {
    assert_eq!(
        parse_rule_value(platform_value!({ "equal": ["from", "to"] })),
        PropertyConstraint::TextCompareProperties {
            comparison: ConstraintComparison::Equal,
            left: text("from"),
            right: text("to"),
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "notEqual": ["status", "meta.state"] })),
        PropertyConstraint::TextCompareProperties {
            comparison: ConstraintComparison::NotEqual,
            left: text("status"),
            right: text("meta.state"),
        }
    );
    // A string and an integer property, or a path inside an expression, stay integer
    // comparisons, which the document type check refuses for the string
    assert!(matches!(
        parse_rule_value(platform_value!({ "equal": ["from", "price"] })),
        PropertyConstraint::Compare { .. }
    ));
    assert!(matches!(
        parse_rule_value(platform_value!({ "equal": [{ "ifAbsent": ["from", 0] }, "to"] })),
        PropertyConstraint::Compare { .. }
    ));

    expect_refusal(
        platform_value!({
            "rule": { "anyOf": [{ "equal": ["fee", 1] }, { "lessThan": ["from", "to"] }] }
        }),
        "rule \"rule\" at anyOf[1].lessThan compares strings, which only equal and notEqual do",
    );
}

/// Two string properties are equal when the document holds the same string in both; one
/// it leaves out equals no string, not even another one it leaves out.
#[test]
fn should_compare_two_string_properties() {
    let equal = parse_rule_value(platform_value!({ "equal": ["from", "to"] }));
    let not_equal = parse_rule_value(platform_value!({ "notEqual": ["from", "to"] }));
    let text = |value: &str| Value::Text(value.to_string());
    for (from, to, same) in [
        (Some(text("USD")), Some(text("USD")), true),
        (Some(text("USD")), Some(text("EUR")), false),
        (Some(text("USD")), Some(text("usd")), false),
        (Some(text("")), Some(text("")), true),
        (Some(text("USD")), None, false),
        (None, None, false),
        (Some(Value::Null), Some(Value::Null), false),
        (Some(Value::U64(1)), Some(Value::U64(1)), false),
    ] {
        let mut entries = Vec::new();
        if let Some(from) = &from {
            entries.push(("from", from.clone()));
        }
        if let Some(to) = &to {
            entries.push(("to", to.clone()));
        }
        let values = data(&entries);
        assert_eq!(
            equal.holds(&values, &DocumentSystemValues::default()),
            Ok(same),
            "equal, {from:?} {to:?}"
        );
        assert_eq!(
            not_equal.holds(&values, &DocumentSystemValues::default()),
            Ok(!same),
            "notEqual, {from:?} {to:?}"
        );
    }

    // equal, from, to
    assert_eq!(equal.node_count(), 3);
    assert_eq!(
        equal.property_reads(),
        [("from", PropertyRead::Text), ("to", PropertyRead::Text)]
    );
    assert!(equal.text_constants().is_empty());
}

/// `{ "ifAbsent": [path, string] }` is a string property with a default: it makes a
/// comparison one of strings wherever it sits, in `equal`, `notEqual` and `in`.
#[test]
fn should_parse_a_string_default() {
    assert_eq!(
        parse_rule_value(platform_value!({
            "equal": [{ "ifAbsent": ["status", "open"] }, { "const": "open" }]
        })),
        PropertyConstraint::TextCompare {
            comparison: ConstraintComparison::Equal,
            property: text_or("status", "open"),
            value: "open".to_string(),
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({
            "notEqual": [{ "ifAbsent": ["from", "USD"] }, "to"]
        })),
        PropertyConstraint::TextCompareProperties {
            comparison: ConstraintComparison::NotEqual,
            left: text_or("from", "USD"),
            right: text("to"),
        }
    );
    // A default makes the comparison one of strings even beside a path the unit tests
    // treat as an integer; the document type check refuses that path
    assert_eq!(
        parse_rule_value(platform_value!({ "equal": ["price", { "ifAbsent": ["to", "x"] }] })),
        PropertyConstraint::TextCompareProperties {
            comparison: ConstraintComparison::Equal,
            left: text("price"),
            right: text_or("to", "x"),
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({
            "in": [{ "ifAbsent": ["status", "open"] }, ["open", "closed"]]
        })),
        PropertyConstraint::TextIn {
            property: text_or("status", "open"),
            values: BTreeSet::from(["closed".to_string(), "open".to_string()]),
        }
    );

    for (condition, needle) in [
        (
            platform_value!({ "lessThan": [{ "ifAbsent": ["status", "a"] }, "to"] }),
            "rule \"rule\" at lessThan compares strings, which only equal and notEqual do",
        ),
        (
            platform_value!({ "equal": [{ "ifAbsent": [1, "a"] }, { "const": "a" }] }),
            "rule \"rule\" at equal[0].ifAbsent must name a property path first",
        ),
        (
            platform_value!({ "equal": [{ "ifAbsent": ["status", "a"] }, 5] }),
            "rule \"rule\" at equal[1] must be the path of a string property",
        ),
        (
            platform_value!({ "in": [{ "ifAbsent": ["status", 1] }, ["a", "b"]] }),
            "rule \"rule\" at in[0] must be the path of a string property or an ifAbsent giving \
             one a string default",
        ),
        (
            platform_value!({ "in": [{ "ifAbsent": ["kind", "a"] }, [1, 2]] }),
            "rule \"rule\" at in[0].ifAbsent gives a string default, which only a comparison of \
             strings takes",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// A string default stands in for a property the document leaves out or sets to null,
/// never for one it holds; two properties left out with the same default are equal.
#[test]
fn should_read_a_string_default_for_a_property_left_out() {
    let open = parse_rule_value(platform_value!({
        "equal": [{ "ifAbsent": ["status", "open"] }, { "const": "open" }]
    }));
    let listed = parse_rule_value(platform_value!({
        "in": [{ "ifAbsent": ["status", "open"] }, ["open", "pending"]]
    }));
    let text_value = |value: &str| Value::Text(value.to_string());
    for (status, is_open) in [
        (None, true),
        (Some(Value::Null), true),
        (Some(text_value("open")), true),
        (Some(text_value("closed")), false),
        // Held, so no default, and not a string, so no match
        (Some(Value::U64(1)), false),
    ] {
        let values = match &status {
            Some(value) => data(&[("status", value.clone())]),
            None => data(&[]),
        };
        assert_eq!(
            open.holds(&values, &DocumentSystemValues::default()),
            Ok(is_open),
            "equal, {status:?}"
        );
        assert_eq!(
            listed.holds(&values, &DocumentSystemValues::default()),
            Ok(is_open),
            "in, {status:?}"
        );
    }

    let same_default = parse_rule_value(platform_value!({
        "equal": [{ "ifAbsent": ["from", "USD"] }, { "ifAbsent": ["to", "USD"] }]
    }));
    assert_eq!(
        same_default.holds(&data(&[]), &DocumentSystemValues::default()),
        Ok(true)
    );
    assert_eq!(
        same_default.holds(
            &data(&[("to", text_value("USD"))]),
            &DocumentSystemValues::default()
        ),
        Ok(true)
    );
    assert_eq!(
        same_default.holds(
            &data(&[("to", text_value("EUR"))]),
            &DocumentSystemValues::default()
        ),
        Ok(false)
    );
    // Without defaults, two properties left out are not equal
    let bare = parse_rule_value(platform_value!({ "equal": ["from", "to"] }));
    assert_eq!(
        bare.holds(&data(&[]), &DocumentSystemValues::default()),
        Ok(false)
    );

    // A default is part of the node it sits in, and listed for the enum check apart
    // from the constants compared
    assert_eq!(open.node_count(), 3);
    assert_eq!(open.text_constants(), [("status", "open")]);
    assert_eq!(open.text_defaults(), [("status", "open")]);
    assert_eq!(
        same_default.text_defaults(),
        [("from", "USD"), ("to", "USD")]
    );
    assert!(bare.text_defaults().is_empty());

    // A default makes a different condition from the bare path
    let rule = parse_rule_value(platform_value!({
        "anyOf": [
            { "equal": ["status", { "const": "open" }] },
            { "equal": [{ "ifAbsent": ["status", "open"] }, { "const": "open" }] }
        ]
    }));
    assert_eq!(rule.repeated_condition(), None);
}

// ── identifiers ─────────────────────────────────────────────────────────

/// The identifier made of 32 copies of `byte`, and its base58 spelling.
fn identifier(byte: u8) -> (Identifier, String) {
    let identifier = Identifier::new([byte; 32]);
    let base58 = identifier.to_string(Encoding::Base58);
    (identifier, base58)
}

/// A `const` or a listed value compared with an identifier property is a base58
/// identifier; two bare paths naming identifier properties compare identifiers.
#[test]
fn should_parse_identifier_comparisons() {
    let (a, a58) = identifier(1);
    let (b, b58) = identifier(2);
    assert_eq!(
        parse_rule_value(platform_value!({ "equal": ["buyerId", { "const": a58.clone() }] })),
        PropertyConstraint::IdentifierCompare {
            comparison: ConstraintComparison::Equal,
            path: "buyerId".to_string(),
            value: a,
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({
            "notEqual": [{ "const": b58.clone() }, "meta.ownerRef"]
        })),
        PropertyConstraint::IdentifierCompare {
            comparison: ConstraintComparison::NotEqual,
            path: "meta.ownerRef".to_string(),
            value: b,
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "notEqual": ["buyerId", "sellerId"] })),
        PropertyConstraint::IdentifierCompareProperties {
            comparison: ConstraintComparison::NotEqual,
            left: "buyerId".to_string(),
            right: "sellerId".to_string(),
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "in": ["sellerId", [b58.clone(), a58.clone()]] })),
        PropertyConstraint::IdentifierIn {
            path: "sellerId".to_string(),
            values: BTreeSet::from([a, b]),
        }
    );
    // An identifier property beside an integer stays an integer comparison, which the
    // document type check refuses for the identifier
    assert!(matches!(
        parse_rule_value(platform_value!({ "equal": ["buyerId", 5] })),
        PropertyConstraint::Compare { .. }
    ));

    for (condition, needle) in [
        (
            platform_value!({ "lessThan": ["buyerId", "sellerId"] }),
            "rule \"rule\" at lessThan compares identifiers, which only equal and notEqual do",
        ),
        (
            platform_value!({ "notEqual": ["buyerId", "status"] }),
            "rule \"rule\" at notEqual compares a string property with an identifier property",
        ),
        (
            platform_value!({ "equal": ["buyerId", { "const": "not base58!" }] }),
            "rule \"rule\" at equal[1].const holds \"not base58!\", which is not a base58 \
             identifier of 32 bytes",
        ),
        (
            platform_value!({ "equal": ["buyerId", { "const": "2" }] }),
            "rule \"rule\" at equal[1].const holds \"2\", which is not a base58 identifier of 32 \
             bytes",
        ),
        (
            platform_value!({ "equal": ["buyerId", { "const": 5 }] }),
            "rule \"rule\" at equal[1].const must be an identifier, written base58",
        ),
        (
            platform_value!({ "equal": [{ "ifAbsent": ["buyerId", "x"] }, "sellerId"] }),
            "rule \"rule\" at equal[0] gives an identifier property a default, which identifiers \
             do not take",
        ),
        (
            platform_value!({ "in": ["buyerId", [a58.clone()]] }),
            "rule \"rule\" at in[1] must list two or more identifiers",
        ),
        (
            platform_value!({ "in": ["buyerId", [a58.clone(), "bad"]] }),
            "rule \"rule\" at in[1][1] holds \"bad\", which is not a base58 identifier",
        ),
        (
            platform_value!({ "in": ["buyerId", [a58.clone(), 2]] }),
            "rule \"rule\" at in[1][1] must be an identifier, written base58",
        ),
        (
            platform_value!({ "in": ["buyerId", [a58.clone(), b58.clone(), a58.clone()]] }),
            "rule \"rule\" at in[1][2] repeats the value at in[1][0]",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// An identifier property equals a constant or another one when both hold the same 32
/// bytes, in whichever form the document gives them; one it leaves out equals nothing.
#[test]
fn should_compare_identifier_properties() {
    let (a, a58) = identifier(1);
    let (b, b58) = identifier(2);
    let is_a =
        parse_rule_value(platform_value!({ "equal": ["buyerId", { "const": a58.clone() }] }));
    let listed = parse_rule_value(platform_value!({ "in": ["buyerId", [a58, b58]] }));
    for (buyer, equals_a, is_listed) in [
        (Some(Value::Identifier(a.to_buffer())), true, true),
        (Some(Value::Bytes32(a.to_buffer())), true, true),
        (Some(Value::Bytes(a.to_vec())), true, true),
        (Some(Value::Identifier(b.to_buffer())), false, true),
        (Some(Value::Identifier([3; 32])), false, false),
        (Some(Value::Null), false, false),
        (Some(Value::U64(1)), false, false),
        (None, false, false),
    ] {
        let values = match &buyer {
            Some(value) => data(&[("buyerId", value.clone())]),
            None => data(&[]),
        };
        assert_eq!(
            is_a.holds(&values, &DocumentSystemValues::default()),
            Ok(equals_a),
            "equal, {buyer:?}"
        );
        assert_eq!(
            listed.holds(&values, &DocumentSystemValues::default()),
            Ok(is_listed),
            "in, {buyer:?}"
        );
    }

    let distinct = parse_rule_value(platform_value!({ "notEqual": ["buyerId", "sellerId"] }));
    let pair = |buyer: Option<Identifier>, seller: Option<Identifier>| {
        let mut entries = Vec::new();
        if let Some(buyer) = buyer {
            entries.push(("buyerId", Value::Identifier(buyer.to_buffer())));
        }
        if let Some(seller) = seller {
            entries.push(("sellerId", Value::Bytes32(seller.to_buffer())));
        }
        data(&entries)
    };
    assert_eq!(
        distinct.holds(&pair(Some(a), Some(b)), &DocumentSystemValues::default()),
        Ok(true)
    );
    assert_eq!(
        distinct.holds(&pair(Some(a), Some(a)), &DocumentSystemValues::default()),
        Ok(false)
    );
    assert_eq!(
        distinct.holds(&pair(Some(a), None), &DocumentSystemValues::default()),
        Ok(true)
    );
    // Two identifiers left out are not equal
    assert_eq!(
        distinct.holds(&pair(None, None), &DocumentSystemValues::default()),
        Ok(true)
    );

    // A comparison of a path with a constant is three nodes, an in two plus one per value
    assert_eq!(is_a.node_count(), 3);
    assert_eq!(distinct.node_count(), 3);
    assert_eq!(listed.node_count(), 4);
    assert_eq!(
        distinct.property_reads(),
        [
            ("buyerId", PropertyRead::Identifier),
            ("sellerId", PropertyRead::Identifier)
        ]
    );
    // Identifier constants are not string constants: no enum check reads them
    assert!(is_a.text_constants().is_empty());
}

// ── $ownerId ───────────────────────────────────────────────────────────

/// `$ownerId`, the document's owner, is an identifier operand: beside an identifier
/// property, a base58 constant, or as the operand of an `in` over identifiers.
#[test]
fn should_parse_the_owner_as_an_identifier_operand() {
    let (a, a58) = identifier(1);
    let (b, b58) = identifier(2);
    assert_eq!(
        parse_rule_value(platform_value!({ "equal": ["buyerId", "$ownerId"] })),
        PropertyConstraint::IdentifierCompareProperties {
            comparison: ConstraintComparison::Equal,
            left: "buyerId".to_string(),
            right: "$ownerId".to_string(),
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "notEqual": [{ "const": a58.clone() }, "$ownerId"] })),
        PropertyConstraint::IdentifierCompare {
            comparison: ConstraintComparison::NotEqual,
            path: "$ownerId".to_string(),
            value: a,
        }
    );
    assert_eq!(
        parse_rule_value(platform_value!({ "in": ["$ownerId", [a58.clone(), b58.clone()]] })),
        PropertyConstraint::IdentifierIn {
            path: "$ownerId".to_string(),
            values: BTreeSet::from([a, b]),
        }
    );

    for (condition, needle) in [
        (
            platform_value!({ "equal": ["$ownerId", "$ownerId"] }),
            "rule \"rule\" at equal compares \"$ownerId\" with itself, so it would hold for every \
             document or for none",
        ),
        (
            platform_value!({ "notEqual": ["buyerId", "buyerId"] }),
            "rule \"rule\" at notEqual compares \"buyerId\" with itself",
        ),
        (
            platform_value!({ "equal": ["status", "status"] }),
            "rule \"rule\" at equal compares \"status\" with itself",
        ),
        (
            platform_value!({ "lessThan": ["$ownerId", "buyerId"] }),
            "rule \"rule\" at lessThan compares identifiers, which only equal and notEqual do",
        ),
        (
            platform_value!({ "equal": ["$ownerId", "status"] }),
            "rule \"rule\" at equal compares a string property with an identifier property",
        ),
        (
            platform_value!({ "equal": ["$ownerId", { "const": "closed" }] }),
            "rule \"rule\" at equal[1].const holds \"closed\", which is not a base58 identifier",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": condition }), needle);
    }
}

/// `$ownerId` reads the owner the caller gives, and equals nothing when it gives none; it
/// is no property, so a rule reading it lists no path for it, and says it reads the owner.
#[test]
fn should_compare_the_owner() {
    let (a, a58) = identifier(1);
    let (b, b58) = identifier(2);
    let (c, _) = identifier(3);
    let buyer_owns = parse_rule_value(platform_value!({ "equal": ["buyerId", "$ownerId"] }));
    let allowed_writers = parse_rule_value(platform_value!({ "in": ["$ownerId", [a58, b58]] }));
    let buyer =
        |identifier: Identifier| data(&[("buyerId", Value::Identifier(identifier.to_buffer()))]);

    assert_eq!(
        buyer_owns.holds(&buyer(a), &DocumentSystemValues::owned_by(a)),
        Ok(true)
    );
    assert_eq!(
        buyer_owns.holds(&buyer(a), &DocumentSystemValues::owned_by(b)),
        Ok(false)
    );
    assert_eq!(
        buyer_owns.holds(&buyer(a), &DocumentSystemValues::default()),
        Ok(false)
    );
    assert_eq!(
        buyer_owns.holds(&data(&[]), &DocumentSystemValues::owned_by(a)),
        Ok(false)
    );
    assert_eq!(
        allowed_writers.holds(&data(&[]), &DocumentSystemValues::owned_by(b)),
        Ok(true)
    );
    assert_eq!(
        allowed_writers.holds(&data(&[]), &DocumentSystemValues::owned_by(c)),
        Ok(false)
    );
    assert_eq!(
        allowed_writers.holds(&data(&[]), &DocumentSystemValues::default()),
        Ok(false)
    );

    assert_eq!(
        buyer_owns.property_reads(),
        [("buyerId", PropertyRead::Identifier)]
    );
    assert!(allowed_writers.property_reads().is_empty());
    assert!(buyer_owns.reads_owner());
    assert!(allowed_writers.reads_owner());
    assert!(parse_rule_value(platform_value!({
        "anyOf": [{ "equal": ["fee", 1] }, { "not": { "equal": ["sellerId", "$ownerId"] } }]
    }))
    .reads_owner());
    assert!(
        !parse_rule_value(platform_value!({ "notEqual": ["buyerId", "sellerId"] })).reads_owner()
    );
    // equal, buyerId, $ownerId
    assert_eq!(buyer_owns.node_count(), 3);
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
             lessThan, lessThanOrEqual, greaterThan, greaterThanOrEqual), in, notIn, \
             startsWith, endsWith, contains, present, absent, anyOf, allOf, not, ifThen or \
             ifThenElse",
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
    assert_eq!(
        rule.violation(&order(10, 2, 3, 36), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        rule.violation(&order(10, 2, 3, 100), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        rule.violation(&order(10, 2, 3, 35), &DocumentSystemValues::default()),
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
        both_sides_fail.violation(&values, &DocumentSystemValues::default()),
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
        assert_eq!(
            any_of.holds(&values, &DocumentSystemValues::default()),
            Ok(either),
            "a {a}, b {b}: anyOf"
        );
        assert_eq!(
            all_of.holds(&values, &DocumentSystemValues::default()),
            Ok(both),
            "a {a}, b {b}: allOf"
        );
        assert_eq!(
            not.holds(&values, &DocumentSystemValues::default()),
            Ok(!either),
            "a {a}, b {b}: not"
        );
        assert_eq!(
            any_of.violation(&values, &DocumentSystemValues::default()),
            (!either).then_some(PropertyConstraintViolation::NotMet),
            "a {a}, b {b}"
        );
    }
    // An absent property still counts as 0
    assert_eq!(
        any_of.holds(
            &data(&[("b", Value::U64(5))]),
            &DocumentSystemValues::default()
        ),
        Ok(true)
    );
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
    assert_eq!(
        guarded_any_of.violation(&zero_divisor, &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        unguarded_any_of.violation(&zero_divisor, &DocumentSystemValues::default()),
        Some(PropertyConstraintViolation::DivisionByZero)
    );
    assert_eq!(
        guarded_all_of.violation(&zero_divisor, &DocumentSystemValues::default()),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(
        negated.violation(&zero_divisor, &DocumentSystemValues::default()),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    // 4 / 2 = 2, 6 / 2 = 3
    for rule in [&guarded_any_of, &unguarded_any_of, &guarded_all_of] {
        assert_eq!(
            rule.violation(&values(4, 2), &DocumentSystemValues::default()),
            None,
            "{rule:?}"
        );
        assert_eq!(
            rule.violation(&values(6, 2), &DocumentSystemValues::default()),
            Some(PropertyConstraintViolation::NotMet),
            "{rule:?}"
        );
    }
    assert_eq!(
        negated.violation(&values(4, 2), &DocumentSystemValues::default()),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(
        negated.violation(&values(6, 2), &DocumentSystemValues::default()),
        None
    );

    // An allOf stops at the first condition that fails, before a later fault
    let fails_before_the_fault = parse_rule_value(platform_value!({
        "allOf": [{ "equal": ["a", 1] }, { "equal": [{ "divide": ["a", "b"] }, 2] }]
    }));
    assert_eq!(
        fails_before_the_fault.violation(&zero_divisor, &DocumentSystemValues::default()),
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
        ("hollow", platform_value!({})),
        ("nested", platform_value!({ "inner": {}, "gone": null })),
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
        // An object with no member present is not kept in storage
        ("hollow", false),
        ("nested", false),
        ("nested.inner", false),
    ] {
        let present_rule = parse_rule_value(platform_value!({ "present": path }));
        let absent_rule = parse_rule_value(platform_value!({ "absent": path }));
        assert_eq!(
            present_rule.holds(&values, &DocumentSystemValues::default()),
            Ok(present),
            "present {path}"
        );
        assert_eq!(
            absent_rule.holds(&values, &DocumentSystemValues::default()),
            Ok(!present),
            "absent {path}"
        );
    }

    // Optional, but above zero when given: an operand alone reads a discount left out
    // as 0, so it cannot say this
    let rule = parse_rule_value(platform_value!({
        "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
    }));
    assert_eq!(
        rule.violation(&data(&[]), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        rule.violation(
            &data(&[("discount", Value::U64(5))]),
            &DocumentSystemValues::default()
        ),
        None
    );
    assert_eq!(
        rule.violation(
            &data(&[("discount", Value::U64(0))]),
            &DocumentSystemValues::default()
        ),
        Some(PropertyConstraintViolation::NotMet)
    );
}

/// A boolean reads as 1 for true and 0 for false, and one the document leaves out as 0
/// or its `ifAbsent` value, as any operand does.
#[test]
fn should_read_a_boolean_as_one_or_zero() {
    let values = data(&[
        ("yes", Value::Bool(true)),
        ("no", Value::Bool(false)),
        ("fee", Value::U64(10)),
    ]);
    for (expression, expected) in [
        (platform_value!("yes"), 1),
        (platform_value!("no"), 0),
        (platform_value!("missing"), 0),
        (platform_value!({ "ifAbsent": ["missing", 1] }), 1),
        (platform_value!({ "ifAbsent": ["no", 1] }), 0),
        (platform_value!({ "add": ["yes", "yes", "no"] }), 2),
        (platform_value!({ "multiply": ["yes", "fee"] }), 10),
        (platform_value!({ "multiply": ["no", "fee"] }), 0),
    ] {
        assert_eq!(
            evaluate(expression.clone(), &values),
            Ok(expected),
            "{expression:?}"
        );
    }

    // A waived fee is 0: `waived * fee == 0`
    let rule = parse_rule_value(platform_value!({
        "equal": [{ "multiply": ["waived", "fee"] }, 0]
    }));
    let order =
        |waived: bool, fee: u64| data(&[("waived", Value::Bool(waived)), ("fee", Value::U64(fee))]);
    assert_eq!(
        rule.violation(&order(true, 0), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        rule.violation(&order(false, 10), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        rule.violation(&order(true, 10), &DocumentSystemValues::default()),
        Some(PropertyConstraintViolation::NotMet)
    );
}

// ── sizes ───────────────────────────────────────────────────────────────

/// `length`, `byteLength` and `count` are operands naming a property, one node
/// each, read by their size.
#[test]
fn should_parse_the_size_operands() {
    for (key, measure, read) in [
        ("length", SizeMeasure::Length, PropertyRead::Length),
        ("byteLength", SizeMeasure::ByteLength, PropertyRead::Length),
        ("count", SizeMeasure::Count, PropertyRead::Count),
    ] {
        let rule = parse_rule_value(platform_value!({
            "lessThanOrEqual": [{ key: "meta.body" }, "limit"]
        }));
        assert_eq!(
            rule,
            PropertyConstraint::Compare {
                comparison: ConstraintComparison::LessThanOrEqual,
                left: ConstraintExpression::Size {
                    measure,
                    path: "meta.body".to_string(),
                },
                right: property("limit"),
            },
            "{key}"
        );
        assert_eq!(measure.wire_name(), key);
        assert_eq!(rule.node_count(), 3, "{key}");
        assert_eq!(
            rule.property_reads(),
            [("meta.body", read), ("limit", PropertyRead::Value)],
            "{key}"
        );
    }

    // A size reads a property, so it alone keeps a comparison with a literal
    // meaningful, inside arithmetic and in an `in` too
    let rule = parse_rule_value(platform_value!({
        "in": [{ "add": [{ "count": "tags" }, 1] }, [1, 2, 3]]
    }));
    assert_eq!(rule.property_reads(), [("tags", PropertyRead::Count)]);
    assert_eq!(rule.node_count(), 7);

    // Two measures of one property are different conditions
    let rules = parse(platform_value!({
        "rule": {
            "anyOf": [
                { "lessThanOrEqual": [{ "length": "title" }, 10] },
                { "lessThanOrEqual": [{ "byteLength": "title" }, 10] }
            ]
        }
    }))
    .expect("parses");
    assert_eq!(rules["rule"].repeated_condition(), None);
}

#[test]
fn should_refuse_a_malformed_size_operand() {
    for (operand, needle) in [
        (
            platform_value!({ "length": 5 }),
            "at lessThan[0].length must name a property path",
        ),
        (
            platform_value!({ "byteLength": ["title"] }),
            "at lessThan[0].byteLength must name a property path",
        ),
        (
            platform_value!({ "count": { "add": ["a", 1] } }),
            "at lessThan[0].count must name a property path",
        ),
        (
            platform_value!({ "size": "title" }),
            "names \"size\", which is not one of add, subtract, multiply, divide, modulo, \
             power, min, max, abs, ifAbsent, length, byteLength, count, countOf or sumOf",
        ),
    ] {
        expect_refusal(
            platform_value!({ "rule": { "lessThan": [operand, 10] } }),
            needle,
        );
    }
}

/// `length` counts characters, as `maxLength` does, and `byteLength` UTF-8
/// bytes, as `maxBytes` does.
#[test]
fn should_measure_a_string_in_characters_and_in_bytes() {
    for (text, characters, bytes) in [
        ("", 0, 0),
        ("hello", 5, 5),
        ("héllo", 5, 6),
        ("日本", 2, 6),
        ("👍🏽", 2, 8),
    ] {
        let values = data(&[("title", Value::Text(text.to_string()))]);
        assert_eq!(
            evaluate(platform_value!({ "length": "title" }), &values),
            Ok(characters),
            "{text:?}"
        );
        assert_eq!(
            evaluate(platform_value!({ "byteLength": "title" }), &values),
            Ok(bytes),
            "{text:?}"
        );
    }
}

/// `count` counts the items of an array, and the bytes of a byte array in every
/// form a document gives one in.
#[test]
fn should_count_the_items_of_an_array_and_the_bytes_of_a_byte_array() {
    for (value, items) in [
        (Value::Array(vec![]), 0),
        (
            Value::Array(vec![
                Value::Text("a".to_string()),
                Value::Text("b".to_string()),
                Value::Text("c".to_string()),
            ]),
            3,
        ),
        (Value::Bytes(vec![7; 10]), 10),
        (Value::Bytes20([7; 20]), 20),
        (Value::Bytes32([7; 32]), 32),
        (Value::Identifier([7; 32]), 32),
        (Value::Bytes36([7; 36]), 36),
    ] {
        let values = data(&[("tags", value.clone())]);
        assert_eq!(
            evaluate(platform_value!({ "count": "tags" }), &values),
            Ok(items),
            "{value:?}"
        );
    }
}

/// A size never faults: a property left out or set to null has size 0, and so
/// does a value of another type, which the schema validation reported first
/// refuses.
#[test]
fn should_take_a_size_of_zero_for_a_property_left_out_or_of_another_type() {
    let values = data(&[
        ("empty", Value::Null),
        ("number", Value::U64(12345)),
        ("title", Value::Text("hello".to_string())),
        ("tags", Value::Array(vec![Value::U8(1), Value::U8(2)])),
    ]);
    for (expression, expected) in [
        (platform_value!({ "length": "missing" }), 0),
        (platform_value!({ "byteLength": "empty" }), 0),
        (platform_value!({ "count": "meta.missing" }), 0),
        (platform_value!({ "length": "number" }), 0),
        (platform_value!({ "length": "tags" }), 0),
        (platform_value!({ "count": "title" }), 0),
        (platform_value!({ "count": "number" }), 0),
    ] {
        assert_eq!(
            evaluate(expression.clone(), &values),
            Ok(expected),
            "{expression:?}"
        );
    }

    // A rule over a size left out holds or not as 0 says
    let rule = parse_rule_value(platform_value!({
        "greaterThanOrEqual": [{ "count": "tags" }, 1]
    }));
    assert_eq!(
        rule.violation(&data(&[]), &DocumentSystemValues::default()),
        Some(PropertyConstraintViolation::NotMet)
    );
}

/// A size compares with other properties: here a list holds at most as many
/// tags as its `maxTags`, and a free listing's title is short.
#[test]
fn should_compare_a_size_with_other_properties() {
    let tags_within_limit = parse_rule_value(platform_value!({
        "lessThanOrEqual": [{ "count": "tags" }, "maxTags"]
    }));
    let tags = |count: usize| Value::Array(vec![Value::Text("tag".to_string()); count]);
    assert_eq!(
        tags_within_limit.violation(
            &data(&[("tags", tags(2)), ("maxTags", Value::U8(3))]),
            &DocumentSystemValues::default()
        ),
        None
    );
    assert_eq!(
        tags_within_limit.violation(
            &data(&[("tags", tags(4)), ("maxTags", Value::U8(3))]),
            &DocumentSystemValues::default()
        ),
        Some(PropertyConstraintViolation::NotMet)
    );

    let short_title_when_free = parse_rule_value(platform_value!({
        "anyOf": [
            { "greaterThan": ["fee", 0] },
            { "lessThanOrEqual": [{ "length": "title" }, 5] }
        ]
    }));
    let listing =
        |fee: u64, title: &str| data(&[("fee", Value::U64(fee)), ("title", Value::from(title))]);
    assert_eq!(
        short_title_when_free.violation(&listing(0, "héllo"), &DocumentSystemValues::default()),
        None
    );
    assert_eq!(
        short_title_when_free.violation(
            &listing(10, "a long title"),
            &DocumentSystemValues::default()
        ),
        None
    );
    assert_eq!(
        short_title_when_free.violation(
            &listing(0, "a long title"),
            &DocumentSystemValues::default()
        ),
        Some(PropertyConstraintViolation::NotMet)
    );
}

// ── system times and heights ────────────────────────────────────────────

/// Each system time and height is an integer operand named as `required` names
/// it, one node, read from the system values rather than the properties.
#[test]
fn should_parse_the_system_times_and_heights_as_operands() {
    for system in SystemProperty::ALL {
        assert_eq!(SystemProperty::from_name(system.name()), Some(system));
        let rule = parse_rule_value(platform_value!({
            "lessThan": [system.name(), "deadline"]
        }));
        assert_eq!(
            rule,
            PropertyConstraint::Compare {
                comparison: ConstraintComparison::LessThan,
                left: ConstraintExpression::System(system),
                right: property("deadline"),
            },
            "{}",
            system.name()
        );
        assert_eq!(rule.node_count(), 3);
        assert_eq!(rule.property_reads(), [("deadline", PropertyRead::Value)]);
        assert_eq!(rule.system_reads(), [system]);
        assert!(!rule.reads_owner());
    }
    for name in ["$ownerId", "$id", "$revision", "$createdat", "createdAt"] {
        assert_eq!(SystemProperty::from_name(name), None, "{name}");
    }

    // A system value alone keeps a comparison with a literal meaningful: it
    // differs from document to document
    let rule = parse_rule_value(platform_value!({
        "greaterThan": ["$createdAtBlockHeight", 1000]
    }));
    assert_eq!(rule.system_reads(), [SystemProperty::CreatedAtBlockHeight]);
}

#[test]
fn should_refuse_a_default_for_a_system_property() {
    expect_refusal(
        platform_value!({
            "rule": { "lessThan": [{ "ifAbsent": ["$createdAt", 0] }, "deadline"] }
        }),
        "at lessThan[0].ifAbsent gives $createdAt a default, but a system property a rule \
         reads is always set: name it on its own",
    );
}

/// A rule reads the system values it is judged with: here a listing ends
/// within a week of its creation.
#[test]
fn should_read_the_system_values_the_rule_is_judged_with() {
    let rule = parse_rule_value(platform_value!({
        "lessThanOrEqual": [{ "subtract": ["endsAt", "$createdAt"] }, 604800000]
    }));
    let created_at = |time: u64| DocumentSystemValues {
        created_at: Some(time),
        ..Default::default()
    };
    let listing = |ends_at: u64| data(&[("endsAt", Value::U64(ends_at))]);
    let day = 86_400_000u64;
    assert_eq!(
        rule.violation(&listing(10 * day), &created_at(4 * day)),
        None
    );
    assert_eq!(
        rule.violation(&listing(12 * day), &created_at(4 * day)),
        Some(PropertyConstraintViolation::NotMet)
    );

    // Every system value reads its own field
    let values = DocumentSystemValues {
        owner_id: None,
        created_at: Some(1),
        updated_at: Some(2),
        transferred_at: Some(3),
        created_at_block_height: Some(4),
        updated_at_block_height: Some(5),
        transferred_at_block_height: Some(6),
        created_at_core_block_height: Some(7),
        updated_at_core_block_height: Some(8),
        transferred_at_core_block_height: Some(9),
        aggregates: None,
    };
    for (index, property) in SystemProperty::ALL.into_iter().enumerate() {
        let expected = i128::try_from(index + 1).expect("small");
        assert_eq!(
            values.value(property),
            Some(expected),
            "{}",
            property.name()
        );
        let rule = parse_rule_value(platform_value!({ "equal": [property.name(), "expected"] }));
        let expected_data = data(&[("expected", Value::I128(expected))]);
        assert_eq!(
            rule.violation(&expected_data, &values),
            None,
            "{}",
            property.name()
        );
    }
}

/// Consensus gives every system value a type records, the only ones a rule may
/// read; a client that does not know one skips the rule rather than guess.
#[test]
fn should_not_judge_a_rule_reading_a_system_value_not_given() {
    let rule = parse_rule_value(platform_value!({
        "anyOf": [
            { "absent": "endsAt" },
            { "greaterThan": ["endsAt", "$updatedAtBlockHeight"] }
        ]
    }));
    let early = data(&[("endsAt", Value::U64(5))]);
    assert_eq!(
        rule.violation(&early, &DocumentSystemValues::default()),
        None
    );
    // Given, it is judged
    let at_height = |height: u64| DocumentSystemValues {
        updated_at_block_height: Some(height),
        ..Default::default()
    };
    assert_eq!(rule.violation(&early, &at_height(3)), None);
    assert_eq!(
        rule.violation(&early, &at_height(9)),
        Some(PropertyConstraintViolation::NotMet)
    );
}

/// A transfer or a purchase can break the rules reading the owner or the
/// transfer's time and heights, a price update those reading the update's.
#[test]
fn should_tell_which_writes_a_rule_answers_to() {
    let banned = Identifier::new([9; 32]).to_string(Encoding::Base58);
    let also_banned = Identifier::new([8; 32]).to_string(Encoding::Base58);
    for (rule, transfer, price_update) in [
        // notIn, ifThen and ifThenElse answer to what their conditions read
        (
            platform_value!({ "notIn": ["$ownerId", [banned.clone(), also_banned.clone()]] }),
            true,
            false,
        ),
        (
            platform_value!({ "notIn": ["$updatedAtBlockHeight", [1, 2]] }),
            false,
            true,
        ),
        (
            platform_value!({
                "ifThen": [{ "present": "endsAt" }, { "lessThan": ["$transferredAt", "endsAt"] }]
            }),
            true,
            false,
        ),
        (
            platform_value!({
                "ifThen": [{ "lessThan": ["$updatedAt", 5] }, { "present": "endsAt" }]
            }),
            false,
            true,
        ),
        // The else branch counts though it is taken only when the condition fails
        (
            platform_value!({
                "ifThenElse": [
                    { "absent": "endsAt" },
                    { "present": "note" },
                    { "lessThan": ["$transferredAt", "endsAt"] }
                ]
            }),
            true,
            false,
        ),
        (
            platform_value!({ "lessThan": ["$transferredAt", "endsAt"] }),
            true,
            false,
        ),
        (
            platform_value!({ "not": { "in": ["$transferredAtCoreBlockHeight", [1, 2]] } }),
            true,
            false,
        ),
        (
            platform_value!({ "lessThan": ["$updatedAt", "endsAt"] }),
            false,
            true,
        ),
        (
            platform_value!({
                "anyOf": [
                    { "absent": "endsAt" },
                    { "lessThan": [{ "add": ["$updatedAtBlockHeight", 1] }, "endsAt"] }
                ]
            }),
            false,
            true,
        ),
        (
            platform_value!({ "lessThan": ["$createdAt", "endsAt"] }),
            false,
            false,
        ),
        (
            platform_value!({ "equal": ["sellerId", "$ownerId"] }),
            true,
            false,
        ),
        (
            platform_value!({ "lessThan": ["price", "endsAt"] }),
            false,
            false,
        ),
    ] {
        let parsed = parse_rule_value(rule.clone());
        assert_eq!(
            parsed.reads_change(SystemChange::Transfer),
            transfer,
            "transfer, {rule:?}"
        );
        assert_eq!(
            parsed.reads_change(SystemChange::PriceUpdate),
            price_update,
            "price update, {rule:?}"
        );
    }
}

/// A create records every time and height at its block; a stored document's
/// values are read back from it.
#[test]
fn should_take_the_system_values_of_a_create_and_of_a_document() {
    let owner = Identifier::new([3; 32]);
    let block = BlockInfo {
        time_ms: 1_700_000_000_000,
        height: 42,
        core_height: 2_100_000,
        ..Default::default()
    };
    let created = DocumentSystemValues::created_in_block(owner, &block);
    assert_eq!(created.owner_id, Some(owner));
    for property in SystemProperty::ALL {
        let expected = match property {
            SystemProperty::CreatedAt
            | SystemProperty::UpdatedAt
            | SystemProperty::TransferredAt => 1_700_000_000_000,
            SystemProperty::CreatedAtBlockHeight
            | SystemProperty::UpdatedAtBlockHeight
            | SystemProperty::TransferredAtBlockHeight => 42,
            _ => 2_100_000,
        };
        assert_eq!(
            created.value(property),
            Some(expected),
            "{}",
            property.name()
        );
    }

    let document: Document = crate::document::DocumentV0 {
        owner_id: owner,
        created_at: Some(10),
        updated_at: Some(20),
        transferred_at: None,
        created_at_block_height: Some(1),
        updated_at_core_block_height: Some(7),
        ..Default::default()
    }
    .into();
    let stored = DocumentSystemValues::of_document(&document);
    assert_eq!(
        stored,
        DocumentSystemValues {
            owner_id: Some(owner),
            created_at: Some(10),
            updated_at: Some(20),
            created_at_block_height: Some(1),
            updated_at_core_block_height: Some(7),
            ..Default::default()
        }
    );
}

// ── contains ────────────────────────────────────────────────────────────

/// What a `contains` looks for is read as the array's elements are: a const
/// and a bare path among strings or identifiers, an integer expression
/// otherwise.
#[test]
fn should_parse_contains_by_the_kind_of_the_array() {
    let member = Identifier::new([4; 32]);
    for (rule, needle, reads, nodes) in [
        (
            platform_value!({ "contains": ["labels", { "const": "sale" }] }),
            ContainsNeedle::TextConstant("sale".to_string()),
            vec![("labels", PropertyRead::Elements(ElementKind::Text))],
            3,
        ),
        (
            platform_value!({ "contains": ["labels", "status"] }),
            ContainsNeedle::TextProperty(TextProperty {
                path: "status".to_string(),
                if_absent: None,
            }),
            vec![
                ("labels", PropertyRead::Elements(ElementKind::Text)),
                ("status", PropertyRead::Text),
            ],
            3,
        ),
        (
            platform_value!({ "contains": ["labels", { "ifAbsent": ["status", "sale"] }] }),
            ContainsNeedle::TextProperty(TextProperty {
                path: "status".to_string(),
                if_absent: Some("sale".to_string()),
            }),
            vec![
                ("labels", PropertyRead::Elements(ElementKind::Text)),
                ("status", PropertyRead::Text),
            ],
            3,
        ),
        (
            platform_value!({
                "contains": ["members", { "const": member.to_string(Encoding::Base58) }]
            }),
            ContainsNeedle::IdentifierConstant(member),
            vec![("members", PropertyRead::Elements(ElementKind::Identifier))],
            3,
        ),
        (
            platform_value!({ "contains": ["members", "buyerId"] }),
            ContainsNeedle::IdentifierProperty("buyerId".to_string()),
            vec![
                ("members", PropertyRead::Elements(ElementKind::Identifier)),
                ("buyerId", PropertyRead::Identifier),
            ],
            3,
        ),
        (
            platform_value!({ "contains": ["members", "$ownerId"] }),
            ContainsNeedle::IdentifierProperty("$ownerId".to_string()),
            vec![("members", PropertyRead::Elements(ElementKind::Identifier))],
            3,
        ),
        (
            platform_value!({ "contains": ["scores", { "add": ["bonus", 1] }] }),
            ContainsNeedle::Integer(ConstraintExpression::Add(vec![
                property("bonus"),
                ConstraintExpression::Value(1),
            ])),
            vec![
                ("scores", PropertyRead::Elements(ElementKind::Integer)),
                ("bonus", PropertyRead::Value),
            ],
            5,
        ),
    ] {
        let parsed = parse_rule_value(rule.clone());
        let PropertyConstraint::Contains {
            array,
            needle: parsed_needle,
        } = &parsed
        else {
            panic!("{rule:?}: expected a contains, got {parsed:?}");
        };
        assert_eq!(array, reads[0].0, "{rule:?}");
        assert_eq!(parsed_needle, &needle, "{rule:?}");
        assert_eq!(parsed.property_reads(), reads, "{rule:?}");
        assert_eq!(parsed.node_count(), nodes, "{rule:?}");
    }

    // The owner read makes a transfer answer to it; a const is checked against
    // the elements' enum; a default against the property's
    let owner_rule = parse_rule_value(platform_value!({ "contains": ["members", "$ownerId"] }));
    assert!(owner_rule.reads_owner());
    assert!(owner_rule.reads_change(SystemChange::Transfer));
    let sale = parse_rule_value(platform_value!({ "contains": ["labels", { "const": "sale" }] }));
    assert_eq!(sale.text_constants(), [("labels", "sale")]);
    let defaulted = parse_rule_value(platform_value!({
        "contains": ["labels", { "ifAbsent": ["status", "sale"] }]
    }));
    assert_eq!(defaulted.text_defaults(), [("status", "sale")]);
    // A system value looked for among integers is read like any operand
    let created = parse_rule_value(platform_value!({ "contains": ["scores", "$createdAt"] }));
    assert_eq!(created.system_reads(), [SystemProperty::CreatedAt]);
}

#[test]
fn should_refuse_a_malformed_contains() {
    for (rule, needle) in [
        (
            platform_value!({ "contains": ["labels"] }),
            "at contains must list an array property path and the value looked for among its \
             elements",
        ),
        (
            platform_value!({ "contains": [5, 1] }),
            "at contains[0] must name an array property path",
        ),
        (
            platform_value!({ "contains": ["$ownerId", 1] }),
            "at contains[0] must name an array property path",
        ),
        (
            platform_value!({ "contains": ["scores", { "const": "10" }] }),
            "at contains[1] is a const, but scores holds no strings or identifiers",
        ),
        (
            platform_value!({ "contains": ["members", { "const": "not base58" }] }),
            "which is not a base58 identifier of 32 bytes",
        ),
        (
            platform_value!({ "contains": ["labels", 5] }),
            "at contains[1] must be the path of a string property",
        ),
        (
            platform_value!({ "contains": ["scores", { "divide": ["bonus", 0] }] }),
            "divides by 0",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": rule }), needle);
    }
}

/// A `contains` holds when an element equals what it looks for, whatever form
/// the document gives an identifier in; an array, a string or an identifier
/// the document leaves out holds or matches nothing; a fault in the integer it
/// looks for breaks the rule.
#[test]
fn should_look_for_a_value_among_the_elements() {
    let text =
        |values: &[&str]| Value::Array(values.iter().map(|value| Value::from(*value)).collect());
    let none = DocumentSystemValues::default();

    let sale = parse_rule_value(platform_value!({ "contains": ["labels", { "const": "sale" }] }));
    assert_eq!(
        sale.holds(&data(&[("labels", text(&["new", "sale"]))]), &none),
        Ok(true)
    );
    assert_eq!(
        sale.holds(&data(&[("labels", text(&["new"]))]), &none),
        Ok(false)
    );
    assert_eq!(sale.holds(&data(&[]), &none), Ok(false));
    assert_eq!(
        sale.holds(&data(&[("labels", Value::Null)]), &none),
        Ok(false)
    );

    let own_status = parse_rule_value(platform_value!({
        "contains": ["labels", { "ifAbsent": ["status", "sale"] }]
    }));
    let listing = |status: Option<&str>| {
        let mut entries = vec![("labels", text(&["new", "sale"]))];
        if let Some(status) = status {
            entries.push(("status", Value::from(status)));
        }
        data(&entries)
    };
    assert_eq!(own_status.holds(&listing(Some("new")), &none), Ok(true));
    assert_eq!(own_status.holds(&listing(Some("used")), &none), Ok(false));
    // Left out, the status takes its default
    assert_eq!(own_status.holds(&listing(None), &none), Ok(true));

    let [a, b, c] = [[1u8; 32], [2; 32], [3; 32]];
    let members = Value::Array(vec![Value::Identifier(a), Value::Bytes32(b)]);
    let owner_is_member =
        parse_rule_value(platform_value!({ "contains": ["members", "$ownerId"] }));
    let group = data(&[("members", members.clone())]);
    for (owner, expected) in [
        (Some(Identifier::new(a)), true),
        (Some(Identifier::new(b)), true),
        (Some(Identifier::new(c)), false),
        (None, false),
    ] {
        let system = DocumentSystemValues {
            owner_id: owner,
            ..Default::default()
        };
        assert_eq!(
            owner_is_member.holds(&group, &system),
            Ok(expected),
            "{owner:?}"
        );
    }
    let buyer_is_member = parse_rule_value(platform_value!({ "contains": ["members", "buyerId"] }));
    assert_eq!(
        buyer_is_member.holds(
            &data(&[
                ("members", members.clone()),
                ("buyerId", Value::Identifier(b))
            ]),
            &none
        ),
        Ok(true)
    );
    // A buyer left out is a member of no group
    assert_eq!(buyer_is_member.holds(&group, &none), Ok(false));

    let next_score = parse_rule_value(platform_value!({
        "contains": ["scores", { "add": ["bonus", 1] }]
    }));
    let scores = |bonus: u64| {
        data(&[
            ("scores", Value::Array(vec![Value::U8(3), Value::U64(10)])),
            ("bonus", Value::U64(bonus)),
        ])
    };
    assert_eq!(next_score.holds(&scores(9), &none), Ok(true));
    assert_eq!(next_score.holds(&scores(1), &none), Ok(false));
    let per_unit = parse_rule_value(platform_value!({
        "contains": ["scores", { "divide": [100, "bonus"] }]
    }));
    assert_eq!(
        per_unit.violation(&data(&[("bonus", Value::U64(0))]), &none),
        Some(PropertyConstraintViolation::DivisionByZero)
    );
}

// ── startsWith and endsWith ─────────────────────────────────────────────

/// Each side is a const or a string property, with or without a default;
/// a string constant looked for in a property is listed for the enum check.
#[test]
fn should_parse_starts_with_and_ends_with() {
    for (key, position) in [
        ("startsWith", AffixPosition::Start),
        ("endsWith", AffixPosition::End),
    ] {
        assert_eq!(position.wire_name(), key);
        let rule = parse_rule_value(platform_value!({ key: ["status", { "const": "op" }] }));
        assert_eq!(
            rule,
            PropertyConstraint::TextAffix {
                position,
                text: TextOperand::Property(TextProperty {
                    path: "status".to_string(),
                    if_absent: None,
                }),
                affix: TextOperand::Constant("op".to_string()),
            },
            "{key}"
        );
        assert_eq!(rule.node_count(), 3);
        assert_eq!(rule.property_reads(), [("status", PropertyRead::Text)]);
        assert_eq!(rule.text_affixes(), [("status", "op", position)]);
        // A prefix or a suffix is not a whole value: no equality enum check
        assert!(rule.text_constants().is_empty());

        let both = parse_rule_value(platform_value!({
            key: [{ "ifAbsent": ["to", "x"] }, "from"]
        }));
        assert_eq!(
            both.property_reads(),
            [("to", PropertyRead::Text), ("from", PropertyRead::Text)]
        );
        assert_eq!(both.text_defaults(), [("to", "x")]);
        assert!(both.text_affixes().is_empty());

        // A constant tested for a property's affix is no enum typo to check
        let constant_text = parse_rule_value(platform_value!({
            key: [{ "const": "https://example.org" }, "status"]
        }));
        assert!(constant_text.text_affixes().is_empty());
    }
}

#[test]
fn should_refuse_a_malformed_starts_with() {
    for (rule, needle) in [
        (
            platform_value!({ "startsWith": ["status"] }),
            "at startsWith must list two strings: the one tested, then the one it must start with",
        ),
        (
            platform_value!({ "endsWith": ["status", "from", "to"] }),
            "at endsWith must list two strings: the one tested, then the one it must end with",
        ),
        (
            platform_value!({ "startsWith": [{ "const": "a" }, { "const": "b" }] }),
            "rule \"rule\" reads no property",
        ),
        (
            platform_value!({ "endsWith": ["status", "status"] }),
            "at endsWith tests \"status\" against itself",
        ),
        (
            platform_value!({ "startsWith": ["status", 5] }),
            "at startsWith[1] must be the path of a string property",
        ),
        (
            platform_value!({ "startsWith": ["status", { "const": 5 }] }),
            "at startsWith[1].const must be a string",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": rule }), needle);
    }
}

/// Byte for byte, with no case folding: a string starts and ends with the
/// empty one and with itself; a property left out without a default takes no
/// string, and the condition does not hold for it.
#[test]
fn should_test_whether_a_string_starts_or_ends_with_another() {
    let none = DocumentSystemValues::default();
    let https =
        parse_rule_value(platform_value!({ "startsWith": ["status", { "const": "https://" }] }));
    let domain =
        parse_rule_value(platform_value!({ "endsWith": ["status", { "const": ".dash" }] }));
    for (status, starts, ends) in [
        (Some("https://pay.dash"), true, true),
        (Some("HTTPS://pay.dash"), false, true),
        (Some("http://pay.dash/"), false, false),
        (Some("https://"), true, false),
        (Some(""), false, false),
        (None, false, false),
    ] {
        let values = match status {
            Some(status) => data(&[("status", Value::from(status))]),
            None => data(&[]),
        };
        assert_eq!(https.holds(&values, &none), Ok(starts), "{status:?}");
        assert_eq!(domain.holds(&values, &none), Ok(ends), "{status:?}");
    }

    // Multibyte text compares byte for byte, which for valid strings is
    // character for character
    let accented =
        parse_rule_value(platform_value!({ "startsWith": ["status", { "const": "é" }] }));
    assert_eq!(
        accented.holds(&data(&[("status", Value::from("été"))]), &none),
        Ok(true)
    );
    assert_eq!(
        accented.holds(&data(&[("status", Value::from("e"))]), &none),
        Ok(false)
    );

    // Two properties: a reply's path starts with its thread's
    let nested = parse_rule_value(platform_value!({ "startsWith": ["to", "from"] }));
    let paths = |to: &str, from: Option<&str>| {
        let mut entries = vec![("to", Value::from(to))];
        if let Some(from) = from {
            entries.push(("from", Value::from(from)));
        }
        data(&entries)
    };
    assert_eq!(nested.holds(&paths("a/b/c", Some("a/b")), &none), Ok(true));
    assert_eq!(nested.holds(&paths("a/c", Some("a/b")), &none), Ok(false));
    assert_eq!(nested.holds(&paths("a/b", None), &none), Ok(false));
    // A default fills a property left out
    let defaulted = parse_rule_value(platform_value!({
        "startsWith": ["to", { "ifAbsent": ["from", ""] }]
    }));
    assert_eq!(defaulted.holds(&paths("a/b", None), &none), Ok(true));
    // `not` refuses a prefix
    let not_draft = parse_rule_value(platform_value!({
        "not": { "startsWith": ["status", { "const": "draft:" }] }
    }));
    assert_eq!(
        not_draft.violation(&data(&[("status", Value::from("draft:1"))]), &none),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert_eq!(
        not_draft.violation(&data(&[("status", Value::from("final"))]), &none),
        None
    );
}

// ── min, max, abs, ifThen, ifThenElse and notIn ──────────────────────────

/// `min` and `max` take two or more operands and evaluate every one; `abs`
/// takes one. Each is one node plus its operands.
#[test]
fn should_evaluate_min_max_and_abs() {
    let values = data(&[
        ("a", Value::U64(5)),
        ("b", Value::U64(2)),
        ("zero", Value::U64(0)),
    ]);
    for (expression, expected) in [
        (platform_value!({ "min": ["a", "b", 3] }), 2),
        (platform_value!({ "max": ["a", "b", 3] }), 5),
        (
            platform_value!({ "max": [{ "subtract": ["b", "a"] }, -10] }),
            -3,
        ),
        (platform_value!({ "abs": { "subtract": ["b", "a"] } }), 3),
        (platform_value!({ "abs": "a" }), 5),
        (platform_value!({ "min": ["missing", "a"] }), 0),
    ] {
        assert_eq!(
            evaluate(expression.clone(), &values),
            Ok(expected),
            "{expression:?}"
        );
    }

    // Every operand is evaluated: a later, smaller one does not hide a fault
    assert_eq!(
        evaluate(
            platform_value!({ "min": [{ "divide": ["a", "zero"] }, -1] }),
            &values
        ),
        Err(PropertyConstraintViolation::DivisionByZero)
    );
    // The absolute value of the least i128 does not fit
    assert_eq!(
        evaluate(
            platform_value!({ "abs": { "subtract": [Value::I128(i128::MIN + 1), 1] } }),
            &values
        ),
        Err(PropertyConstraintViolation::Overflow)
    );

    let rule = parse_rule_value(platform_value!({
        "lessThanOrEqual": [{ "abs": { "subtract": ["a", "b"] } }, { "max": ["a", "b", 3] }]
    }));
    assert_eq!(rule.node_count(), 9);
    assert_eq!(rule.property_paths(), ["a", "b", "a", "b"]);

    for (rule, needle) in [
        (
            platform_value!({ "equal": [{ "min": ["a"] }, 1] }),
            "at equal[0].min must list two or more operands",
        ),
        (
            platform_value!({ "equal": [{ "max": "a" }, 1] }),
            "at equal[0].max must list two or more operands",
        ),
        (
            platform_value!({ "equal": [{ "abs": ["a"] }, 1] }),
            "at equal[0].abs must be one operand, not a list: abs takes a single operand",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": rule }), needle);
    }
}

/// `ifThen` holds when its second condition holds whenever its first does;
/// the second is evaluated only when the first holds, and a fault in either
/// breaks the rule.
#[test]
fn should_hold_the_then_branch_of_an_if_then_only_when_its_condition_holds() {
    let none = DocumentSystemValues::default();
    let rule = parse_rule_value(platform_value!({
        "ifThen": [
            { "greaterThan": ["discount", 0] },
            { "greaterThanOrEqual": [{ "divide": ["price", "discount"] }, 10] }
        ]
    }));
    assert_eq!(rule.node_count(), 1 + 3 + 5);
    assert_eq!(rule.property_paths(), ["discount", "price", "discount"]);
    let offer = |price: u64, discount: u64| {
        data(&[
            ("price", Value::U64(price)),
            ("discount", Value::U64(discount)),
        ])
    };
    // No discount: the then branch, which would divide by zero, is not evaluated
    assert_eq!(rule.violation(&offer(100, 0), &none), None);
    assert_eq!(rule.violation(&offer(100, 10), &none), None);
    assert_eq!(
        rule.violation(&offer(100, 20), &none),
        Some(PropertyConstraintViolation::NotMet)
    );
    // A fault in the condition breaks the rule
    let faulty = parse_rule_value(platform_value!({
        "ifThen": [
            { "greaterThan": [{ "divide": ["price", "discount"] }, 0] },
            { "present": "note" }
        ]
    }));
    assert_eq!(
        faulty.violation(&offer(100, 0), &none),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    // An owner read in either condition makes a transfer answer to it
    let owned = parse_rule_value(platform_value!({
        "ifThen": [{ "present": "sellerId" }, { "equal": ["sellerId", "$ownerId"] }]
    }));
    assert!(owned.reads_owner());

    for (rule, needle) in [
        (
            platform_value!({ "ifThen": [{ "present": "a" }] }),
            "at ifThen must list two conditions: the condition, then the one that must hold \
             when it does",
        ),
        (
            platform_value!({ "ifThen": [{ "present": "a" }, { "present": "b" }, { "present": "c" }] }),
            "at ifThen must list two conditions",
        ),
        (
            platform_value!({ "ifThen": { "present": "a" } }),
            "at ifThen must list two conditions",
        ),
        (
            platform_value!({ "ifThen": [{ "present": "a" }, { "exists": "b" }] }),
            "at ifThen[1] names \"exists\"",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": rule }), needle);
    }
}

/// `ifThenElse` holds its second condition when its first holds and its third
/// when it does not, evaluating only the branch taken; a fault in the condition
/// or in the branch taken breaks the rule.
#[test]
fn should_hold_the_branch_an_if_then_else_selects() {
    let none = DocumentSystemValues::default();
    // A discount needs at least ten times its value in price; without one the
    // price is at most 1000
    let rule = parse_rule_value(platform_value!({
        "ifThenElse": [
            { "greaterThan": ["discount", 0] },
            { "greaterThanOrEqual": [{ "divide": ["price", "discount"] }, 10] },
            { "lessThanOrEqual": ["price", 1000] }
        ]
    }));
    assert_eq!(rule.node_count(), 1 + 3 + 5 + 3);
    assert_eq!(
        rule.property_paths(),
        ["discount", "price", "discount", "price"]
    );
    let offer = |price: u64, discount: u64| {
        data(&[
            ("price", Value::U64(price)),
            ("discount", Value::U64(discount)),
        ])
    };
    // The then branch
    assert_eq!(rule.violation(&offer(100, 10), &none), None);
    assert_eq!(
        rule.violation(&offer(100, 20), &none),
        Some(PropertyConstraintViolation::NotMet)
    );
    // The else branch, taken with no discount, so the then branch's division by
    // zero is never evaluated
    assert_eq!(rule.violation(&offer(1000, 0), &none), None);
    assert_eq!(
        rule.violation(&offer(1001, 0), &none),
        Some(PropertyConstraintViolation::NotMet)
    );

    // A fault in the branch taken breaks the rule, one in the branch not taken
    // does not
    let faulty_else = parse_rule_value(platform_value!({
        "ifThenElse": [
            { "greaterThan": ["discount", 0] },
            { "present": "price" },
            { "greaterThan": [{ "divide": ["price", "discount"] }, 0] }
        ]
    }));
    assert_eq!(faulty_else.violation(&offer(100, 10), &none), None);
    assert_eq!(
        faulty_else.violation(&offer(100, 0), &none),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    // An owner read in the else branch alone makes a transfer answer to it
    let owned = parse_rule_value(platform_value!({
        "ifThenElse": [
            { "absent": "sellerId" },
            { "present": "note" },
            { "equal": ["sellerId", "$ownerId"] }
        ]
    }));
    assert!(owned.reads_owner());

    for (rule, needle) in [
        (
            platform_value!({ "ifThenElse": [{ "present": "a" }, { "present": "b" }] }),
            "at ifThenElse must list three conditions: the condition, the one that must hold \
             when it does, and the one that must hold when it does not",
        ),
        (
            platform_value!({
                "ifThenElse": [
                    { "present": "a" },
                    { "present": "b" },
                    { "present": "c" },
                    { "present": "d" }
                ]
            }),
            "at ifThenElse must list three conditions",
        ),
        (
            platform_value!({
                "ifThenElse": [{ "present": "a" }, { "present": "b" }, { "exists": "c" }]
            }),
            "at ifThenElse[2] names \"exists\"",
        ),
    ] {
        expect_refusal(platform_value!({ "rule": rule }), needle);
    }
}

/// An `ifThen` or `ifThenElse` holding two alike conditions says what a
/// simpler rule says, and is reported as a repeat, like an `anyOf` listing a
/// condition twice.
#[test]
fn should_report_an_if_then_holding_two_alike_conditions() {
    let rules = parse(platform_value!({
        "same": { "ifThen": [{ "present": "a" }, { "present": "a" }] },
        "nested": {
            "anyOf": [
                { "equal": ["a", 1] },
                { "ifThen": [{ "equal": ["b", 1] }, { "equal": ["b", 1.0] }] }
            ]
        },
        "fine": { "ifThen": [{ "present": "a" }, { "present": "b" }] },
        "sameBranches": {
            "ifThenElse": [{ "present": "a" }, { "present": "b" }, { "present": "b" }]
        },
        "elseIsCondition": {
            "ifThenElse": [{ "present": "a" }, { "present": "b" }, { "present": "a" }]
        },
        "fineElse": {
            "ifThenElse": [{ "present": "a" }, { "present": "b" }, { "present": "c" }]
        }
    }))
    .expect("parses");
    for (name, found) in [
        ("same", Some(("ifThen[1]", "ifThen[0]"))),
        ("nested", Some(("anyOf[1].ifThen[1]", "anyOf[1].ifThen[0]"))),
        ("fine", None),
        ("sameBranches", Some(("ifThenElse[2]", "ifThenElse[1]"))),
        ("elseIsCondition", Some(("ifThenElse[2]", "ifThenElse[0]"))),
        ("fineElse", None),
    ] {
        assert_eq!(
            rules[name].repeated_condition(),
            found.map(|(repeat, earlier)| (repeat.to_string(), earlier.to_string())),
            "{name}"
        );
    }
}

/// `notIn` takes what `in` takes, integers, strings or identifiers, holds when
/// the operand takes none of the values, and costs what the `in` costs.
#[test]
fn should_negate_an_in_with_not_in() {
    let none = DocumentSystemValues::default();
    let seller = Identifier::new([5; 32]);
    for (rule, in_rule) in [
        (
            platform_value!({ "notIn": ["fee", [13, 666]] }),
            platform_value!({ "in": ["fee", [13, 666]] }),
        ),
        (
            platform_value!({ "notIn": ["status", ["banned", "hidden"]] }),
            platform_value!({ "in": ["status", ["banned", "hidden"]] }),
        ),
        (
            platform_value!({
                "notIn": ["buyerId", [seller.to_string(Encoding::Base58), Identifier::new([6; 32]).to_string(Encoding::Base58)]]
            }),
            platform_value!({
                "in": ["buyerId", [seller.to_string(Encoding::Base58), Identifier::new([6; 32]).to_string(Encoding::Base58)]]
            }),
        ),
    ] {
        let negated = parse_rule_value(rule.clone());
        let listed = parse_rule_value(in_rule);
        assert_eq!(
            negated,
            PropertyConstraint::NotIn(Box::new(listed.clone())),
            "{rule:?}"
        );
        assert_eq!(negated.node_count(), listed.node_count(), "{rule:?}");
        assert_eq!(
            negated.property_reads(),
            listed.property_reads(),
            "{rule:?}"
        );
    }

    let fee = parse_rule_value(platform_value!({ "notIn": ["fee", [13, 666]] }));
    assert_eq!(
        fee.violation(&data(&[("fee", Value::U64(10))]), &none),
        None
    );
    assert_eq!(
        fee.violation(&data(&[("fee", Value::U64(13))]), &none),
        Some(PropertyConstraintViolation::NotMet)
    );
    // A string left out takes none of the values
    let status = parse_rule_value(platform_value!({ "notIn": ["status", ["banned", "hidden"]] }));
    assert_eq!(status.violation(&data(&[]), &none), None);
    assert_eq!(
        status.violation(&data(&[("status", Value::from("hidden"))]), &none),
        Some(PropertyConstraintViolation::NotMet)
    );
    // Its strings face the enum check, as an in's do
    assert_eq!(
        status.text_constants(),
        [("status", "banned"), ("status", "hidden")]
    );
    // A fault in the operand still breaks the rule
    let divided = parse_rule_value(platform_value!({
        "notIn": [{ "divide": ["fee", "zero"] }, [1, 2]]
    }));
    assert_eq!(
        divided.violation(&data(&[("fee", Value::U64(4))]), &none),
        Some(PropertyConstraintViolation::DivisionByZero)
    );

    expect_refusal(
        platform_value!({ "rule": { "notIn": ["fee"] } }),
        "at notIn must list an integer expression and the values it may not take",
    );
    expect_refusal(
        platform_value!({ "rule": { "notIn": [5, ["a", "b"]] } }),
        "a notIn over strings reads a string property",
    );
    // A not over a notIn says what the in says, as a not over a not does
    expect_refusal(
        platform_value!({ "rule": { "not": { "notIn": ["fee", [13, 666]] } } }),
        "at not.notIn is a notIn directly inside a not, which says what an in of the same \
         values says: declare that in",
    );
    parse_rule_value(platform_value!({ "not": { "in": ["fee", [13, 666]] } }));
    expect_refusal(
        platform_value!({ "rule": { "notIn": ["fee", [1, 1]] } }),
        "at notIn[1]",
    );
}

/// `countOf` and `sumOf` parse to an [`AggregateRead`]: the type they total,
/// what a `sumOf` totals, and their filter, each key bound to a property of the
/// document (read as its kind compares), `$ownerId`, an integer or a constant.
/// The document's own type is marked, and a filter costs a node per key.
#[test]
fn should_parse_count_of_and_sum_of_with_their_filters() {
    let per_owner = parse_rule_value(platform_value!({
        "lessThanOrEqual": [{ "countOf": ["order", { "$ownerId": "$ownerId" }] }, 10]
    }));
    let owner_read = AggregateRead {
        kind: AggregateKind::Count,
        document_type: "order".to_string(),
        filter: BTreeMap::from([(OWNER_ID.to_string(), AggregateBinding::Owner)]),
        of_own_type: true,
    };
    assert_eq!(per_owner.aggregate_reads(), [&owner_read]);
    assert_eq!(per_owner.node_count(), 1 + 2 + 1);
    assert!(per_owner.property_reads().is_empty());
    assert!(per_owner.reads_owner());
    assert!(per_owner.reads_change(SystemChange::Transfer));
    assert!(!per_owner.reads_change(SystemChange::PriceUpdate));

    let pledged = parse_rule_value(platform_value!({
        "lessThanOrEqual": [
            {
                "sumOf": [
                    "pledge",
                    "amount",
                    { "campaignId": "sellerId", "status": { "const": "open" }, "tier": 2 }
                ]
            },
            "deposit"
        ]
    }));
    assert_eq!(
        pledged.aggregate_reads(),
        [&AggregateRead {
            kind: AggregateKind::Sum {
                property: "amount".to_string()
            },
            document_type: "pledge".to_string(),
            filter: BTreeMap::from([
                (
                    "campaignId".to_string(),
                    AggregateBinding::Property {
                        path: "sellerId".to_string(),
                        kind: Some(EqualityKind::Identifier),
                    }
                ),
                (
                    "status".to_string(),
                    AggregateBinding::Constant("open".to_string())
                ),
                ("tier".to_string(), AggregateBinding::Integer(2)),
            ]),
            of_own_type: false,
        }]
    );
    assert_eq!(pledged.node_count(), 1 + 4 + 1);
    assert_eq!(
        pledged.property_reads(),
        [
            ("sellerId", PropertyRead::Identifier),
            ("deposit", PropertyRead::Value)
        ]
    );
    assert!(!pledged.reads_owner());

    // A total over a whole type reads no property, but is no constant either
    let listed = parse_rule_value(platform_value!({
        "greaterThan": [{ "countOf": ["listing"] }, 0]
    }));
    assert_eq!(listed.node_count(), 3);
    assert_eq!(listed.aggregate_reads()[0].filter, BTreeMap::new());
    assert!(!listed.reads_owner());

    // The owner matters when a binding reads it, or when the type is the
    // writer's own and the document counts by its owner
    for (rule, reads_owner) in [
        (
            platform_value!({ "countOf": ["listing", { "sellerId": "$ownerId" }] }),
            true,
        ),
        (
            platform_value!({ "countOf": ["listing", { "$ownerId": "sellerId" }] }),
            false,
        ),
        (
            platform_value!({ "countOf": ["order", { "$ownerId": "sellerId" }] }),
            true,
        ),
        (
            platform_value!({ "countOf": ["order", { "status": "status" }] }),
            false,
        ),
    ] {
        let parsed = parse_rule_value(platform_value!({ "lessThan": [rule.clone(), 5] }));
        assert_eq!(parsed.reads_owner(), reads_owner, "{rule:?}");
    }
}

#[test]
fn should_refuse_a_malformed_aggregate() {
    for (operand, needle) in [
        (
            platform_value!({ "countOf": "listing" }),
            "at lessThan[0].countOf must list the document type to count",
        ),
        (
            platform_value!({ "countOf": [] }),
            "must list the document type to count",
        ),
        (
            platform_value!({ "countOf": ["listing", { "a": 1 }, 2] }),
            "must list the document type to count",
        ),
        (
            platform_value!({ "sumOf": ["pledge"] }),
            "at lessThan[0].sumOf must list the document type, the integer property of it to \
             total",
        ),
        (
            platform_value!({ "countOf": [7] }),
            "at lessThan[0].countOf must name a document type first",
        ),
        (
            platform_value!({ "countOf": [""] }),
            "must name a document type first",
        ),
        (
            platform_value!({ "sumOf": ["pledge", 3] }),
            "at lessThan[0].sumOf must name the property to total second",
        ),
        (
            platform_value!({ "countOf": ["listing", {}] }),
            "at lessThan[0].countOf[1] must match its documents by one or more keys",
        ),
        (
            platform_value!({ "sumOf": ["pledge", "amount", [1]] }),
            "at lessThan[0].sumOf[2] must match its documents by one or more keys",
        ),
        (
            platform_value!({ "countOf": ["listing", { "$createdAt": 1 }] }),
            "matches by $createdAt, but the one system value a key names is $ownerId",
        ),
        (
            platform_value!({ "countOf": ["listing", { "a": "$createdAt" }] }),
            "at lessThan[0].countOf[1].a takes $createdAt, but the one system value a key takes \
             is $ownerId",
        ),
        (
            platform_value!({ "countOf": ["listing", { "a": true }] }),
            "at lessThan[0].countOf[1].a must be a property path of the document, $ownerId, an \
             integer or a { \"const\": ... }",
        ),
        (
            platform_value!({ "countOf": ["listing", { "a": { "const": 3 } }] }),
            "must be a property path of the document",
        ),
        (
            platform_value!({ "countOf": ["listing", { "a": 1.5 }] }),
            "at lessThan[0].countOf[1].a holds 1.5, which is not an integer",
        ),
    ] {
        expect_refusal(
            platform_value!({ "rule": { "lessThan": [operand, 10] } }),
            needle,
        );
    }
}

/// An aggregate takes its value in the system values: consensus gives each one
/// a rule reads, and a rule reading one it is not given is not judged, as a
/// client, which reads no state, gives none.
#[test]
fn should_read_an_aggregate_from_the_system_values_and_skip_a_rule_not_given_one() {
    let rule = parse_rule_value(platform_value!({
        "anyOf": [
            { "greaterThan": ["price", 1000] },
            { "lessThanOrEqual": [{ "countOf": ["order", { "$ownerId": "$ownerId" }] }, 10] }
        ]
    }));
    let read = rule.aggregate_reads()[0].clone();
    let cheap = data(&[("price", Value::U64(5))]);
    let with_total = |total: i128| DocumentSystemValues {
        aggregates: Some(BTreeMap::from([(read.clone(), total)])),
        ..DocumentSystemValues::default()
    };

    assert_eq!(
        rule.violation(&cheap, &DocumentSystemValues::default()),
        None
    );
    assert_eq!(rule.violation(&cheap, &with_total(10)), None);
    assert_eq!(
        rule.violation(&cheap, &with_total(11)),
        Some(PropertyConstraintViolation::NotMet)
    );
    // The first condition holds, so the total is never compared
    let dear = data(&[("price", Value::U64(5000))]);
    assert_eq!(rule.violation(&dear, &with_total(11)), None);
    // Consensus's totals lacking this one: the rule is not evaluated, and the
    // missing total is reported, which `validate_property_constraints` turns
    // into an error; a client's, which reads none, only skips the rule
    let other = DocumentSystemValues {
        aggregates: Some(BTreeMap::from([(
            AggregateRead {
                document_type: "listing".to_string(),
                ..read.clone()
            },
            99,
        )])),
        ..DocumentSystemValues::default()
    };
    assert_eq!(rule.violation(&cheap, &other), None);
    assert_eq!(rule.unread_aggregate(&other), Some(&read));
    assert_eq!(rule.unread_aggregate(&with_total(3)), None);
    assert_eq!(
        rule.unread_aggregate(&DocumentSystemValues::default()),
        None
    );
}
