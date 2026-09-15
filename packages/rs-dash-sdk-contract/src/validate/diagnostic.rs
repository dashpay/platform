//! Diagnostics: typed, append-only, with stable codes.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::declare::{CapabilityRequirement, DeclarationOrigin};
use crate::identity::InvalidName;

/// Where in the declaration a diagnostic points.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationPath {
    /// The contract as a whole.
    Contract,
    /// An attribute, optionally one of its options. Nested option lists use
    /// dotted attribute names (`index.time_range`).
    Attribute {
        /// The attribute name.
        attribute: String,
        /// The option name, when the diagnostic is about one option.
        option: Option<String>,
    },
    /// A module.
    Module(String),
    /// An interface.
    Interface(String),
    /// A document collection or singleton.
    Collection(String),
    /// A stored field, by dotted path.
    Field {
        /// The collection.
        collection: String,
        /// The dotted field path.
        field: String,
    },
    /// An index.
    Index {
        /// The collection.
        collection: String,
        /// The index name.
        index: String,
    },
    /// A typed specialized collection.
    TypedCollection(String),
    /// An entry.
    Entry(String),
    /// An entry parameter or its return value.
    EntryParam {
        /// The entry.
        entry: String,
        /// The parameter name, or `return`.
        param: String,
    },
    /// A rule.
    Rule {
        /// The collection.
        collection: String,
        /// The rule name.
        rule: String,
    },
    /// A capability requirement.
    Capability(CapabilityRequirement),
    /// An interface function parameter or its return value.
    InterfaceParam {
        /// The interface.
        interface: String,
        /// The function.
        function: String,
        /// The parameter name, or `return`.
        param: String,
    },
}

impl DeclarationPath {
    /// An attribute path.
    pub fn attribute(attribute: &str, option: Option<&str>) -> Self {
        DeclarationPath::Attribute {
            attribute: attribute.to_string(),
            option: option.map(|option| option.to_string()),
        }
    }

    /// A collection path.
    pub fn collection(name: impl AsRef<str>) -> Self {
        DeclarationPath::Collection(name.as_ref().to_string())
    }

    /// A field path.
    pub fn field(collection: impl AsRef<str>, field: impl AsRef<str>) -> Self {
        DeclarationPath::Field {
            collection: collection.as_ref().to_string(),
            field: field.as_ref().to_string(),
        }
    }

    /// An index path.
    pub fn index(collection: impl AsRef<str>, index: impl AsRef<str>) -> Self {
        DeclarationPath::Index {
            collection: collection.as_ref().to_string(),
            index: index.as_ref().to_string(),
        }
    }

    /// An entry path.
    pub fn entry(name: impl AsRef<str>) -> Self {
        DeclarationPath::Entry(name.as_ref().to_string())
    }

    /// An entry parameter path.
    pub fn entry_param(entry: impl AsRef<str>, param: impl AsRef<str>) -> Self {
        DeclarationPath::EntryParam {
            entry: entry.as_ref().to_string(),
            param: param.as_ref().to_string(),
        }
    }

    /// A rule path.
    pub fn rule(collection: impl AsRef<str>, rule: impl AsRef<str>) -> Self {
        DeclarationPath::Rule {
            collection: collection.as_ref().to_string(),
            rule: rule.as_ref().to_string(),
        }
    }

    /// A module path.
    pub fn module(name: impl AsRef<str>) -> Self {
        DeclarationPath::Module(name.as_ref().to_string())
    }

    /// An interface path.
    pub fn interface(name: impl AsRef<str>) -> Self {
        DeclarationPath::Interface(name.as_ref().to_string())
    }

    /// A typed collection path.
    pub fn typed_collection(name: impl AsRef<str>) -> Self {
        DeclarationPath::TypedCollection(name.as_ref().to_string())
    }

    /// An interface function parameter path.
    pub fn interface_param(
        interface: impl AsRef<str>,
        function: impl AsRef<str>,
        param: impl AsRef<str>,
    ) -> Self {
        DeclarationPath::InterfaceParam {
            interface: interface.as_ref().to_string(),
            function: function.as_ref().to_string(),
            param: param.as_ref().to_string(),
        }
    }
}

impl fmt::Display for DeclarationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeclarationPath::Contract => f.write_str("contract"),
            DeclarationPath::Attribute { attribute, option } => match option {
                Some(option) => write!(f, "attribute {attribute}, option {option}"),
                None => write!(f, "attribute {attribute}"),
            },
            DeclarationPath::Module(name) => write!(f, "module {name}"),
            DeclarationPath::Interface(name) => write!(f, "interface {name}"),
            DeclarationPath::Collection(name) => write!(f, "collection {name}"),
            DeclarationPath::Field { collection, field } => {
                write!(f, "collection {collection}, field {field}")
            }
            DeclarationPath::Index { collection, index } => {
                write!(f, "collection {collection}, index {index}")
            }
            DeclarationPath::TypedCollection(name) => write!(f, "typed collection {name}"),
            DeclarationPath::Entry(name) => write!(f, "entry {name}"),
            DeclarationPath::EntryParam { entry, param } => {
                write!(f, "entry {entry}, parameter {param}")
            }
            DeclarationPath::Rule { collection, rule } => {
                write!(f, "collection {collection}, rule {rule}")
            }
            DeclarationPath::Capability(requirement) => write!(f, "capability {requirement}"),
            DeclarationPath::InterfaceParam {
                interface,
                function,
                param,
            } => write!(
                f,
                "interface {interface}, function {function}, parameter {param}"
            ),
        }
    }
}

/// What went wrong.
///
/// Variants are appended, never reordered or removed: their position is
/// their stable code (`DSC0001` onwards, provisional). The rule each one
/// enforces is on the variant.
// @append_only
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// The attribute is not in the grammar.
    UnknownAttribute {
        /// The attribute as written.
        attribute: String,
    },
    /// The option is not in the attribute's grammar.
    UnknownOption {
        /// The attribute.
        attribute: String,
        /// The option as written.
        option: String,
    },
    /// The option's value has the wrong shape or is outside its closed set.
    InvalidOptionValue {
        /// The attribute.
        attribute: String,
        /// The option.
        option: String,
        /// What was expected.
        reason: String,
    },
    /// A required option is absent.
    MissingOption {
        /// The attribute.
        attribute: String,
        /// The option.
        option: String,
    },
    /// An option is given twice.
    DuplicateOption {
        /// The attribute.
        attribute: String,
        /// The option.
        option: String,
    },
    /// Exactly one option of a group must be present (`guard` or
    /// `predicate` on a rule).
    ExactlyOneOptionRequired {
        /// The attribute.
        attribute: String,
        /// The group.
        options: Vec<String>,
        /// How many were given.
        given: usize,
    },
    /// Two options describe the same thing differently (`average` naming a
    /// different property than `sum`, `range_average` without `average`).
    ConflictingOption {
        /// The two options.
        options: Vec<String>,
        /// Why they conflict.
        reason: String,
    },
    /// An attribute and a builder declare the same item differently. The
    /// origins are reported attribute first and builder second whatever the
    /// declaration order.
    ConflictingDeclaration {
        /// What kind of item.
        what: String,
        /// The attribute origin.
        first: DeclarationOrigin,
        /// The builder origin.
        second: DeclarationOrigin,
    },
    /// Two declarations of the same origin use one collection name.
    DuplicateCollection,
    /// Two declarations of the same origin use one index name in one
    /// collection.
    DuplicateIndex,
    /// Two declarations of the same origin use one method name.
    DuplicateMethod,
    /// Two entries produce the same export symbol.
    DuplicateExportSymbol {
        /// The symbol.
        export: String,
    },
    /// Two declarations of the same origin use one rule name in one
    /// collection.
    DuplicateRule,
    /// Two declarations of the same origin use one module name.
    DuplicateModule,
    /// Two declarations of the same origin use one interface name.
    DuplicateInterface,
    /// Two fields at one nesting level share a name.
    DuplicateProperty,
    /// Two fields at one nesting level share a position.
    DuplicatePosition {
        /// The position.
        position: u32,
    },
    /// Positions at one nesting level are not `0..n`.
    NonContiguousPositions {
        /// The positions as declared, sorted.
        positions: Vec<u32>,
    },
    /// A token cost is declared twice for one action.
    DuplicateTokenCostAction {
        /// The action.
        action: String,
    },
    /// A name fails its identity grammar.
    InvalidName(InvalidName),
    /// A string, byte array or list declares no maximum.
    UnboundedField,
    /// Integer bounds fall outside the declared Rust width, or `min > max`.
    IntegerBoundsOutsideType {
        /// The Rust width.
        width: String,
    },
    /// A bound cannot be expressed in the native schema, which reads bounds
    /// as signed 64-bit integers; omit the bound to get the full width.
    IntegerBoundNotNativelyRepresentable {
        /// The bound.
        bound: i128,
    },
    /// An index names a property the collection does not declare.
    IndexPropertyUnknown {
        /// The property path.
        property: String,
    },
    /// A sum names a property the collection does not declare.
    SummablePropertyUnknown {
        /// The property.
        property: String,
    },
    /// A contested field match names a property the index does not cover.
    ContestedFieldNotIndexed {
        /// The property path.
        property: String,
    },
    /// A time range buckets a property that is not the index's first.
    TimeRangeSourceNotFirst {
        /// The property path.
        property: String,
    },
    /// An index-only terminal is neither `$ownerId` nor a declared property.
    TerminalNotAProperty {
        /// The property path.
        property: String,
    },
    /// A ranked level names a property the index does not cover.
    RankedLevelNotIndexed {
        /// The property path.
        property: String,
    },
    /// `terminal`, `preallocated` or `skip_if_absent` on a collection that is
    /// not index-only.
    IndexOnlyOptionOnStoredCollection,
    /// A singleton declares indexes.
    SingletonWithIndexes,
    /// A singleton marks a field `#[document_id]`.
    SingletonWithDocumentIdField,
    /// A persistent struct has no `#[document_id]` field.
    MissingDocumentIdField,
    /// The schema revision is 0.
    InvalidSchemaRevision,
    /// A reference names a collection this contract does not declare.
    ReferenceCollectionUnknown {
        /// The collection.
        collection: String,
    },
    /// A reference names a property that does not exist.
    ReferencePropertyUnknown {
        /// The property path.
        property: String,
    },
    /// An entry receiver names a collection this contract does not declare.
    ReceiverCollectionUnknown {
        /// The collection.
        collection: String,
    },
    /// A `&mut self` entry on a collection whose documents cannot be
    /// replaced: there is no write for the wrapper to stage.
    MutableReceiverOnImmutableCollection {
        /// The collection.
        collection: String,
    },
    /// A read-only entry with a `&mut self` receiver.
    ReadOnlyEntryWithMutableReceiver,
    /// Two parameters of one entry or interface function share a name.
    DuplicateParameter {
        /// The parameter.
        param: String,
    },
    /// An entry binds to a module this contract does not declare.
    EntryModuleUnknown {
        /// The module.
        module: String,
    },
    /// An entry names no module and the contract declares several.
    EntryModuleRequired,
    /// An interface's provider is not a declared module.
    InterfaceProviderUnknown {
        /// The module.
        module: String,
    },
    /// A module uses an interface no declaration provides.
    UsedInterfaceUnknown {
        /// The interface.
        interface: String,
    },
    /// A module uses an interface it provides itself.
    ModuleSelfImport {
        /// The interface.
        interface: String,
    },
    /// The module import graph has a cycle.
    ModuleGraphCycle {
        /// The modules on the cycle, in import order.
        modules: Vec<String>,
    },
    /// Two functions of one interface share a name.
    DuplicateInterfaceFunction {
        /// The function.
        function: String,
    },
    /// A predicate names a module this contract does not declare.
    PredicateModuleUnknown {
        /// The module.
        module: String,
    },
    /// A rule guards a collection this contract does not declare.
    RuleCollectionUnknown {
        /// The collection.
        collection: String,
    },
    /// A rule names no action.
    RuleWithoutActions,
    /// A guard reads a property the guarded collection does not declare.
    GuardFieldUnknown {
        /// The property path.
        property: String,
    },
    /// A capability whose interface is disabled is required (the private
    /// document store).
    CapabilityInterfaceDisabled {
        /// The requirement.
        requirement: CapabilityRequirement,
    },
    /// A derived capability is listed as an explicit requirement.
    CapabilityNotDeclarable {
        /// The requirement.
        requirement: CapabilityRequirement,
    },
    /// Reserved and never produced: the model has no raw path, raw element or
    /// database handle grammar, so there is nothing to reject. The variant
    /// documents that absence.
    RawPathDeclaration,
    /// A string or byte array declares a minimum above its maximum.
    LengthBoundsInverted {
        /// The minimum.
        min: u16,
        /// The maximum.
        max: u16,
    },
    /// Two members of one wire struct share a name.
    DuplicateStructMember {
        /// The member.
        member: String,
    },
    /// A contested index names one field match property twice.
    DuplicateContestedField {
        /// The property path.
        property: String,
    },
    /// A reference agreement names one referring property twice.
    DuplicateAgreementProperty {
        /// The property path.
        property: String,
    },
}

impl DiagnosticKind {
    fn code_and_name(&self) -> (&'static str, &'static str) {
        match self {
            DiagnosticKind::UnknownAttribute { .. } => ("DSC0001", "UnknownAttribute"),
            DiagnosticKind::UnknownOption { .. } => ("DSC0002", "UnknownOption"),
            DiagnosticKind::InvalidOptionValue { .. } => ("DSC0003", "InvalidOptionValue"),
            DiagnosticKind::MissingOption { .. } => ("DSC0004", "MissingOption"),
            DiagnosticKind::DuplicateOption { .. } => ("DSC0005", "DuplicateOption"),
            DiagnosticKind::ExactlyOneOptionRequired { .. } => {
                ("DSC0006", "ExactlyOneOptionRequired")
            }
            DiagnosticKind::ConflictingOption { .. } => ("DSC0007", "ConflictingOption"),
            DiagnosticKind::ConflictingDeclaration { .. } => ("DSC0008", "ConflictingDeclaration"),
            DiagnosticKind::DuplicateCollection => ("DSC0009", "DuplicateCollection"),
            DiagnosticKind::DuplicateIndex => ("DSC0010", "DuplicateIndex"),
            DiagnosticKind::DuplicateMethod => ("DSC0011", "DuplicateMethod"),
            DiagnosticKind::DuplicateExportSymbol { .. } => ("DSC0012", "DuplicateExportSymbol"),
            DiagnosticKind::DuplicateRule => ("DSC0013", "DuplicateRule"),
            DiagnosticKind::DuplicateModule => ("DSC0014", "DuplicateModule"),
            DiagnosticKind::DuplicateInterface => ("DSC0015", "DuplicateInterface"),
            DiagnosticKind::DuplicateProperty => ("DSC0016", "DuplicateProperty"),
            DiagnosticKind::DuplicatePosition { .. } => ("DSC0017", "DuplicatePosition"),
            DiagnosticKind::NonContiguousPositions { .. } => ("DSC0018", "NonContiguousPositions"),
            DiagnosticKind::DuplicateTokenCostAction { .. } => {
                ("DSC0019", "DuplicateTokenCostAction")
            }
            DiagnosticKind::InvalidName(_) => ("DSC0020", "InvalidName"),
            DiagnosticKind::UnboundedField => ("DSC0021", "UnboundedField"),
            DiagnosticKind::IntegerBoundsOutsideType { .. } => {
                ("DSC0022", "IntegerBoundsOutsideType")
            }
            DiagnosticKind::IntegerBoundNotNativelyRepresentable { .. } => {
                ("DSC0023", "IntegerBoundNotNativelyRepresentable")
            }
            DiagnosticKind::IndexPropertyUnknown { .. } => ("DSC0024", "IndexPropertyUnknown"),
            DiagnosticKind::SummablePropertyUnknown { .. } => {
                ("DSC0025", "SummablePropertyUnknown")
            }
            DiagnosticKind::ContestedFieldNotIndexed { .. } => {
                ("DSC0026", "ContestedFieldNotIndexed")
            }
            DiagnosticKind::TimeRangeSourceNotFirst { .. } => {
                ("DSC0027", "TimeRangeSourceNotFirst")
            }
            DiagnosticKind::TerminalNotAProperty { .. } => ("DSC0028", "TerminalNotAProperty"),
            DiagnosticKind::RankedLevelNotIndexed { .. } => ("DSC0029", "RankedLevelNotIndexed"),
            DiagnosticKind::IndexOnlyOptionOnStoredCollection => {
                ("DSC0030", "IndexOnlyOptionOnStoredCollection")
            }
            DiagnosticKind::SingletonWithIndexes => ("DSC0031", "SingletonWithIndexes"),
            DiagnosticKind::SingletonWithDocumentIdField => {
                ("DSC0032", "SingletonWithDocumentIdField")
            }
            DiagnosticKind::MissingDocumentIdField => ("DSC0033", "MissingDocumentIdField"),
            DiagnosticKind::InvalidSchemaRevision => ("DSC0034", "InvalidSchemaRevision"),
            DiagnosticKind::ReferenceCollectionUnknown { .. } => {
                ("DSC0035", "ReferenceCollectionUnknown")
            }
            DiagnosticKind::ReferencePropertyUnknown { .. } => {
                ("DSC0036", "ReferencePropertyUnknown")
            }
            DiagnosticKind::ReceiverCollectionUnknown { .. } => {
                ("DSC0037", "ReceiverCollectionUnknown")
            }
            DiagnosticKind::MutableReceiverOnImmutableCollection { .. } => {
                ("DSC0038", "MutableReceiverOnImmutableCollection")
            }
            DiagnosticKind::ReadOnlyEntryWithMutableReceiver => {
                ("DSC0039", "ReadOnlyEntryWithMutableReceiver")
            }
            DiagnosticKind::DuplicateParameter { .. } => ("DSC0040", "DuplicateParameter"),
            DiagnosticKind::EntryModuleUnknown { .. } => ("DSC0041", "EntryModuleUnknown"),
            DiagnosticKind::EntryModuleRequired => ("DSC0042", "EntryModuleRequired"),
            DiagnosticKind::InterfaceProviderUnknown { .. } => {
                ("DSC0043", "InterfaceProviderUnknown")
            }
            DiagnosticKind::UsedInterfaceUnknown { .. } => ("DSC0044", "UsedInterfaceUnknown"),
            DiagnosticKind::ModuleSelfImport { .. } => ("DSC0045", "ModuleSelfImport"),
            DiagnosticKind::ModuleGraphCycle { .. } => ("DSC0046", "ModuleGraphCycle"),
            DiagnosticKind::DuplicateInterfaceFunction { .. } => {
                ("DSC0047", "DuplicateInterfaceFunction")
            }
            DiagnosticKind::PredicateModuleUnknown { .. } => ("DSC0048", "PredicateModuleUnknown"),
            DiagnosticKind::RuleCollectionUnknown { .. } => ("DSC0049", "RuleCollectionUnknown"),
            DiagnosticKind::RuleWithoutActions => ("DSC0050", "RuleWithoutActions"),
            DiagnosticKind::GuardFieldUnknown { .. } => ("DSC0051", "GuardFieldUnknown"),
            DiagnosticKind::CapabilityInterfaceDisabled { .. } => {
                ("DSC0052", "CapabilityInterfaceDisabled")
            }
            DiagnosticKind::CapabilityNotDeclarable { .. } => {
                ("DSC0053", "CapabilityNotDeclarable")
            }
            DiagnosticKind::RawPathDeclaration => ("DSC0054", "RawPathDeclaration"),
            DiagnosticKind::LengthBoundsInverted { .. } => ("DSC0055", "LengthBoundsInverted"),
            DiagnosticKind::DuplicateStructMember { .. } => ("DSC0056", "DuplicateStructMember"),
            DiagnosticKind::DuplicateContestedField { .. } => {
                ("DSC0057", "DuplicateContestedField")
            }
            DiagnosticKind::DuplicateAgreementProperty { .. } => {
                ("DSC0058", "DuplicateAgreementProperty")
            }
        }
    }

    /// The stable code, `DSC` followed by four digits. Provisional.
    pub fn code(&self) -> &'static str {
        self.code_and_name().0
    }

    /// The variant name.
    pub fn name(&self) -> &'static str {
        self.code_and_name().1
    }
}

impl fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticKind::UnknownAttribute { attribute } => {
                write!(f, "unknown attribute `{attribute}`")
            }
            DiagnosticKind::UnknownOption { attribute, option } => {
                write!(f, "unknown option `{option}` on `{attribute}`")
            }
            DiagnosticKind::InvalidOptionValue {
                attribute,
                option,
                reason,
            } => write!(f, "invalid value for `{option}` on `{attribute}`: {reason}"),
            DiagnosticKind::MissingOption { attribute, option } => {
                write!(f, "`{attribute}` requires option `{option}`")
            }
            DiagnosticKind::DuplicateOption { attribute, option } => {
                write!(f, "option `{option}` on `{attribute}` is given twice")
            }
            DiagnosticKind::ExactlyOneOptionRequired {
                attribute,
                options,
                given,
            } => write!(
                f,
                "`{attribute}` requires exactly one of {options:?}, {given} given"
            ),
            DiagnosticKind::ConflictingOption { options, reason } => {
                write!(f, "options {options:?} conflict: {reason}")
            }
            DiagnosticKind::ConflictingDeclaration {
                what,
                first,
                second,
            } => write!(
                f,
                "the {what} is declared differently by an {first} and a {second}"
            ),
            DiagnosticKind::DuplicateCollection => f.write_str("collection declared twice"),
            DiagnosticKind::DuplicateIndex => f.write_str("index declared twice"),
            DiagnosticKind::DuplicateMethod => f.write_str("method declared twice"),
            DiagnosticKind::DuplicateExportSymbol { export } => {
                write!(f, "export symbol `{export}` produced twice")
            }
            DiagnosticKind::DuplicateRule => f.write_str("rule declared twice"),
            DiagnosticKind::DuplicateModule => f.write_str("module declared twice"),
            DiagnosticKind::DuplicateInterface => f.write_str("interface declared twice"),
            DiagnosticKind::DuplicateProperty => f.write_str("property declared twice"),
            DiagnosticKind::DuplicatePosition { position } => {
                write!(f, "position {position} used twice")
            }
            DiagnosticKind::NonContiguousPositions { positions } => {
                write!(f, "positions {positions:?} are not contiguous from 0")
            }
            DiagnosticKind::DuplicateTokenCostAction { action } => {
                write!(f, "token cost declared twice for action `{action}`")
            }
            DiagnosticKind::InvalidName(error) => write!(f, "{error}"),
            DiagnosticKind::UnboundedField => {
                f.write_str("variable-length value declares no maximum")
            }
            DiagnosticKind::IntegerBoundsOutsideType { width } => {
                write!(f, "integer bounds do not fit `{width}` or are inverted")
            }
            DiagnosticKind::IntegerBoundNotNativelyRepresentable { bound } => write!(
                f,
                "bound {bound} cannot be expressed natively (bounds are signed 64-bit); omit it to keep the full width"
            ),
            DiagnosticKind::IndexPropertyUnknown { property } => {
                write!(f, "index property `{property}` is not declared")
            }
            DiagnosticKind::SummablePropertyUnknown { property } => {
                write!(f, "summed property `{property}` is not declared")
            }
            DiagnosticKind::ContestedFieldNotIndexed { property } => {
                write!(f, "contested field `{property}` is not an index property")
            }
            DiagnosticKind::TimeRangeSourceNotFirst { property } => {
                write!(f, "time range source `{property}` is not the index's first property")
            }
            DiagnosticKind::TerminalNotAProperty { property } => {
                write!(f, "terminal `{property}` is neither `$ownerId` nor a declared property")
            }
            DiagnosticKind::RankedLevelNotIndexed { property } => {
                write!(f, "ranked level `{property}` is not an index property")
            }
            DiagnosticKind::IndexOnlyOptionOnStoredCollection => {
                f.write_str("terminal, preallocated and skip_if_absent need an index-only collection")
            }
            DiagnosticKind::SingletonWithIndexes => f.write_str("a singleton has no indexes"),
            DiagnosticKind::SingletonWithDocumentIdField => {
                f.write_str("a singleton has no document id field")
            }
            DiagnosticKind::MissingDocumentIdField => {
                f.write_str("a persistent struct needs a `#[document_id]` field")
            }
            DiagnosticKind::InvalidSchemaRevision => f.write_str("schema revision must be at least 1"),
            DiagnosticKind::ReferenceCollectionUnknown { collection } => {
                write!(f, "referenced collection `{collection}` is not declared")
            }
            DiagnosticKind::ReferencePropertyUnknown { property } => {
                write!(f, "referenced property `{property}` is not declared")
            }
            DiagnosticKind::ReceiverCollectionUnknown { collection } => {
                write!(f, "receiver collection `{collection}` is not declared")
            }
            DiagnosticKind::MutableReceiverOnImmutableCollection { collection } => write!(
                f,
                "`&mut self` on collection `{collection}` whose documents cannot be replaced"
            ),
            DiagnosticKind::ReadOnlyEntryWithMutableReceiver => {
                f.write_str("a read-only entry cannot take `&mut self`")
            }
            DiagnosticKind::DuplicateParameter { param } => {
                write!(f, "parameter `{param}` declared twice")
            }
            DiagnosticKind::EntryModuleUnknown { module } => {
                write!(f, "module `{module}` is not declared")
            }
            DiagnosticKind::EntryModuleRequired => {
                f.write_str("the contract declares several modules; name the entry's module")
            }
            DiagnosticKind::InterfaceProviderUnknown { module } => {
                write!(f, "provider module `{module}` is not declared")
            }
            DiagnosticKind::UsedInterfaceUnknown { interface } => {
                write!(f, "interface `{interface}` is not declared")
            }
            DiagnosticKind::ModuleSelfImport { interface } => {
                write!(f, "module uses interface `{interface}` that it provides itself")
            }
            DiagnosticKind::ModuleGraphCycle { modules } => {
                write!(f, "module imports form a cycle: {modules:?}")
            }
            DiagnosticKind::DuplicateInterfaceFunction { function } => {
                write!(f, "interface function `{function}` declared twice")
            }
            DiagnosticKind::PredicateModuleUnknown { module } => {
                write!(f, "predicate module `{module}` is not declared")
            }
            DiagnosticKind::RuleCollectionUnknown { collection } => {
                write!(f, "rule collection `{collection}` is not declared")
            }
            DiagnosticKind::RuleWithoutActions => f.write_str("a rule needs at least one action"),
            DiagnosticKind::GuardFieldUnknown { property } => {
                write!(f, "guard reads property `{property}` that is not declared")
            }
            DiagnosticKind::CapabilityInterfaceDisabled { requirement } => write!(
                f,
                "capability `{requirement}` is catalogued but its interface is disabled"
            ),
            DiagnosticKind::CapabilityNotDeclarable { requirement } => write!(
                f,
                "capability `{requirement}` is derived from declarations and cannot be required explicitly"
            ),
            DiagnosticKind::RawPathDeclaration => {
                f.write_str("raw database paths are not declarable")
            }
            DiagnosticKind::LengthBoundsInverted { min, max } => {
                write!(f, "minimum length {min} is above maximum length {max}")
            }
            DiagnosticKind::DuplicateStructMember { member } => {
                write!(f, "struct member `{member}` declared twice")
            }
            DiagnosticKind::DuplicateContestedField { property } => {
                write!(f, "contested field match `{property}` declared twice")
            }
            DiagnosticKind::DuplicateAgreementProperty { property } => {
                write!(f, "agreement property `{property}` declared twice")
            }
        }
    }
}

/// One diagnostic: where and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Where the diagnostic points.
    pub path: DeclarationPath,
    /// What went wrong.
    pub kind: DiagnosticKind,
}

impl Diagnostic {
    /// A diagnostic at `path`.
    pub fn new(path: DeclarationPath, kind: DiagnosticKind) -> Self {
        Diagnostic { path, kind }
    }

    /// The stable code of the kind.
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Where the diagnostic points.
    pub fn path(&self) -> &DeclarationPath {
        &self.path
    }

    /// Wraps a name grammar failure.
    pub fn invalid_name(path: DeclarationPath, error: InvalidName) -> Self {
        Diagnostic::new(path, DiagnosticKind::InvalidName(error))
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.kind.code(), self.path, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_render_code_path_and_message() {
        let diagnostic = Diagnostic::new(
            DeclarationPath::collection("scores"),
            DiagnosticKind::DuplicateCollection,
        );
        assert_eq!(
            diagnostic.to_string(),
            "DSC0009 at collection scores: collection declared twice"
        );
        assert_eq!(diagnostic.kind.name(), "DuplicateCollection");
    }

    #[test]
    fn should_render_every_path_shape() {
        let paths = [
            DeclarationPath::Contract,
            DeclarationPath::attribute("index", Some("fields")),
            DeclarationPath::attribute("index", None),
            DeclarationPath::module("main"),
            DeclarationPath::interface("math"),
            DeclarationPath::collection("scores"),
            DeclarationPath::field("scores", "profile.age"),
            DeclarationPath::index("scores", "by_class"),
            DeclarationPath::typed_collection("totals"),
            DeclarationPath::entry("score.add"),
            DeclarationPath::entry_param("score.add", "delta"),
            DeclarationPath::rule("scores", "monotonic"),
            DeclarationPath::Capability(CapabilityRequirement::PrivateStore),
            DeclarationPath::interface_param("math", "add", "return"),
        ];
        for path in paths {
            assert!(!path.to_string().is_empty());
        }
    }
}
