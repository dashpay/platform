//! Where a `$ref` in a document type schema points, as the document type parser and as the
//! document validator resolve it.
//!
//! Both resolve a `$ref` in the document type schema with the contract's `$defs` added
//! (`enrich_with_base_schema`), but read the pointer differently:
//!
//! - The parser ([`resolve_uri`]) reads `#/` followed by segments separated by `/` or `.`, a
//!   segment `name[i]` taking element `i` of the array `name`, and any other segment a map key.
//! - The validator reads a JSON pointer (`#` or `#/...`): segments separated by `/` only, `~1`
//!   standing for `/` and `~0` for `~`, a segment indexing an array when it is a decimal number
//!   without sign or leading zero, and a segment naming a repeated map key reaching its last
//!   entry.
//!
//! A `$ref` the parser reads for a property is resolved alike by both, or the validation schema
//! refuses the contract (registration from protocol version 15). A `$ref` only the validator
//! reads, such as one written in an entry beside another `$ref`, is resolved the validator's way
//! only. The walks that follow a `$ref` (the depth and cycle check, the validation schema, the
//! member order and the `required` lists of what a contract gains) resolve it here, the way the
//! reader they judge for does.

use super::last_value;
use crate::util::json_schema::resolve_uri;
use platform_value::Value;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::BTreeMap;

/// The key `$defs` takes in the document type schema the parser and the validator resolve in.
const DEFINITIONS: &str = "$defs";

/// A contract's `$defs`, borrowed, as a `$ref` resolves into them.
pub(crate) struct DefinitionsRoot<'a> {
    definitions: Option<&'a BTreeMap<String, Value>>,
    /// `{ "$defs": <definitions> }`, built the first time a pointer into `$defs` needs the
    /// parser's array syntax on a definition's name, which no plain name does.
    root: OnceCell<Value>,
}

impl<'a> DefinitionsRoot<'a> {
    pub(crate) fn new(definitions: Option<&'a BTreeMap<String, Value>>) -> Self {
        Self {
            definitions,
            root: OnceCell::new(),
        }
    }

    pub(crate) fn definitions(&self) -> Option<&'a BTreeMap<String, Value>> {
        self.definitions
    }

    /// The schema `reference` points at as the document type parser resolves it: a pointer
    /// into `$defs` among these definitions, any other in `type_schema`, the document type
    /// schema it is written in (`None` where no type schema is at hand, as for a definition read
    /// on its own).
    pub(crate) fn resolve_as_parsed<'s>(
        &'s self,
        reference: &str,
        type_schema: Option<&'s Value>,
    ) -> Option<&'s Value> {
        let path = reference.strip_prefix("#/")?.replace('/', ".");
        let mut segments = path.splitn(3, '.');
        if segments.next() != Some(DEFINITIONS) {
            return resolve_uri(type_schema?, reference).ok();
        }
        let Some(name) = segments.next() else {
            return self.root().get_value_at_path(DEFINITIONS).ok();
        };
        let definition = if name.contains('[') {
            // The parser's array syntax applies to the name: resolved as written
            self.root()
                .get_value_at_path(&format!("{DEFINITIONS}.{name}"))
                .ok()?
        } else {
            self.definitions?.get(name)?
        };
        match segments.next() {
            Some(rest) => definition.get_value_at_path(rest).ok(),
            None => Some(definition),
        }
    }

    /// The schema `reference` points at as the document validator resolves it: a pointer into
    /// `$defs` among these definitions, any other in `type_schema`, the document type schema it
    /// is written in. `None` for a reference that is no JSON pointer the validator reads, or
    /// that points at nothing. `#` reaches `type_schema` as written, where the validator reaches
    /// it with the `$defs` added and the system timestamps left out of its `required`; the depth
    /// check refuses a document type reaching its own root through a `$ref`, as a cycle.
    pub(crate) fn resolve_as_validated<'s>(
        &'s self,
        reference: &str,
        type_schema: Option<&'s Value>,
    ) -> Option<&'s Value> {
        let pointer = json_pointer_of(reference)?;
        let Some(segments) = pointer.strip_prefix('/') else {
            return type_schema;
        };
        let mut segments = segments.splitn(3, '/');
        if segments.next().map(unescape).as_deref() != Some(DEFINITIONS) {
            return follow_json_pointer(type_schema?, pointer);
        }
        let Some(name) = segments.next() else {
            return follow_json_pointer(self.root(), "/$defs");
        };
        let definition = self.definitions?.get(&*unescape(name))?;
        match segments.next() {
            Some(rest) => follow_json_pointer(definition, &format!("/{rest}")),
            None => Some(definition),
        }
    }

    fn root(&self) -> &Value {
        self.root.get_or_init(|| {
            let definitions = self
                .definitions
                .into_iter()
                .flatten()
                .map(|(name, definition)| (Value::Text(name.clone()), definition.clone()))
                .collect();
            Value::Map(vec![(
                Value::Text(DEFINITIONS.to_string()),
                Value::Map(definitions),
            )])
        })
    }
}

/// The JSON pointer of `reference` as the document validator reads one, the part after `#`
/// (empty, or starting with `/`): `None` for a reference holding `%`, whitespace or a control
/// character, which the validator parses into a URL that drops, encodes or decodes them, so it
/// would follow another pointer than the one written.
pub(crate) fn json_pointer_of(reference: &str) -> Option<&str> {
    reference
        .strip_prefix('#')
        .filter(|pointer| pointer.is_empty() || pointer.starts_with('/'))
        .filter(|pointer| {
            !pointer.contains(|c: char| c == '%' || c.is_whitespace() || c.is_control())
        })
}

/// The value the JSON pointer `pointer` (empty, or starting with `/`) reaches from `root`, as
/// the document validator follows one: `~1` is `/` and `~0` is `~` in a segment, an array index
/// has no sign and no leading zero, and a segment naming a repeated map key reaches its last
/// entry.
pub(crate) fn follow_json_pointer<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    let Some(pointer) = pointer.strip_prefix('/') else {
        return pointer.is_empty().then_some(root);
    };
    pointer
        .split('/')
        .try_fold(root, |target, segment| match target {
            Value::Map(map) => last_value(map, &unescape(segment)),
            Value::Array(elements) => array_index(segment).and_then(|index| elements.get(index)),
            _ => None,
        })
}

/// A JSON pointer segment with `~1` read as `/` and `~0` as `~`.
fn unescape(segment: &str) -> Cow<'_, str> {
    if segment.contains('~') {
        Cow::Owned(segment.replace("~1", "/").replace("~0", "~"))
    } else {
        Cow::Borrowed(segment)
    }
}

/// An array index in a JSON pointer segment, as the document validator reads one: decimal
/// digits with no sign and no leading zero.
fn array_index(segment: &str) -> Option<usize> {
    if segment.starts_with('+') || (segment.starts_with('0') && segment.len() != 1) {
        None
    } else {
        segment.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_value::platform_value;

    fn definitions() -> BTreeMap<String, Value> {
        BTreeMap::from([(
            "a".to_string(),
            platform_value!({
                "properties": { "b": { "type": "string" } },
                "enum": [{ "type": "object" }],
                "x.y": { "type": "integer" },
            }),
        )])
    }

    /// The parser reads `/` and `.` alike and an index only in `name[i]`; the validator reads
    /// `/` only, `~` escapes, and a number as an index: each walk resolves the way the reader it
    /// judges for does.
    #[test]
    fn should_resolve_a_ref_as_the_parser_and_as_the_validator_each_do() {
        let definitions = definitions();
        let root = DefinitionsRoot::new(Some(&definitions));
        let type_schema = platform_value!({ "properties": { "p": { "type": "boolean" } } });
        let parsed = |reference| root.resolve_as_parsed(reference, Some(&type_schema));
        let validated = |reference| root.resolve_as_validated(reference, Some(&type_schema));
        let string = platform_value!({ "type": "string" });
        let object = platform_value!({ "type": "object" });
        let integer = platform_value!({ "type": "integer" });

        assert_eq!(parsed("#/$defs/a/properties/b"), Some(&string));
        assert_eq!(validated("#/$defs/a/properties/b"), Some(&string));
        assert_eq!(parsed("#/$defs/a.properties.b"), Some(&string));
        assert_eq!(validated("#/$defs/a.properties.b"), None);
        assert_eq!(parsed("#/$defs/a/enum[0]"), Some(&object));
        assert_eq!(validated("#/$defs/a/enum/0"), Some(&object));
        assert_eq!(parsed("#/$defs/a/enum/0"), None);
        assert_eq!(validated("#/$defs/a/enum/00"), None);
        assert_eq!(validated("#/$defs/a/x.y"), Some(&integer));
        assert_eq!(
            validated("#/properties/p"),
            Some(&platform_value!({ "type": "boolean" }))
        );
        assert_eq!(parsed("#/properties/p"), validated("#/properties/p"));
        assert_eq!(validated("#"), Some(&type_schema));
        assert_eq!(validated("#/$defs/a/properties/%62"), None);
        assert!(
            root.root.get().is_none(),
            "a plain definition name is looked up without copying the definitions"
        );
        assert_eq!(parsed("#/$defs/a[0]"), None);
        assert!(
            root.root.get().is_some(),
            "array syntax on a name reads the copy"
        );
        assert!(parsed("#/$defs").is_some_and(|value| value.as_map().is_some()));
        assert_eq!(parsed("#/$defs"), validated("#/$defs"));
    }
}
