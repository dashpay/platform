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
