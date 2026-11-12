use platform_value::Value;

fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(
        entries
            .iter()
            .map(|(key, value)| ((*key).into(), value.clone()))
            .collect(),
    )
}

fn repeated(key: &str) -> Value {
    map(&[(key, Value::Null), (key, Value::Null)])
}

#[test]
fn should_find_equal_duplicates_and_check_a_parent_before_its_children() {
    let value = map(&[("parent", repeated("child")), ("parent", Value::Null)]);
    assert_eq!(value.first_repeated_text_key(), Some("parent"));
    let value = map(&[
        ("z", Value::Null),
        ("z", Value::Bool(true)),
        ("a", Value::Null),
        ("a", Value::Null),
    ]);
    assert_eq!(value.first_repeated_text_key(), Some("z"));
}

#[test]
fn should_visit_map_values_and_array_elements_in_input_order() {
    let value = map(&[
        (
            "z",
            Value::Array(vec![repeated("first"), repeated("second")]),
        ),
        ("a", repeated("third")),
    ]);
    assert_eq!(value.first_repeated_text_key(), Some("first"));
    assert_eq!(
        Value::Array(vec![Value::Null, repeated("array")]).first_repeated_text_key(),
        Some("array")
    );
}

#[test]
fn should_keep_maps_independent_and_compare_keys_without_normalization() {
    let value = map(&[
        ("a", map(&[("same", Value::Null)])),
        ("b", map(&[("same", Value::Null)])),
    ]);
    assert_eq!(value.first_repeated_text_key(), None);
    assert_eq!(
        map(&[
            ("é", Value::Null),
            ("e\u{301}", Value::Null),
            ("É", Value::Null)
        ])
        .first_repeated_text_key(),
        None
    );
}

#[test]
fn should_leave_non_text_keys_and_primitive_collections_to_value_validation() {
    let value = Value::Map(vec![
        (Value::U32(1), Value::Null),
        (Value::U32(1), Value::Null),
        (repeated("invalid-key"), Value::Null),
    ]);
    assert_eq!(value.first_repeated_text_key(), None);
    for value in [
        Value::Bytes(vec![1, 1]),
        Value::EnumString(vec!["a".into(), "a".into()]),
        Value::Array(vec![Value::U32(1), Value::U32(1)]),
        Value::Null,
        Value::Map(vec![]),
    ] {
        assert_eq!(value.first_repeated_text_key(), None);
    }
}

#[test]
fn should_scan_deep_and_wide_values_iteratively() {
    let value = (0..4_000).fold(repeated("deep"), |value, depth| {
        if depth % 2 == 0 {
            Value::Array(vec![value])
        } else {
            Value::Map(vec![("nested".into(), value)])
        }
    });
    assert_eq!(value.first_repeated_text_key(), Some("deep"));
    // Dropping a Value is recursive; this test exercises only the iterative scan.
    std::mem::forget(value);
    let mut entries: Vec<_> = (0..4_000)
        .map(|index| (format!("key{index}").into(), Value::Null))
        .collect();
    entries.push(("key3999".into(), Value::Null));
    assert_eq!(
        Value::Map(entries).first_repeated_text_key(),
        Some("key3999")
    );
}
