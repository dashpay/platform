//! Capability checks: explicit requirements, derived requirements and the
//! interface-disabled rejection.

use alloc::vec::Vec;

use crate::declare::{
    CapabilityRequirement, CapabilityStatus, ContractDeclaration, ReceiptPolicy, RuleKind, Store,
    WritePolicy,
};
use crate::manifest::{
    CapabilityEntry, CapabilityTable, CollectionManifest, MethodTable, ModuleTable, RuleManifest,
    TypedCollectionManifest,
};
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};

pub(super) fn validate_capabilities(
    declaration: &ContractDeclaration,
    modules: &ModuleTable,
    collections: &[CollectionManifest],
    typed_collections: &[TypedCollectionManifest],
    methods: &MethodTable,
    rules: &[RuleManifest],
    diagnostics: &mut Vec<Diagnostic>,
) -> CapabilityTable {
    let mut requirements: Vec<CapabilityRequirement> = Vec::new();

    for requirement in &declaration.capabilities {
        if !requirement.is_explicit() {
            diagnostics.push(Diagnostic::new(
                DeclarationPath::Capability(*requirement),
                DiagnosticKind::CapabilityNotDeclarable {
                    requirement: *requirement,
                },
            ));
            continue;
        }
        requirements.push(*requirement);
    }

    for collection in collections {
        if collection.write == WritePolicy::Contract {
            requirements.push(CapabilityRequirement::ContractWrites);
        }
        if collection.store == Store::Private {
            requirements.push(CapabilityRequirement::PrivateStore);
        }
    }
    for typed in typed_collections {
        requirements.push(CapabilityRequirement::TypedCollections(typed.kind));
    }
    for rule in rules {
        requirements.push(match rule.kind {
            RuleKind::NativeGuard(_) => CapabilityRequirement::NativeGuards,
            RuleKind::WasmPredicate { .. } => CapabilityRequirement::WasmPredicates,
        });
    }
    if !methods.entries.is_empty() {
        requirements.push(CapabilityRequirement::Entries);
    }
    if modules.modules.len() > 1 {
        requirements.push(CapabilityRequirement::Modules);
    }
    if declaration.receipts == ReceiptPolicy::Stored {
        requirements.push(CapabilityRequirement::StoredReceipts);
    }

    requirements.sort();
    requirements.dedup();

    let entries: Vec<CapabilityEntry> = requirements
        .into_iter()
        .map(|requirement| {
            let status = requirement.status();
            if status == CapabilityStatus::InterfaceDisabled {
                diagnostics.push(Diagnostic::new(
                    DeclarationPath::Capability(requirement),
                    DiagnosticKind::CapabilityInterfaceDisabled { requirement },
                ));
            }
            CapabilityEntry {
                requirement,
                status,
            }
        })
        .collect();

    CapabilityTable { entries }
}
