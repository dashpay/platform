//! Module, interface and binding tables.

use alloc::vec::Vec;

use crate::declare::InternalFunctionSpec;
use crate::identity::{InterfaceName, ModuleName};

/// One module of the bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleEntry {
    /// The module's identity.
    pub name: ModuleName,
    /// Interfaces it imports, sorted.
    pub uses: Vec<InterfaceName>,
}

/// One interface of the bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceEntry {
    /// The interface's identity.
    pub name: InterfaceName,
    /// The providing module.
    pub provider: ModuleName,
    /// Its functions, sorted by name.
    pub functions: Vec<InternalFunctionSpec>,
}

/// An importer-to-provider binding through an interface.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    /// The importing module.
    pub importer: ModuleName,
    /// The providing module.
    pub provider: ModuleName,
    /// The interface.
    pub interface: InterfaceName,
}

/// The module table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleTable {
    /// Modules, sorted by name. Never empty: a package that declares no
    /// module has the implicit `main`.
    pub modules: Vec<ModuleEntry>,
    /// Interfaces, sorted by name.
    pub interfaces: Vec<InterfaceEntry>,
    /// Bindings, sorted by `(importer, provider, interface)`.
    pub bindings: Vec<Binding>,
}

impl ModuleTable {
    /// The module names.
    pub fn names(&self) -> impl Iterator<Item = &ModuleName> {
        self.modules.iter().map(|module| &module.name)
    }
}
