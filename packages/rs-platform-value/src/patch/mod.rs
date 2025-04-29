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

fn unescape(s: &str) -> Cow<str> {
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
