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
