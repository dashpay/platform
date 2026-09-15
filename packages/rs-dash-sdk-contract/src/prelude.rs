//! Re-exports of the declaration types and builders.

pub use crate::declare::{
    ActionScope, BoundedKeyRequirement, CapabilityRequirement, CapabilityStatus, CollectionKind,
    CollectionSpec, ContestedResolution, ContestedSpec, ContractDeclaration, Countability,
    DeclarationOrigin, EntrySpec, FieldContext, FieldSpec, FieldType, GasPaidBy, GuardExpr,
    IndexOnlySpec, IndexSpec, IntegerBounds, IntegerWidth, InterfaceSpec, InternalFunctionSpec,
    Literal, ModuleSpec, ParamSpec, RankedCount, Ranking, ReceiptPolicy, Receiver, ReferenceTarget,
    RuleKind, RuleSpec, SecurityLevel, Store, TimeRangeSpec, TokenCost, TokenCostEffect,
    TokenCostSpec, TradeMode, TypedCollectionKind, TypedCollectionSpec, ValueType, WritePolicy,
};
pub use crate::identity::{
    entry_export_symbol, CollectionName, IndexName, InterfaceName, InvalidName, MethodName,
    ModuleName, NameKind, PropertyName, PropertyPath, RuleName,
};
pub use crate::manifest::CanonicalManifest;
pub use crate::persistence::{HostManaged, NeverStages, StagingPoint};
pub use crate::validate::{validate, DeclarationPath, Diagnostic, DiagnosticKind};
