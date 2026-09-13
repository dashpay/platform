//! Rule checks: identity, scope, kind and guard field references.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::declare::{ContractDeclaration, FieldContext, RuleKind};
use crate::manifest::{CollectionManifest, ModuleTable, RuleManifest};
use crate::validate::collections::has_property_path;
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};
use crate::validate::merge::{dedupe, sorted};

pub(super) fn validate_rules(
    declaration: &ContractDeclaration,
    modules: &ModuleTable,
    collections: &[CollectionManifest],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RuleManifest> {
    let rules = dedupe(
        &declaration.rules,
        |spec| DeclarationPath::rule(&spec.collection, &spec.name),
        diagnostics,
    );

    let mut manifests: Vec<RuleManifest> = rules
        .iter()
        .map(|rule| {
            let path = DeclarationPath::rule(&rule.collection, &rule.name);
            let collection = collections
                .iter()
                .find(|collection| collection.name == rule.collection);
            if collection.is_none() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::RuleCollectionUnknown {
                        collection: rule.collection.to_string(),
                    },
                ));
            }
            let actions = sorted(&rule.actions);
            if actions.is_empty() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::RuleWithoutActions,
                ));
            }
            match &rule.kind {
                RuleKind::NativeGuard(guard) => {
                    if let Some(collection) = collection {
                        for (context, property) in guard.field_references() {
                            let known = match context {
                                FieldContext::Old | FieldContext::New => {
                                    has_property_path(&collection.fields, property)
                                }
                                // The action context's fields are host defined.
                                FieldContext::Context => true,
                            };
                            if !known {
                                diagnostics.push(Diagnostic::new(
                                    path.clone(),
                                    DiagnosticKind::GuardFieldUnknown {
                                        property: property.to_string(),
                                    },
                                ));
                            }
                        }
                    }
                }
                RuleKind::WasmPredicate { module, .. } => {
                    if !modules.names().any(|name| name == module) {
                        diagnostics.push(Diagnostic::new(
                            path.clone(),
                            DiagnosticKind::PredicateModuleUnknown {
                                module: module.to_string(),
                            },
                        ));
                    }
                }
            }
            RuleManifest {
                collection: rule.collection.clone(),
                name: rule.name.clone(),
                actions,
                kind: rule.kind.clone(),
            }
        })
        .collect();
    manifests.sort_by(|a, b| (&a.collection, &a.name).cmp(&(&b.collection, &b.name)));
    manifests
}
