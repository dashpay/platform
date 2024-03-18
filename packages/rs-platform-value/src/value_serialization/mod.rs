use crate::value_serialization::ser::Serializer;
use crate::{Error, Value};
use serde::Deserialize;
use serde::Serialize;

pub mod de;
pub mod ser;

/// Convert a `T` into `platform_value::Value` which is an enum that can represent
/// data.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use platform_value::platform_value;
///
/// use std::error::Error;
///
/// #[derive(Serialize)]
/// struct User {
///     fingerprint: String,
///     location: String,
/// }
///
/// fn compare_platform_values() -> Result<(), Box<dyn Error>> {
///     let u = User {
///         fingerprint: "0xF9BA143B95FF6D82".to_owned(),
///         location: "Menlo Park, CA".to_owned(),
///     };
///
///     // The type of `expected` is `serde_json::Value`
///     let expected = platform_value!({
///         "fingerprint": "0xF9BA143B95FF6D82",
///         "location": "Menlo Park, CA",
///     });
///
///     let v = platform_value::to_value(u).unwrap();
///     assert_eq!(v, expected);
///
///     Ok(())
/// }
/// #
/// # compare_platform_values().unwrap();
/// ```
///
/// # Errors
///
/// This conversion can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
///
/// ```
/// use std::collections::BTreeMap;
///
/// // The keys in this map are vectors, not strings.
/// let mut map = BTreeMap::new();
/// map.insert(vec![32, 64], "x86");
///
/// println!("{}", platform_value::to_value(map).unwrap_err());
/// ```
pub fn to_value<T>(value: T) -> Result<Value, Error>
where
    T: Serialize,
{
    value.serialize(Serializer)
}

/// Interpret a `serde_json::Value` as an instance of type `T`.
///
/// # Example
///
/// ```
/// use serde::Deserialize;
/// use platform_value::platform_value;
///
/// #[derive(Deserialize, Debug)]
/// struct User {
///     fingerprint: String,
///     location: String,
/// }
///
/// // The type of `j` is `serde_json::Value`
/// let j = platform_value!({
///     "fingerprint": "0xF9BA143B95FF6D82",
///     "location": "Menlo Park, CA"
/// });
///
/// let u: User = platform_value::from_value(j).unwrap();
/// println!("{:#?}", u);
/// ```
///
/// # Errors
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a JSON map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the JSON map or some number is too big to fit in the expected primitive
/// type.
pub fn from_value<'de, T>(value: Value) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    T::deserialize(de::Deserializer(value))
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn yeet() {
        #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
        struct Yeet {
            arr: Vec<String>,
            map: HashMap<String, char>,
            number: i32,
            //todo: manage static strings
            //static_string: &'static str,
        }

        let mut hm = HashMap::new();
        hm.insert("wow".to_owned(), 'a');
        hm.insert("lol".to_owned(), 'd');

        let yeet = Yeet {
            arr: vec!["kek".to_owned(), "top".to_owned()],
            map: hm,
            number: 420,
            //static_string: "pizza",
        };

        let platform_value = to_value(yeet.clone()).expect("please");
        let yeet_back: Yeet = from_value(platform_value).expect("please once again");

        assert_eq!(yeet, yeet_back);
    }
}
