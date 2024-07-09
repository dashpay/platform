use json_schema_compatibility_validator::{
    validate_schemas_compatibility, CompatibilityRuleExample, Options, KEYWORD_COMPATIBILITY_RULES,
};

#[test]
fn test_schema_keyword_rules() {
    for (keyword, rule) in KEYWORD_COMPATIBILITY_RULES.iter() {
        println!("Testing `{}` keyword", keyword);

        assert_examples(keyword, &rule.examples);

        if let Some(inner_rule) = &rule.inner {
            assert_examples(keyword, &inner_rule.examples);
        }
    }
}

fn assert_examples(keyword: &str, examples: &[CompatibilityRuleExample]) {
    let options = Options::default();
    for example in examples {
        let result =
            validate_schemas_compatibility(&example.original_schema, &example.new_schema, &options)
                .expect("should not fail");

        if let Some(change) = &example.incompatible_change {
            let expected_change = vec![change.clone()];

            assert_eq!(
                result.incompatible_changes(),
                &expected_change,
                r"assertion failed: expected incompatible change of '{}'

From: {:?}
To: {:?}",
                keyword,
                &example.original_schema,
                &example.new_schema
            );
        } else {
            assert!(
                result.is_compatible(),
                r"assertion failed: '{keyword}' modification is not compatible: {:?}
From: {:?}
To: {:?}",
                result.incompatible_changes(),
                &example.original_schema,
                &example.new_schema
            );
        }
    }
}
