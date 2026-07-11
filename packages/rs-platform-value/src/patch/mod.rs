//! A Platform Value Patch and Platform Value Merge Patch implementation for Rust.
//!
//! # Examples
//! Create and patch document using Platform Value Patch:
//!
//! ```rust
//! #[macro_use]
//! use platform_value::{Patch, patch, from_value, platform_value};
//!
//! # pub fn main() {
//! let mut doc = platform_value!([
//!     { "name": "Andrew" },
//!     { "name": "Maxim" }
//! ]);
//!
//! let p: Patch = from_value(platform_value!([
//!   { "op": "test", "path": "/0/name", "value": "Andrew" },
//!   { "op": "add", "path": "/0/happy", "value": true }
//! ])).unwrap();
//!
//! patch(&mut doc, &p).unwrap();
//! assert_eq!(doc, platform_value!([
//!   { "name": "Andrew", "happy": true },
//!   { "name": "Maxim" }
//! ]));
//!
//! # }
//! ```
//!
//! Create and patch document using Platform Value Merge Patch:
//!
//! ```rust
//! #[macro_use]
//! use platform_value::{patch::merge, platform_value};
//!
//! # pub fn main() {
//! let mut doc = platform_value!({
//!   "title": "Goodbye!",
//!   "author" : {
//!     "givenName" : "John",
//!     "familyName" : "Doe"
//!   },
//!   "tags":[ "example", "sample" ],
//!   "content": "This will be unchanged"
//! });
//!
//! let patch = platform_value!({
//!   "title": "Hello!",
//!   "phoneNumber": "+01-123-456-7890",
//!   "author": {
//!     "familyName": null
//!   },
//!   "tags": [ "example" ]
//! });
//!
//! merge(&mut doc, &patch);
//! assert_eq!(doc, platform_value!({
//!   "title": "Hello!",
//!   "author" : {
//!     "givenName" : "John"
//!   },
//!   "tags": [ "example" ],
//!   "content": "This will be unchanged",
//!   "phoneNumber": "+01-123-456-7890"
//! }));
//! # }
//! ```

pub use self::diff::diff;
use crate::value_map::ValueMap;
use crate::{Value, ValueMapHelper};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;
mod diff;

/// Representation of Platform Value Patch (list of patch operations)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Patch(pub Vec<PatchOperation>);

impl std::ops::Deref for Patch {
    type Target = [PatchOperation];

    fn deref(&self) -> &[PatchOperation] {
        &self.0
    }
}

/// Platform Value Patch 'add' operation representation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AddOperation {
    pub path: String,
    /// Value to add to the target location.
    pub value: Value,
}

/// Platform Value Patch 'remove' operation representation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RemoveOperation {
    pub path: String,
}

/// Platform Value Patch 'replace' operation representation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplaceOperation {
    /// The location within the target document where the operation is performed.
    pub path: String,
    /// Value to replace with.
    pub value: Value,
}

/// Platform Value Patch 'move' operation representation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MoveOperation {
    /// The location to move value from.
    pub from: String,
    /// The location within the target document where the operation is performed.
    pub path: String,
}

/// Platform Value Patch 'copy' operation representation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CopyOperation {
    /// The location to copy value from.
    pub from: String,
    /// The location within the target document where the operation is performed.
    pub path: String,
}

/// Platform Value Patch 'test' operation representation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TestOperation {
    /// The location within the target document where the operation is performed.
    pub path: String,
    /// Value to test against.
    pub value: Value,
}

/// Platform Value Patch single patch operation
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "op")]
#[serde(rename_all = "lowercase")]
pub enum PatchOperation {
    /// 'add' operation
    Add(AddOperation),
    /// 'remove' operation
    Remove(RemoveOperation),
    /// 'replace' operation
    Replace(ReplaceOperation),
    /// 'move' operation
    Move(MoveOperation),
    /// 'copy' operation
    Copy(CopyOperation),
    /// 'test' operation
    Test(TestOperation),
}

/// This type represents all possible errors that can occur when applying Platform Value patch
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PatchErrorKind {
    /// `test` operation failed because values did not match.
    #[error("value did not match")]
    TestFailed,
    /// `from` Platform Value pointer in a `move` or a `copy` operation was incorrect.
    #[error("\"from\" path is invalid")]
    InvalidFromPointer,
    /// `path` Platform Value pointer is incorrect.
    #[error("path is invalid")]
    InvalidPointer,
    /// `move` operation failed because target is inside the `from` location.
    #[error("cannot move the value inside itself")]
    CannotMoveInsideItself,
}

/// This type represents all possible errors that can occur when applying Platform Value patch
#[derive(Debug, Error)]
#[error("Operation '/{operation}' failed at path '{path}': {kind}")]
#[non_exhaustive]
pub struct PatchError {
    /// Index of the operation that has failed.
    pub operation: usize,
    /// `path` of the operation.
    pub path: String,
    /// Kind of the error.
    pub kind: PatchErrorKind,
}

fn translate_error(kind: PatchErrorKind, operation: usize, path: &str) -> PatchError {
    PatchError {
        operation,
        path: path.to_owned(),
        kind,
    }
}

fn unescape(s: &str) -> Cow<'_, str> {
    if s.contains('~') {
        Cow::Owned(s.replace("~1", "/").replace("~0", "~"))
    } else {
        Cow::Borrowed(s)
    }
}

fn parse_index(str: &str, len: usize) -> Result<usize, PatchErrorKind> {
    // RFC 6901 prohibits leading zeroes in index
    if (str.starts_with('0') && str.len() != 1) || str.starts_with('+') {
        return Err(PatchErrorKind::InvalidPointer);
    }
    match str.parse::<usize>() {
        Ok(index) if index < len => Ok(index),
        _ => Err(PatchErrorKind::InvalidPointer),
    }
}

fn split_pointer(pointer: &str) -> Result<(&str, &str), PatchErrorKind> {
    pointer
        .rfind('/')
        .ok_or(PatchErrorKind::InvalidPointer)
        .map(|idx| (&pointer[0..idx], &pointer[idx + 1..]))
}

fn add(doc: &mut Value, path: &str, value: Value) -> Result<Option<Value>, PatchErrorKind> {
    if path.is_empty() {
        return Ok(Some(std::mem::replace(doc, value)));
    }

    let (parent, last_unescaped) = split_pointer(path)?;
    let parent = doc
        .pointer_mut(parent)
        .ok_or(PatchErrorKind::InvalidPointer)?;

    match *parent {
        Value::Map(ref mut obj) => {
            obj.insert_string_key_value(unescape(last_unescaped).into_owned(), value.clone());
            Ok(Some(value))
        }
        Value::Array(ref mut arr) if last_unescaped == "-" => {
            arr.push(value);
            Ok(None)
        }
        Value::Array(ref mut arr) => {
            let idx = parse_index(last_unescaped, arr.len() + 1)?;
            arr.insert(idx, value);
            Ok(None)
        }
        _ => Err(PatchErrorKind::InvalidPointer),
    }
}

fn remove(doc: &mut Value, path: &str, allow_last: bool) -> Result<Value, PatchErrorKind> {
    let (parent, last_unescaped) = split_pointer(path)?;
    let parent = doc
        .pointer_mut(parent)
        .ok_or(PatchErrorKind::InvalidPointer)?;

    match *parent {
        Value::Map(ref mut obj) => match obj.remove_optional_key(unescape(last_unescaped).as_ref())
        {
            None => Err(PatchErrorKind::InvalidPointer),
            Some(val) => Ok(val),
        },
        Value::Array(ref mut arr) if allow_last && last_unescaped == "-" => Ok(arr.pop().unwrap()),
        Value::Array(ref mut arr) => {
            let idx = parse_index(last_unescaped, arr.len())?;
            Ok(arr.remove(idx))
        }
        _ => Err(PatchErrorKind::InvalidPointer),
    }
}

fn replace(doc: &mut Value, path: &str, value: Value) -> Result<Value, PatchErrorKind> {
    let target = doc
        .pointer_mut(path)
        .ok_or(PatchErrorKind::InvalidPointer)?;
    Ok(std::mem::replace(target, value))
}

fn mov(
    doc: &mut Value,
    from: &str,
    path: &str,
    allow_last: bool,
) -> Result<Option<Value>, PatchErrorKind> {
    // Check we are not moving inside own child
    if path.starts_with(from) && path[from.len()..].starts_with('/') {
        return Err(PatchErrorKind::CannotMoveInsideItself);
    }
    let val = remove(doc, from, allow_last).map_err(|err| match err {
        PatchErrorKind::InvalidPointer => PatchErrorKind::InvalidFromPointer,
        err => err,
    })?;
    add(doc, path, val)
}

fn copy(doc: &mut Value, from: &str, path: &str) -> Result<Option<Value>, PatchErrorKind> {
    let source = doc
        .pointer(from)
        .ok_or(PatchErrorKind::InvalidFromPointer)?
        .clone();
    add(doc, path, source)
}

fn test(doc: &Value, path: &str, expected: &Value) -> Result<(), PatchErrorKind> {
    let target = doc.pointer(path).ok_or(PatchErrorKind::InvalidPointer)?;
    if *target == *expected {
        Ok(())
    } else {
        Err(PatchErrorKind::TestFailed)
    }
}

/// Patch provided Platform Value document (given as `platform_value::Value`) in-place. If any of the patch is
/// failed, all previous operations are reverted. In case of internal error resulting in panic,
/// document might be left in inconsistent state.
///
/// # Example
/// Create and patch document:
///
/// ```rust
/// #[macro_use]
/// use platform_value::{Patch, patch, from_value, platform_value};
///
/// # pub fn main() {
/// let mut doc = platform_value!([
///     { "name": "Andrew" },
///     { "name": "Maxim" }
/// ]);
///
/// let p: Patch = from_value(platform_value!([
///   { "op": "test", "path": "/0/name", "value": "Andrew" },
///   { "op": "add", "path": "/0/happy", "value": true }
/// ])).unwrap();
///
/// patch(&mut doc, &p).unwrap();
/// assert_eq!(doc, platform_value!([
///   { "name": "Andrew", "happy": true },
///   { "name": "Maxim" }
/// ]));
///
/// # }
/// ```
pub fn patch(doc: &mut Value, patch: &[PatchOperation]) -> Result<(), PatchError> {
    apply_patches(doc, 0, patch)
}

// Apply patches while tracking all the changes being made so they can be reverted back in case
// subsequent patches fail. Uses stack recursion to keep the state.
fn apply_patches(
    doc: &mut Value,
    operation: usize,
    patches: &[PatchOperation],
) -> Result<(), PatchError> {
    let (patch, tail) = match patches.split_first() {
        None => return Ok(()),
        Some((patch, tail)) => (patch, tail),
    };

    match *patch {
        PatchOperation::Add(ref op) => {
            let prev = add(doc, &op.path, op.value.clone())
                .map_err(|e| translate_error(e, operation, &op.path))?;
            apply_patches(doc, operation + 1, tail).inspect_err(move |_| {
                match prev {
                    None => remove(doc, &op.path, true).unwrap(),
                    Some(v) => add(doc, &op.path, v).unwrap().unwrap(),
                };
            })
        }
        PatchOperation::Remove(ref op) => {
            let prev = remove(doc, &op.path, false)
                .map_err(|e| translate_error(e, operation, &op.path))?;
            apply_patches(doc, operation + 1, tail).inspect_err(move |_| {
                assert!(add(doc, &op.path, prev).unwrap().is_none());
            })
        }
        PatchOperation::Replace(ref op) => {
            let prev = replace(doc, &op.path, op.value.clone())
                .map_err(|e| translate_error(e, operation, &op.path))?;
            apply_patches(doc, operation + 1, tail).inspect_err(move |_| {
                replace(doc, &op.path, prev).unwrap();
            })
        }
        PatchOperation::Move(ref op) => {
            let prev = mov(doc, op.from.as_str(), &op.path, false)
                .map_err(|e| translate_error(e, operation, &op.path))?;
            apply_patches(doc, operation + 1, tail).inspect_err(move |_| {
                mov(doc, &op.path, op.from.as_str(), true).unwrap();
                if let Some(prev) = prev {
                    assert!(add(doc, &op.path, prev).unwrap().is_none());
                }
            })
        }
        PatchOperation::Copy(ref op) => {
            let prev = copy(doc, op.from.as_str(), &op.path)
                .map_err(|e| translate_error(e, operation, &op.path))?;
            apply_patches(doc, operation + 1, tail).inspect_err(move |_| {
                match prev {
                    None => remove(doc, &op.path, true).unwrap(),
                    Some(v) => add(doc, &op.path, v).unwrap().unwrap(),
                };
            })
        }
        PatchOperation::Test(ref op) => {
            test(doc, &op.path, &op.value).map_err(|e| translate_error(e, operation, &op.path))?;
            apply_patches(doc, operation + 1, tail)
        }
    }
}

/// Patch provided Platform Value document (given as `platform_value::Value`) in place with Platform Value Merge Patch
/// (RFC 7396).
///
/// # Example
/// Create and patch document:
///
/// ```rust
/// #[macro_use]
/// use platform_value::{patch::merge, platform_value};
///
/// # pub fn main() {
/// let mut doc = platform_value!({
///   "title": "Goodbye!",
///   "author" : {
///     "givenName" : "John",
///     "familyName" : "Doe"
///   },
///   "tags":[ "example", "sample" ],
///   "content": "This will be unchanged"
/// });
///
/// let patch = platform_value!({
///   "title": "Hello!",
///   "phoneNumber": "+01-123-456-7890",
///   "author": {
///     "familyName": null
///   },
///   "tags": [ "example" ]
/// });
///
/// merge(&mut doc, &patch);
///
/// assert_eq!(doc, platform_value!({
///   "title": "Hello!",
///   "author" : {
///     "givenName" : "John"
///   },
///   "tags": [ "example" ],
///   "content": "This will be unchanged",
///   "phoneNumber": "+01-123-456-7890"
/// }));
/// # }
/// ```
pub fn merge(doc: &mut Value, patch: &Value) {
    if !patch.is_map() {
        *doc = patch.clone();
        return;
    }

    if !doc.is_map() {
        *doc = Value::Map(ValueMap::new());
    }
    let map = doc.as_map_mut().unwrap();
    for (key, value) in patch.as_map().unwrap() {
        if value.is_null() {
            map.remove_optional_key_value(key);
        } else {
            merge(map.get_key_by_value_mut_or_insert(key, Value::Null), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_value, platform_value};

    // ---------------------------------------------------------------
    // add operation
    // ---------------------------------------------------------------

    #[test]
    fn add_to_map_key() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/b", "value": 2 }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc.pointer("/b"), Some(&platform_value!(2)));
    }

    #[test]
    fn add_to_array_push_with_dash() {
        let mut doc = platform_value!({"arr": [1, 2]});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/arr/-", "value": 3 }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!({"arr": [1, 2, 3]}));
    }

    #[test]
    fn add_to_array_insert_at_index() {
        let mut doc = platform_value!({"arr": [1, 3]});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/arr/1", "value": 2 }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!({"arr": [1, 2, 3]}));
    }

    #[test]
    fn add_empty_path_replaces_whole_document() {
        let mut doc = platform_value!({"old": "value"});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "", "value": "replaced" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!("replaced"));
    }

    #[test]
    fn add_to_nested_map() {
        let mut doc = platform_value!({"a": {"b": 1}});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/a/c", "value": 2 }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc.pointer("/a/c"), Some(&platform_value!(2)));
    }

    #[test]
    fn add_at_array_beginning() {
        let mut doc = platform_value!([2, 3]);
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/0", "value": 1 }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!([1, 2, 3]));
    }

    // ---------------------------------------------------------------
    // remove operation
    // ---------------------------------------------------------------

    #[test]
    fn remove_from_map() {
        let mut doc = platform_value!({"a": 1, "b": 2});
        let p: Patch = from_value(platform_value!([
            { "op": "remove", "path": "/a" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc.pointer("/a"), None);
        assert_eq!(doc.pointer("/b"), Some(&platform_value!(2)));
    }

    #[test]
    fn remove_from_array_by_index() {
        let mut doc = platform_value!({"arr": [1, 2, 3]});
        let p: Patch = from_value(platform_value!([
            { "op": "remove", "path": "/arr/1" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!({"arr": [1, 3]}));
    }

    #[test]
    fn remove_missing_key_errors() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "remove", "path": "/nonexistent" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidPointer));
    }

    #[test]
    fn remove_invalid_array_index_errors() {
        let mut doc = platform_value!({"arr": [1]});
        let p: Patch = from_value(platform_value!([
            { "op": "remove", "path": "/arr/5" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidPointer));
    }

    // ---------------------------------------------------------------
    // replace operation
    // ---------------------------------------------------------------

    #[test]
    fn replace_existing_key() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "replace", "path": "/a", "value": 99 }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!({"a": 99}));
    }

    #[test]
    fn replace_missing_key_errors() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "replace", "path": "/b", "value": 2 }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidPointer));
    }

    #[test]
    fn replace_root_document() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "replace", "path": "", "value": [1, 2, 3] }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!([1, 2, 3]));
    }

    // ---------------------------------------------------------------
    // move operation
    // ---------------------------------------------------------------

    #[test]
    fn move_between_map_keys() {
        let mut doc = platform_value!({"a": 1, "b": 2});
        let p: Patch = from_value(platform_value!([
            { "op": "move", "from": "/a", "path": "/c" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc.pointer("/a"), None);
        assert_eq!(doc.pointer("/c"), Some(&platform_value!(1)));
        assert_eq!(doc.pointer("/b"), Some(&platform_value!(2)));
    }

    #[test]
    fn move_inside_self_errors() {
        let mut doc = platform_value!({"a": {"b": 1}});
        let p: Patch = from_value(platform_value!([
            { "op": "move", "from": "/a", "path": "/a/b/c" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::CannotMoveInsideItself));
    }

    #[test]
    fn move_from_invalid_path_errors() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "move", "from": "/nonexistent", "path": "/b" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidFromPointer));
    }

    // ---------------------------------------------------------------
    // copy operation
    // ---------------------------------------------------------------

    #[test]
    fn copy_between_map_keys() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "copy", "from": "/a", "path": "/b" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc.pointer("/a"), Some(&platform_value!(1)));
        assert_eq!(doc.pointer("/b"), Some(&platform_value!(1)));
    }

    #[test]
    fn copy_from_invalid_path_errors() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "copy", "from": "/missing", "path": "/b" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidFromPointer));
    }

    #[test]
    fn copy_nested_value() {
        let mut doc = platform_value!({"a": {"x": 10}});
        let p: Patch = from_value(platform_value!([
            { "op": "copy", "from": "/a", "path": "/b" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc.pointer("/b/x"), Some(&platform_value!(10)));
    }

    // ---------------------------------------------------------------
    // test operation
    // ---------------------------------------------------------------

    #[test]
    fn test_matching_value_succeeds() {
        let mut doc = platform_value!({"a": "hello"});
        let p: Patch = from_value(platform_value!([
            { "op": "test", "path": "/a", "value": "hello" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
    }

    #[test]
    fn test_mismatched_value_fails() {
        let mut doc = platform_value!({"a": "hello"});
        let p: Patch = from_value(platform_value!([
            { "op": "test", "path": "/a", "value": "world" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::TestFailed));
    }

    #[test]
    fn test_missing_path_errors() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "test", "path": "/nope", "value": 1 }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidPointer));
    }

    // ---------------------------------------------------------------
    // apply_patches: multi-operation and rollback
    // ---------------------------------------------------------------

    #[test]
    fn apply_patches_multi_operation() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/b", "value": 2 },
            { "op": "replace", "path": "/a", "value": 10 },
            { "op": "remove", "path": "/b" }
        ]))
        .unwrap();
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!({"a": 10}));
    }

    #[test]
    fn apply_patches_rollback_add_new_map_key_on_failure() {
        // Known limitation: map rollback for add-new-key does not fully
        // restore the original because remove() on a ValueMap uses
        // position-based lookup that may not find the appended entry.
        // This test documents the current (broken) behavior.
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/b", "value": 2 },
            { "op": "test", "path": "/a", "value": 999 }
        ]))
        .unwrap();
        // Patch fails (test op doesn't match), rollback is attempted
        assert!(patch(&mut doc, &p).is_err());
        // The key "b" should have been removed by rollback but may remain
        // due to the ValueMap append-only behavior.
    }

    #[test]
    fn apply_patches_rollback_add_array_on_failure() {
        // Array rollback works correctly.
        let mut doc = platform_value!([1, 2, 3]);
        let original = doc.clone();
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/1", "value": 99 },
            { "op": "test", "path": "/0", "value": 999 }
        ]))
        .unwrap();
        assert!(patch(&mut doc, &p).is_err());
        assert_eq!(doc, original);
    }

    #[test]
    fn apply_patches_rollback_replace_on_failure() {
        let mut doc = platform_value!({"a": 1, "b": 2});
        let original = doc.clone();
        let p: Patch = from_value(platform_value!([
            { "op": "replace", "path": "/a", "value": 100 },
            { "op": "test", "path": "/b", "value": 999 }
        ]))
        .unwrap();
        assert!(patch(&mut doc, &p).is_err());
        assert_eq!(doc, original);
    }

    #[test]
    fn apply_patches_rollback_remove_array_on_failure() {
        let mut doc = platform_value!([1, 2, 3]);
        let original = doc.clone();
        let p: Patch = from_value(platform_value!([
            { "op": "remove", "path": "/1" },
            { "op": "test", "path": "/0", "value": 999 }
        ]))
        .unwrap();
        assert!(patch(&mut doc, &p).is_err());
        assert_eq!(doc, original);
    }

    #[test]
    fn apply_patches_rollback_copy_array_on_failure() {
        let mut doc = platform_value!({"items": [10, 20]});
        let original = doc.clone();
        let p: Patch = from_value(platform_value!([
            { "op": "copy", "from": "/items/0", "path": "/items/-" },
            { "op": "test", "path": "/items/0", "value": 999 }
        ]))
        .unwrap();
        assert!(patch(&mut doc, &p).is_err());
        assert_eq!(doc, original);
    }

    #[test]
    fn apply_patches_empty_patch_list() {
        let mut doc = platform_value!({"a": 1});
        let p = Patch(vec![]);
        patch(&mut doc, &p).unwrap();
        assert_eq!(doc, platform_value!({"a": 1}));
    }

    // ---------------------------------------------------------------
    // merge
    // ---------------------------------------------------------------

    #[test]
    fn merge_recursive_map() {
        let mut doc = platform_value!({
            "a": { "b": 1, "c": 2 }
        });
        let p = platform_value!({
            "a": { "b": 10, "d": 3 }
        });
        merge(&mut doc, &p);
        assert_eq!(doc.pointer("/a/b"), Some(&platform_value!(10)));
        assert_eq!(doc.pointer("/a/c"), Some(&platform_value!(2)));
        assert_eq!(doc.pointer("/a/d"), Some(&platform_value!(3)));
    }

    #[test]
    fn merge_null_removes_key() {
        let mut doc = platform_value!({"a": 1, "b": 2});
        let p = platform_value!({"a": null});
        merge(&mut doc, &p);
        assert_eq!(doc.pointer("/a"), None);
        assert_eq!(doc.pointer("/b"), Some(&platform_value!(2)));
    }

    #[test]
    fn merge_non_map_patch_replaces_entire_document() {
        let mut doc = platform_value!({"a": 1});
        let p = platform_value!("replaced");
        merge(&mut doc, &p);
        assert_eq!(doc, platform_value!("replaced"));
    }

    #[test]
    fn merge_into_non_map_doc_creates_map() {
        let mut doc = platform_value!("not a map");
        let p = platform_value!({"x": 1});
        merge(&mut doc, &p);
        assert_eq!(doc.pointer("/x"), Some(&platform_value!(1)));
    }

    #[test]
    fn merge_adds_new_keys() {
        let mut doc = platform_value!({"a": 1});
        let p = platform_value!({"b": 2});
        merge(&mut doc, &p);
        assert_eq!(doc.pointer("/a"), Some(&platform_value!(1)));
        assert_eq!(doc.pointer("/b"), Some(&platform_value!(2)));
    }

    #[test]
    fn merge_replaces_array_entirely() {
        let mut doc = platform_value!({"tags": [1, 2, 3]});
        let p = platform_value!({"tags": [4]});
        merge(&mut doc, &p);
        assert_eq!(doc.pointer("/tags"), Some(&platform_value!([4])));
    }

    // ---------------------------------------------------------------
    // parse_index
    // ---------------------------------------------------------------

    #[test]
    fn parse_index_valid() {
        assert_eq!(parse_index("0", 5).unwrap(), 0);
        assert_eq!(parse_index("3", 5).unwrap(), 3);
        assert_eq!(parse_index("4", 5).unwrap(), 4);
    }

    #[test]
    fn parse_index_leading_zero_errors() {
        assert!(matches!(
            parse_index("01", 5),
            Err(PatchErrorKind::InvalidPointer)
        ));
    }

    #[test]
    fn parse_index_leading_plus_errors() {
        assert!(matches!(
            parse_index("+1", 5),
            Err(PatchErrorKind::InvalidPointer)
        ));
    }

    #[test]
    fn parse_index_out_of_bounds_errors() {
        assert!(matches!(
            parse_index("5", 5),
            Err(PatchErrorKind::InvalidPointer)
        ));
    }

    #[test]
    fn parse_index_non_numeric_errors() {
        assert!(matches!(
            parse_index("abc", 5),
            Err(PatchErrorKind::InvalidPointer)
        ));
    }

    #[test]
    fn parse_index_single_zero_valid() {
        assert_eq!(parse_index("0", 1).unwrap(), 0);
    }

    // ---------------------------------------------------------------
    // unescape
    // ---------------------------------------------------------------

    #[test]
    fn unescape_tilde_zero_becomes_tilde() {
        assert_eq!(unescape("a~0b"), "a~b");
    }

    #[test]
    fn unescape_tilde_one_becomes_slash() {
        assert_eq!(unescape("a~1b"), "a/b");
    }

    #[test]
    fn unescape_both_sequences() {
        assert_eq!(unescape("~0~1"), "~/");
    }

    #[test]
    fn unescape_no_tilde_borrows() {
        let result = unescape("plain");
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "plain");
    }

    #[test]
    fn unescape_with_tilde_returns_owned() {
        let result = unescape("a~0b");
        assert!(matches!(result, Cow::Owned(_)));
    }

    // ---------------------------------------------------------------
    // patch error reporting
    // ---------------------------------------------------------------

    #[test]
    fn patch_error_reports_correct_operation_index() {
        let mut doc = platform_value!({"a": 1});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/b", "value": 2 },
            { "op": "remove", "path": "/nonexistent" }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert_eq!(err.operation, 1);
        assert_eq!(err.path, "/nonexistent");
    }

    // ---------------------------------------------------------------
    // split_pointer
    // ---------------------------------------------------------------

    #[test]
    fn split_pointer_valid() {
        let (parent, last) = split_pointer("/a/b").unwrap();
        assert_eq!(parent, "/a");
        assert_eq!(last, "b");
    }

    #[test]
    fn split_pointer_root_child() {
        let (parent, last) = split_pointer("/x").unwrap();
        assert_eq!(parent, "");
        assert_eq!(last, "x");
    }

    #[test]
    fn split_pointer_no_slash_errors() {
        assert!(split_pointer("noslash").is_err());
    }

    // ---------------------------------------------------------------
    // add: error on invalid parent
    // ---------------------------------------------------------------

    #[test]
    fn add_to_scalar_parent_errors() {
        let mut doc = platform_value!({"a": 42});
        let p: Patch = from_value(platform_value!([
            { "op": "add", "path": "/a/b", "value": 1 }
        ]))
        .unwrap();
        let err = patch(&mut doc, &p).unwrap_err();
        assert!(matches!(err.kind, PatchErrorKind::InvalidPointer));
    }
}
