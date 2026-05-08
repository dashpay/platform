use crate::{Value, ValueMapHelper};
use std::mem;

fn parse_index(s: &str) -> Option<usize> {
    if s.starts_with('+') || (s.starts_with('0') && s.len() != 1) {
        return None;
    }
    s.parse().ok()
}

impl Value {
    /// Looks up a value by a Platform Value Pointer.
    ///
    /// Platform Value Pointer defines a string syntax for identifying a specific value
    /// within a Platform Value document.
    ///
    /// A Pointer is a Unicode string with the reference tokens separated by `/`.
    /// Inside tokens `/` is replaced by `~1` and `~` is replaced by `~0`. The
    /// addressed value is returned and if there is no such value `None` is
    /// returned.
    ///
    /// For more information read [RFC6901](https://tools.ietf.org/html/rfc6901).
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
    /// assert_eq!(data.pointer("/x/y/1").unwrap(), &platform_value!("zz"));
    /// assert_eq!(data.pointer("/a/b/c"), None);
    /// ```
    pub fn pointer(&self, pointer: &str) -> Option<&Value> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Value::Map(map) => map.get_optional_key(&token),
                Value::Array(list) => parse_index(&token).and_then(|x| list.get(x)),
                _ => None,
            })
    }

    /// Looks up a value by a Platform Value Pointer and returns a mutable reference to
    /// that value.
    ///
    /// Platform Value Pointer defines a string syntax for identifying a specific value
    /// within a Platform Value document.
    ///
    /// A Pointer is a Unicode string with the reference tokens separated by `/`.
    /// Inside tokens `/` is replaced by `~1` and `~` is replaced by `~0`. The
    /// addressed value is returned and if there is no such value `None` is
    /// returned.
    ///
    /// For more information read [RFC6901](https://tools.ietf.org/html/rfc6901).
    ///
    /// # Example of Use
    ///
    /// ```
    /// use platform_value::Value;
    ///
    /// use platform_value::platform_value;
    /// let mut value: Value = platform_value!({"x": 1.0, "y": 2.0});
    ///
    /// // Check value using read-only pointer
    /// assert_eq!(value.pointer("/x"), Some(&1.0.into()));
    /// // Change value with direct assignment
    /// *value.pointer_mut("/x").unwrap() = 1.5.into();
    /// // Check that new value was written
    /// assert_eq!(value.pointer("/x"), Some(&1.5.into()));
    /// // Or change the value only if it exists
    /// value.pointer_mut("/x").map(|v| *v = 1.5.into());
    ///
    /// // "Steal" ownership of a value. Can replace with any valid Value.
    /// let old_x = value.pointer_mut("/x").map(Value::take).unwrap();
    /// assert_eq!(old_x, 1.5);
    /// assert_eq!(value.pointer("/x").unwrap(), &Value::Null);
    /// ```
    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut Value> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Value::Map(map) => map.get_optional_key_mut(&token),
                Value::Array(list) => parse_index(&token).and_then(move |x| list.get_mut(x)),
                _ => None,
            })
    }

    /// Takes the value out of the `Value`, leaving a `Null` in its place.
    ///
    /// ```
    /// # use platform_value::platform_value;
    /// #
    /// let mut v = platform_value!({ "x": "y" });
    /// assert_eq!(v["x"].take(), platform_value!("y"));
    /// assert_eq!(v, platform_value!({ "x": null }));
    /// ```
    pub fn take(&mut self) -> Value {
        mem::replace(self, Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use crate::{platform_value, Value};

    // ---------------------------------------------------------------
    // parse_index (module-private helper)
    // ---------------------------------------------------------------

    #[test]
    fn parse_index_valid_number() {
        assert_eq!(super::parse_index("0"), Some(0));
        assert_eq!(super::parse_index("1"), Some(1));
        assert_eq!(super::parse_index("42"), Some(42));
    }

    #[test]
    fn parse_index_leading_plus_returns_none() {
        assert_eq!(super::parse_index("+1"), None);
    }

    #[test]
    fn parse_index_leading_zero_returns_none() {
        assert_eq!(super::parse_index("01"), None);
        assert_eq!(super::parse_index("007"), None);
    }

    #[test]
    fn parse_index_single_zero_is_valid() {
        assert_eq!(super::parse_index("0"), Some(0));
    }

    #[test]
    fn parse_index_non_numeric_returns_none() {
        assert_eq!(super::parse_index("abc"), None);
        assert_eq!(super::parse_index(""), None);
    }

    // ---------------------------------------------------------------
    // pointer() — read-only access
    // ---------------------------------------------------------------

    #[test]
    fn pointer_empty_string_returns_self() {
        let data = platform_value!({"a": 1});
        assert_eq!(data.pointer(""), Some(&data));
    }

    #[test]
    fn pointer_no_leading_slash_returns_none() {
        let data = platform_value!({"a": 1});
        assert_eq!(data.pointer("a"), None);
        assert_eq!(data.pointer("a/b"), None);
    }

    #[test]
    fn pointer_simple_key_lookup() {
        let data = platform_value!({"a": 1, "b": 2});
        assert_eq!(data.pointer("/a"), Some(&platform_value!(1)));
        assert_eq!(data.pointer("/b"), Some(&platform_value!(2)));
    }

    #[test]
    fn pointer_nested_key_lookup() {
        let data = platform_value!({"x": {"y": {"z": 42}}});
        assert_eq!(data.pointer("/x/y/z"), Some(&platform_value!(42)));
    }

    #[test]
    fn pointer_missing_path_returns_none() {
        let data = platform_value!({"a": 1});
        assert_eq!(data.pointer("/b"), None);
        assert_eq!(data.pointer("/a/b/c"), None);
    }

    #[test]
    fn pointer_array_index() {
        let data = platform_value!({"arr": [10, 20, 30]});
        assert_eq!(data.pointer("/arr/0"), Some(&platform_value!(10)));
        assert_eq!(data.pointer("/arr/1"), Some(&platform_value!(20)));
        assert_eq!(data.pointer("/arr/2"), Some(&platform_value!(30)));
    }

    #[test]
    fn pointer_array_out_of_bounds_returns_none() {
        let data = platform_value!({"arr": [10]});
        assert_eq!(data.pointer("/arr/5"), None);
    }

    #[test]
    fn pointer_tilde_escape_tilde_zero_becomes_tilde() {
        // Key contains a literal ~ character, encoded as ~0
        let data = platform_value!({"a~b": 1});
        assert_eq!(data.pointer("/a~0b"), Some(&platform_value!(1)));
    }

    #[test]
    fn pointer_tilde_escape_tilde_one_becomes_slash() {
        // Key contains a literal / character, encoded as ~1
        let data = platform_value!({"a/b": 1});
        assert_eq!(data.pointer("/a~1b"), Some(&platform_value!(1)));
    }

    #[test]
    fn pointer_combined_tilde_escapes() {
        let data = platform_value!({"~/key": 99});
        // ~ is ~0, / is ~1, so "~/key" is encoded as "~0~1key"
        assert_eq!(data.pointer("/~0~1key"), Some(&platform_value!(99)));
    }

    #[test]
    fn pointer_scalar_value_returns_none_for_child() {
        let data = platform_value!(42);
        assert_eq!(data.pointer("/anything"), None);
    }

    #[test]
    fn pointer_nested_array_in_map() {
        let data = platform_value!({
            "x": {
                "y": ["z", "zz"]
            }
        });
        assert_eq!(data.pointer("/x/y/0"), Some(&platform_value!("z")));
        assert_eq!(data.pointer("/x/y/1"), Some(&platform_value!("zz")));
    }

    #[test]
    fn pointer_root_is_array() {
        let data = platform_value!(["a", "b", "c"]);
        assert_eq!(data.pointer("/0"), Some(&platform_value!("a")));
        assert_eq!(data.pointer("/2"), Some(&platform_value!("c")));
    }

    #[test]
    fn pointer_leading_zero_index_rejected() {
        let data = platform_value!({"arr": [10, 20, 30]});
        // "01" has a leading zero (and len > 1), so parse_index returns None
        assert_eq!(data.pointer("/arr/01"), None);
    }

    // ---------------------------------------------------------------
    // pointer_mut() — mutable access
    // ---------------------------------------------------------------

    #[test]
    fn pointer_mut_empty_string_returns_self() {
        let mut data = platform_value!({"a": 1});
        let reference = data.pointer_mut("");
        assert!(reference.is_some());
    }

    #[test]
    fn pointer_mut_no_leading_slash_returns_none() {
        let mut data = platform_value!({"a": 1});
        assert!(data.pointer_mut("a").is_none());
    }

    #[test]
    fn pointer_mut_modify_nested_value() {
        let mut data = platform_value!({"x": {"y": 1}});
        *data.pointer_mut("/x/y").unwrap() = platform_value!(99);
        assert_eq!(data.pointer("/x/y"), Some(&platform_value!(99)));
    }

    #[test]
    fn pointer_mut_modify_array_element() {
        let mut data = platform_value!({"arr": [10, 20, 30]});
        *data.pointer_mut("/arr/1").unwrap() = platform_value!(999);
        assert_eq!(data.pointer("/arr/1"), Some(&platform_value!(999)));
    }

    #[test]
    fn pointer_mut_missing_path_returns_none() {
        let mut data = platform_value!({"a": 1});
        assert!(data.pointer_mut("/nonexistent").is_none());
        assert!(data.pointer_mut("/a/b/c").is_none());
    }

    #[test]
    fn pointer_mut_tilde_escapes() {
        let mut data = platform_value!({"a/b": 1, "c~d": 2});
        *data.pointer_mut("/a~1b").unwrap() = platform_value!(10);
        *data.pointer_mut("/c~0d").unwrap() = platform_value!(20);
        assert_eq!(data.pointer("/a~1b"), Some(&platform_value!(10)));
        assert_eq!(data.pointer("/c~0d"), Some(&platform_value!(20)));
    }

    #[test]
    fn pointer_mut_scalar_returns_none() {
        let mut data = platform_value!("hello");
        assert!(data.pointer_mut("/anything").is_none());
    }

    // ---------------------------------------------------------------
    // take() — replace with Null and return old value
    // ---------------------------------------------------------------

    #[test]
    fn take_replaces_with_null() {
        let mut data = platform_value!({"x": "y"});
        let taken = data.pointer_mut("/x").unwrap().take();
        assert_eq!(taken, platform_value!("y"));
        assert_eq!(data.pointer("/x"), Some(&Value::Null));
    }

    #[test]
    fn take_on_integer() {
        let mut val: Value = platform_value!(42);
        let taken = val.take();
        assert_eq!(taken, platform_value!(42));
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn take_on_null_returns_null() {
        let mut val = Value::Null;
        let taken = val.take();
        assert_eq!(taken, Value::Null);
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn take_on_array() {
        let mut val = platform_value!([1, 2, 3]);
        let taken = val.take();
        assert_eq!(taken, platform_value!([1, 2, 3]));
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn take_nested_via_pointer_mut() {
        let mut data = platform_value!({"a": {"b": "deep"}});
        let taken = data.pointer_mut("/a/b").map(Value::take).unwrap();
        assert_eq!(taken, platform_value!("deep"));
        assert_eq!(data.pointer("/a/b"), Some(&Value::Null));
    }
}
