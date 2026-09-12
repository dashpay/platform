//! Bundle-level preparation: names, bindings, graph, entries, digest.

use crate::bundle::{BundleBinding, EntryRef, ModuleName, PreparedBundle, PreparedModule};
use crate::errors::{BundleError, PreparationError};
use crate::hashing::BundleDigest;
use crate::prepared_module::prepare_module;
use crate::profile::PreparationProfile;
use std::collections::{BTreeMap, BTreeSet};

/// One module of a bundle as submitted: its name and canonical bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleInput<'a> {
    /// The module name; validated against the profile's name rules.
    pub name: &'a str,
    /// The canonical bytes.
    pub canonical_bytes: &'a [u8],
}

/// One binding as the manifest declares it. Bindings must match the imports the code
/// establishes exactly; the declaration exists so a manifest can be checked against the code
/// and so the digest covers what was declared.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredBinding<'a> {
    /// The importing module.
    pub importer: &'a str,
    /// The target module.
    pub target: &'a str,
    /// The target's export.
    pub export: &'a str,
}

/// Prepares every module and validates the bundle.
///
/// `entries` names the exported functions the bundle exposes as `(module, export)`; each must
/// exist with the entry signature. `declared_bindings` must equal the set of internal imports
/// the code establishes. Modules may be given in any order; the result is in canonical name
/// order.
pub fn validate_and_prepare_bundle(
    inputs: &[BundleInput<'_>],
    declared_bindings: &[DeclaredBinding<'_>],
    entries: &[(&str, &str)],
    profile: &PreparationProfile,
) -> Result<PreparedBundle, PreparationError> {
    let limits = &profile.limits;
    if inputs.is_empty() || inputs.len() > usize::from(limits.max_modules_per_bundle) {
        return Err(BundleError::ModuleCount {
            actual: inputs.len(),
            max: limits.max_modules_per_bundle,
        }
        .into());
    }
    let mut names = BTreeSet::new();
    let mut parsed: Vec<(ModuleName, &[u8])> = Vec::with_capacity(inputs.len());
    for input in inputs {
        let name = ModuleName::parse(input.name, limits.max_module_name_bytes)?;
        if !names.insert(name.clone()) {
            return Err(BundleError::DuplicateModuleName { name }.into());
        }
        parsed.push((name, input.canonical_bytes));
    }
    parsed.sort_by(|a, b| a.0.cmp(&b.0));

    let mut modules: Vec<PreparedModule> = Vec::with_capacity(parsed.len());
    for (name, canonical_bytes) in parsed {
        let module = prepare_module(name.clone(), canonical_bytes, profile)
            .map_err(|source| PreparationError::Module { name, source })?;
        modules.push(module);
    }

    let by_name: BTreeMap<&ModuleName, &PreparedModule> = modules
        .iter()
        .map(|module| (&module.name, module))
        .collect();

    // Resolve every internal import against the target's exports.
    let mut bindings = Vec::new();
    for module in &modules {
        for import in &module.interface.internal_imports {
            let Some(target) = by_name.get(&import.target) else {
                return Err(BundleError::UnknownBindingTarget {
                    importer: module.name.clone(),
                    target: import.target.clone(),
                }
                .into());
            };
            let Some(export) = target.interface.export(&import.export) else {
                return Err(BundleError::MissingBindingExport {
                    importer: module.name.clone(),
                    target: import.target.clone(),
                    export: import.export.clone(),
                }
                .into());
            };
            if export.signature != import.signature {
                return Err(BundleError::BindingSignature {
                    importer: module.name.clone(),
                    target: import.target.clone(),
                    export: import.export.clone(),
                    imported: Box::new(import.signature.clone()),
                    exported: Box::new(export.signature.clone()),
                }
                .into());
            }
            bindings.push(BundleBinding {
                importer: module.name.clone(),
                target: import.target.clone(),
                export: import.export.clone(),
                signature: import.signature.clone(),
            });
        }
    }
    bindings.sort();
    bindings.dedup();
    check_declared_bindings(&bindings, declared_bindings)?;

    let initialization_order = topological_order(&modules, &bindings)?;

    // Entries.
    let mut resolved_entries = Vec::with_capacity(entries.len());
    for (module_name, export) in entries {
        let module = ModuleName::parse(module_name, limits.max_module_name_bytes)?;
        let Some(prepared) = by_name.get(&module) else {
            return Err(BundleError::EntryModuleUnknown {
                module,
                export: (*export).to_owned(),
            }
            .into());
        };
        match prepared.interface.export(export) {
            Some(exported) if exported.is_entry_candidate => {}
            _ => {
                return Err(BundleError::EntryNotFound {
                    module,
                    export: (*export).to_owned(),
                }
                .into());
            }
        }
        resolved_entries.push(EntryRef {
            module,
            export: (*export).to_owned(),
        });
    }
    resolved_entries.sort();
    if let Some(window) = resolved_entries.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(BundleError::DuplicateEntry {
            module: window[0].module.clone(),
            export: window[0].export.clone(),
        }
        .into());
    }
    if resolved_entries.is_empty() {
        return Err(BundleError::NoEntries.into());
    }

    let mut bundle = PreparedBundle {
        modules,
        bindings,
        entries: resolved_entries,
        initialization_order,
        preparation_generation: profile.generation,
        metering_generation: profile.metering,
        digest: BundleDigest([0; 32]),
    };
    bundle.digest = BundleDigest::of_encoding(&bundle.digest_input());
    Ok(bundle)
}

fn check_declared_bindings(
    resolved: &[BundleBinding],
    declared: &[DeclaredBinding<'_>],
) -> Result<(), BundleError> {
    let mut declared: Vec<(&str, &str, &str)> = declared
        .iter()
        .map(|binding| (binding.importer, binding.target, binding.export))
        .collect();
    declared.sort();
    declared.dedup();
    let resolved: Vec<(&str, &str, &str)> = resolved
        .iter()
        .map(|binding| {
            (
                binding.importer.as_str(),
                binding.target.as_str(),
                binding.export.as_str(),
            )
        })
        .collect();
    if declared == resolved {
        return Ok(());
    }
    let detail = match (
        declared.iter().find(|binding| !resolved.contains(binding)),
        resolved.iter().find(|binding| !declared.contains(binding)),
    ) {
        (Some((importer, target, export)), _) => {
            format!("declared binding `{importer}` -> `{target}`.`{export}` is not imported by the code")
        }
        (None, Some((importer, target, export))) => {
            format!("the code binds `{importer}` -> `{target}`.`{export}` which is not declared")
        }
        (None, None) => "declared bindings differ from the code".to_owned(),
    };
    Err(BundleError::BindingsMismatch { detail })
}

/// Kahn's algorithm over the binding graph with canonical-name tie-break. Every module is
/// included whether or not any entry reaches it; the runtime selects the closure it needs.
fn topological_order(
    modules: &[PreparedModule],
    bindings: &[BundleBinding],
) -> Result<Vec<ModuleName>, BundleError> {
    let mut in_degree: BTreeMap<&ModuleName, usize> =
        modules.iter().map(|module| (&module.name, 0)).collect();
    let mut edges: BTreeMap<&ModuleName, BTreeSet<&ModuleName>> = BTreeMap::new();
    for binding in bindings {
        // The target must be initialised before the importer.
        if edges
            .entry(&binding.target)
            .or_default()
            .insert(&binding.importer)
        {
            if let Some(degree) = in_degree.get_mut(&binding.importer) {
                *degree += 1;
            }
        }
    }
    let mut ready: BTreeSet<&ModuleName> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(name, _)| *name)
        .collect();
    let mut order = Vec::with_capacity(modules.len());
    while let Some(next) = ready.pop_first() {
        order.push(next.clone());
        if let Some(dependents) = edges.get(next) {
            for dependent in dependents {
                let degree = in_degree
                    .get_mut(dependent)
                    .expect("every binding endpoint is a module of the bundle");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(dependent);
                }
            }
        }
    }
    if order.len() != modules.len() {
        let stuck = in_degree
            .into_iter()
            .filter(|(_, degree)| *degree > 0)
            .map(|(name, _)| name.clone())
            .collect();
        return Err(BundleError::DependencyCycle { modules: stuck });
    }
    Ok(order)
}
