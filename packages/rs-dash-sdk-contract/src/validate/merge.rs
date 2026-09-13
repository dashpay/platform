//! Merging attribute and builder declarations of one item.
//!
//! Two specs with the same identity are one item when they come from
//! different origins and agree; a builder may restate what an attribute
//! declared. Two specs of the same origin with one identity are a duplicate,
//! and two specs of different origins that disagree are a conflict.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::declare::{
    CollectionSpec, DeclarationOrigin, EntrySpec, IndexSpec, InterfaceSpec, ModuleSpec, RuleSpec,
    TypedCollectionSpec,
};
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};

/// A spec with a declared identity and an origin.
pub(super) trait Identified: Clone {
    /// What the item is called in a conflict diagnostic.
    const WHAT: &'static str;

    /// Whether the two specs declare the same item.
    fn same_identity(&self, other: &Self) -> bool;

    /// Where the spec came from.
    fn origin(&self) -> DeclarationOrigin;

    /// Whether the two specs agree apart from origin (and whatever `merge`
    /// may legitimately union).
    fn equivalent(&self, other: &Self) -> bool;

    /// Unions what a restatement may add; nothing by default.
    fn merge(&mut self, _other: &Self) {}

    /// The diagnostic for two specs of one origin sharing an identity.
    fn duplicate() -> DiagnosticKind;
}

/// Same content apart from origin.
fn equal_ignoring_origin<T: Clone + PartialEq + HasOrigin>(a: &T, b: &T) -> bool {
    let mut left = a.clone();
    left.set_origin(b.origin());
    left == *b
}

/// Origin access shared by the spec types.
trait HasOrigin {
    fn origin(&self) -> DeclarationOrigin;
    fn set_origin(&mut self, origin: DeclarationOrigin);
}

macro_rules! has_origin {
    ($($ty:ty),* $(,)?) => {
        $(
            impl HasOrigin for $ty {
                fn origin(&self) -> DeclarationOrigin {
                    self.origin
                }

                fn set_origin(&mut self, origin: DeclarationOrigin) {
                    self.origin = origin;
                }
            }
        )*
    };
}

has_origin!(
    ModuleSpec,
    InterfaceSpec,
    CollectionSpec,
    IndexSpec,
    TypedCollectionSpec,
    EntrySpec,
    RuleSpec,
);

impl Identified for ModuleSpec {
    const WHAT: &'static str = "module";

    fn same_identity(&self, other: &Self) -> bool {
        self.name == other.name
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        sorted(&self.uses) == sorted(&other.uses)
    }

    fn merge(&mut self, other: &Self) {
        for interface in &other.uses {
            if !self.uses.contains(interface) {
                self.uses.push(interface.clone());
            }
        }
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateModule
    }
}

impl Identified for InterfaceSpec {
    const WHAT: &'static str = "interface";

    fn same_identity(&self, other: &Self) -> bool {
        self.name == other.name
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        let mut left = self.functions.clone();
        left.sort_by(|a, b| a.name.cmp(&b.name));
        let mut right = other.functions.clone();
        right.sort_by(|a, b| a.name.cmp(&b.name));
        self.provider == other.provider && left == right
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateInterface
    }
}

impl Identified for CollectionSpec {
    const WHAT: &'static str = "collection";

    fn same_identity(&self, other: &Self) -> bool {
        self.name == other.name
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        self.same_shape_ignoring_indexes(other)
    }

    fn merge(&mut self, other: &Self) {
        self.indexes.extend(other.indexes.iter().cloned());
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateCollection
    }
}

impl Identified for IndexSpec {
    const WHAT: &'static str = "index";

    fn same_identity(&self, other: &Self) -> bool {
        self.name == other.name
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        equal_ignoring_origin(self, other)
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateIndex
    }
}

impl Identified for TypedCollectionSpec {
    const WHAT: &'static str = "typed collection";

    fn same_identity(&self, other: &Self) -> bool {
        self.id == other.id
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        equal_ignoring_origin(self, other)
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateCollection
    }
}

impl Identified for EntrySpec {
    const WHAT: &'static str = "entry";

    fn same_identity(&self, other: &Self) -> bool {
        self.name == other.name
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        equal_ignoring_origin(self, other)
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateMethod
    }
}

impl Identified for RuleSpec {
    const WHAT: &'static str = "rule";

    fn same_identity(&self, other: &Self) -> bool {
        self.collection == other.collection && self.name == other.name
    }

    fn origin(&self) -> DeclarationOrigin {
        self.origin
    }

    fn equivalent(&self, other: &Self) -> bool {
        let mut left = self.clone();
        left.origin = other.origin;
        left.actions = sorted(&left.actions);
        let mut right = other.clone();
        right.actions = sorted(&right.actions);
        left == right
    }

    fn duplicate() -> DiagnosticKind {
        DiagnosticKind::DuplicateRule
    }
}

/// Sorted and deduplicated copy.
pub(super) fn sorted<T: Clone + Ord>(items: &[T]) -> Vec<T> {
    let mut sorted = items.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted
}

/// Deduplicates `items` by identity, reporting duplicates and conflicts at
/// `path`, and returns the kept specs in first-seen order.
pub(super) fn dedupe<T: Identified>(
    items: &[T],
    path: impl Fn(&T) -> DeclarationPath,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<T> {
    let mut kept: Vec<T> = Vec::new();
    for item in items {
        let Some(existing) = kept
            .iter_mut()
            .find(|existing| existing.same_identity(item))
        else {
            kept.push(item.clone());
            continue;
        };
        if existing.origin() == item.origin() {
            diagnostics.push(Diagnostic::new(path(item), T::duplicate()));
        } else if existing.equivalent(item) {
            existing.merge(item);
        } else {
            diagnostics.push(Diagnostic::new(
                path(item),
                DiagnosticKind::ConflictingDeclaration {
                    what: T::WHAT.to_string(),
                    first: existing.origin(),
                    second: item.origin(),
                },
            ));
        }
    }
    kept
}
