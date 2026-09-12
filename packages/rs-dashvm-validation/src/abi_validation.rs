//! The import allowlist, the export rules and the memory and table shape rules, applied to a
//! measured module to produce its [`ModuleInterface`].

use crate::abi_names::{
    alloc_signature, entry_signature, host_envelope, ALLOC_EXPORT, HOST_MODULE,
    INTERNAL_MODULE_PREFIX, MEMORY_EXPORT, VM_MODULE,
};
use crate::bundle::{ExportedFunction, HostImport, InternalImport, ModuleInterface, ModuleName};
use crate::errors::{ExportRejection, ImportRejection, MemoryRule, ModuleError};
use crate::profile::PreparationProfile;
use crate::structure::{ImportKind, ModuleFacts};
use wasmparser::ExternalKind;

/// Applies the rules to a submitted module's facts.
pub(crate) fn interface_of_submitted(
    facts: &ModuleFacts,
    profile: &PreparationProfile,
) -> Result<ModuleInterface, ModuleError> {
    let memory = facts
        .memory
        .ok_or(ModuleError::Memory(MemoryRule::Missing))?;
    // The table, when present, was shape-checked during measurement: its element type is
    // nullable funcref (any other reference type is a forbidden feature), it is the only one,
    // it is not imported and it declares a maximum.
    let host_functions = host_envelope();
    let mut host_imports = Vec::new();
    let mut internal_imports = Vec::new();
    for import in &facts.imports {
        let refuse = |reason| ModuleError::Import {
            module: import.module.clone(),
            name: import.name.clone(),
            reason,
        };
        let type_index = match import.kind {
            ImportKind::Function { type_index } => type_index,
            ImportKind::Global { .. } => return Err(refuse(ImportRejection::NotAFunction)),
        };
        let signature = facts
            .types
            .get(type_index as usize)
            .ok_or_else(|| ModuleError::Internal("import type out of range".to_owned()))?
            .clone();
        if import.module == VM_MODULE {
            return Err(refuse(ImportRejection::ReservedModule));
        }
        if import.module == HOST_MODULE {
            let Some((_, expected)) = host_functions.iter().find(|(name, _)| *name == import.name)
            else {
                return Err(refuse(ImportRejection::UnknownHostFunction));
            };
            if *expected != signature {
                return Err(refuse(ImportRejection::HostSignature {
                    expected: expected.clone(),
                    actual: signature,
                }));
            }
            host_imports.push(HostImport {
                name: import.name.clone(),
                signature,
            });
            continue;
        }
        if let Some(target) = import.module.strip_prefix(INTERNAL_MODULE_PREFIX) {
            let target = ModuleName::parse(target, profile.limits.max_module_name_bytes)
                .map_err(|error| refuse(ImportRejection::InvalidTargetName(error)))?;
            internal_imports.push(InternalImport {
                target,
                export: import.name.clone(),
                signature,
            });
            continue;
        }
        return Err(refuse(ImportRejection::UnknownModule));
    }

    let imported = facts.imported_functions();
    let entry = entry_signature();
    let mut exports = Vec::new();
    let mut memory_exported = false;
    let mut alloc: Option<&ExportedFunction> = None;
    for export in &facts.exports {
        match export.kind {
            ExternalKind::Func => {
                if export.index < imported {
                    return Err(ModuleError::Export(ExportRejection::OfImportedFunction {
                        name: export.name.clone(),
                    }));
                }
                let signature = facts
                    .function_signature(export.index)
                    .ok_or_else(|| {
                        ModuleError::Internal("export function out of range".to_owned())
                    })?
                    .clone();
                if export.name == MEMORY_EXPORT {
                    return Err(ModuleError::Export(ExportRejection::MemoryNotMemory));
                }
                exports.push(ExportedFunction {
                    is_entry_candidate: signature == entry,
                    name: export.name.clone(),
                    signature,
                });
            }
            ExternalKind::Memory => {
                if export.name != MEMORY_EXPORT || memory_exported {
                    return Err(ModuleError::Export(ExportRejection::MemoryName {
                        name: export.name.clone(),
                    }));
                }
                memory_exported = true;
            }
            ExternalKind::Table => {
                return Err(ModuleError::Export(ExportRejection::Table {
                    name: export.name.clone(),
                }));
            }
            ExternalKind::Global => {
                // Exported globals are harmless: nothing imports them (globals may not be
                // imported), so they are simply not part of the interface.
            }
            ExternalKind::Tag => {
                return Err(ModuleError::Internal(
                    "tag export passed measurement".to_owned(),
                ));
            }
        }
    }
    if !memory_exported {
        return Err(ModuleError::Memory(MemoryRule::NotExported));
    }
    for export in &exports {
        if export.name == ALLOC_EXPORT {
            alloc = Some(export);
        }
    }
    match alloc {
        None => return Err(ModuleError::Export(ExportRejection::MissingAlloc)),
        Some(alloc) if alloc.signature != alloc_signature() => {
            return Err(ModuleError::Export(ExportRejection::AllocSignature {
                expected: alloc_signature(),
            }));
        }
        Some(_) => {}
    }
    if !exports.iter().any(|export| export.is_entry_candidate) {
        return Err(ModuleError::Export(ExportRejection::NoEntry {
            expected: entry,
        }));
    }
    Ok(ModuleInterface {
        host_imports,
        internal_imports,
        exports,
        memory,
        table: facts.table,
    })
}
