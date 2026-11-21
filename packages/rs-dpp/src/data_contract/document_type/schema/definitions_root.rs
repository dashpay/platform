//! A contract's `$defs` as the document type parser resolves a `$ref` into them.

use crate::util::json_schema::resolve_uri;
use platform_value::Value;
use std::cell::OnceCell;
use std::collections::BTreeMap;

/// A contract's `$defs`, borrowed, and the root a `$ref` into them resolves against: the parser
/// resolves a `$ref` in the document type schema with the `$defs` added, so a pointer into
/// `$defs` is resolved here in `{ "$defs": <definitions> }`, which is built the first time one
/// is resolved and never changes after.
pub(crate) struct DefinitionsRoot<'a> {
    definitions: Option<&'a BTreeMap<String, Value>>,
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

    /// The schema `reference` points at, as the parser resolves it: a pointer into `$defs`
    /// among these definitions, any other in `type_schema`, the document type schema it is
    /// written in (`None` where no type schema is at hand, as for a definition read on its own).
    pub(crate) fn resolve<'s>(
        &'s self,
        reference: &str,
        type_schema: Option<&'s Value>,
    ) -> Option<&'s Value> {
        if !points_into_definitions(reference) {
            return resolve_uri(type_schema?, reference).ok();
        }
        let root = self.root.get_or_init(|| {
            let definitions = self
                .definitions
                .into_iter()
                .flatten()
                .map(|(name, definition)| (Value::Text(name.clone()), definition.clone()))
                .collect();
            Value::Map(vec![(
                Value::Text("$defs".to_string()),
                Value::Map(definitions),
            )])
        });
        resolve_uri(root, reference).ok()
    }
}

/// Whether the parser finds `reference` in the contract's `$defs`: its path's first segment,
/// which ends at a `/` or a `.`, is `$defs`.
pub(crate) fn points_into_definitions(reference: &str) -> bool {
    reference
        .strip_prefix("#/")
        .and_then(|path| path.split(['/', '.']).next())
        == Some("$defs")
}
