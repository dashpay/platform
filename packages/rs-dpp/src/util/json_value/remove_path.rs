use serde_json::Value;

use crate::util::json_path::JsonPathStep;

/// Removes value from provided path. If path does't exist None is returned.
pub(super) fn remove_path(path: &[JsonPathStep], value: &mut Value) -> Option<Value> {
    let mut last_ptr: &mut Value = value;
    let last_index = path.len() - 1;

    for (i, step) in path.iter().enumerate() {
        match step {
            JsonPathStep::Index(index) => {
                if i == last_index {
                    if let Value::Array(ref mut arr) = last_ptr {
                        if *index >= arr.len() {
                            return None;
                        }
                        return Some(arr.remove(*index));
                    }
                } else {
                    last_ptr = last_ptr.get_mut(index)?;
                }
            }
            JsonPathStep::Key(key) => {
                if i == last_index {
                    if let Value::Object(ref mut map) = last_ptr {
                        return map.remove(key);
                    }
                } else {
                    last_ptr = last_ptr.get_mut(key)?;
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn should_remove_data_from_path_to_object() {
        let mut document = json!({
            "root" :  {
                "from" : {
                    "id": "123",
                    "message": "text_message",
                },
            }
        });

        let removed = remove_path(
            &[
                JsonPathStep::Key("root".to_string()),
                JsonPathStep::Key("from".to_string()),
                JsonPathStep::Key("message".to_string()),
            ],
            &mut document,
        )
        .expect("the value should be removed");
        assert_eq!(Value::String("text_message".to_string()), removed);
        assert!(document["root"]["from"].get("message").is_none());
    }

    #[test]
    fn should_remove_data_from_path_to_array() {
        let mut document = json!({
            "root" :  {
                "from" : [
                    "alpha",
                    "bravo",
                    "charlie",
                    "delta"
                ],
            }
        });

        let removed = remove_path(
            &[
                JsonPathStep::Key("root".to_string()),
                JsonPathStep::Key("from".to_string()),
                JsonPathStep::Index(1),
            ],
            &mut document,
        )
        .expect("the value should be removed");

        assert_eq!(Value::String("bravo".to_string()), removed);
        assert_eq!(3, document["root"]["from"].as_array().unwrap().len());
    }

    #[test]
    fn should_return_none_if_index_out_of_range() {
        let mut document = json!({
            "root" :  {
                "from" : [
                    "alpha",
                    "bravo",
                    "charlie",
                    "delta"
                ],
            }
        });

        let removed = remove_path(
            &[
                JsonPathStep::Key("root".to_string()),
                JsonPathStep::Key("from".to_string()),
                JsonPathStep::Index(4),
            ],
            &mut document,
        );

        assert!(removed.is_none());
    }

    #[test]
    fn should_return_none_if_object_not_exist() {
        let mut document = json!({
            "root" :  {
                "from" : [
                    "alpha",
                    "bravo",
                    "charlie",
                    "delta"
                ],
            }
        });

        let removed = remove_path(
            &[
                JsonPathStep::Key("root".to_string()),
                JsonPathStep::Key("to".to_string()),
            ],
            &mut document,
        );

        assert!(removed.is_none());
    }
}
