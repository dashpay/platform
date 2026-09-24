//! Stable identities for collections, properties, indexes, methods, modules,
//! interfaces and rules.
//!
//! A declaration is identified by its declared name, never by the Rust path,
//! impl block, source module or declaration order that produced it. The
//! grammars here are the ones the manifest enforces; a name that fails its
//! grammar is reported by the validator as an `InvalidName` diagnostic.
//!
//! | Identity | Grammar | Source of the rule |
//! |---|---|---|
//! | [`CollectionName`] | `^[a-zA-Z0-9_-]{1,64}$` | native document type name rule |
//! | [`PropertyName`] | `^[a-zA-Z0-9_-]{1,64}$` | document meta-schema property names |
//! | [`PropertyPath`] | dotted property names, at most 256 bytes, or a system property | index property name limit |
//! | [`IndexName`] | 1 to 32 characters | document meta-schema index name |
//! | [`MethodName`] | `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`, at most 64 bytes | provisional SDK rule |
//! | [`ModuleName`] | `^[a-z0-9_]{1,64}$` | provisional, aligned with the bundle validation crate |
//! | [`InterfaceName`] | `^[a-z0-9_]{1,64}$` | provisional SDK rule |
//! | [`RuleName`] | `^[a-z][a-z0-9_]{0,63}$` | provisional SDK rule |
//!
//! The method, module, interface and rule grammars are provisional values under
//! the shared allocation register (Rust macro grammar); the numeric method and
//! type identifiers derived from these names are allocated by the ABI work.

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;

/// Maximum byte length of a collection or property name (native rule).
pub const MAX_NAME_BYTES: usize = 64;
/// Maximum byte length of an index property path (native meta-schema rule).
pub const MAX_PROPERTY_PATH_BYTES: usize = 256;
/// Maximum character count of an index name (native meta-schema rule).
pub const MAX_INDEX_NAME_CHARS: usize = 32;
/// Maximum byte length of a method name. Provisional.
pub const MAX_METHOD_NAME_BYTES: usize = 64;
/// Maximum byte length of a module or interface name. Provisional, aligned
/// with the bundle validation crate's module name bound.
pub const MAX_MODULE_NAME_BYTES: usize = 64;
/// Maximum byte length of a rule name. Provisional.
pub const MAX_RULE_NAME_BYTES: usize = 64;

/// Prefix of every entry export symbol. Provisional.
pub const ENTRY_EXPORT_PREFIX: &str = "dash_entry_";

/// Document system properties that an index may name without the collection
/// declaring them. Whether a given system property is admissible for a given
/// document type (for example `$creatorId` needs a transferable type) is a
/// native rule and is not repeated here.
pub const SYSTEM_PROPERTIES: &[&str] = &[
    "$id",
    "$ownerId",
    "$creatorId",
    "$createdAt",
    "$updatedAt",
    "$transferredAt",
    "$createdAtBlockHeight",
    "$updatedAtBlockHeight",
    "$transferredAtBlockHeight",
    "$createdAtCoreBlockHeight",
    "$updatedAtCoreBlockHeight",
    "$transferredAtCoreBlockHeight",
];

/// Returns whether `name` is one of the document system properties.
pub fn is_system_property(name: &str) -> bool {
    SYSTEM_PROPERTIES.contains(&name)
}

/// Which identity a name was checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NameKind {
    /// A collection (native document type) name.
    Collection,
    /// A single property name.
    Property,
    /// A dotted property path or system property used by an index.
    PropertyPath,
    /// An index name.
    Index,
    /// A method (entry) name.
    Method,
    /// A WASM module name.
    Module,
    /// An interface name.
    Interface,
    /// A rule name.
    Rule,
}

impl fmt::Display for NameKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            NameKind::Collection => "collection",
            NameKind::Property => "property",
            NameKind::PropertyPath => "property path",
            NameKind::Index => "index",
            NameKind::Method => "method",
            NameKind::Module => "module",
            NameKind::Interface => "interface",
            NameKind::Rule => "rule",
        };
        f.write_str(text)
    }
}

/// A name that does not satisfy its identity grammar.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid {kind} name {name:?}: {reason}")]
pub struct InvalidName {
    /// The identity the name was checked against.
    pub kind: NameKind,
    /// The offending name, verbatim.
    pub name: String,
    /// Why the grammar rejected it.
    pub reason: String,
}

impl InvalidName {
    fn new(kind: NameKind, name: &str, reason: impl Into<String>) -> Self {
        InvalidName {
            kind,
            name: name.to_string(),
            reason: reason.into(),
        }
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn check_native_name(kind: NameKind, name: &str) -> Result<(), InvalidName> {
    if name.is_empty() {
        return Err(InvalidName::new(kind, name, "must not be empty"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(InvalidName::new(
            kind,
            name,
            format!("longer than {MAX_NAME_BYTES} bytes"),
        ));
    }
    if !name.chars().all(is_name_char) {
        return Err(InvalidName::new(
            kind,
            name,
            "only ASCII letters, digits, `_` and `-` are allowed",
        ));
    }
    Ok(())
}

fn check_lower_segment(kind: NameKind, name: &str, segment: &str) -> Result<(), InvalidName> {
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => {
            return Err(InvalidName::new(
                kind,
                name,
                "each segment must start with a lowercase ASCII letter",
            ))
        }
    }
    if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(InvalidName::new(
            kind,
            name,
            "only lowercase ASCII letters, digits and `_` are allowed after the first character",
        ));
    }
    Ok(())
}

fn check_module_style_name(kind: NameKind, name: &str) -> Result<(), InvalidName> {
    if name.is_empty() {
        return Err(InvalidName::new(kind, name, "must not be empty"));
    }
    if name.len() > MAX_MODULE_NAME_BYTES {
        return Err(InvalidName::new(
            kind,
            name,
            format!("longer than {MAX_MODULE_NAME_BYTES} bytes"),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(InvalidName::new(
            kind,
            name,
            "only lowercase ASCII letters, digits and `_` are allowed",
        ));
    }
    Ok(())
}

macro_rules! name_newtype {
    ($(#[$meta:meta])* $name:ident, $kind:expr, $check:expr) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Checks `name` against the grammar and wraps it.
            pub fn new(name: impl AsRef<str>) -> Result<Self, InvalidName> {
                let name = name.as_ref();
                $check($kind, name)?;
                Ok($name(name.to_string()))
            }

            /// The name as declared.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = InvalidName;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                $name::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidName;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $name::new(value)
            }
        }
    };
}

name_newtype!(
    /// The identity of a collection: its declared name, which is also the native
    /// document type name. Grammar `^[a-zA-Z0-9_-]{1,64}$`.
    CollectionName,
    NameKind::Collection,
    check_native_name
);

name_newtype!(
    /// The identity of one property at one nesting level. Grammar
    /// `^[a-zA-Z0-9_-]{1,64}$`. Together with its collection and position it
    /// identifies a stored field.
    PropertyName,
    NameKind::Property,
    check_native_name
);

name_newtype!(
    /// A dotted path of [`PropertyName`]s (`profile.age`), or one of the
    /// [`SYSTEM_PROPERTIES`], as used by index definitions. At most 256 bytes.
    PropertyPath,
    NameKind::PropertyPath,
    check_property_path
);

name_newtype!(
    /// The identity of an index within its collection: 1 to 32 characters, any
    /// UTF-8.
    IndexName,
    NameKind::Index,
    check_index_name
);

name_newtype!(
    /// The identity of an entry: unique across the whole contract regardless of
    /// which module hosts it. Grammar `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`,
    /// at most 64 bytes, for example `score.add`. Provisional.
    MethodName,
    NameKind::Method,
    check_method_name
);

name_newtype!(
    /// The name of one WASM module in a contract bundle. Grammar
    /// `^[a-z0-9_]{1,64}$`. Provisional.
    ModuleName,
    NameKind::Module,
    check_module_style_name
);

name_newtype!(
    /// The name of an interface a module provides to other modules. Grammar
    /// `^[a-z0-9_]{1,64}$`. Provisional.
    InterfaceName,
    NameKind::Interface,
    check_module_style_name
);

name_newtype!(
    /// The name of a rule, unique within its collection. Grammar
    /// `^[a-z][a-z0-9_]{0,63}$`. Provisional.
    RuleName,
    NameKind::Rule,
    check_rule_name
);

fn check_property_path(kind: NameKind, path: &str) -> Result<(), InvalidName> {
    if is_system_property(path) {
        return Ok(());
    }
    if path.is_empty() {
        return Err(InvalidName::new(kind, path, "must not be empty"));
    }
    if path.len() > MAX_PROPERTY_PATH_BYTES {
        return Err(InvalidName::new(
            kind,
            path,
            format!("longer than {MAX_PROPERTY_PATH_BYTES} bytes"),
        ));
    }
    for segment in path.split('.') {
        check_native_name(NameKind::Property, segment).map_err(|error| {
            InvalidName::new(
                kind,
                path,
                format!("segment {:?}: {}", segment, error.reason),
            )
        })?;
    }
    Ok(())
}

fn check_index_name(kind: NameKind, name: &str) -> Result<(), InvalidName> {
    let chars = name.chars().count();
    if chars == 0 {
        return Err(InvalidName::new(kind, name, "must not be empty"));
    }
    if chars > MAX_INDEX_NAME_CHARS {
        return Err(InvalidName::new(
            kind,
            name,
            format!("longer than {MAX_INDEX_NAME_CHARS} characters"),
        ));
    }
    Ok(())
}

fn check_method_name(kind: NameKind, name: &str) -> Result<(), InvalidName> {
    if name.is_empty() {
        return Err(InvalidName::new(kind, name, "must not be empty"));
    }
    if name.len() > MAX_METHOD_NAME_BYTES {
        return Err(InvalidName::new(
            kind,
            name,
            format!("longer than {MAX_METHOD_NAME_BYTES} bytes"),
        ));
    }
    for segment in name.split('.') {
        check_lower_segment(kind, name, segment)?;
    }
    Ok(())
}

fn check_rule_name(kind: NameKind, name: &str) -> Result<(), InvalidName> {
    if name.is_empty() {
        return Err(InvalidName::new(kind, name, "must not be empty"));
    }
    if name.len() > MAX_RULE_NAME_BYTES {
        return Err(InvalidName::new(
            kind,
            name,
            format!("longer than {MAX_RULE_NAME_BYTES} bytes"),
        ));
    }
    if name.contains('.') {
        return Err(InvalidName::new(kind, name, "`.` is not allowed"));
    }
    check_lower_segment(kind, name, name)
}

impl PropertyPath {
    /// Returns whether the path names a document system property.
    pub fn is_system(&self) -> bool {
        is_system_property(&self.0)
    }

    /// The path's segments; a system property is a single segment.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }
}

/// The WASM export symbol of an entry: the [`ENTRY_EXPORT_PREFIX`] followed by
/// the method name verbatim, so `score.add` exports as `dash_entry_score.add`.
///
/// The mapping is a prefix plus the identity, which makes it injective by
/// construction (`a.b` and `a_b` stay distinct). WebAssembly export names are
/// arbitrary UTF-8 and Rust's `#[export_name]` accepts dots. Provisional.
pub fn entry_export_symbol(name: &MethodName) -> String {
    format!("{ENTRY_EXPORT_PREFIX}{name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn should_accept_collection_names_at_the_boundary() {
        let longest: String = core::iter::repeat_n('a', MAX_NAME_BYTES).collect();
        assert!(CollectionName::new(&longest).is_ok());
        assert!(CollectionName::new("scores-v1_2").is_ok());
        assert!(CollectionName::new("A").is_ok());
    }

    #[test]
    fn should_reject_collection_names_over_the_boundary_or_with_bad_characters() {
        let too_long: String = core::iter::repeat_n('a', MAX_NAME_BYTES + 1).collect();
        let error = CollectionName::new(&too_long).unwrap_err();
        assert_eq!(error.kind, NameKind::Collection);
        assert!(CollectionName::new("").is_err());
        assert!(CollectionName::new("scores.v1").is_err());
        assert!(CollectionName::new("scores v1").is_err());
        assert!(CollectionName::new("scörés").is_err());
    }

    #[test]
    fn should_accept_property_paths_and_system_properties() {
        assert!(PropertyPath::new("class").is_ok());
        assert!(PropertyPath::new("profile.age").is_ok());
        assert!(PropertyPath::new("$ownerId").unwrap().is_system());
        assert!(PropertyPath::new("$createdAt").is_ok());
        let dotted = PropertyPath::new("a.b.c").unwrap();
        let segments: Vec<&str> = dotted.segments().collect();
        assert_eq!(segments, ["a", "b", "c"]);
    }

    #[test]
    fn should_reject_property_paths_with_empty_segments_or_unknown_system_names() {
        assert!(PropertyPath::new("a..b").is_err());
        assert!(PropertyPath::new(".a").is_err());
        assert!(PropertyPath::new("$unknown").is_err());
        assert!(PropertyPath::new("").is_err());
        let too_long: String =
            core::iter::repeat_n("abcdefgh.", 29).collect::<String>() + "abcdefgh";
        assert!(too_long.len() > MAX_PROPERTY_PATH_BYTES);
        assert!(PropertyPath::new(&too_long).is_err());
    }

    #[test]
    fn should_bound_index_names_by_characters_not_bytes() {
        let thirty_two: String = core::iter::repeat_n('ü', MAX_INDEX_NAME_CHARS).collect();
        assert!(IndexName::new(&thirty_two).is_ok());
        let thirty_three: String = core::iter::repeat_n('ü', MAX_INDEX_NAME_CHARS + 1).collect();
        assert!(IndexName::new(&thirty_three).is_err());
        assert!(IndexName::new("").is_err());
    }

    #[test]
    fn should_accept_dotted_method_names_and_reject_bad_segments() {
        assert!(MethodName::new("score.add").is_ok());
        assert!(MethodName::new("a").is_ok());
        assert!(MethodName::new("score.add_pair2").is_ok());
        assert!(MethodName::new("Score.add").is_err());
        assert!(MethodName::new("score..add").is_err());
        assert!(MethodName::new("score.").is_err());
        assert!(MethodName::new("1score").is_err());
        assert!(MethodName::new("score-add").is_err());
        let too_long: String = core::iter::repeat_n('a', MAX_METHOD_NAME_BYTES + 1).collect();
        assert!(MethodName::new(&too_long).is_err());
    }

    #[test]
    fn should_accept_module_and_interface_names_and_reject_uppercase() {
        assert!(ModuleName::new("main").is_ok());
        assert!(ModuleName::new("helpers_2").is_ok());
        assert!(ModuleName::new("Main").is_err());
        assert!(ModuleName::new("").is_err());
        assert!(InterfaceName::new("math").is_ok());
        assert!(InterfaceName::new("math.v1").is_err());
    }

    #[test]
    fn should_accept_rule_names_and_reject_dots() {
        assert!(RuleName::new("points_monotonic").is_ok());
        assert!(RuleName::new("p").is_ok());
        assert!(RuleName::new("points.monotonic").is_err());
        assert!(RuleName::new("_points").is_err());
        let too_long: String = core::iter::repeat_n('a', MAX_RULE_NAME_BYTES + 1).collect();
        assert!(RuleName::new(&too_long).is_err());
    }

    #[test]
    fn should_export_entries_under_the_prefixed_method_name() {
        let name = MethodName::new("score.add").unwrap();
        assert_eq!(entry_export_symbol(&name), "dash_entry_score.add");
    }

    #[test]
    fn should_keep_dotted_and_underscored_method_names_distinct_in_exports() {
        let dotted = MethodName::new("a.b").unwrap();
        let underscored = MethodName::new("a__b").unwrap();
        assert_ne!(
            entry_export_symbol(&dotted),
            entry_export_symbol(&underscored)
        );
    }

    #[test]
    fn should_render_invalid_name_errors_with_kind_and_reason() {
        let error = CollectionName::new("bad name").unwrap_err();
        let text = alloc::format!("{error}");
        assert!(text.contains("collection"));
        assert!(text.contains("bad name"));
    }
}
