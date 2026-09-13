//! Module and interface checks: identity, providers, imports and the acyclic
//! import graph.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::declare::{ContractDeclaration, InterfaceSpec, ModuleSpec, IMPLICIT_MODULE};
use crate::identity::{InterfaceName, ModuleName};
use crate::manifest::{Binding, InterfaceEntry, ModuleEntry, ModuleTable};
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};
use crate::validate::merge::dedupe;

pub(super) fn validate_modules(
    declaration: &ContractDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) -> ModuleTable {
    let mut modules = dedupe(
        &declaration.modules,
        |a, b| a.name == b.name,
        |spec| spec.origin,
        |a, b| sorted_uses(a) == sorted_uses(b),
        |kept, next| {
            for interface in &next.uses {
                if !kept.uses.contains(interface) {
                    kept.uses.push(interface.clone());
                }
            }
        },
        |spec| DeclarationPath::module(&spec.name),
        "module",
        || DiagnosticKind::DuplicateModule,
        diagnostics,
    );
    if modules.is_empty() {
        modules.push(ModuleSpec::new(
            ModuleName::new(IMPLICIT_MODULE)
                .expect("the implicit module name satisfies the grammar"),
        ));
    }

    let interfaces = dedupe(
        &declaration.interfaces,
        |a, b| a.name == b.name,
        |spec| spec.origin,
        |a, b| a.provider == b.provider && sorted_functions(a) == sorted_functions(b),
        |_, _| {},
        |spec| DeclarationPath::interface(&spec.name),
        "interface",
        || DiagnosticKind::DuplicateInterface,
        diagnostics,
    );

    let module_names: Vec<&ModuleName> = modules.iter().map(|module| &module.name).collect();

    for interface in &interfaces {
        if !module_names.contains(&&interface.provider) {
            diagnostics.push(Diagnostic::new(
                DeclarationPath::interface(&interface.name),
                DiagnosticKind::InterfaceProviderUnknown {
                    module: interface.provider.to_string(),
                },
            ));
        }
        let mut seen: Vec<&str> = Vec::new();
        for function in &interface.functions {
            if seen.contains(&function.name.as_str()) {
                diagnostics.push(Diagnostic::new(
                    DeclarationPath::interface(&interface.name),
                    DiagnosticKind::DuplicateInterfaceFunction {
                        function: function.name.clone(),
                    },
                ));
            } else {
                seen.push(&function.name);
            }
            let mut params: Vec<&str> = Vec::new();
            for param in &function.params {
                if params.contains(&param.name.as_str()) {
                    diagnostics.push(Diagnostic::new(
                        DeclarationPath::interface(&interface.name),
                        DiagnosticKind::DuplicateParameter {
                            param: param.name.clone(),
                        },
                    ));
                } else {
                    params.push(&param.name);
                }
            }
        }
    }

    let mut bindings: Vec<Binding> = Vec::new();
    for module in &modules {
        for used in &module.uses {
            let Some(interface) = interfaces.iter().find(|interface| &interface.name == used)
            else {
                diagnostics.push(Diagnostic::new(
                    DeclarationPath::module(&module.name),
                    DiagnosticKind::UsedInterfaceUnknown {
                        interface: used.to_string(),
                    },
                ));
                continue;
            };
            if interface.provider == module.name {
                diagnostics.push(Diagnostic::new(
                    DeclarationPath::module(&module.name),
                    DiagnosticKind::ModuleSelfImport {
                        interface: used.to_string(),
                    },
                ));
                continue;
            }
            let binding = Binding {
                importer: module.name.clone(),
                provider: interface.provider.clone(),
                interface: used.clone(),
            };
            if !bindings.contains(&binding) {
                bindings.push(binding);
            }
        }
    }

    if let Some(cycle) = find_cycle(&module_names, &bindings) {
        diagnostics.push(Diagnostic::new(
            DeclarationPath::module(&cycle[0]),
            DiagnosticKind::ModuleGraphCycle {
                modules: cycle.iter().map(|name| name.to_string()).collect(),
            },
        ));
    }

    let mut module_entries: Vec<ModuleEntry> = modules
        .iter()
        .map(|module| ModuleEntry {
            name: module.name.clone(),
            uses: sorted_uses(module),
        })
        .collect();
    module_entries.sort_by(|a, b| a.name.cmp(&b.name));

    let mut interface_entries: Vec<InterfaceEntry> = interfaces
        .iter()
        .map(|interface| InterfaceEntry {
            name: interface.name.clone(),
            provider: interface.provider.clone(),
            functions: sorted_functions(interface),
        })
        .collect();
    interface_entries.sort_by(|a, b| a.name.cmp(&b.name));

    bindings.sort();

    ModuleTable {
        modules: module_entries,
        interfaces: interface_entries,
        bindings,
    }
}

fn sorted_uses(module: &ModuleSpec) -> Vec<InterfaceName> {
    let mut uses = module.uses.clone();
    uses.sort();
    uses.dedup();
    uses
}

fn sorted_functions(interface: &InterfaceSpec) -> Vec<crate::declare::InternalFunctionSpec> {
    let mut functions = interface.functions.clone();
    functions.sort_by(|a, b| a.name.cmp(&b.name));
    functions
}

/// Depth-first search over the importer-to-provider edges; returns the
/// modules on the first cycle found, starting and ending at the same module
/// omitted.
fn find_cycle(modules: &[&ModuleName], bindings: &[Binding]) -> Option<Vec<ModuleName>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        Active,
        Done,
    }

    fn visit(
        node: usize,
        modules: &[&ModuleName],
        bindings: &[Binding],
        marks: &mut [Mark],
        stack: &mut Vec<usize>,
    ) -> Option<Vec<ModuleName>> {
        marks[node] = Mark::Active;
        stack.push(node);
        for binding in bindings
            .iter()
            .filter(|binding| &binding.importer == modules[node])
        {
            let Some(next) = modules
                .iter()
                .position(|module| **module == binding.provider)
            else {
                continue;
            };
            match marks[next] {
                Mark::Active => {
                    let start = stack.iter().position(|&index| index == next).unwrap_or(0);
                    return Some(
                        stack[start..]
                            .iter()
                            .map(|&index| modules[index].clone())
                            .collect(),
                    );
                }
                Mark::Unvisited => {
                    if let Some(cycle) = visit(next, modules, bindings, marks, stack) {
                        return Some(cycle);
                    }
                }
                Mark::Done => {}
            }
        }
        stack.pop();
        marks[node] = Mark::Done;
        None
    }

    let mut marks = alloc::vec![Mark::Unvisited; modules.len()];
    let mut stack = Vec::new();
    for node in 0..modules.len() {
        if marks[node] == Mark::Unvisited {
            if let Some(cycle) = visit(node, modules, bindings, &mut marks, &mut stack) {
                return Some(cycle);
            }
        }
    }
    None
}
