use crate::Value;
use std::fmt;
use std::fmt::Display;

struct PatchDiffer {
    path: String,
    patch: super::Patch,
    shift: usize,
}

impl PatchDiffer {
    fn new() -> Self {
        Self {
            path: "/".to_string(),
            patch: super::Patch(Vec::new()),
            shift: 0,
        }
    }
}

impl<'a> treediff::Delegate<'a, PlatformItemKey, Value> for PatchDiffer {
    fn push(&mut self, key: &PlatformItemKey) {
        use std::fmt::Write;
        if self.path.len() != 1 {
            self.path.push('/');
        }
        match key {
            PlatformItemKey::Index(idx) => write!(self.path, "{}", *idx).unwrap(),
            PlatformItemKey::String(ref key) => append_path(&mut self.path, key),
            PlatformItemKey::BigSignedIndex(idx) => write!(self.path, "{}", *idx).unwrap(),
            PlatformItemKey::BigIndex(idx) => write!(self.path, "{}", *idx).unwrap(),
            PlatformItemKey::SignedIndex(idx) => write!(self.path, "{}", *idx).unwrap(),
            PlatformItemKey::Bytes(bytes) => write!(self.path, "{}", hex::encode(bytes)).unwrap(),
            PlatformItemKey::ArrayIndex(idx) => write!(self.path, "{}", *idx - self.shift).unwrap(),
        }
    }

    fn pop(&mut self) {
        let mut pos = self.path.rfind('/').unwrap_or(0);
        if pos == 0 {
            pos = 1;
        }
        self.path.truncate(pos);
        self.shift = 0;
    }

    fn removed<'b>(&mut self, k: &'b PlatformItemKey, _v: &'a Value) {
        let len = self.path.len();
        self.push(k);
        self.patch
            .0
            .push(super::PatchOperation::Remove(super::RemoveOperation {
                path: self.path.clone(),
            }));
        // Shift indices, we are deleting array elements
        if let PlatformItemKey::ArrayIndex(_) = k {
            self.shift += 1;
        }
        self.path.truncate(len);
    }

    fn added(&mut self, k: &PlatformItemKey, v: &Value) {
        let len = self.path.len();
        self.push(k);
        self.patch
            .0
            .push(super::PatchOperation::Add(super::AddOperation {
                path: self.path.clone(),
                value: v.clone(),
            }));
        self.path.truncate(len);
    }

    fn modified(&mut self, _old: &'a Value, new: &'a Value) {
        self.patch
            .0
            .push(super::PatchOperation::Replace(super::ReplaceOperation {
                path: self.path.clone(),
                value: new.clone(),
            }));
    }
}

fn append_path(path: &mut String, key: &str) {
    path.reserve(key.len());
    for ch in key.chars() {
        if ch == '~' {
            *path += "~0";
        } else if ch == '/' {
            *path += "~1";
        } else {
            path.push(ch);
        }
    }
}

/// Diff two Platform Value documents and generate a Platform Value Patch (RFC 6902).
///
/// # Example
/// Diff two JSONs:
///
/// ```rust
/// #[macro_use]
///
/// use platform_value::{from_value, patch, platform_value};
///
/// # pub fn main() {
/// use platform_value::patch::diff;
/// let left = platform_value!({
///   "title": "Goodbye!",
///   "author" : {
///     "givenName" : "John",
///     "familyName" : "Doe"
///   },
///   "tags":[ "example", "sample" ],
///   "content": "This will be unchanged"
/// });
///
/// let right = platform_value!({
///   "title": "Hello!",
///   "author" : {
///     "givenName" : "John"
///   },
///   "tags": [ "example" ],
///   "content": "This will be unchanged",
///   "phoneNumber": "+01-123-456-7890"
/// });
///
/// let p = diff(&left, &right);
/// assert_eq!(p, from_value(platform_value!([
///   { "op": "remove", "path": "/author/familyName" },
///   { "op": "remove", "path": "/tags/1" },
///   { "op": "replace", "path": "/title", "value": "Hello!" },
///   { "op": "add", "path": "/phoneNumber", "value": "+01-123-456-7890" },
/// ])).unwrap());
///
/// let mut doc = left.clone();
/// patch(&mut doc, &p).unwrap();
/// assert_eq!(doc, right);
///
/// # }
/// ```
pub fn diff(left: &Value, right: &Value) -> super::Patch {
    let mut differ = PatchDiffer::new();
    treediff::diff(left, right, &mut differ);
    differ.patch
}

/// A representation of all key types typical Value types will assume.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum PlatformItemKey {
    /// A big index
    BigSignedIndex(i128),
    /// A big index
    BigIndex(u128),
    /// An array index
    SignedIndex(i64),
    /// An array index
    Index(u64),
    /// An array index
    ArrayIndex(usize),
    /// Bytes
    Bytes(Vec<u8>),
    /// A string index for mappings
    String(String),
}

impl Display for PlatformItemKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PlatformItemKey::String(ref v) => v.fmt(f),
            PlatformItemKey::Index(ref v) => v.fmt(f),
            PlatformItemKey::BigSignedIndex(ref v) => v.fmt(f),
            PlatformItemKey::BigIndex(ref v) => v.fmt(f),
            PlatformItemKey::SignedIndex(ref v) => v.fmt(f),
            PlatformItemKey::Bytes(ref v) => hex::encode(v).fmt(f),
            PlatformItemKey::ArrayIndex(ref v) => v.fmt(f),
        }
    }
}

impl From<Value> for Option<PlatformItemKey> {
    fn from(value: Value) -> Self {
        match value {
            Value::U128(i) => Some(PlatformItemKey::BigIndex(i)),
            Value::I128(i) => Some(PlatformItemKey::BigSignedIndex(i)),
            Value::U64(i) => Some(PlatformItemKey::Index(i)),
            Value::I64(i) => Some(PlatformItemKey::SignedIndex(i)),
            Value::U32(i) => Some(PlatformItemKey::Index(i as u64)),
            Value::I32(i) => Some(PlatformItemKey::SignedIndex(i as i64)),
            Value::U16(i) => Some(PlatformItemKey::Index(i as u64)),
            Value::I16(i) => Some(PlatformItemKey::SignedIndex(i as i64)),
            Value::U8(i) => Some(PlatformItemKey::Index(i as u64)),
            Value::I8(i) => Some(PlatformItemKey::SignedIndex(i as i64)),
            Value::Bytes(bytes) => Some(PlatformItemKey::Bytes(bytes)),
            Value::Bytes20(bytes) => Some(PlatformItemKey::Bytes(bytes.into())),
            Value::Bytes32(bytes) => Some(PlatformItemKey::Bytes(bytes.into())),
            Value::Bytes36(bytes) => Some(PlatformItemKey::Bytes(bytes.into())),
            Value::EnumU8(_) => None,
            Value::EnumString(_) => None,
            Value::Identifier(bytes) => Some(PlatformItemKey::Bytes(bytes.into())),
            Value::Float(_) => None,
            Value::Text(str) => Some(PlatformItemKey::String(str)),
            Value::Bool(_) => None,
            Value::Null => None,
            Value::Array(_) => None,
            Value::Map(_) => None,
        }
    }
}

impl treediff::Value for Value {
    type Key = PlatformItemKey;
    /// The Value type itself.
    type Item = Value;
    /// Returns `None` if this is a scalar value, and an iterator yielding (Key, Value) pairs
    /// otherwise. It is entirely possible for it to yield no values though.
    #[allow(clippy::type_complexity)]
    fn items<'a>(&'a self) -> Option<Box<dyn Iterator<Item = (Self::Key, &'a Self::Item)> + 'a>> {
        match *self {
            Value::Array(ref inner) => Some(Box::new(
                inner
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (PlatformItemKey::ArrayIndex(i), v)),
            )),
            Value::Map(ref inner) => Some(Box::new(inner.iter().filter_map(|(s, v)| {
                let key: Option<PlatformItemKey> = s.clone().into();
                key.map(|k| (k, v))
            }))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{from_value, platform_value, Value};

    #[test]
    pub fn replace_all() {
        let left = platform_value!({"title": "Hello!"});
        let p = super::diff(&left, &Value::Null);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/", "value": null },
            ]))
            .unwrap()
        );
    }

    #[test]
    pub fn add_all() {
        let right = platform_value!({"title": "Hello!"});
        let p = super::diff(&Value::Null, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/", "value": { "title": "Hello!" } },
            ]))
            .unwrap()
        );
    }

    #[test]
    pub fn remove_all() {
        let left = platform_value!(["hello", "bye"]);
        let right = platform_value!([]);
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "remove", "path": "/0" },
                { "op": "remove", "path": "/0" },
            ]))
            .unwrap()
        );
    }

    #[test]
    pub fn remove_tail() {
        let left = platform_value!(["hello", "bye", "hi"]);
        let right = platform_value!(["hello"]);
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "remove", "path": "/1" },
                { "op": "remove", "path": "/1" },
            ]))
            .unwrap()
        );
    }
    #[test]
    pub fn replace_object() {
        let left = platform_value!(["hello", "bye"]);
        let right = platform_value!({"hello": "bye"});
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "add", "path": "/hello", "value": "bye" },
                { "op": "remove", "path": "/0" },
                { "op": "remove", "path": "/0" },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn escape_json_keys() {
        let mut left = platform_value!({
            "/slashed/path": 1
        });
        let right = platform_value!({
            "/slashed/path": 2,
        });
        let patch = super::diff(&left, &right);

        eprintln!("{:?}", patch);

        crate::patch(&mut left, &patch).unwrap();
        assert_eq!(left, right);
    }

    // ---------------------------------------------------------------
    // append_path: tilde and slash escaping
    // ---------------------------------------------------------------

    #[test]
    fn append_path_escapes_tilde() {
        let mut path = String::from("/");
        super::append_path(&mut path, "a~b");
        assert_eq!(path, "/a~0b");
    }

    #[test]
    fn append_path_escapes_slash() {
        let mut path = String::from("/");
        super::append_path(&mut path, "a/b");
        assert_eq!(path, "/a~1b");
    }

    #[test]
    fn append_path_escapes_both() {
        let mut path = String::from("/");
        super::append_path(&mut path, "~/");
        assert_eq!(path, "/~0~1");
    }

    #[test]
    fn append_path_no_escaping_needed() {
        let mut path = String::from("/");
        super::append_path(&mut path, "plain");
        assert_eq!(path, "/plain");
    }

    #[test]
    fn append_path_empty_key() {
        let mut path = String::from("/");
        super::append_path(&mut path, "");
        assert_eq!(path, "/");
    }

    // ---------------------------------------------------------------
    // diff: identical values returns empty patch
    // ---------------------------------------------------------------

    #[test]
    fn diff_identical_scalars_returns_empty() {
        let v = platform_value!("hello");
        let p = super::diff(&v, &v);
        assert!(p.0.is_empty());
    }

    #[test]
    fn diff_identical_maps_returns_empty() {
        let v = platform_value!({"a": 1, "b": 2});
        let p = super::diff(&v, &v);
        assert!(p.0.is_empty());
    }

    #[test]
    fn diff_identical_arrays_returns_empty() {
        let v = platform_value!([1, 2, 3]);
        let p = super::diff(&v, &v);
        assert!(p.0.is_empty());
    }

    #[test]
    fn diff_identical_nested_returns_empty() {
        let v = platform_value!({"a": {"b": [1, 2]}});
        let p = super::diff(&v, &v);
        assert!(p.0.is_empty());
    }

    // ---------------------------------------------------------------
    // diff: maps with added/removed/modified keys
    // ---------------------------------------------------------------

    #[test]
    fn diff_map_added_key() {
        let left = platform_value!({"a": 1});
        let right = platform_value!({"a": 1, "b": 2});
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "add", "path": "/b", "value": 2 },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn diff_map_removed_key() {
        let left = platform_value!({"a": 1, "b": 2});
        let right = platform_value!({"a": 1});
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "remove", "path": "/b" },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn diff_map_modified_key() {
        let left = platform_value!({"a": 1});
        let right = platform_value!({"a": 2});
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/a", "value": 2 },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn diff_map_added_removed_modified() {
        let left = platform_value!({"a": 1, "b": 2});
        let right = platform_value!({"a": 10, "c": 3});
        let mut p_left = left.clone();
        let patch = super::diff(&left, &right);
        crate::patch(&mut p_left, &patch).unwrap();
        assert_eq!(p_left, right);
    }

    // ---------------------------------------------------------------
    // diff: arrays with insertions/deletions
    // ---------------------------------------------------------------

    #[test]
    fn diff_array_append() {
        let left = platform_value!([1, 2]);
        let right = platform_value!([1, 2, 3]);
        let patch = super::diff(&left, &right);
        let mut doc = left.clone();
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }

    #[test]
    fn diff_array_removal_from_middle() {
        let left = platform_value!([1, 2, 3]);
        let right = platform_value!([1, 3]);
        let patch = super::diff(&left, &right);
        let mut doc = left.clone();
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }

    #[test]
    fn diff_array_complete_replacement() {
        let left = platform_value!([1, 2, 3]);
        let right = platform_value!([4, 5]);
        let patch = super::diff(&left, &right);
        let mut doc = left.clone();
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }

    // ---------------------------------------------------------------
    // diff: nested maps
    // ---------------------------------------------------------------

    #[test]
    fn diff_nested_map_modification() {
        let left = platform_value!({
            "outer": {
                "inner": 1,
                "keep": "same"
            }
        });
        let right = platform_value!({
            "outer": {
                "inner": 99,
                "keep": "same"
            }
        });
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/outer/inner", "value": 99 },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn diff_nested_map_add_and_remove() {
        let left = platform_value!({
            "a": {
                "b": 1,
                "c": 2
            }
        });
        let right = platform_value!({
            "a": {
                "c": 2,
                "d": 3
            }
        });
        let patch = super::diff(&left, &right);
        let mut doc = left.clone();
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }

    #[test]
    fn diff_deeply_nested() {
        let left = platform_value!({
            "l1": {
                "l2": {
                    "l3": "old"
                }
            }
        });
        let right = platform_value!({
            "l1": {
                "l2": {
                    "l3": "new"
                }
            }
        });
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/l1/l2/l3", "value": "new" },
            ]))
            .unwrap()
        );
    }

    // ---------------------------------------------------------------
    // diff: tilde-escaped keys round-trip through patch
    // ---------------------------------------------------------------

    #[test]
    fn diff_tilde_key_round_trip() {
        let left = platform_value!({
            "a~b": 1
        });
        let right = platform_value!({
            "a~b": 2
        });
        let mut doc = left.clone();
        let patch = super::diff(&left, &right);
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }

    #[test]
    fn diff_slash_key_round_trip() {
        let left = platform_value!({
            "x/y": 10
        });
        let right = platform_value!({
            "x/y": 20
        });
        let mut doc = left.clone();
        let patch = super::diff(&left, &right);
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }

    // ---------------------------------------------------------------
    // diff: null values
    // ---------------------------------------------------------------

    #[test]
    fn diff_both_null_returns_empty() {
        let p = super::diff(&Value::Null, &Value::Null);
        assert!(p.0.is_empty());
    }

    #[test]
    fn diff_null_to_value() {
        let left = Value::Null;
        let right = platform_value!(42);
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/", "value": 42 },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn diff_value_to_null() {
        let left = platform_value!(42);
        let right = Value::Null;
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/", "value": null },
            ]))
            .unwrap()
        );
    }

    // ---------------------------------------------------------------
    // diff: mixed types
    // ---------------------------------------------------------------

    #[test]
    fn diff_scalar_to_map() {
        let left = platform_value!(42);
        let right = platform_value!({"a": 1});
        let p = super::diff(&left, &right);
        assert_eq!(
            p,
            from_value(platform_value!([
                { "op": "replace", "path": "/", "value": {"a": 1} },
            ]))
            .unwrap()
        );
    }

    #[test]
    fn diff_array_with_nested_maps() {
        let left = platform_value!([{"name": "alice"}, {"name": "bob"}]);
        let right = platform_value!([{"name": "alice"}, {"name": "charlie"}]);
        let patch = super::diff(&left, &right);
        let mut doc = left.clone();
        crate::patch(&mut doc, &patch).unwrap();
        assert_eq!(doc, right);
    }
}
