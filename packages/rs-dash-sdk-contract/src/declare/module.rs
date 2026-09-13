//! Named WASM modules and the interfaces between them.
//!
//! A contract package may build several WASM module targets. Each entry binds
//! to one module; interfaces declare the functions one module provides to
//! others. The manifest records the module names, the interfaces and the
//! resulting `(importer, provider)` bindings; the graph must be acyclic. A
//! single-module package needs no module declaration: the implicit module is
//! named `main`.

use alloc::string::String;
use alloc::vec::Vec;

use super::entry::{ParamSpec, ValueType};
use super::DeclarationOrigin;
use crate::identity::{InterfaceName, ModuleName};

/// The name of the implicit module of a package that declares none.
/// Provisional.
pub const IMPLICIT_MODULE: &str = "main";

/// A WASM module target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The module's identity.
    pub name: ModuleName,
    /// Interfaces the module imports.
    pub uses: Vec<InterfaceName>,
}

impl ModuleSpec {
    /// A module importing nothing.
    pub fn new(name: ModuleName) -> Self {
        ModuleSpec {
            origin: DeclarationOrigin::Builder,
            name,
            uses: Vec::new(),
        }
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Imports an interface.
    pub fn uses(mut self, interface: InterfaceName) -> Self {
        self.uses.push(interface);
        self
    }
}

/// A function of an interface.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InternalFunctionSpec {
    /// The function name, unique within the interface.
    pub name: String,
    /// Parameters.
    pub params: Vec<ParamSpec>,
    /// Return type.
    pub returns: ValueType,
}

/// An interface one module provides to others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceSpec {
    /// Where the spec came from.
    pub origin: DeclarationOrigin,
    /// The interface's identity.
    pub name: InterfaceName,
    /// The module exporting it.
    pub provider: ModuleName,
    /// Its functions.
    pub functions: Vec<InternalFunctionSpec>,
}

impl InterfaceSpec {
    /// An empty interface provided by `provider`.
    pub fn new(name: InterfaceName, provider: ModuleName) -> Self {
        InterfaceSpec {
            origin: DeclarationOrigin::Builder,
            name,
            provider,
            functions: Vec::new(),
        }
    }

    /// Records the origin.
    pub fn with_origin(mut self, origin: DeclarationOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Adds a function.
    pub fn function(
        mut self,
        name: impl Into<String>,
        params: Vec<ParamSpec>,
        returns: ValueType,
    ) -> Self {
        self.functions.push(InternalFunctionSpec {
            name: name.into(),
            params,
            returns,
        });
        self
    }
}
