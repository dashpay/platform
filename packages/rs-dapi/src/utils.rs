use serde::Deserialize;
use serde::de::{Error as DeError, Visitor};
use serde_json::Value;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static JSONRPC_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn generate_jsonrpc_id() -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let timestamp_ns = elapsed.as_nanos();
    let pid = std::process::id();
    let counter = JSONRPC_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{timestamp_ns}-{pid}-{counter}")
}

pub fn deserialize_string_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: fmt::Display,
{
    struct StringOrNumberVisitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for StringOrNumberVisitor<T>
    where
        T: FromStr,
        <T as FromStr>::Err: fmt::Display,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, integer, float, or boolean")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            T::from_str(v).map_err(|e| DeError::custom(format!("invalid value: {}", e)))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_str(&v)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_string(v.to_string())
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_string(v.to_string())
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_string(v.to_string())
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_string(v.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor(PhantomData))
}

pub fn deserialize_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ToStringVisitor;

    impl<'de> Visitor<'de> for ToStringVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, integer, float, or boolean")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v.to_string())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v.to_string())
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v.to_string())
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v.to_string())
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v.to_string())
        }
    }

    deserializer.deserialize_any(ToStringVisitor)
}

pub fn deserialize_string_number_or_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;

    match value {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        Some(other) => Err(DeError::custom(format!(
            "expected string, number, bool, or null but got {}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    // -- generate_jsonrpc_id --

    #[test]
    fn generate_jsonrpc_id_is_unique() {
        let id1 = generate_jsonrpc_id();
        let id2 = generate_jsonrpc_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn generate_jsonrpc_id_has_expected_format() {
        let id = generate_jsonrpc_id();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3, "id should have 3 hyphen-separated parts");
        // All parts should be numeric
        for part in &parts {
            assert!(
                part.parse::<u64>().is_ok(),
                "part '{}' should be numeric",
                part
            );
        }
    }

    // -- deserialize_string_or_number --

    #[derive(Deserialize)]
    struct TestStruct {
        #[serde(deserialize_with = "deserialize_string_or_number")]
        value: u64,
    }

    #[test]
    fn deserialize_string_or_number_from_string() {
        let json = r#"{"value": "42"}"#;
        let s: TestStruct = serde_json::from_str(json).expect("deserialize string as u64");
        assert_eq!(s.value, 42);
    }

    #[test]
    fn deserialize_string_or_number_from_u64() {
        let json = r#"{"value": 100}"#;
        let s: TestStruct = serde_json::from_str(json).expect("deserialize u64");
        assert_eq!(s.value, 100);
    }

    #[test]
    fn deserialize_string_or_number_from_float() {
        #[derive(Deserialize)]
        struct FloatTest {
            #[serde(deserialize_with = "deserialize_string_or_number")]
            value: f64,
        }
        let json = r#"{"value": 7.25}"#;
        let s: FloatTest = serde_json::from_str(json).expect("deserialize f64");
        assert_eq!(s.value, 7.25);
    }

    #[test]
    fn deserialize_string_or_number_from_bool_true() {
        // bool "true" -> "true" -> u64 from_str will fail, so test with a type that supports it
        #[derive(Deserialize)]
        struct BoolTest {
            #[serde(deserialize_with = "deserialize_string_or_number")]
            value: bool,
        }
        let json = r#"{"value": true}"#;
        let s: BoolTest = serde_json::from_str(json).expect("deserialize bool");
        assert!(s.value);
    }

    #[test]
    fn deserialize_string_or_number_from_negative_i64() {
        #[derive(Deserialize)]
        struct I64Test {
            #[serde(deserialize_with = "deserialize_string_or_number")]
            value: i64,
        }
        let json = r#"{"value": -42}"#;
        let s: I64Test = serde_json::from_str(json).expect("deserialize negative i64");
        assert_eq!(s.value, -42);
    }

    // -- deserialize_to_string --

    #[derive(Deserialize)]
    struct ToStringTest {
        #[serde(deserialize_with = "deserialize_to_string")]
        value: String,
    }

    #[test]
    fn deserialize_to_string_from_string() {
        let json = r#"{"value": "hello"}"#;
        let s: ToStringTest = serde_json::from_str(json).expect("deserialize string to string");
        assert_eq!(s.value, "hello");
    }

    #[test]
    fn deserialize_to_string_from_u64() {
        let json = r#"{"value": 999}"#;
        let s: ToStringTest = serde_json::from_str(json).expect("deserialize u64 to string");
        assert_eq!(s.value, "999");
    }

    #[test]
    fn deserialize_to_string_from_i64() {
        let json = r#"{"value": -50}"#;
        let s: ToStringTest = serde_json::from_str(json).expect("deserialize i64 to string");
        assert_eq!(s.value, "-50");
    }

    #[test]
    fn deserialize_to_string_from_f64() {
        let json = r#"{"value": 3.14}"#;
        let s: ToStringTest = serde_json::from_str(json).expect("deserialize f64 to string");
        assert!(s.value.starts_with("3.14"));
    }

    #[test]
    fn deserialize_to_string_from_bool() {
        let json = r#"{"value": false}"#;
        let s: ToStringTest = serde_json::from_str(json).expect("deserialize bool to string");
        assert_eq!(s.value, "false");
    }

    // -- deserialize_string_number_or_null --

    #[derive(Debug, Deserialize)]
    struct NullableTest {
        #[serde(deserialize_with = "deserialize_string_number_or_null")]
        value: String,
    }

    #[test]
    fn deserialize_string_number_or_null_from_null() {
        let json = r#"{"value": null}"#;
        let s: NullableTest = serde_json::from_str(json).expect("deserialize null");
        assert_eq!(s.value, "");
    }

    #[test]
    fn deserialize_string_number_or_null_from_string() {
        let json = r#"{"value": "test"}"#;
        let s: NullableTest = serde_json::from_str(json).expect("deserialize string as nullable");
        assert_eq!(s.value, "test");
    }

    #[test]
    fn deserialize_string_number_or_null_from_number() {
        let json = r#"{"value": 42}"#;
        let s: NullableTest = serde_json::from_str(json).expect("deserialize number as nullable");
        assert_eq!(s.value, "42");
    }

    #[test]
    fn deserialize_string_number_or_null_from_bool() {
        let json = r#"{"value": true}"#;
        let s: NullableTest = serde_json::from_str(json).expect("deserialize bool as nullable");
        assert_eq!(s.value, "true");
    }

    #[test]
    fn deserialize_string_number_or_null_from_array_fails() {
        let json = r#"{"value": [1, 2]}"#;
        let result: Result<NullableTest, _> = serde_json::from_str(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected string"));
    }
}
