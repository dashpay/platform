//! Entry checks: identity, export symbols, receivers, module bindings and
//! bounded wire types.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::declare::{ContractDeclaration, EntrySpec, Receiver, IMPLICIT_MODULE};
use crate::identity::{entry_export_symbol, ModuleName};
use crate::manifest::{MethodEntry, MethodTable, ModuleTable};
use crate::validate::collections::{check_value_type, CollectionKinds};
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};
use crate::validate::merge::dedupe;

pub(super) fn validate_entries(
    declaration: &ContractDeclaration,
    modules: &ModuleTable,
    collections: &CollectionKinds,
    diagnostics: &mut Vec<Diagnostic>,
) -> MethodTable {
    let entries = dedupe(
        &declaration.entries,
        |a, b| a.name == b.name,
        |spec| spec.origin,
        |a, b| {
            let mut left = a.clone();
            left.origin = b.origin;
            left == *b
        },
        |_, _| {},
        |spec| DeclarationPath::entry(&spec.name),
        "entry",
        || DiagnosticKind::DuplicateMethod,
        diagnostics,
    );

    let single_module: Option<&ModuleName> = match modules.modules.as_slice() {
        [only] => Some(&only.name),
        _ => None,
    };

    let mut exports: Vec<String> = Vec::new();
    let mut table: Vec<MethodEntry> = Vec::new();
    for entry in &entries {
        let path = DeclarationPath::entry(&entry.name);

        let export = entry_export_symbol(&entry.name);
        if exports.contains(&export) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::DuplicateExportSymbol {
                    export: export.clone(),
                },
            ));
        } else {
            exports.push(export.clone());
        }

        let module = resolve_module(entry, modules, single_module, &path, diagnostics);

        let kind = entry.receiver.collection().and_then(|name| {
            collections
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, kind)| *kind)
        });
        if let Some(name) = entry.receiver.collection() {
            if kind.is_none() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::ReceiverCollectionUnknown {
                        collection: name.to_string(),
                    },
                ));
            }
        }
        if entry.receiver.is_mutable() {
            if entry.read_only {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::ReadOnlyEntryWithMutableReceiver,
                ));
            }
            if let Receiver::Mut(name) = &entry.receiver {
                let immutable = declaration
                    .collections
                    .iter()
                    .find(|collection| &collection.name == name)
                    .map(|collection| !collection.mutable)
                    .unwrap_or(false);
                if immutable {
                    diagnostics.push(Diagnostic::new(
                        path.clone(),
                        DiagnosticKind::MutableReceiverOnImmutableCollection {
                            collection: name.to_string(),
                        },
                    ));
                }
            }
        }

        let mut params: Vec<&str> = Vec::new();
        for param in &entry.params {
            let param_path = DeclarationPath::entry_param(&entry.name, &param.name);
            if params.contains(&param.name.as_str()) {
                diagnostics.push(Diagnostic::new(
                    param_path.clone(),
                    DiagnosticKind::DuplicateParameter {
                        param: param.name.clone(),
                    },
                ));
            } else {
                params.push(&param.name);
            }
            check_value_type(&param_path, &param.ty, diagnostics);
        }
        check_value_type(
            &DeclarationPath::entry_param(&entry.name, "return"),
            &entry.returns,
            diagnostics,
        );

        table.push(MethodEntry {
            name: entry.name.clone(),
            module,
            export,
            receiver: entry.receiver.clone(),
            takes_document_id: MethodEntry::takes_document_id(&entry.receiver, kind),
            read_only: entry.read_only,
            params: entry.params.clone(),
            returns: entry.returns.clone(),
        });
    }
    table.sort_by(|a, b| a.name.cmp(&b.name));
    MethodTable { entries: table }
}

fn resolve_module(
    entry: &EntrySpec,
    modules: &ModuleTable,
    single_module: Option<&ModuleName>,
    path: &DeclarationPath,
    diagnostics: &mut Vec<Diagnostic>,
) -> ModuleName {
    match (&entry.module, single_module) {
        (Some(module), _) => {
            if !modules.names().any(|name| name == module) {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::EntryModuleUnknown {
                        module: module.to_string(),
                    },
                ));
            }
            module.clone()
        }
        (None, Some(single)) => single.clone(),
        (None, None) => {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::EntryModuleRequired,
            ));
            modules
                .modules
                .first()
                .map(|module| module.name.clone())
                .unwrap_or_else(|| {
                    ModuleName::new(IMPLICIT_MODULE)
                        .expect("the implicit module name satisfies the grammar")
                })
        }
    }
}
