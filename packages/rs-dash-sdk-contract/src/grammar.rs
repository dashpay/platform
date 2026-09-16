//! The attribute grammar as data.
//!
//! [`ATTRIBUTES`] is the single description of every author attribute
//! (`#[persistent]`, `#[index]`, `#[entry]`, ...), the options each one takes
//! and the values those options accept. The proc macros parse against this
//! table, [`check_keys`] reports grammar diagnostics from it, and the book
//! chapter renders it, so the three cannot drift.
//!
//! The grammar is deliberately closed: an option that is not in the table is
//! an [`DiagnosticKind::UnknownOption`] diagnostic, never silently ignored. The
//! index property order admits only `"asc"` because the native document
//! meta-schema admits only ascending index properties; query direction is a
//! per-query choice. The rule action list admits only ordinary document actions
//! because a contested-index award is a native action that no rule can scope.
//!
//! The spellings themselves are provisional under the shared allocation
//! register entry for the Rust macro grammar.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};

/// What a Rust item an attribute may be placed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeTarget {
    /// A struct declaring a collection.
    Struct,
    /// A field of a persistent struct.
    Field,
    /// A free function or an inherent method.
    Function,
    /// A trait declaring an interface between modules.
    Trait,
    /// A Rust module compiled as one WASM module target.
    Module,
    /// The crate root: contract-wide declarations.
    Crate,
}

/// A closed set of string values an option accepts, with the reason the set
/// is closed (shown in the diagnostic when a value is outside it).
#[derive(Debug, PartialEq, Eq)]
pub struct Choices {
    /// The admitted spellings.
    pub allowed: &'static [&'static str],
    /// Why nothing else is admitted.
    pub explain: &'static str,
}

/// The shape of an option's value.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueShape {
    /// A bare flag (`unique`) or an explicit boolean (`mutable = false`).
    Bool,
    /// Any string.
    Str,
    /// One string from a closed set.
    Choice(&'static Choices),
    /// An integer.
    Int,
    /// A list of strings.
    StrList,
    /// A list of strings from a closed set.
    ChoiceList(&'static Choices),
    /// A bare flag or one string from a closed set (`count` / `count = "offset"`).
    BoolOrChoice(&'static Choices),
    /// A bare flag or a list of strings (`ranked_count` / `ranked_count = ["a"]`).
    BoolOrStrList,
    /// A nested option list with its own keys (`contested(...)`).
    Nested(&'static [KeySpec]),
    /// A nested map whose keys are property paths (`fields(class = "asc")`).
    Map(MapValue),
}

/// The value shape of every entry in a [`ValueShape::Map`].
#[derive(Debug, PartialEq, Eq)]
pub enum MapValue {
    /// Any string.
    Str,
    /// One string from a closed set.
    Choice(&'static Choices),
}

/// One option of an attribute.
#[derive(Debug, PartialEq, Eq)]
pub struct KeySpec {
    /// The option name.
    pub name: &'static str,
    /// The value shape.
    pub value: ValueShape,
    /// Whether the option must be present.
    pub required: bool,
    /// What the option means and its default when absent.
    pub doc: &'static str,
}

/// One attribute of the grammar.
#[derive(Debug, PartialEq, Eq)]
pub struct AttributeSpec {
    /// The attribute name as written after `#[`.
    pub name: &'static str,
    /// Where it may be placed.
    pub target: AttributeTarget,
    /// Whether the attribute may appear several times on one item.
    pub repeatable: bool,
    /// The options it takes.
    pub keys: &'static [KeySpec],
    /// Groups of options of which exactly one must be present.
    pub exactly_one_of: &'static [&'static [&'static str]],
}

/// Index property order: ascending only.
pub const ORDER: Choices = Choices {
    allowed: &["asc"],
    explain: "native indexes are ascending only; the direction of a query is chosen per query",
};

/// Ordinary document actions a rule or a token cost may scope.
pub const ACTIONS: Choices = Choices {
    allowed: &[
        "create",
        "replace",
        "delete",
        "transfer",
        "purchase",
        "update_price",
    ],
    explain: "only ordinary document actions have a rule scope; a contested-index award is a native action that no guest rule, guard or predicate can scope",
};

/// Who may create documents of a collection.
pub const WRITE: Choices = Choices {
    allowed: &["any", "owner", "contract"],
    explain: "creation restriction modes; `contract` requires the pending native contract-write authority",
};

/// Marketplace trade mode.
pub const TRADE: Choices = Choices {
    allowed: &["none", "direct_purchase"],
    explain: "native trade modes",
};

/// Signature security level required to write.
pub const SECURITY_LEVEL: Choices = Choices {
    allowed: &["critical", "high", "medium"],
    explain: "native signature security levels",
};

/// Identity bounded key requirement for encryption or decryption.
pub const KEY_REQUIREMENT: Choices = Choices {
    allowed: &["unique", "multiple", "multiple_reference_to_latest"],
    explain: "native storage key requirements",
};

/// Document store kind.
pub const STORE: Choices = Choices {
    allowed: &["public", "private"],
    explain: "`private` is catalogued but its interface is disabled until encryption, key control, query visibility and proof behaviour are specified",
};

/// Count fast-path form on an index.
pub const COUNT: Choices = Choices {
    allowed: &["offset"],
    explain:
        "`count` alone selects a count tree; `count = \"offset\"` selects a provable count tree",
};

/// Reference targets.
pub const REFERS_TO: Choices = Choices {
    allowed: &[
        "identity",
        "contract",
        "token",
        "permanent_document",
        "identity_public_key",
    ],
    explain: "native reference targets",
};

/// Token cost effect.
pub const TOKEN_EFFECT: Choices = Choices {
    allowed: &["transfer_to_contract_owner", "burn"],
    explain: "native token cost effects",
};

/// Who pays gas for a token-priced action.
pub const GAS_PAID_BY: Choices = Choices {
    allowed: &["document_owner", "contract_owner", "prefer_contract_owner"],
    explain: "native gas payer options",
};

/// Contested index resolution.
pub const RESOLUTION: Choices = Choices {
    allowed: &["masternode_vote"],
    explain: "the only native contested resolution",
};

/// Receipt policy.
pub const RECEIPTS: Choices = Choices {
    allowed: &["stored", "disabled"],
    explain: "receipts default to stored and may be disabled by the contract",
};

/// Declarable capability requirements.
pub const REQUIRES: Choices = Choices {
    allowed: &["acl", "randomness"],
    explain:
        "capabilities a contract declares explicitly; the others are derived from its declarations",
};

const CONTESTED_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "field_matches",
        value: ValueShape::Map(MapValue::Str),
        required: false,
        doc: "index property to regular expression the value must match to be contested",
    },
    KeySpec {
        name: "resolution",
        value: ValueShape::Choice(&RESOLUTION),
        required: true,
        doc: "how a contest is resolved",
    },
    KeySpec {
        name: "description",
        value: ValueShape::Str,
        required: false,
        doc: "human description of the contest",
    },
];

const TIME_RANGE_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "on",
        value: ValueShape::Str,
        required: true,
        doc: "the index's first property, a system timestamp",
    },
    KeySpec {
        name: "range_secs",
        value: ValueShape::Int,
        required: true,
        doc: "window length in seconds, a multiple of `step_secs`",
    },
    KeySpec {
        name: "step_secs",
        value: ValueShape::Int,
        required: true,
        doc: "interval between window starts in seconds",
    },
    KeySpec {
        name: "phase_secs",
        value: ValueShape::Int,
        required: false,
        doc: "window grid offset in seconds, default 0",
    },
    KeySpec {
        name: "ttl_secs",
        value: ValueShape::Int,
        required: false,
        doc: "time to live in seconds; entries expire this long after their bucket starts, default indefinite",
    },
];

const PERSISTENT_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "collection",
        value: ValueShape::Str,
        required: true,
        doc: "collection name (native document type name); the collection's identity",
    },
    KeySpec {
        name: "schema",
        value: ValueShape::Int,
        required: false,
        doc: "author-declared schema revision, at least 1, default 1",
    },
    KeySpec {
        name: "write",
        value: ValueShape::Choice(&WRITE),
        required: false,
        doc: "who may create documents, default `any`",
    },
    KeySpec {
        name: "mutable",
        value: ValueShape::Bool,
        required: false,
        doc: "documents may be replaced, default true",
    },
    KeySpec {
        name: "deletable",
        value: ValueShape::Bool,
        required: false,
        doc: "documents may be deleted, default true",
    },
    KeySpec {
        name: "keep_history",
        value: ValueShape::Bool,
        required: false,
        doc: "keep every revision, default false",
    },
    KeySpec {
        name: "keep_transfer_history",
        value: ValueShape::Bool,
        required: false,
        doc: "keep transfer revisions, default false",
    },
    KeySpec {
        name: "keep_purchase_history",
        value: ValueShape::Bool,
        required: false,
        doc: "keep purchase revisions, default false",
    },
    KeySpec {
        name: "keep_pricing_history",
        value: ValueShape::Bool,
        required: false,
        doc: "keep price-update revisions, default false",
    },
    KeySpec {
        name: "transferable",
        value: ValueShape::Bool,
        required: false,
        doc: "documents may be transferred, default false",
    },
    KeySpec {
        name: "trade",
        value: ValueShape::Choice(&TRADE),
        required: false,
        doc: "marketplace mode, default `none`",
    },
    KeySpec {
        name: "security_level",
        value: ValueShape::Choice(&SECURITY_LEVEL),
        required: false,
        doc: "signature security level required to write, default `high`",
    },
    KeySpec {
        name: "encryption_key",
        value: ValueShape::Choice(&KEY_REQUIREMENT),
        required: false,
        doc: "identity encryption bounded key requirement, default none",
    },
    KeySpec {
        name: "decryption_key",
        value: ValueShape::Choice(&KEY_REQUIREMENT),
        required: false,
        doc: "identity decryption bounded key requirement, default none",
    },
    KeySpec {
        name: "count",
        value: ValueShape::Bool,
        required: false,
        doc: "count tree on the primary key, default false",
    },
    KeySpec {
        name: "range_count",
        value: ValueShape::Bool,
        required: false,
        doc: "provable count on the primary key, needs `count`",
    },
    KeySpec {
        name: "sum",
        value: ValueShape::Str,
        required: false,
        doc: "integer property summed on the primary key",
    },
    KeySpec {
        name: "range_sum",
        value: ValueShape::Bool,
        required: false,
        doc: "provable sum on the primary key, needs `sum`",
    },
    KeySpec {
        name: "average",
        value: ValueShape::Str,
        required: false,
        doc: "sugar for `count` plus `sum = <property>`, expanded before the manifest",
    },
    KeySpec {
        name: "range_average",
        value: ValueShape::Bool,
        required: false,
        doc: "sugar for `range_count` plus `range_sum`, expanded before the manifest",
    },
    KeySpec {
        name: "index_only",
        value: ValueShape::Bool,
        required: false,
        doc: "documents live only in their indexes, default false",
    },
    KeySpec {
        name: "requires",
        value: ValueShape::StrList,
        required: false,
        doc: "system properties every document carries, such as `$createdAt`; a time-range index on a system timestamp needs it here",
    },
    KeySpec {
        name: "store",
        value: ValueShape::Choice(&STORE),
        required: false,
        doc: "document store, default `public`",
    },
];

const SINGLETON_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "collection",
        value: ValueShape::Str,
        required: true,
        doc: "collection name; the singleton's identity",
    },
    KeySpec {
        name: "schema",
        value: ValueShape::Int,
        required: false,
        doc: "author-declared schema revision, at least 1, default 1",
    },
    KeySpec {
        name: "write",
        value: ValueShape::Choice(&WRITE),
        required: false,
        doc: "who may write the singleton, default `any`",
    },
    KeySpec {
        name: "security_level",
        value: ValueShape::Choice(&SECURITY_LEVEL),
        required: false,
        doc: "signature security level required to write, default `high`",
    },
    KeySpec {
        name: "encryption_key",
        value: ValueShape::Choice(&KEY_REQUIREMENT),
        required: false,
        doc: "identity encryption bounded key requirement, default none",
    },
    KeySpec {
        name: "decryption_key",
        value: ValueShape::Choice(&KEY_REQUIREMENT),
        required: false,
        doc: "identity decryption bounded key requirement, default none",
    },
    KeySpec {
        name: "requires",
        value: ValueShape::StrList,
        required: false,
        doc: "system properties the record carries, such as `$updatedAt`",
    },
    KeySpec {
        name: "store",
        value: ValueShape::Choice(&STORE),
        required: false,
        doc: "document store, default `public`",
    },
];

const TOKEN_COST_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "on",
        value: ValueShape::Choice(&ACTIONS),
        required: true,
        doc: "the priced action; one token cost per action",
    },
    KeySpec {
        name: "token_position",
        value: ValueShape::Int,
        required: true,
        doc: "token position in the token contract",
    },
    KeySpec {
        name: "amount",
        value: ValueShape::Int,
        required: true,
        doc: "token amount charged",
    },
    KeySpec {
        name: "contract",
        value: ValueShape::Str,
        required: false,
        doc: "token contract id in base58, default the declaring contract",
    },
    KeySpec {
        name: "effect",
        value: ValueShape::Choice(&TOKEN_EFFECT),
        required: false,
        doc: "default `transfer_to_contract_owner`",
    },
    KeySpec {
        name: "gas_paid_by",
        value: ValueShape::Choice(&GAS_PAID_BY),
        required: false,
        doc: "default `document_owner`",
    },
];

const INDEX_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "name",
        value: ValueShape::Str,
        required: true,
        doc: "index name; the index's identity within its collection",
    },
    KeySpec {
        name: "fields",
        value: ValueShape::Map(MapValue::Choice(&ORDER)),
        required: true,
        doc: "indexed property paths in order; system properties are written as string keys",
    },
    KeySpec {
        name: "unique",
        value: ValueShape::Bool,
        required: false,
        doc: "default false",
    },
    KeySpec {
        name: "null_searchable",
        value: ValueShape::Bool,
        required: false,
        doc: "default true",
    },
    KeySpec {
        name: "contested",
        value: ValueShape::Nested(CONTESTED_KEYS),
        required: false,
        doc: "contested index parameters",
    },
    KeySpec {
        name: "count",
        value: ValueShape::BoolOrChoice(&COUNT),
        required: false,
        doc: "count tree; `= \"offset\"` for a provable count tree",
    },
    KeySpec {
        name: "range_count",
        value: ValueShape::Bool,
        required: false,
        doc: "range counts, needs `count`",
    },
    KeySpec {
        name: "sum",
        value: ValueShape::Str,
        required: false,
        doc: "integer property summed at the index",
    },
    KeySpec {
        name: "range_sum",
        value: ValueShape::Bool,
        required: false,
        doc: "range sums, needs `sum`",
    },
    KeySpec {
        name: "average",
        value: ValueShape::Str,
        required: false,
        doc: "sugar for `count` plus `sum = <property>`, expanded before the manifest",
    },
    KeySpec {
        name: "range_average",
        value: ValueShape::Bool,
        required: false,
        doc: "sugar for `range_count` plus `range_sum`, expanded before the manifest",
    },
    KeySpec {
        name: "ranked_count",
        value: ValueShape::BoolOrStrList,
        required: false,
        doc: "count ranking at the terminal level, or at the named prefix levels",
    },
    KeySpec {
        name: "ranked_sum",
        value: ValueShape::Bool,
        required: false,
        doc: "sum ranking at the terminal level",
    },
    KeySpec {
        name: "ranked_average",
        value: ValueShape::Bool,
        required: false,
        doc: "average ranking at the terminal level",
    },
    KeySpec {
        name: "time_range",
        value: ValueShape::Nested(TIME_RANGE_KEYS),
        required: false,
        doc: "bucket the first property into time windows",
    },
    KeySpec {
        name: "terminal",
        value: ValueShape::Str,
        required: false,
        doc: "index-only member key property, default `$ownerId`",
    },
    KeySpec {
        name: "preallocated",
        value: ValueShape::Bool,
        required: false,
        doc: "index-only path preallocation",
    },
    KeySpec {
        name: "skip_if_absent",
        value: ValueShape::Bool,
        required: false,
        doc: "index-only conditional participation on the first property",
    },
];

const FIELD_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "position",
        value: ValueShape::Int,
        required: true,
        doc: "stable serialization position, contiguous from 0 per nesting level",
    },
    KeySpec {
        name: "max_chars",
        value: ValueShape::Int,
        required: false,
        doc: "string bound, required for strings",
    },
    KeySpec {
        name: "min_chars",
        value: ValueShape::Int,
        required: false,
        doc: "string lower bound",
    },
    KeySpec {
        name: "max_len",
        value: ValueShape::Int,
        required: false,
        doc: "byte array bound, required for byte arrays",
    },
    KeySpec {
        name: "min_len",
        value: ValueShape::Int,
        required: false,
        doc: "byte array lower bound",
    },
    KeySpec {
        name: "min",
        value: ValueShape::Int,
        required: false,
        doc: "integer lower bound, must fit the Rust type",
    },
    KeySpec {
        name: "max",
        value: ValueShape::Int,
        required: false,
        doc: "integer upper bound, must fit the Rust type",
    },
    KeySpec {
        name: "values",
        value: ValueShape::StrList,
        required: false,
        doc: "closed set of string values",
    },
    KeySpec {
        name: "required",
        value: ValueShape::Bool,
        required: false,
        doc: "default true",
    },
    KeySpec {
        name: "transient",
        value: ValueShape::Bool,
        required: false,
        doc: "validated but not stored, default false",
    },
    KeySpec {
        name: "refers_to",
        value: ValueShape::Choice(&REFERS_TO),
        required: false,
        doc: "reference target of an identifier field",
    },
    KeySpec {
        name: "document_type",
        value: ValueShape::Str,
        required: false,
        doc: "referenced collection for `permanent_document`",
    },
    KeySpec {
        name: "contract",
        value: ValueShape::Str,
        required: false,
        doc: "referenced contract id in base58 for `permanent_document`, default the declaring contract",
    },
    KeySpec {
        name: "agreement",
        value: ValueShape::Map(MapValue::Str),
        required: false,
        doc: "referring property to referenced property equalities for `permanent_document`",
    },
    KeySpec {
        name: "key_id_field",
        value: ValueShape::Str,
        required: false,
        doc: "property carrying the key id for `identity_public_key`",
    },
    KeySpec {
        name: "description",
        value: ValueShape::Str,
        required: false,
        doc: "human description",
    },
];

const ENTRY_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "name",
        value: ValueShape::Str,
        required: true,
        doc: "method name; the entry's identity across the whole contract",
    },
    KeySpec {
        name: "read_only",
        value: ValueShape::Bool,
        required: false,
        doc: "the entry stages no writes, default false",
    },
    KeySpec {
        name: "module",
        value: ValueShape::Str,
        required: false,
        doc: "hosting WASM module, default the single module",
    },
];

const RULE_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "name",
        value: ValueShape::Str,
        required: true,
        doc: "rule name, unique within the collection",
    },
    KeySpec {
        name: "on",
        value: ValueShape::ChoiceList(&ACTIONS),
        required: true,
        doc: "ordinary actions the rule guards",
    },
    KeySpec {
        name: "guard",
        value: ValueShape::Str,
        required: false,
        doc: "name of a native guard expression constant",
    },
    KeySpec {
        name: "predicate",
        value: ValueShape::Str,
        required: false,
        doc: "read-only WASM predicate as `module::export`",
    },
];

const CONTRACT_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "receipts",
        value: ValueShape::Choice(&RECEIPTS),
        required: false,
        doc: "default `stored`",
    },
    KeySpec {
        name: "requires",
        value: ValueShape::ChoiceList(&REQUIRES),
        required: false,
        doc: "explicitly required capabilities",
    },
];

const MODULE_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "name",
        value: ValueShape::Str,
        required: true,
        doc: "module name; the module's identity",
    },
    KeySpec {
        name: "uses",
        value: ValueShape::StrList,
        required: false,
        doc: "interfaces imported from other modules",
    },
];

const INTERFACE_KEYS: &[KeySpec] = &[
    KeySpec {
        name: "name",
        value: ValueShape::Str,
        required: true,
        doc: "interface name; the interface's identity",
    },
    KeySpec {
        name: "provider",
        value: ValueShape::Str,
        required: true,
        doc: "the module exporting the interface",
    },
];

/// Every attribute of the author grammar.
pub const ATTRIBUTES: &[AttributeSpec] = &[
    AttributeSpec {
        name: "persistent",
        target: AttributeTarget::Struct,
        repeatable: false,
        keys: PERSISTENT_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "singleton",
        target: AttributeTarget::Struct,
        repeatable: false,
        keys: SINGLETON_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "token_cost",
        target: AttributeTarget::Struct,
        repeatable: true,
        keys: TOKEN_COST_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "index",
        target: AttributeTarget::Struct,
        repeatable: true,
        keys: INDEX_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "field",
        target: AttributeTarget::Field,
        repeatable: false,
        keys: FIELD_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "document_id",
        target: AttributeTarget::Field,
        repeatable: false,
        keys: &[],
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "entry",
        target: AttributeTarget::Function,
        repeatable: false,
        keys: ENTRY_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "rule",
        target: AttributeTarget::Struct,
        repeatable: true,
        keys: RULE_KEYS,
        exactly_one_of: &[&["guard", "predicate"]],
    },
    AttributeSpec {
        name: "contract",
        target: AttributeTarget::Crate,
        repeatable: false,
        keys: CONTRACT_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "module",
        target: AttributeTarget::Module,
        repeatable: false,
        keys: MODULE_KEYS,
        exactly_one_of: &[],
    },
    AttributeSpec {
        name: "interface",
        target: AttributeTarget::Trait,
        repeatable: false,
        keys: INTERFACE_KEYS,
        exactly_one_of: &[],
    },
];

/// Looks an attribute up by name.
pub fn attribute(name: &str) -> Option<&'static AttributeSpec> {
    ATTRIBUTES.iter().find(|spec| spec.name == name)
}

/// A value as the macro parsed it, before any typing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GivenValue<'a> {
    /// A bare flag or an explicit boolean.
    Bool(bool),
    /// A string literal.
    Str(&'a str),
    /// An integer literal.
    Int(i128),
    /// A list of string literals.
    StrList(&'a [&'a str]),
    /// A nested option list or map.
    Nested(&'a [GivenOption<'a>]),
}

/// One option as the macro parsed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GivenOption<'a> {
    /// The option name (or the map key).
    pub name: &'a str,
    /// The value.
    pub value: GivenValue<'a>,
}

/// Checks the options given to `attribute` against the grammar and returns
/// every diagnostic: unknown attribute, unknown option, wrong value shape,
/// repeated option, missing required option, and violated exactly-one groups.
///
/// The check is structural. Whether a value makes sense against the rest of
/// the declaration (a property path that exists, a bound inside the Rust type)
/// is the validator's job.
pub fn check_keys(attribute: &str, given: &[GivenOption<'_>]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(spec) = self::attribute(attribute) else {
        diagnostics.push(Diagnostic::new(
            DeclarationPath::attribute(attribute, None),
            DiagnosticKind::UnknownAttribute {
                attribute: attribute.to_string(),
            },
        ));
        return diagnostics;
    };
    check_options(
        attribute,
        spec.keys,
        spec.exactly_one_of,
        given,
        &mut diagnostics,
    );
    diagnostics
}

fn check_options(
    attribute: &str,
    keys: &'static [KeySpec],
    exactly_one_of: &'static [&'static [&'static str]],
    given: &[GivenOption<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen: Vec<&str> = Vec::new();
    for option in given {
        let path = DeclarationPath::attribute(attribute, Some(option.name));
        let Some(key) = keys.iter().find(|key| key.name == option.name) else {
            diagnostics.push(Diagnostic::new(
                path,
                DiagnosticKind::UnknownOption {
                    attribute: attribute.to_string(),
                    option: option.name.to_string(),
                },
            ));
            continue;
        };
        if seen.contains(&option.name) {
            diagnostics.push(Diagnostic::new(
                path,
                DiagnosticKind::DuplicateOption {
                    attribute: attribute.to_string(),
                    option: option.name.to_string(),
                },
            ));
            continue;
        }
        seen.push(option.name);
        if let Err(reason) = check_value(attribute, key, option.value, diagnostics) {
            diagnostics.push(Diagnostic::new(
                path,
                DiagnosticKind::InvalidOptionValue {
                    attribute: attribute.to_string(),
                    option: option.name.to_string(),
                    reason,
                },
            ));
        }
    }
    for key in keys.iter().filter(|key| key.required) {
        if !seen.contains(&key.name) {
            diagnostics.push(Diagnostic::new(
                DeclarationPath::attribute(attribute, Some(key.name)),
                DiagnosticKind::MissingOption {
                    attribute: attribute.to_string(),
                    option: key.name.to_string(),
                },
            ));
        }
    }
    for group in exactly_one_of {
        let present = group.iter().filter(|name| seen.contains(name)).count();
        if present != 1 {
            diagnostics.push(Diagnostic::new(
                DeclarationPath::attribute(attribute, None),
                DiagnosticKind::ExactlyOneOptionRequired {
                    attribute: attribute.to_string(),
                    options: group.iter().map(|name| name.to_string()).collect(),
                    given: present,
                },
            ));
        }
    }
}

fn choice_reason(value: &str, choices: &Choices) -> String {
    let allowed: Vec<String> = choices
        .allowed
        .iter()
        .map(|allowed| format!("\"{allowed}\""))
        .collect();
    format!(
        "value \"{value}\" is not one of [{}]: {}",
        allowed.join(", "),
        choices.explain
    )
}

fn check_choice(value: &str, choices: &Choices) -> Result<(), String> {
    if choices.allowed.contains(&value) {
        Ok(())
    } else {
        Err(choice_reason(value, choices))
    }
}

fn check_value(
    attribute: &str,
    key: &'static KeySpec,
    value: GivenValue<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), String> {
    match (&key.value, value) {
        (ValueShape::Bool, GivenValue::Bool(_)) => Ok(()),
        (ValueShape::Bool, _) => Err("expects a bare flag or `= true` / `= false`".to_string()),
        (ValueShape::Str, GivenValue::Str(_)) => Ok(()),
        (ValueShape::Str, _) => Err("expects a string".to_string()),
        (ValueShape::Choice(choices), GivenValue::Str(value)) => check_choice(value, choices),
        (ValueShape::Choice(_), _) => Err("expects a string".to_string()),
        (ValueShape::Int, GivenValue::Int(_)) => Ok(()),
        (ValueShape::Int, _) => Err("expects an integer".to_string()),
        (ValueShape::StrList, GivenValue::StrList(_)) => Ok(()),
        (ValueShape::StrList, _) => Err("expects a list of strings".to_string()),
        (ValueShape::ChoiceList(choices), GivenValue::StrList(values)) => values
            .iter()
            .try_for_each(|value| check_choice(value, choices)),
        (ValueShape::ChoiceList(_), _) => Err("expects a list of strings".to_string()),
        (ValueShape::BoolOrChoice(_), GivenValue::Bool(_)) => Ok(()),
        (ValueShape::BoolOrChoice(choices), GivenValue::Str(value)) => check_choice(value, choices),
        (ValueShape::BoolOrChoice(_), _) => {
            Err("expects a bare flag, a boolean or a string".to_string())
        }
        (ValueShape::BoolOrStrList, GivenValue::Bool(_) | GivenValue::StrList(_)) => Ok(()),
        (ValueShape::BoolOrStrList, _) => {
            Err("expects a bare flag, a boolean or a list of strings".to_string())
        }
        (ValueShape::Nested(keys), GivenValue::Nested(options)) => {
            let nested = format!("{attribute}.{}", key.name);
            check_options(&nested, keys, &[], options, diagnostics);
            Ok(())
        }
        (ValueShape::Nested(_), _) => Err("expects a nested option list".to_string()),
        (ValueShape::Map(map_value), GivenValue::Nested(entries)) => {
            if entries.is_empty() {
                return Err("expects at least one entry".to_string());
            }
            let mut seen: Vec<&str> = Vec::new();
            for entry in entries {
                if seen.contains(&entry.name) {
                    return Err(format!("key {} is repeated", entry.name));
                }
                seen.push(entry.name);
                match (map_value, entry.value) {
                    (MapValue::Str, GivenValue::Str(_)) => {}
                    (MapValue::Choice(choices), GivenValue::Str(value)) => {
                        check_choice(value, choices)?;
                    }
                    _ => {
                        return Err(format!("key {} expects a string value", entry.name));
                    }
                }
            }
            Ok(())
        }
        (ValueShape::Map(_), _) => Err("expects a nested map".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A snapshot of the grammar, attribute by attribute, so an unintended
    /// option change fails here. The book chapter's table is checked against
    /// `ATTRIBUTES` by the `book_table` integration test.
    const GRAMMAR_SNAPSHOT: &[(&str, &[&str])] = &[
        (
            "persistent",
            &[
                "collection",
                "schema",
                "write",
                "mutable",
                "deletable",
                "keep_history",
                "keep_transfer_history",
                "keep_purchase_history",
                "keep_pricing_history",
                "transferable",
                "trade",
                "security_level",
                "encryption_key",
                "decryption_key",
                "count",
                "range_count",
                "sum",
                "range_sum",
                "average",
                "range_average",
                "index_only",
                "requires",
                "store",
            ],
        ),
        (
            "singleton",
            &[
                "collection",
                "schema",
                "write",
                "security_level",
                "encryption_key",
                "decryption_key",
                "requires",
                "store",
            ],
        ),
        (
            "token_cost",
            &[
                "on",
                "token_position",
                "amount",
                "contract",
                "effect",
                "gas_paid_by",
            ],
        ),
        (
            "index",
            &[
                "name",
                "fields",
                "unique",
                "null_searchable",
                "contested",
                "count",
                "range_count",
                "sum",
                "range_sum",
                "average",
                "range_average",
                "ranked_count",
                "ranked_sum",
                "ranked_average",
                "time_range",
                "terminal",
                "preallocated",
                "skip_if_absent",
            ],
        ),
        (
            "field",
            &[
                "position",
                "max_chars",
                "min_chars",
                "max_len",
                "min_len",
                "min",
                "max",
                "values",
                "required",
                "transient",
                "refers_to",
                "document_type",
                "contract",
                "agreement",
                "key_id_field",
                "description",
            ],
        ),
        ("document_id", &[]),
        ("entry", &["name", "read_only", "module"]),
        ("rule", &["name", "on", "guard", "predicate"]),
        ("contract", &["receipts", "requires"]),
        ("module", &["name", "uses"]),
        ("interface", &["name", "provider"]),
    ];

    fn kinds(diagnostics: &[Diagnostic]) -> Vec<&'static str> {
        diagnostics.iter().map(|d| d.kind.name()).collect()
    }

    #[test]
    fn should_match_the_grammar_snapshot() {
        assert_eq!(ATTRIBUTES.len(), GRAMMAR_SNAPSHOT.len());
        for (name, options) in GRAMMAR_SNAPSHOT {
            let spec = attribute(name).unwrap_or_else(|| panic!("attribute {name} missing"));
            let declared: Vec<&str> = spec.keys.iter().map(|key| key.name).collect();
            assert_eq!(&declared, options, "options of {name}");
        }
    }

    #[test]
    fn should_keep_attribute_and_option_names_unique_and_groups_resolvable() {
        let mut names: Vec<&str> = ATTRIBUTES.iter().map(|spec| spec.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ATTRIBUTES.len());
        for spec in ATTRIBUTES {
            let mut keys: Vec<&str> = spec.keys.iter().map(|key| key.name).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), spec.keys.len(), "options of {}", spec.name);
            for group in spec.exactly_one_of {
                for option in *group {
                    assert!(keys.contains(option), "{option} of {}", spec.name);
                }
            }
        }
    }

    #[test]
    fn should_report_unknown_attribute() {
        let diagnostics = check_keys("persisted", &[]);
        assert_eq!(kinds(&diagnostics), ["UnknownAttribute"]);
        assert_eq!(diagnostics[0].code(), "DSC0001");
    }

    #[test]
    fn should_report_unknown_option() {
        let diagnostics = check_keys(
            "entry",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("score.add"),
                },
                GivenOption {
                    name: "readonly",
                    value: GivenValue::Bool(true),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["UnknownOption"]);
    }

    #[test]
    fn should_report_invalid_option_value_for_wrong_shape() {
        let diagnostics = check_keys(
            "entry",
            &[GivenOption {
                name: "name",
                value: GivenValue::Int(1),
            }],
        );
        assert_eq!(kinds(&diagnostics), ["InvalidOptionValue"]);
    }

    #[test]
    fn should_report_missing_option() {
        let diagnostics = check_keys("entry", &[]);
        assert_eq!(kinds(&diagnostics), ["MissingOption"]);
    }

    #[test]
    fn should_report_duplicate_option() {
        let diagnostics = check_keys(
            "entry",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("a"),
                },
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("b"),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["DuplicateOption"]);
    }

    #[test]
    fn should_reject_descending_index_fields_naming_the_native_rule() {
        let fields = [
            GivenOption {
                name: "class",
                value: GivenValue::Str("asc"),
            },
            GivenOption {
                name: "points",
                value: GivenValue::Str("desc"),
            },
        ];
        let diagnostics = check_keys(
            "index",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("ranking"),
                },
                GivenOption {
                    name: "fields",
                    value: GivenValue::Nested(&fields),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["InvalidOptionValue"]);
        let DiagnosticKind::InvalidOptionValue { reason, .. } = &diagnostics[0].kind else {
            panic!("expected an invalid option value");
        };
        assert!(reason.contains("ascending only"), "{reason}");
    }

    #[test]
    fn should_reject_award_as_a_rule_action() {
        let diagnostics = check_keys(
            "rule",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("no_award"),
                },
                GivenOption {
                    name: "on",
                    value: GivenValue::StrList(&["create", "award"]),
                },
                GivenOption {
                    name: "guard",
                    value: GivenValue::Str("GUARD"),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["InvalidOptionValue"]);
        let DiagnosticKind::InvalidOptionValue { reason, .. } = &diagnostics[0].kind else {
            panic!("expected an invalid option value");
        };
        assert!(reason.contains("award"), "{reason}");
    }

    #[test]
    fn should_report_exactly_one_option_required_for_rule_kind() {
        let both = check_keys(
            "rule",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("r"),
                },
                GivenOption {
                    name: "on",
                    value: GivenValue::StrList(&["create"]),
                },
                GivenOption {
                    name: "guard",
                    value: GivenValue::Str("GUARD"),
                },
                GivenOption {
                    name: "predicate",
                    value: GivenValue::Str("main::check"),
                },
            ],
        );
        assert_eq!(kinds(&both), ["ExactlyOneOptionRequired"]);
        let neither = check_keys(
            "rule",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("r"),
                },
                GivenOption {
                    name: "on",
                    value: GivenValue::StrList(&["create"]),
                },
            ],
        );
        assert_eq!(kinds(&neither), ["ExactlyOneOptionRequired"]);
    }

    #[test]
    fn should_check_nested_options_under_their_dotted_attribute_name() {
        let time_range = [
            GivenOption {
                name: "on",
                value: GivenValue::Str("$createdAt"),
            },
            GivenOption {
                name: "range_secs",
                value: GivenValue::Int(3600),
            },
            GivenOption {
                name: "stepp",
                value: GivenValue::Int(60),
            },
        ];
        let fields = [GivenOption {
            name: "$createdAt",
            value: GivenValue::Str("asc"),
        }];
        let diagnostics = check_keys(
            "index",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("recent"),
                },
                GivenOption {
                    name: "fields",
                    value: GivenValue::Nested(&fields),
                },
                GivenOption {
                    name: "time_range",
                    value: GivenValue::Nested(&time_range),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["UnknownOption", "MissingOption"]);
        assert_eq!(
            diagnostics[0].path().to_string(),
            "attribute index.time_range, option stepp"
        );
    }

    #[test]
    fn should_accept_count_as_flag_and_as_offset_and_ranked_count_as_list() {
        let fields = [GivenOption {
            name: "class",
            value: GivenValue::Str("asc"),
        }];
        let diagnostics = check_keys(
            "index",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("by_class"),
                },
                GivenOption {
                    name: "fields",
                    value: GivenValue::Nested(&fields),
                },
                GivenOption {
                    name: "count",
                    value: GivenValue::Str("offset"),
                },
                GivenOption {
                    name: "ranked_count",
                    value: GivenValue::StrList(&["class"]),
                },
            ],
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let diagnostics = check_keys(
            "index",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("by_class"),
                },
                GivenOption {
                    name: "fields",
                    value: GivenValue::Nested(&fields),
                },
                GivenOption {
                    name: "count",
                    value: GivenValue::Str("provable"),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["InvalidOptionValue"]);
    }

    #[test]
    fn should_reject_an_empty_or_repeated_fields_map() {
        let empty: [GivenOption<'_>; 0] = [];
        let diagnostics = check_keys(
            "index",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("x"),
                },
                GivenOption {
                    name: "fields",
                    value: GivenValue::Nested(&empty),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["InvalidOptionValue"]);
        let repeated = [
            GivenOption {
                name: "class",
                value: GivenValue::Str("asc"),
            },
            GivenOption {
                name: "class",
                value: GivenValue::Str("asc"),
            },
        ];
        let diagnostics = check_keys(
            "index",
            &[
                GivenOption {
                    name: "name",
                    value: GivenValue::Str("x"),
                },
                GivenOption {
                    name: "fields",
                    value: GivenValue::Nested(&repeated),
                },
            ],
        );
        assert_eq!(kinds(&diagnostics), ["InvalidOptionValue"]);
    }

    #[test]
    fn should_accept_the_sketch_persistent_attribute() {
        let diagnostics = check_keys(
            "persistent",
            &[
                GivenOption {
                    name: "collection",
                    value: GivenValue::Str("scores"),
                },
                GivenOption {
                    name: "schema",
                    value: GivenValue::Int(1),
                },
                GivenOption {
                    name: "write",
                    value: GivenValue::Str("contract"),
                },
            ],
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn should_reject_a_private_store_value_outside_the_choice_set_but_accept_private() {
        let private = check_keys(
            "persistent",
            &[
                GivenOption {
                    name: "collection",
                    value: GivenValue::Str("secrets"),
                },
                GivenOption {
                    name: "store",
                    value: GivenValue::Str("private"),
                },
            ],
        );
        assert!(private.is_empty(), "{private:?}");
        let hidden = check_keys(
            "persistent",
            &[
                GivenOption {
                    name: "collection",
                    value: GivenValue::Str("secrets"),
                },
                GivenOption {
                    name: "store",
                    value: GivenValue::Str("hidden"),
                },
            ],
        );
        assert_eq!(kinds(&hidden), ["InvalidOptionValue"]);
    }
}
