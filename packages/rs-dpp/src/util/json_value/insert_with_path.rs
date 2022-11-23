use super::JsonValueExt;
use crate::util::json_path::JsonPathStep;
use anyhow::{bail, Context};
use serde_json::Value;

/// Inserts the value specified by the json path. If intermediate object doesn't exist, crates a one.
/// If `Value::Null` is encountered while traversing the path, they are replaced with the required structure.
pub(super) fn insert_with_path(
    data: &mut Value,
    json_path: &[JsonPathStep],
    value: Value,
) -> Result<(), anyhow::Error> {
    let mut current_level = data;
    let last_index = json_path.len() - 1;

    for (i, key) in json_path.iter().enumerate() {
        match key {
            JsonPathStep::Index(json_index) => {
                if i == last_index {
                    if current_level.is_null() {
                        *current_level = Value::Array(Default::default());
                    }
                    fill_empty_indexes(current_level, *json_index);
                    insert_into_array(current_level, *json_index, value).with_context(|| {
                        format!("failed inserting on position {json_index} into {current_level:#?}")
                    })?;
                    break;
                }

                if current_level.get(json_index).is_none() {
                    if current_level.is_null() {
                        *current_level = Value::Array(Default::default());
                    }
                    fill_empty_indexes(current_level, *json_index);
                    current_level.push(new_value_based_on_next_step(&json_path[i + 1]))?;
                }

                let new_level = current_level.get_mut(json_index).unwrap();
                current_level = new_level;
            }

            JsonPathStep::Key(key) => {
                if i == last_index {
                    if current_level.is_null() {
                        *current_level = Value::Object(Default::default());
                    }
                    current_level.insert(key.to_string(), value)?;
                    break;
                }

                if current_level.get(key).is_none() {
                    if current_level.is_null() {
                        *current_level = Value::Object(Default::default());
                    }
                    current_level.insert(
                        key.to_string(),
                        new_value_based_on_next_step(&json_path[i + 1]),
                    )?;
                }
                let new_level = current_level.get_mut(key).unwrap();
                current_level = new_level;
            }
        }
    }

    Ok(())
}

fn insert_into_array(
    maybe_array: &mut Value,
    position: usize,
    value: Value,
) -> Result<(), anyhow::Error> {
    match maybe_array.as_array_mut() {
        Some(ref mut array) => {
            if position >= array.len() {
                array.push(value)
            } else {
                array[position] = value;
            }
            Ok(())
        }
        None => bail!("expected array"),
    }
}

fn fill_empty_indexes(current_level: &mut Value, i: usize) {
    let index = i as i64;
    if let Some(array) = current_level.as_array_mut() {
        let positions_to_fill = index - (array.len() as i64 - 1) - 1;
        array.extend((0..positions_to_fill).map(|_| Value::Null));
    }
}

fn new_value_based_on_next_step(next_step: &JsonPathStep) -> Value {
    match next_step {
        JsonPathStep::Index(_) => Value::Array(Default::default()),
        JsonPathStep::Key(_) => Value::Object(Default::default()),
    }
}

#[cfg(test)]
mod test_set {
    use serde_json::json;

    use super::*;

    #[test]
    fn set_onto_nil_when_object_given() {
        let mut data = Value::Null;
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
            JsonPathStep::Key("c".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"]["b"]["c"], json!("alpha"))
    }

    #[test]
    fn set_onto_nil_when_array_given() {
        let mut data = Value::Null;
        let keys = [JsonPathStep::Index(0)];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data[0], json!("alpha"))
    }

    #[test]
    fn set_new_value_only_maps() {
        let mut data = Value::Object(Default::default());
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
            JsonPathStep::Key("c".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");
        assert_eq!(data["a"]["b"]["c"], json!("alpha"))
    }

    #[test]
    fn set_value_with_array() {
        let mut data = Value::Object(Default::default());
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
            JsonPathStep::Index(0),
            JsonPathStep::Key("c".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"]["b"][0]["c"], json!("alpha"))
    }

    #[test]
    fn set_value_with_array_padding() {
        let mut data = Value::Object(Default::default());
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
            JsonPathStep::Index(3),
            JsonPathStep::Key("c".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"]["b"][0]["c"], Value::Null);
        assert_eq!(data["a"]["b"][1]["c"], Value::Null);
        assert_eq!(data["a"]["b"][2]["c"], Value::Null);
        assert_eq!(data["a"]["b"][3]["c"], json!("alpha"));
    }

    #[test]
    fn test_set_the_existing_path() {
        let mut data = json!({
            "a":  {
                "b" : vec![ Value::Null, Value::Null]
             }
        });
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
            JsonPathStep::Index(1),
            JsonPathStep::Key("c".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"]["b"][1]["c"], json!("alpha"));
    }

    #[test]
    fn set_the_existing_root_path() {
        let mut data = json!({
            "a":  {}
        });
        let keys = [JsonPathStep::Key("a".to_string())];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"], json!("alpha"));
    }

    #[test]
    fn errors_if_existing_path_has_different_types() {
        let mut data = json!({
            "a":  {
                "b" : "some_string"
            }
        });
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
            JsonPathStep::Key("c".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect_err("error should be returned");
    }

    #[test]
    fn replace_if_existing_object_has_different_type() {
        let mut data = json!({
            "a":  {
                "b" : { "c": "bravo"}
            }
        });
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("b".to_string()),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"]["b"], json!("alpha"));
    }

    #[test]
    fn replace_if_existing_object_has_different_type_in_array() {
        let mut data = json!({ "a": [json!("already_taken")] });
        let keys = [JsonPathStep::Key("a".to_string()), JsonPathStep::Index(0)];

        insert_with_path(&mut data, &keys, json!("alpha")).expect("no errors");

        assert_eq!(data["a"][0], json!("alpha"));
    }

    #[test]
    fn error_if_try_set_the_array_index_when_object_is_not_array() {
        let mut data = json!({ "a": {
            "not_array" : {
                "c" :{},
            }
        } });
        let keys = [
            JsonPathStep::Key("a".to_string()),
            JsonPathStep::Key("not_array".to_string()),
            JsonPathStep::Index(0),
        ];

        insert_with_path(&mut data, &keys, json!("alpha")).expect_err("inserting error");
    }
}
