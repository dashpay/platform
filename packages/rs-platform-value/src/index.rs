use core::fmt::{self, Display};
use core::ops;

use super::Value;
use crate::value_map::{ValueMap, ValueMapHelper};

/// A type that can be used to index into a `platform_value::Value`.
///
/// The [`get`] and [`get_mut`] methods of `Value` accept any type that
/// implements `Index`, as does the [square-bracket indexing operator]. This
/// trait is implemented for strings which are used as the index into a JSON
/// map, and for `usize` which is used as the index into a JSON array.
///
/// [`get`]: ../enum.Value.html#method.get
/// [`get_mut`]: ../enum.Value.html#method.get_mut
/// [square-bracket indexing operator]: ../enum.Value.html#impl-Index%3CI%3E
///
/// This trait is sealed and cannot be implemented for types outside of
/// `platform_value`.
///
/// # Examples
///
/// ```
/// # use platform_value::platform_value;
/// #
/// let data = platform_value!({ "inner": [1, 2, 3] });
///
/// // Data is a JSON map so it can be indexed with a string.
/// let inner = &data["inner"];
///
/// // Inner is a JSON array so it can be indexed with an integer.
/// let first = &inner[0];
///
/// assert_eq!(first, 1);
/// ```
pub trait Index: private::Sealed {
    /// Return None if the key is not already in the array or object.
    #[doc(hidden)]
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;

    /// Return None if the key is not already in the array or object.
    #[doc(hidden)]
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value>;

    /// Panic if array index out of bounds. If key is not already in the object,
    /// insert it with a value of null. Panic if Value is a type that cannot be
    /// indexed into, except if Value is null then it can be treated as an empty
    /// object.
    #[doc(hidden)]
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value;
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Array(vec) => vec.get(*self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Array(vec) => vec.get_mut(*self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::Array(vec) => {
                let len = vec.len();
                vec.get_mut(*self).unwrap_or_else(|| {
                    panic!(
                        "cannot access index {} of JSON array of length {}",
                        self, len
                    )
                })
            }
            _ => panic!("cannot access index {} of JSON {}", self, Type(v)),
        }
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Map(map) => map.get_optional_key(self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Map(map) => map.get_optional_key_mut(self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        if let Value::Null = v {
            *v = Value::Map(ValueMap::new());
        }
        match v {
            Value::Map(map) => map.get_key_mut_or_insert(self, Value::Null),
            _ => panic!("cannot access key {:?} in JSON {}", self, Type(v)),
        }
    }
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self[..].index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        self[..].index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        self[..].index_or_insert(v)
    }
}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(v)
    }
}

// Prevent users from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<T> Sealed for &T where T: ?Sized + Sealed {}
}

/// Used in panic messages.
struct Type<'a>(&'a Value);

impl Display for Type<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            Value::Null => formatter.write_str("null"),
            Value::Bool(_) => formatter.write_str("boolean"),
            Value::Float(_) => formatter.write_str("float"),
            Value::Text(_) => formatter.write_str("string"),
            Value::Array(_) => formatter.write_str("array"),
            Value::Map(_) => formatter.write_str("map"),
            Value::U128(_) => formatter.write_str("u128"),
            Value::I128(_) => formatter.write_str("i128"),
            Value::U64(_) => formatter.write_str("u64"),
            Value::I64(_) => formatter.write_str("i64"),
            Value::U32(_) => formatter.write_str("u32"),
            Value::I32(_) => formatter.write_str("i32"),
            Value::U16(_) => formatter.write_str("u16"),
            Value::I16(_) => formatter.write_str("i16"),
            Value::U8(_) => formatter.write_str("u8"),
            Value::I8(_) => formatter.write_str("i8"),
            Value::Bytes(_) => formatter.write_str("bytes"),
            Value::Bytes20(_) => formatter.write_str("bytes20"),
            Value::Bytes32(_) => formatter.write_str("bytes32"),
            Value::Bytes36(_) => formatter.write_str("bytes36"),
            Value::Identifier(_) => formatter.write_str("identifier"),
            Value::EnumU8(_) => formatter.write_str("enum u8"),
            Value::EnumString(_) => formatter.write_str("enum string"),
        }
    }
}

// The usual semantics of Index is to panic on invalid indexing.
//
// That said, the usual semantics are for things like Vec and BTreeMap which
// have different use cases than Value. If you are working with a Vec, you know
// that you are working with a Vec and you can get the len of the Vec and make
// sure your indices are within bounds. The Value use cases are more
// loosey-goosey. You got some JSON from an endpoint and you want to pull values
// out of it. Outside of this Index impl, you already have the option of using
// value.as_array() and working with the Vec directly, or matching on
// Value::Array and getting the Vec directly. The Index impl means you can skip
// that and index directly into the thing using a concise syntax. You don't have
// to check the type, you don't have to check the len, it is all about what you
// expect the Value to look like.
//
// Basically the use cases that would be well served by panicking here are
// better served by using one of the other approaches: get and get_mut,
// as_array, or match. The value of this impl is that it adds a way of working
// with Value that is not well served by the existing approaches: concise and
// careless and sometimes that is exactly what you want.
impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Value;

    /// Index into a `serde_json::Value` using the syntax `value[0]` or
    /// `value["k"]`.
    ///
    /// Returns `Value::Null` if the type of `self` does not match the type of
    /// the index, for example if the index is a string and `self` is an array
    /// or a number. Also returns `Value::Null` if the given key does not exist
    /// in the map or the given index is not within the bounds of the array.
    ///
    /// For retrieving deeply nested values, you should have a look at the
    /// `Value::pointer` method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use platform_value::platform_value;
    /// #
    /// let data = platform_value!({
    ///     "x": {
    ///         "y": ["z", "zz"]
    ///     }
    /// });
    ///
    /// assert_eq!(data["x"]["y"], platform_value!(["z", "zz"]));
    /// assert_eq!(data["x"]["y"][0], platform_value!("z"));
    ///
    /// assert_eq!(data["a"], platform_value!(null)); // returns null for undefined values
    /// assert_eq!(data["a"]["b"], platform_value!(null)); // does not panic
    /// ```
    fn index(&self, index: I) -> &Value {
        static NULL: Value = Value::Null;
        index.index_into(self).unwrap_or(&NULL)
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    /// Write into a `serde_json::Value` using the syntax `value[0] = ...` or
    /// `value["k"] = ...`.
    ///
    /// If the index is a number, the value must be an array of length bigger
    /// than the index. Indexing into a value that is not an array or an array
    /// that is too small will panic.
    ///
    /// If the index is a string, the value must be an object or null which is
    /// treated like an empty object. If the key is not already present in the
    /// object, it will be inserted with a value of null. Indexing into a value
    /// that is neither an object nor null will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use platform_value::platform_value;
    /// #
    /// let mut data = platform_value!({ "x": 0 });
    ///
    /// // replace an existing key
    /// data["x"] = platform_value!(1);
    ///
    /// // insert a new key
    /// data["y"] = platform_value!([false, false, false]);
    ///
    /// // replace an array value
    /// data["y"][0] = platform_value!(true);
    ///
    /// // inserted a deeply nested key
    /// data["a"]["b"]["c"]["d"] = platform_value!(true);
    ///
    /// println!("{}", data);
    /// ```
    fn index_mut(&mut self, index: I) -> &mut Value {
        index.index_or_insert(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_value;

    // ===============================================================
    // Index<usize> for Value — access array element
    // ===============================================================

    #[test]
    fn index_usize_access_array_element() {
        let value = platform_value!([10, 20, 30]);
        assert_eq!(value[0], platform_value!(10));
        assert_eq!(value[1], platform_value!(20));
        assert_eq!(value[2], platform_value!(30));
    }

    // ===============================================================
    // Index<usize> for Value — out-of-bounds returns Null
    // ===============================================================

    #[test]
    fn index_usize_out_of_bounds_returns_null() {
        let value = platform_value!([10, 20]);
        // The ops::Index impl returns &NULL for missing indices
        assert_eq!(value[99], Value::Null);
    }

    // ===============================================================
    // Index<usize> for Value — non-array returns Null
    // ===============================================================

    #[test]
    fn index_usize_on_non_array_returns_null() {
        let value = platform_value!(42);
        // ops::Index returns &NULL when index_into returns None
        assert_eq!(value[0], Value::Null);
    }

    #[test]
    fn index_usize_on_map_returns_null() {
        let value = platform_value!({ "key": "val" });
        assert_eq!(value[0], Value::Null);
    }

    // ===============================================================
    // IndexMut<usize> — panic on out-of-bounds
    // ===============================================================

    #[test]
    #[should_panic(expected = "cannot access index 5 of JSON array of length 2")]
    fn index_mut_usize_out_of_bounds_panics() {
        let mut value = platform_value!([10, 20]);
        value[5] = platform_value!(99);
    }

    // ===============================================================
    // IndexMut<usize> — panic on non-array
    // ===============================================================

    #[test]
    #[should_panic(expected = "cannot access index 0 of JSON")]
    fn index_mut_usize_on_non_array_panics() {
        let mut value = platform_value!(42);
        value[0] = platform_value!(99);
    }

    // ===============================================================
    // IndexMut<usize> — successfully write
    // ===============================================================

    #[test]
    fn index_mut_usize_write() {
        let mut value = platform_value!([10, 20, 30]);
        value[1] = platform_value!(99);
        assert_eq!(value[1], platform_value!(99));
    }

    // ===============================================================
    // Index<&str> for Value — access map key
    // ===============================================================

    #[test]
    fn index_str_access_map_key() {
        let value = platform_value!({ "name": "Alice", "age": 30 });
        assert_eq!(value["name"], platform_value!("Alice"));
        assert_eq!(value["age"], platform_value!(30));
    }

    // ===============================================================
    // Index<&str> for Value — missing key returns Null
    // ===============================================================

    #[test]
    fn index_str_missing_key_returns_null() {
        let value = platform_value!({ "name": "Alice" });
        assert_eq!(value["missing"], Value::Null);
    }

    // ===============================================================
    // Index<&str> for Value — non-map returns Null
    // ===============================================================

    #[test]
    fn index_str_on_non_map_returns_null() {
        let value = platform_value!(42);
        assert_eq!(value["key"], Value::Null);
    }

    #[test]
    fn index_str_on_array_returns_null() {
        let value = platform_value!([1, 2, 3]);
        assert_eq!(value["key"], Value::Null);
    }

    // ===============================================================
    // Index<&str> for Value — nested access
    // ===============================================================

    #[test]
    fn index_str_nested_access() {
        let value = platform_value!({
            "outer": {
                "inner": {
                    "deep": 42
                }
            }
        });
        assert_eq!(value["outer"]["inner"]["deep"], platform_value!(42));
    }

    #[test]
    fn index_str_nested_missing_returns_null_chain() {
        let value = platform_value!({ "a": { "b": 1 } });
        // "a" -> "c" -> doesn't exist, returns Null
        // then Null["anything"] also returns Null
        assert_eq!(value["a"]["c"], Value::Null);
        assert_eq!(value["a"]["c"]["d"], Value::Null);
    }

    // ===============================================================
    // IndexMut<&str> — write to existing key
    // ===============================================================

    #[test]
    fn index_mut_str_write_existing() {
        let mut value = platform_value!({ "x": 0 });
        value["x"] = platform_value!(42);
        assert_eq!(value["x"], platform_value!(42));
    }

    // ===============================================================
    // IndexMut<&str> — insert new key
    // ===============================================================

    #[test]
    fn index_mut_str_insert_new_key() {
        let mut value = platform_value!({ "x": 0 });
        value["y"] = platform_value!("hello");
        assert_eq!(value["y"], platform_value!("hello"));
    }

    // ===============================================================
    // IndexMut<&str> — Null becomes empty map
    // ===============================================================

    #[test]
    fn index_mut_str_null_becomes_map() {
        let mut value = Value::Null;
        value["key"] = platform_value!(1);
        assert_eq!(value["key"], platform_value!(1));
        assert!(value.is_map());
    }

    // ===============================================================
    // IndexMut<&str> — deeply nested insert via Null
    // ===============================================================

    #[test]
    fn index_mut_str_deeply_nested_insert() {
        let mut value = platform_value!({ "x": 0 });
        // "a" -> inserts Null, then Null becomes map for "b", etc.
        value["a"]["b"]["c"] = platform_value!(true);
        assert_eq!(value["a"]["b"]["c"], platform_value!(true));
    }

    // ===============================================================
    // IndexMut<&str> — panic on non-map non-null
    // ===============================================================

    #[test]
    #[should_panic(expected = "cannot access key")]
    fn index_mut_str_on_non_map_panics() {
        let mut value = platform_value!(42);
        value["key"] = platform_value!(1);
    }

    // ===============================================================
    // Index<String> delegates to str
    // ===============================================================

    #[test]
    fn index_string_delegates_to_str() {
        let value = platform_value!({ "name": "Bob" });
        let key = String::from("name");
        assert_eq!(value[&key], platform_value!("Bob"));
    }

    // ===============================================================
    // IndexMut<String> delegates to str
    // ===============================================================

    #[test]
    fn index_mut_string_delegates_to_str() {
        let mut value = platform_value!({ "name": "Bob" });
        let key = String::from("name");
        value[&key] = platform_value!("Alice");
        assert_eq!(value["name"], platform_value!("Alice"));
    }

    // ===============================================================
    // index_into — returns None for various non-matching types
    // ===============================================================

    #[test]
    fn index_into_usize_returns_none_for_non_array() {
        let value = Value::Text("hello".into());
        assert!(0usize.index_into(&value).is_none());
    }

    #[test]
    fn index_into_str_returns_none_for_non_map() {
        let value = Value::Array(vec![Value::U32(1)]);
        assert!("key".index_into(&value).is_none());
    }

    // ===============================================================
    // index_into_mut — returns None for non-matching types
    // ===============================================================

    #[test]
    fn index_into_mut_usize_returns_none_for_non_array() {
        let mut value = Value::Bool(true);
        assert!(0usize.index_into_mut(&mut value).is_none());
    }

    #[test]
    fn index_into_mut_str_returns_none_for_non_map() {
        let mut value = Value::U64(100);
        assert!("key".index_into_mut(&mut value).is_none());
    }

    // ===============================================================
    // index_into_mut — returns Some for valid accesses
    // ===============================================================

    #[test]
    fn index_into_mut_usize_returns_some() {
        let mut value = platform_value!([10, 20]);
        let got = 0usize.index_into_mut(&mut value);
        assert!(got.is_some());
        *got.unwrap() = platform_value!(99);
        assert_eq!(value[0], platform_value!(99));
    }

    #[test]
    fn index_into_mut_str_returns_some() {
        let mut value = platform_value!({ "k": 1 });
        let got = "k".index_into_mut(&mut value);
        assert!(got.is_some());
        *got.unwrap() = platform_value!(42);
        assert_eq!(value["k"], platform_value!(42));
    }

    // ===============================================================
    // Combined array + map indexing
    // ===============================================================

    #[test]
    fn combined_array_map_indexing() {
        let value = platform_value!({
            "items": [
                { "name": "first" },
                { "name": "second" }
            ]
        });
        assert_eq!(value["items"][0]["name"], platform_value!("first"));
        assert_eq!(value["items"][1]["name"], platform_value!("second"));
    }

    #[test]
    fn combined_array_map_indexing_mut() {
        let mut value = platform_value!({
            "items": [
                { "name": "first" },
                { "name": "second" }
            ]
        });
        value["items"][0]["name"] = platform_value!("updated");
        assert_eq!(value["items"][0]["name"], platform_value!("updated"));
    }

    // ===============================================================
    // get() method — returns Some for existing, None for missing
    // ===============================================================

    #[test]
    fn get_method_returns_some_for_existing_key() {
        let value = platform_value!({ "x": 10 });
        let result = value.get("x").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &platform_value!(10));
    }

    #[test]
    fn get_method_returns_none_for_missing_key() {
        let value = platform_value!({ "x": 10 });
        let result = value.get("y").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_method_errors_on_non_map() {
        let value = platform_value!(42);
        let result = value.get("key");
        assert!(result.is_err());
    }

    // ===============================================================
    // Type display coverage (used in panic messages)
    // ===============================================================

    #[test]
    fn type_display_covers_all_variants() {
        use core::fmt::Write;
        let variants: Vec<Value> = vec![
            Value::Null,
            Value::Bool(true),
            Value::Float(1.0),
            Value::Text("s".into()),
            Value::Array(vec![]),
            Value::Map(vec![]),
            Value::U128(1),
            Value::I128(1),
            Value::U64(1),
            Value::I64(1),
            Value::U32(1),
            Value::I32(1),
            Value::U16(1),
            Value::I16(1),
            Value::U8(1),
            Value::I8(1),
            Value::Bytes(vec![]),
            Value::Bytes20([0u8; 20]),
            Value::Bytes32([0u8; 32]),
            Value::Bytes36([0u8; 36]),
            Value::Identifier([0u8; 32]),
            Value::EnumU8(vec![]),
            Value::EnumString(vec![]),
        ];
        for v in &variants {
            let t = Type(v);
            let mut buf = String::new();
            write!(buf, "{}", t).unwrap();
            assert!(!buf.is_empty());
        }
    }
}
