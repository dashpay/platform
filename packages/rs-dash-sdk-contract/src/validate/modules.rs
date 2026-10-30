//! Module and interface checks: identity, providers, imports and the acyclic
//! import graph.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::declare::{
    implicit_module, ContractDeclaration, InterfaceSpec, InternalFunctionSpec, ModuleSpec,
};
use crate::identity::{InterfaceName, ModuleName};
use crate::manifest::{Binding, InterfaceEntry, ModuleEntry, ModuleTable};
use crate::validate::collections::check_value_type;
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};
use crate::validate::merge::{dedupe, sorted};

pub(super) fn validate_modules(
    declaration: &ContractDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) -> ModuleTable {
    let modules = dedupe(
        &declaration.modules,
        |spec| DeclarationPath::module(&spec.name),
        diagnostics,
    );
    let interfaces = dedupe(
        &declaration.interfaces,
        |spec| DeclarationPath::interface(&spec.name),
        diagnostics,
    );
    let (mut modules, discarded_modules) = (modules.kept, modules.discarded);
    let (interfaces, discarded_interfaces) = (interfaces.kept, interfaces.discarded);
    if modules.is_empty() {
        modules.push(ModuleSpec::new(implicit_module()));
    }

    let module_names: Vec<&ModuleName> = modules.iter().map(|module| &module.name).collect();

    for interface in interfaces.iter().chain(&discarded_interfaces) {
        check_interface(interface, &module_names, diagnostics);
    }

    let mut bindings: Vec<Binding> = Vec::new();
    for module in &modules {
        for binding in module_bindings(module, &interfaces, diagnostics) {
            if !bindings.contains(&binding) {
                bindings.push(binding);
            }
        }
    }
    for module in &discarded_modules {
        module_bindings(module, &interfaces, diagnostics);
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
    sorted(&module.uses)
}

fn sorted_functions(interface: &InterfaceSpec) -> Vec<InternalFunctionSpec> {
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
                    // An active module is on the stack by construction: it was
                    // pushed when marked and is popped only when marked done.
                    let start = stack
                        .iter()
                        .position(|&index| index == next)
                        .expect("an active module is on the search stack");
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

/// The checks of one interface that need no other interface: its provider is
/// a module, and its functions have unique names, unique parameters and
/// bounded wire types.
fn check_interface(
    interface: &InterfaceSpec,
    module_names: &[&ModuleName],
    diagnostics: &mut Vec<Diagnostic>,
) {
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
            let param_path =
                DeclarationPath::interface_param(&interface.name, &function.name, &param.name);
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
            &DeclarationPath::interface_param(&interface.name, &function.name, "return"),
            &function.returns,
            diagnostics,
        );
    }
}

/// The bindings one module's imports make, with a diagnostic for each import
/// of an unknown interface or of an interface the module provides itself.
fn module_bindings(
    module: &ModuleSpec,
    interfaces: &[InterfaceSpec],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Binding> {
    let mut bindings: Vec<Binding> = Vec::new();
    for used in &module.uses {
        let Some(interface) = interfaces.iter().find(|interface| &interface.name == used) else {
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
    bindings
}
