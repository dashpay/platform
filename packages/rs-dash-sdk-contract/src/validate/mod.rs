//! Validation: from a [`ContractDeclaration`] to a [`CanonicalManifest`] or a
//! complete list of [`Diagnostic`]s.
//!
//! The validator owns what the native host cannot know: attribute grammar,
//! identity uniqueness, conflicts between an attribute and a builder,
//! boundedness of fields and wire types, cross-references between
//! declarations, module graph shape and the SDK's own semantic rules (a
//! mutable receiver on an immutable collection, a singleton with indexes).
//! It deliberately does not mirror native numeric limits (index counts, name
//! lengths, indexed string sizes, time-range overlap caps) or native schema
//! dependencies (a range count needs a count): those are enforced once, by
//! Dash Platform Protocol, and the build crate surfaces them.
//!
//! Every check runs and every diagnostic is collected; there is no
//! first-error return.

pub mod diagnostic;

mod capabilities;
mod collections;
mod entries;
mod merge;
mod modules;
mod rules;
#[cfg(test)]
mod tests;

use alloc::vec::Vec;

pub use diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};

use crate::declare::ContractDeclaration;
use crate::manifest::CanonicalManifest;

/// Validates the declaration and builds its canonical manifest, or returns
/// every diagnostic found.
pub fn validate(declaration: &ContractDeclaration) -> Result<CanonicalManifest, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    let module_table = modules::validate_modules(declaration, &mut diagnostics);
    let (collection_manifests, collection_kinds) =
        collections::validate_collections(declaration, &mut diagnostics);
    let typed_collections = collections::validate_typed_collections(
        declaration,
        &collection_manifests,
        &mut diagnostics,
    );
    let method_table = entries::validate_entries(
        declaration,
        &module_table,
        &collection_kinds,
        &mut diagnostics,
    );
    let rule_manifests = rules::validate_rules(
        declaration,
        &module_table,
        &collection_manifests,
        &mut diagnostics,
    );
    let capability_table = capabilities::validate_capabilities(
        declaration,
        &module_table,
        &collection_manifests,
        &typed_collections,
        &method_table,
        &rule_manifests,
        &mut diagnostics,
    );

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(CanonicalManifest {
        modules: module_table,
        collections: collection_manifests,
        typed_collections,
        methods: method_table,
        rules: rule_manifests,
        capabilities: capability_table,
        receipts: declaration.receipts,
    })
}
