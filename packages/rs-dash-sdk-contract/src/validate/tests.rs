//! Validator tests: the sketch, one `should_report_*` test per producible
//! diagnostic, sugar expansion and manifest order independence.

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::declare::*;
use crate::identity::*;
use crate::manifest::CanonicalManifest;
use crate::validate::{validate, DeclarationPath, Diagnostic, DiagnosticKind};

fn collection(name: &str) -> CollectionName {
    CollectionName::new(name).unwrap()
}

fn property(name: &str) -> PropertyName {
    PropertyName::new(name).unwrap()
}

fn path(name: &str) -> PropertyPath {
    PropertyPath::new(name).unwrap()
}

fn index_name(name: &str) -> IndexName {
    IndexName::new(name).unwrap()
}

fn method(name: &str) -> MethodName {
    MethodName::new(name).unwrap()
}

fn module(name: &str) -> ModuleName {
    ModuleName::new(name).unwrap()
}

fn interface(name: &str) -> InterfaceName {
    InterfaceName::new(name).unwrap()
}

fn rule(name: &str) -> RuleName {
    RuleName::new(name).unwrap()
}

/// The `Score` collection of the issue's API sketch, with the ascending-only
/// correction (`points = "desc"` is not expressible natively) and the given
/// write policy.
pub(crate) fn score_collection(write: WritePolicy) -> CollectionSpec {
    CollectionSpec::documents(collection("scores"))
        .with_origin(DeclarationOrigin::Attribute)
        .schema_revision(1)
        .write(write)
        .document_id_field("id")
        .field(FieldSpec::new(property("class"), 0, FieldType::string(64)))
        .field(FieldSpec::new(
            property("points"),
            1,
            FieldType::bounded_integer(IntegerWidth::I64, 0, 1_000_000),
        ))
        .field(FieldSpec::new(property("owner"), 2, FieldType::identity()))
        .index(
            IndexSpec::new(index_name("by_class"), vec![path("class")])
                .with_origin(DeclarationOrigin::Attribute)
                .count()
                .sum(property("points")),
        )
        .index(
            IndexSpec::new(index_name("by_owner"), vec![path("owner")])
                .with_origin(DeclarationOrigin::Attribute),
        )
        .index(
            IndexSpec::new(index_name("ranking"), vec![path("class"), path("points")])
                .with_origin(DeclarationOrigin::Attribute)
                .ranked_count(),
        )
}

/// The sketch's entries: a mutable receiver, a free creator, a free pair edit
/// and a read-only aggregate.
pub(crate) fn score_entries() -> Vec<EntrySpec> {
    vec![
        EntrySpec::new(method("score.add"))
            .with_origin(DeclarationOrigin::Attribute)
            .receiver(Receiver::Mut(collection("scores")))
            .param("delta", ValueType::Integer(IntegerWidth::I64)),
        EntrySpec::new(method("score.create"))
            .with_origin(DeclarationOrigin::Attribute)
            .param("class", ValueType::string(64))
            .param("points", ValueType::Integer(IntegerWidth::I64))
            .returns(ValueType::DocumentId),
        EntrySpec::new(method("score.add_pair"))
            .with_origin(DeclarationOrigin::Attribute)
            .param("first", ValueType::DocumentId)
            .param("second", ValueType::DocumentId),
        EntrySpec::new(method("score.total"))
            .with_origin(DeclarationOrigin::Attribute)
            .read_only(true)
            .param("class", ValueType::string(64))
            .returns(ValueType::Struct(vec![
                ("count".to_string(), ValueType::Integer(IntegerWidth::U64)),
                ("sum".to_string(), ValueType::Integer(IntegerWidth::I64)),
            ])),
    ]
}

pub(crate) fn score_sketch(write: WritePolicy) -> ContractDeclaration {
    let mut declaration = ContractDeclaration::new().collection(score_collection(write));
    for entry in score_entries() {
        declaration = declaration.entry(entry);
    }
    declaration.require(CapabilityRequirement::Acl)
}

fn kinds(diagnostics: &[Diagnostic]) -> Vec<&'static str> {
    diagnostics.iter().map(|d| d.kind.name()).collect()
}

fn expect_diagnostics(declaration: &ContractDeclaration) -> Vec<Diagnostic> {
    match validate(declaration) {
        Ok(_) => panic!("expected diagnostics"),
        Err(diagnostics) => diagnostics,
    }
}

fn expect_manifest(declaration: &ContractDeclaration) -> CanonicalManifest {
    match validate(declaration) {
        Ok(manifest) => manifest,
        Err(diagnostics) => panic!("unexpected diagnostics: {diagnostics:#?}"),
    }
}

fn assert_reports(declaration: &ContractDeclaration, kind: &str) -> Vec<Diagnostic> {
    let diagnostics = expect_diagnostics(declaration);
    assert!(
        kinds(&diagnostics).contains(&kind),
        "expected {kind}, got {:?}",
        kinds(&diagnostics)
    );
    diagnostics
}

fn minimal(name: &str) -> CollectionSpec {
    CollectionSpec::documents(collection(name))
        .document_id_field("id")
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool))
}

#[test]
fn should_validate_the_sketch_with_owner_writes() {
    let manifest = expect_manifest(&score_sketch(WritePolicy::Owner));
    let scores = manifest.collection("scores").unwrap();
    assert_eq!(scores.indexes.len(), 3);
    let by_class = scores.index("by_class").unwrap();
    assert_eq!(by_class.count, Countability::Countable);
    assert_eq!(by_class.sum, Some(property("points")));
    assert_eq!(
        scores.index("ranking").unwrap().ranked.count,
        RankedCount::Terminal
    );
    let names: Vec<&str> = manifest.methods.names().map(|n| n.as_str()).collect();
    assert_eq!(
        names,
        ["score.add", "score.add_pair", "score.create", "score.total"]
    );
    let add = manifest.method("score.add").unwrap();
    assert_eq!(add.module.as_str(), "main");
    assert_eq!(add.export, "dash_entry_score.add");
    assert!(add.takes_document_id);
    assert!(!manifest.method("score.create").unwrap().takes_document_id);
    assert!(manifest.methods.entry("score.total").unwrap().read_only);
    assert!(manifest.capabilities.requires(CapabilityRequirement::Acl));
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::Entries));
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::StoredReceipts));
    assert!(!manifest
        .capabilities
        .requires(CapabilityRequirement::ContractWrites));
    assert!(!manifest
        .capabilities
        .requires(CapabilityRequirement::Modules));
    assert_eq!(manifest.receipts, ReceiptPolicy::Stored);
}

#[test]
fn should_validate_the_sketch_with_contract_writes_as_a_derived_requirement() {
    let manifest = expect_manifest(&score_sketch(WritePolicy::Contract));
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::ContractWrites));
    assert!(manifest
        .capabilities
        .pending()
        .any(|entry| entry.requirement == CapabilityRequirement::ContractWrites));
}

#[test]
fn should_validate_an_empty_declaration_to_the_implicit_module() {
    let manifest = expect_manifest(&ContractDeclaration::new());
    let names: Vec<&str> = manifest.modules.names().map(|n| n.as_str()).collect();
    assert_eq!(names, ["main"]);
    assert!(manifest.methods.entries.is_empty());
    assert!(!manifest
        .capabilities
        .requires(CapabilityRequirement::Entries));
}

// Conflicts and duplicates

#[test]
fn should_report_conflicting_declaration_between_attribute_and_builder() {
    let attribute = minimal("scores").with_origin(DeclarationOrigin::Attribute);
    let builder = minimal("scores")
        .with_origin(DeclarationOrigin::Builder)
        .mutable(false);
    let declaration = ContractDeclaration::new()
        .collection(attribute)
        .collection(builder);
    let diagnostics = assert_reports(&declaration, "ConflictingDeclaration");
    let DiagnosticKind::ConflictingDeclaration { first, second, .. } = &diagnostics[0].kind else {
        panic!()
    };
    assert_eq!(*first, DeclarationOrigin::Attribute);
    assert_eq!(*second, DeclarationOrigin::Builder);
}

#[test]
fn should_report_conflicting_declaration_attribute_first_whatever_the_order() {
    let attribute = minimal("scores").with_origin(DeclarationOrigin::Attribute);
    let builder = minimal("scores")
        .with_origin(DeclarationOrigin::Builder)
        .mutable(false);
    let declaration = ContractDeclaration::new()
        .collection(builder)
        .collection(attribute);
    let diagnostics = expect_diagnostics(&declaration);
    assert_eq!(kinds(&diagnostics), ["ConflictingDeclaration"]);
    let DiagnosticKind::ConflictingDeclaration { first, second, .. } = &diagnostics[0].kind else {
        panic!()
    };
    assert_eq!(*first, DeclarationOrigin::Attribute);
    assert_eq!(*second, DeclarationOrigin::Builder);
}

#[test]
fn should_report_duplicate_builder_declarations_whatever_the_order() {
    let attribute = minimal("scores").with_origin(DeclarationOrigin::Attribute);
    let first = minimal("scores")
        .with_origin(DeclarationOrigin::Builder)
        .index(IndexSpec::new(index_name("by_a"), vec![path("a")]));
    let second = minimal("scores")
        .with_origin(DeclarationOrigin::Builder)
        .index(IndexSpec::new(index_name("by_a_too"), vec![path("a")]));
    let orders = [
        [attribute.clone(), first.clone(), second.clone()],
        [first.clone(), attribute.clone(), second.clone()],
        [first, second, attribute],
    ];
    for order in orders {
        let mut declaration = ContractDeclaration::new();
        for collection in order {
            declaration = declaration.collection(collection);
        }
        let diagnostics = expect_diagnostics(&declaration);
        assert_eq!(kinds(&diagnostics), ["DuplicateCollection"]);
    }
}

#[test]
fn should_report_both_a_duplicate_and_a_conflict_whatever_the_order() {
    let attribute = minimal("scores").with_origin(DeclarationOrigin::Attribute);
    let agreeing = minimal("scores").with_origin(DeclarationOrigin::Builder);
    let disagreeing = minimal("scores")
        .with_origin(DeclarationOrigin::Builder)
        .mutable(false);
    let orders = [
        [attribute.clone(), agreeing.clone(), disagreeing.clone()],
        [attribute.clone(), disagreeing.clone(), agreeing.clone()],
        [disagreeing, agreeing, attribute],
    ];
    for order in orders {
        let mut declaration = ContractDeclaration::new();
        for collection in order {
            declaration = declaration.collection(collection);
        }
        let diagnostics = expect_diagnostics(&declaration);
        assert_eq!(
            kinds(&diagnostics),
            ["DuplicateCollection", "ConflictingDeclaration"]
        );
    }
}

#[test]
fn should_treat_a_reordered_restatement_as_equivalent() {
    let ordered = CollectionSpec::documents(collection("c"))
        .with_origin(DeclarationOrigin::Attribute)
        .document_id_field("id")
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool))
        .field(FieldSpec::new(
            property("nested"),
            1,
            FieldType::Object(vec![
                FieldSpec::new(property("x"), 0, FieldType::Bool),
                FieldSpec::new(property("y"), 1, FieldType::Bool),
            ]),
        ))
        .token_cost(ActionScope::Create, TokenCost::new(0, 5))
        .token_cost(ActionScope::Delete, TokenCost::new(1, 3))
        .index(
            IndexSpec::new(index_name("i"), vec![path("a")])
                .with_origin(DeclarationOrigin::Attribute)
                .count()
                .range_count(true)
                .ranked_count_at(vec![path("a")])
                .contested(ContestedSpec::masternode_vote(vec![(
                    path("a"),
                    "^x".to_string(),
                )])),
        );
    let reordered = CollectionSpec::documents(collection("c"))
        .with_origin(DeclarationOrigin::Builder)
        .document_id_field("id")
        .field(FieldSpec::new(
            property("nested"),
            1,
            FieldType::Object(vec![
                FieldSpec::new(property("y"), 1, FieldType::Bool),
                FieldSpec::new(property("x"), 0, FieldType::Bool),
            ]),
        ))
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool))
        .token_cost(ActionScope::Delete, TokenCost::new(1, 3))
        .token_cost(ActionScope::Create, TokenCost::new(0, 5))
        .index(
            IndexSpec::new(index_name("i"), vec![path("a")])
                .with_origin(DeclarationOrigin::Builder)
                .count()
                .range_count(true)
                .ranked_count_at(vec![path("a")])
                .contested(ContestedSpec::masternode_vote(vec![(
                    path("a"),
                    "^x".to_string(),
                )])),
        );
    let merged = expect_manifest(
        &ContractDeclaration::new()
            .collection(ordered.clone())
            .collection(reordered),
    );
    assert_eq!(
        merged,
        expect_manifest(&ContractDeclaration::new().collection(ordered))
    );
}

#[test]
fn should_let_a_builder_extend_an_attribute_collection_with_indexes() {
    let attribute = minimal("scores").with_origin(DeclarationOrigin::Attribute);
    let builder = minimal("scores")
        .with_origin(DeclarationOrigin::Builder)
        .index(IndexSpec::new(index_name("by_a"), vec![path("a")]));
    let declaration = ContractDeclaration::new()
        .collection(attribute)
        .collection(builder);
    let manifest = expect_manifest(&declaration);
    assert_eq!(manifest.collection("scores").unwrap().indexes.len(), 1);
}

#[test]
fn should_report_duplicate_collection() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("scores"))
        .collection(minimal("scores"));
    assert_reports(&declaration, "DuplicateCollection");
}

#[test]
fn should_report_duplicate_collection_between_typed_and_document_collections() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("totals"))
        .typed_collection(TypedCollectionSpec::new(
            collection("totals"),
            TypedCollectionKind::Sum,
            ValueType::Identifier,
            ValueType::Integer(IntegerWidth::I64),
        ));
    assert_reports(&declaration, "DuplicateCollection");
}

#[test]
fn should_report_duplicate_index() {
    let spec = minimal("scores")
        .index(IndexSpec::new(index_name("by_a"), vec![path("a")]))
        .index(IndexSpec::new(index_name("by_a"), vec![path("a")]).unique(true));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "DuplicateIndex",
    );
}

#[test]
fn should_report_duplicate_method() {
    let declaration = ContractDeclaration::new()
        .entry(EntrySpec::new(method("a")))
        .entry(EntrySpec::new(method("a")).read_only(true));
    assert_reports(&declaration, "DuplicateMethod");
}

#[test]
fn should_report_duplicate_rule() {
    let guard = GuardExpr::boolean(true);
    let declaration = ContractDeclaration::new()
        .collection(minimal("scores"))
        .rule(RuleSpec::guard(
            collection("scores"),
            rule("r"),
            vec![ActionScope::Create],
            guard.clone(),
        ))
        .rule(RuleSpec::guard(
            collection("scores"),
            rule("r"),
            vec![ActionScope::Delete],
            guard,
        ));
    assert_reports(&declaration, "DuplicateRule");
}

#[test]
fn should_report_duplicate_module() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .module(ModuleSpec::new(module("main")));
    assert_reports(&declaration, "DuplicateModule");
}

#[test]
fn should_report_duplicate_interface() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .interface(InterfaceSpec::new(interface("math"), module("main")))
        .interface(InterfaceSpec::new(interface("math"), module("main")));
    assert_reports(&declaration, "DuplicateInterface");
}

#[test]
fn should_report_duplicate_property() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool))
        .field(FieldSpec::new(property("a"), 1, FieldType::Bool));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "DuplicateProperty",
    );
}

#[test]
fn should_report_duplicate_position() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool))
        .field(FieldSpec::new(property("b"), 0, FieldType::Bool));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "DuplicatePosition",
    );
}

#[test]
fn should_report_non_contiguous_positions_at_nested_levels() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("profile"),
            0,
            FieldType::Object(vec![
                FieldSpec::new(property("age"), 0, FieldType::integer(IntegerWidth::U8)),
                FieldSpec::new(property("name"), 2, FieldType::string(10)),
            ]),
        ));
    let diagnostics = assert_reports(
        &ContractDeclaration::new().collection(spec),
        "NonContiguousPositions",
    );
    assert_eq!(
        diagnostics[0].path().to_string(),
        "collection c, field profile"
    );
}

#[test]
fn should_report_duplicate_token_cost_action() {
    let spec = minimal("c")
        .token_cost(ActionScope::Create, TokenCost::new(0, 1))
        .token_cost(ActionScope::Create, TokenCost::new(0, 2));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "DuplicateTokenCostAction",
    );
}

#[test]
fn should_keep_export_symbols_distinct_for_distinct_method_names() {
    // The export scheme is injective, so `DuplicateExportSymbol` is a defence
    // against a future scheme change and is not producible today: identical
    // names are caught as a duplicate or conflicting method first.
    let declaration = ContractDeclaration::new()
        .entry(EntrySpec::new(method("a.b")).with_origin(DeclarationOrigin::Attribute))
        .entry(
            EntrySpec::new(method("a.b"))
                .with_origin(DeclarationOrigin::Builder)
                .read_only(true),
        );
    let diagnostics = expect_diagnostics(&declaration);
    assert_eq!(kinds(&diagnostics), ["ConflictingDeclaration"]);
    let declaration = ContractDeclaration::new()
        .entry(EntrySpec::new(method("a.b")))
        .entry(EntrySpec::new(method("a__b")));
    let manifest = expect_manifest(&declaration);
    assert_eq!(manifest.methods.entries.len(), 2);
    assert_ne!(
        manifest.method("a.b").unwrap().export,
        manifest.method("a__b").unwrap().export
    );
}

// Names and bounds

#[test]
fn should_report_invalid_name_through_the_newtypes() {
    let error = CollectionName::new("bad name").unwrap_err();
    let diagnostic = Diagnostic::invalid_name(DeclarationPath::collection("bad name"), error);
    assert_eq!(diagnostic.kind.name(), "InvalidName");
    assert_eq!(diagnostic.code(), "DSC0020");
}

#[test]
fn should_report_unbounded_field_for_strings_bytes_and_lists() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("s"),
            0,
            FieldType::String {
                min_chars: None,
                max_chars: None,
            },
        ))
        .field(FieldSpec::new(
            property("b"),
            1,
            FieldType::Bytes {
                min_len: None,
                max_len: None,
            },
        ));
    let declaration = ContractDeclaration::new().collection(spec).entry(
        EntrySpec::new(method("f"))
            .param(
                "items",
                ValueType::List {
                    max_len: None,
                    item: alloc::boxed::Box::new(ValueType::Bool),
                },
            )
            .returns(ValueType::String { max_chars: None }),
    );
    let diagnostics = expect_diagnostics(&declaration);
    assert_eq!(
        kinds(&diagnostics),
        [
            "UnboundedField",
            "UnboundedField",
            "UnboundedField",
            "UnboundedField"
        ]
    );
}

#[test]
fn should_report_length_bounds_inverted() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("s"),
            0,
            FieldType::String {
                min_chars: Some(9),
                max_chars: Some(8),
            },
        ))
        .field(FieldSpec::new(
            property("b"),
            1,
            FieldType::Bytes {
                min_len: Some(2),
                max_len: Some(1),
            },
        ));
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        ["LengthBoundsInverted", "LengthBoundsInverted"]
    );
}

#[test]
fn should_report_duplicate_struct_member() {
    let declaration = ContractDeclaration::new().entry(EntrySpec::new(method("f")).returns(
        ValueType::Struct(vec![
            ("count".to_string(), ValueType::Bool),
            ("count".to_string(), ValueType::Bool),
        ]),
    ));
    assert_reports(&declaration, "DuplicateStructMember");
}

#[test]
fn should_report_duplicate_contested_field() {
    let spec = minimal("c").index(
        IndexSpec::new(index_name("i"), vec![path("a")])
            .unique(true)
            .contested(ContestedSpec::masternode_vote(vec![
                (path("a"), "^x".to_string()),
                (path("a"), "^y".to_string()),
            ])),
    );
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "DuplicateContestedField",
    );
}

#[test]
fn should_report_integer_bounds_outside_type() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("a"),
            0,
            FieldType::bounded_integer(IntegerWidth::U8, 0, 256),
        ))
        .field(FieldSpec::new(
            property("b"),
            1,
            FieldType::bounded_integer(IntegerWidth::I32, 10, 1),
        ))
        .field(FieldSpec::new(
            property("c"),
            2,
            FieldType::bounded_integer(IntegerWidth::U16, -1, 5),
        ));
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        [
            "IntegerBoundsOutsideType",
            "IntegerBoundsOutsideType",
            "IntegerBoundsOutsideType"
        ]
    );
}

#[test]
fn should_report_integer_bound_not_natively_representable() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("a"),
            0,
            FieldType::bounded_integer(IntegerWidth::U64, 0, i64::MAX as i128 + 1),
        ));
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        ["IntegerBoundNotNativelyRepresentable"]
    );
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("a"),
            0,
            FieldType::integer(IntegerWidth::U64),
        ));
    expect_manifest(&ContractDeclaration::new().collection(spec));
}

#[test]
fn should_report_invalid_schema_revision() {
    let declaration = ContractDeclaration::new().collection(minimal("c").schema_revision(0));
    assert_reports(&declaration, "InvalidSchemaRevision");
}

// Cross-references

#[test]
fn should_report_index_property_unknown() {
    let spec = minimal("c").index(IndexSpec::new(index_name("i"), vec![path("missing")]));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "IndexPropertyUnknown",
    );
}

#[test]
fn should_accept_system_properties_and_nested_paths_in_indexes() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("profile"),
            0,
            FieldType::Object(vec![FieldSpec::new(
                property("age"),
                0,
                FieldType::integer(IntegerWidth::U8),
            )]),
        ))
        .index(IndexSpec::new(
            index_name("i"),
            vec![path("$ownerId"), path("profile.age"), path("$createdAt")],
        ));
    expect_manifest(&ContractDeclaration::new().collection(spec));
}

#[test]
fn should_report_summable_property_unknown_at_index_and_collection_level() {
    let spec = minimal("c")
        .sum(property("missing"))
        .index(IndexSpec::new(index_name("i"), vec![path("a")]).sum(property("gone")));
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        ["SummablePropertyUnknown", "SummablePropertyUnknown"]
    );
}

#[test]
fn should_report_contested_field_not_indexed() {
    let spec = minimal("c").index(
        IndexSpec::new(index_name("i"), vec![path("a")])
            .unique(true)
            .contested(ContestedSpec::masternode_vote(vec![(
                path("other"),
                "^.*$".to_string(),
            )])),
    );
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "ContestedFieldNotIndexed",
    );
}

#[test]
fn should_accept_a_contested_index_next_to_a_create_rule() {
    let spec = CollectionSpec::documents(collection("names"))
        .document_id_field("id")
        .field(FieldSpec::new(property("label"), 0, FieldType::string(63)))
        .index(
            IndexSpec::new(index_name("by_label"), vec![path("label")])
                .unique(true)
                .contested(
                    ContestedSpec::masternode_vote(vec![(
                        path("label"),
                        "^[a-z]{3,19}$".to_string(),
                    )])
                    .description("short names are contested"),
                ),
        );
    let declaration = ContractDeclaration::new()
        .collection(spec)
        .rule(RuleSpec::guard(
            collection("names"),
            rule("label_present"),
            vec![ActionScope::Create],
            GuardExpr::Exists(FieldContext::New, path("label")),
        ));
    let manifest = expect_manifest(&declaration);
    assert_eq!(manifest.rules.len(), 1);
    assert!(manifest
        .collection("names")
        .unwrap()
        .index("by_label")
        .unwrap()
        .contested
        .is_some());
}

#[test]
fn should_report_time_range_source_not_first() {
    let spec = minimal("c").index(
        IndexSpec::new(index_name("i"), vec![path("a"), path("$createdAt")]).time_range(
            TimeRangeSpec {
                on: path("$createdAt"),
                range_secs: 3600,
                step_secs: 60,
                phase_secs: 0,
            },
        ),
    );
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "TimeRangeSourceNotFirst",
    );
}

#[test]
fn should_report_terminal_not_a_property() {
    let spec = CollectionSpec::documents(collection("likes"))
        .document_id_field("id")
        .mutable(false)
        .index_only(true)
        .field(FieldSpec::new(property("post"), 0, FieldType::identity()))
        .index(
            IndexSpec::new(index_name("by_post"), vec![path("post")]).index_only(IndexOnlySpec {
                terminal: Some(path("$createdAt")),
                preallocated: false,
                skip_if_absent: false,
            }),
        )
        .index(
            IndexSpec::new(index_name("by_post2"), vec![path("post")]).index_only(IndexOnlySpec {
                terminal: Some(path("liker")),
                preallocated: false,
                skip_if_absent: false,
            }),
        );
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        ["TerminalNotAProperty", "TerminalNotAProperty"]
    );
}

#[test]
fn should_report_ranked_level_not_indexed() {
    let spec = minimal("c").index(
        IndexSpec::new(index_name("i"), vec![path("a")])
            .count()
            .range_count(true)
            .ranked_count_at(vec![path("missing")]),
    );
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "RankedLevelNotIndexed",
    );
}

#[test]
fn should_report_index_only_option_on_stored_collection() {
    let spec = minimal("c").index(IndexSpec::new(index_name("i"), vec![path("a")]).index_only(
        IndexOnlySpec {
            terminal: None,
            preallocated: true,
            skip_if_absent: false,
        },
    ));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "IndexOnlyOptionOnStoredCollection",
    );
}

#[test]
fn should_report_reference_collection_unknown_for_same_contract_permanent_documents() {
    let spec = CollectionSpec::documents(collection("likes"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("post"),
            0,
            FieldType::Reference(ReferenceTarget::PermanentDocument {
                contract: None,
                document_type: collection("posts"),
                agreement: vec![],
            }),
        ));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "ReferenceCollectionUnknown",
    );
}

#[test]
fn should_report_reference_property_unknown_for_agreements_and_key_ids() {
    let posts = CollectionSpec::documents(collection("posts"))
        .document_id_field("id")
        .deletable(false)
        .field(FieldSpec::new(property("author"), 0, FieldType::identity()));
    let likes = CollectionSpec::documents(collection("likes"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("post"),
            0,
            FieldType::Reference(ReferenceTarget::PermanentDocument {
                contract: None,
                document_type: collection("posts"),
                agreement: vec![(path("missing_here"), path("missing_there"))],
            }),
        ))
        .field(FieldSpec::new(
            property("signer"),
            1,
            FieldType::Reference(ReferenceTarget::IdentityPublicKey {
                key_id_field: path("key_id"),
            }),
        ));
    let diagnostics = expect_diagnostics(
        &ContractDeclaration::new()
            .collection(posts)
            .collection(likes),
    );
    assert_eq!(
        kinds(&diagnostics),
        [
            "ReferencePropertyUnknown",
            "ReferencePropertyUnknown",
            "ReferencePropertyUnknown"
        ]
    );
}

#[test]
fn should_resolve_nested_reference_paths_from_the_document_root() {
    let posts = CollectionSpec::documents(collection("posts"))
        .document_id_field("id")
        .deletable(false)
        .field(FieldSpec::new(property("author"), 0, FieldType::identity()));
    let likes = CollectionSpec::documents(collection("likes"))
        .document_id_field("id")
        .field(FieldSpec::new(property("author"), 0, FieldType::identity()))
        .field(FieldSpec::new(
            property("nested"),
            1,
            FieldType::Object(vec![
                FieldSpec::new(property("key_id"), 0, FieldType::integer(IntegerWidth::U32)),
                FieldSpec::new(
                    property("signer"),
                    1,
                    FieldType::Reference(ReferenceTarget::IdentityPublicKey {
                        key_id_field: path("nested.key_id"),
                    }),
                ),
                FieldSpec::new(
                    property("post"),
                    2,
                    FieldType::Reference(ReferenceTarget::PermanentDocument {
                        contract: None,
                        document_type: collection("posts"),
                        agreement: vec![(path("author"), path("author"))],
                    }),
                ),
            ]),
        ));
    expect_manifest(
        &ContractDeclaration::new()
            .collection(posts)
            .collection(likes),
    );
}

#[test]
fn should_report_reference_property_unknown_for_a_sibling_relative_path() {
    // Native resolves reference paths from the document root, so a nested
    // field naming its sibling without the object prefix is unknown.
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("nested"),
            0,
            FieldType::Object(vec![
                FieldSpec::new(property("key_id"), 0, FieldType::integer(IntegerWidth::U32)),
                FieldSpec::new(
                    property("signer"),
                    1,
                    FieldType::Reference(ReferenceTarget::IdentityPublicKey {
                        key_id_field: path("key_id"),
                    }),
                ),
            ]),
        ));
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(kinds(&diagnostics), ["ReferencePropertyUnknown"]);
}

#[test]
fn should_canonicalize_agreement_order_and_report_duplicate_agreement_property() {
    let posts = CollectionSpec::documents(collection("posts"))
        .document_id_field("id")
        .deletable(false)
        .field(FieldSpec::new(property("a"), 0, FieldType::identity()))
        .field(FieldSpec::new(property("b"), 1, FieldType::identity()));
    let likes = |agreement: Vec<(PropertyPath, PropertyPath)>| {
        CollectionSpec::documents(collection("likes"))
            .document_id_field("id")
            .field(FieldSpec::new(property("a"), 0, FieldType::identity()))
            .field(FieldSpec::new(property("b"), 1, FieldType::identity()))
            .field(FieldSpec::new(
                property("post"),
                2,
                FieldType::Reference(ReferenceTarget::PermanentDocument {
                    contract: None,
                    document_type: collection("posts"),
                    agreement,
                }),
            ))
    };
    let forward = expect_manifest(
        &ContractDeclaration::new()
            .collection(posts.clone())
            .collection(likes(vec![(path("a"), path("a")), (path("b"), path("b"))])),
    );
    let backward = expect_manifest(
        &ContractDeclaration::new()
            .collection(posts.clone())
            .collection(likes(vec![(path("b"), path("b")), (path("a"), path("a"))])),
    );
    assert_eq!(forward, backward);
    let diagnostics = expect_diagnostics(
        &ContractDeclaration::new()
            .collection(posts)
            .collection(likes(vec![(path("a"), path("a")), (path("a"), path("b"))])),
    );
    assert_eq!(kinds(&diagnostics), ["DuplicateAgreementProperty"]);
}

#[test]
fn should_accept_a_cross_contract_permanent_document_reference_without_checking_it() {
    let likes = CollectionSpec::documents(collection("likes"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("post"),
            0,
            FieldType::Reference(ReferenceTarget::PermanentDocument {
                contract: Some([7; 32]),
                document_type: collection("posts"),
                agreement: vec![],
            }),
        ));
    expect_manifest(&ContractDeclaration::new().collection(likes));
}

// Collection kinds

#[test]
fn should_report_singleton_with_indexes() {
    let spec = CollectionSpec::singleton(collection("config"))
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool))
        .index(IndexSpec::new(index_name("i"), vec![path("a")]));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "SingletonWithIndexes",
    );
}

#[test]
fn should_report_singleton_with_document_id_field() {
    let spec = CollectionSpec::singleton(collection("config"))
        .document_id_field("id")
        .field(FieldSpec::new(property("a"), 0, FieldType::Bool));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "SingletonWithDocumentIdField",
    );
}

#[test]
fn should_report_missing_document_id_field() {
    let spec = CollectionSpec::documents(collection("c")).field(FieldSpec::new(
        property("a"),
        0,
        FieldType::Bool,
    ));
    assert_reports(
        &ContractDeclaration::new().collection(spec),
        "MissingDocumentIdField",
    );
}

#[test]
fn should_accept_a_singleton_receiver_without_a_document_id_on_the_wire() {
    let config = CollectionSpec::singleton(collection("config")).field(FieldSpec::new(
        property("a"),
        0,
        FieldType::Bool,
    ));
    let declaration = ContractDeclaration::new().collection(config).entry(
        EntrySpec::new(method("config.set"))
            .receiver(Receiver::Mut(collection("config")))
            .param("a", ValueType::Bool),
    );
    let manifest = expect_manifest(&declaration);
    assert!(!manifest.method("config.set").unwrap().takes_document_id);
}

// Entries

#[test]
fn should_report_receiver_collection_unknown() {
    let declaration = ContractDeclaration::new()
        .entry(EntrySpec::new(method("f")).receiver(Receiver::Ref(collection("missing"))));
    assert_reports(&declaration, "ReceiverCollectionUnknown");
}

#[test]
fn should_report_mutable_receiver_on_immutable_collection() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("frozen").mutable(false))
        .entry(EntrySpec::new(method("f")).receiver(Receiver::Mut(collection("frozen"))));
    assert_reports(&declaration, "MutableReceiverOnImmutableCollection");
    let declaration = ContractDeclaration::new()
        .collection(minimal("frozen").mutable(false))
        .entry(EntrySpec::new(method("f")).receiver(Receiver::Ref(collection("frozen"))));
    expect_manifest(&declaration);
}

#[test]
fn should_report_read_only_entry_with_mutable_receiver() {
    let declaration = ContractDeclaration::new().collection(minimal("c")).entry(
        EntrySpec::new(method("f"))
            .receiver(Receiver::Mut(collection("c")))
            .read_only(true),
    );
    assert_reports(&declaration, "ReadOnlyEntryWithMutableReceiver");
}

#[test]
fn should_report_duplicate_parameter() {
    let declaration = ContractDeclaration::new().entry(
        EntrySpec::new(method("f"))
            .param("a", ValueType::Bool)
            .param("a", ValueType::Bool),
    );
    assert_reports(&declaration, "DuplicateParameter");
}

#[test]
fn should_report_entry_module_unknown() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .entry(EntrySpec::new(method("f")).module(module("helpers")));
    assert_reports(&declaration, "EntryModuleUnknown");
}

#[test]
fn should_report_entry_module_required_with_several_modules() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .module(ModuleSpec::new(module("helpers")))
        .entry(EntrySpec::new(method("f")));
    assert_reports(&declaration, "EntryModuleRequired");
}

// Modules

#[test]
fn should_report_interface_provider_unknown() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .interface(InterfaceSpec::new(interface("math"), module("ghost")));
    assert_reports(&declaration, "InterfaceProviderUnknown");
}

#[test]
fn should_report_used_interface_unknown() {
    let declaration =
        ContractDeclaration::new().module(ModuleSpec::new(module("main")).uses(interface("math")));
    assert_reports(&declaration, "UsedInterfaceUnknown");
}

#[test]
fn should_report_module_self_import() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")).uses(interface("math")))
        .interface(InterfaceSpec::new(interface("math"), module("main")));
    assert_reports(&declaration, "ModuleSelfImport");
}

#[test]
fn should_report_module_graph_cycle() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("a")).uses(interface("from_b")))
        .module(ModuleSpec::new(module("b")).uses(interface("from_c")))
        .module(ModuleSpec::new(module("c")).uses(interface("from_a")))
        .interface(InterfaceSpec::new(interface("from_a"), module("a")))
        .interface(InterfaceSpec::new(interface("from_b"), module("b")))
        .interface(InterfaceSpec::new(interface("from_c"), module("c")));
    let diagnostics = assert_reports(&declaration, "ModuleGraphCycle");
    let DiagnosticKind::ModuleGraphCycle { modules } = &diagnostics[0].kind else {
        panic!()
    };
    assert_eq!(modules.len(), 3);
}

#[test]
fn should_report_duplicate_interface_function() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .interface(
            InterfaceSpec::new(interface("math"), module("main"))
                .function("add", vec![], ValueType::Unit)
                .function("add", vec![], ValueType::Bool),
        );
    assert_reports(&declaration, "DuplicateInterfaceFunction");
}

#[test]
fn should_report_unbounded_field_in_interface_parameters_and_returns() {
    let declaration = ContractDeclaration::new()
        .module(ModuleSpec::new(module("main")))
        .interface(
            InterfaceSpec::new(interface("text"), module("main")).function(
                "join",
                vec![
                    ParamSpec {
                        name: "parts".to_string(),
                        ty: ValueType::list(
                            8,
                            ValueType::Struct(vec![(
                                "text".to_string(),
                                ValueType::String { max_chars: None },
                            )]),
                        ),
                    },
                    ParamSpec {
                        name: "separator".to_string(),
                        ty: ValueType::string(4),
                    },
                ],
                ValueType::option(ValueType::Bytes { max_len: None }),
            ),
        );
    let diagnostics = expect_diagnostics(&declaration);
    assert_eq!(kinds(&diagnostics), ["UnboundedField", "UnboundedField"]);
    assert_eq!(
        diagnostics[0].path().to_string(),
        "interface text, function join, parameter parts"
    );
    assert_eq!(
        diagnostics[1].path().to_string(),
        "interface text, function join, parameter return"
    );
}

#[test]
fn should_bind_a_dag_of_modules_and_sort_bindings() {
    let declaration = ContractDeclaration::new()
        .module(
            ModuleSpec::new(module("main"))
                .uses(interface("math"))
                .uses(interface("text")),
        )
        .module(ModuleSpec::new(module("helpers")).uses(interface("text")))
        .module(ModuleSpec::new(module("core")))
        .interface(InterfaceSpec::new(interface("math"), module("helpers")))
        .interface(InterfaceSpec::new(interface("text"), module("core")))
        .entry(EntrySpec::new(method("f")).module(module("main")));
    let manifest = expect_manifest(&declaration);
    let bindings: Vec<(&str, &str, &str)> = manifest
        .modules
        .bindings
        .iter()
        .map(|b| {
            (
                b.importer.as_str(),
                b.provider.as_str(),
                b.interface.as_str(),
            )
        })
        .collect();
    assert_eq!(
        bindings,
        [
            ("helpers", "core", "text"),
            ("main", "core", "text"),
            ("main", "helpers", "math")
        ]
    );
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::Modules));
}

// Rules

#[test]
fn should_report_predicate_module_unknown() {
    let declaration =
        ContractDeclaration::new()
            .collection(minimal("c"))
            .rule(RuleSpec::predicate(
                collection("c"),
                rule("p"),
                vec![ActionScope::Create],
                module("ghost"),
                "check",
            ));
    assert_reports(&declaration, "PredicateModuleUnknown");
}

#[test]
fn should_report_rule_collection_unknown() {
    let declaration = ContractDeclaration::new().rule(RuleSpec::guard(
        collection("ghost"),
        rule("r"),
        vec![ActionScope::Create],
        GuardExpr::boolean(true),
    ));
    assert_reports(&declaration, "RuleCollectionUnknown");
}

#[test]
fn should_report_rule_without_actions() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("c"))
        .rule(RuleSpec::guard(
            collection("c"),
            rule("r"),
            vec![],
            GuardExpr::boolean(true),
        ));
    assert_reports(&declaration, "RuleWithoutActions");
}

#[test]
fn should_report_guard_field_unknown() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("c"))
        .rule(RuleSpec::guard(
            collection("c"),
            rule("r"),
            vec![ActionScope::Replace],
            GuardExpr::field(FieldContext::New, path("points"))
                .ge(GuardExpr::field(FieldContext::Old, path("a"))),
        ));
    let diagnostics = assert_reports(&declaration, "GuardFieldUnknown");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn should_sort_and_dedupe_rule_actions() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("c"))
        .rule(RuleSpec::guard(
            collection("c"),
            rule("r"),
            vec![
                ActionScope::Delete,
                ActionScope::Create,
                ActionScope::Delete,
            ],
            GuardExpr::boolean(true),
        ));
    let manifest = expect_manifest(&declaration);
    assert_eq!(
        manifest.rules[0].actions,
        [ActionScope::Create, ActionScope::Delete]
    );
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::NativeGuards));
}

// Capabilities

#[test]
fn should_report_capability_interface_disabled_for_the_private_store() {
    let declaration =
        ContractDeclaration::new().collection(minimal("secrets").store(Store::Private));
    let diagnostics = expect_diagnostics(&declaration);
    assert_eq!(kinds(&diagnostics), ["CapabilityInterfaceDisabled"]);
    let DiagnosticKind::CapabilityInterfaceDisabled { requirement } = &diagnostics[0].kind else {
        panic!()
    };
    assert_eq!(*requirement, CapabilityRequirement::PrivateStore);
}

#[test]
fn should_report_capability_not_declarable_for_derived_requirements() {
    let declaration = ContractDeclaration::new().require(CapabilityRequirement::ContractWrites);
    assert_reports(&declaration, "CapabilityNotDeclarable");
}

#[test]
fn should_derive_typed_collection_and_predicate_requirements() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("c"))
        .typed_collection(TypedCollectionSpec::new(
            collection("totals"),
            TypedCollectionKind::Sum,
            ValueType::Identifier,
            ValueType::Integer(IntegerWidth::I64),
        ))
        .rule(RuleSpec::predicate(
            collection("c"),
            rule("p"),
            vec![ActionScope::Create],
            module("main"),
            "check",
        ))
        .receipts(ReceiptPolicy::Disabled);
    let manifest = expect_manifest(&declaration);
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::TypedCollections(
            TypedCollectionKind::Sum
        )));
    assert!(manifest
        .capabilities
        .requires(CapabilityRequirement::WasmPredicates));
    assert!(!manifest
        .capabilities
        .requires(CapabilityRequirement::StoredReceipts));
    assert_eq!(manifest.typed_collections.len(), 1);
}

// Sugar

#[test]
fn should_expand_average_sugar_into_count_and_sum_at_both_levels() {
    let longhand = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("class"), 0, FieldType::string(8)))
        .field(FieldSpec::new(
            property("points"),
            1,
            FieldType::integer(IntegerWidth::I64),
        ))
        .count(true)
        .range_count(true)
        .sum(property("points"))
        .range_sum(true)
        .index(
            IndexSpec::new(index_name("i"), vec![path("class")])
                .count()
                .range_count(true)
                .sum(property("points"))
                .range_sum(true),
        );
    let sugar = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("class"), 0, FieldType::string(8)))
        .field(FieldSpec::new(
            property("points"),
            1,
            FieldType::integer(IntegerWidth::I64),
        ))
        .average(property("points"))
        .range_average(true)
        .index(
            IndexSpec::new(index_name("i"), vec![path("class")])
                .average(property("points"))
                .range_average(true),
        );
    let a = expect_manifest(&ContractDeclaration::new().collection(longhand));
    let b = expect_manifest(&ContractDeclaration::new().collection(sugar));
    assert_eq!(a, b);
    let c = a.collection("c").unwrap();
    assert!(c.count && c.range_count && c.range_sum);
    assert_eq!(c.sum, Some(property("points")));
}

#[test]
fn should_keep_offset_count_when_average_sugar_is_added() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("class"), 0, FieldType::string(8)))
        .field(FieldSpec::new(
            property("points"),
            1,
            FieldType::integer(IntegerWidth::I64),
        ))
        .index(
            IndexSpec::new(index_name("i"), vec![path("class")])
                .count_allowing_offset()
                .average(property("points")),
        );
    let manifest = expect_manifest(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        manifest.collection("c").unwrap().index("i").unwrap().count,
        Countability::CountableAllowingOffset
    );
}

#[test]
fn should_report_conflicting_option_for_average_against_a_different_sum() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(
            property("a"),
            0,
            FieldType::integer(IntegerWidth::I64),
        ))
        .field(FieldSpec::new(
            property("b"),
            1,
            FieldType::integer(IntegerWidth::I64),
        ))
        .sum(property("a"))
        .average(property("b"))
        .index(
            IndexSpec::new(index_name("i"), vec![path("a")])
                .sum(property("a"))
                .average(property("b")),
        );
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        ["ConflictingOption", "ConflictingOption"]
    );
}

#[test]
fn should_report_conflicting_option_for_explicit_false_next_to_average_sugar() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("class"), 0, FieldType::string(8)))
        .field(FieldSpec::new(
            property("points"),
            1,
            FieldType::integer(IntegerWidth::I64),
        ))
        .count(false)
        .range_count(false)
        .range_sum(false)
        .average(property("points"))
        .range_average(true)
        .index(
            IndexSpec::new(index_name("i"), vec![path("class")])
                .countability(Countability::NotCountable)
                .range_count(false)
                .range_sum(false)
                .average(property("points"))
                .range_average(true),
        );
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        [
            "ConflictingOption",
            "ConflictingOption",
            "ConflictingOption",
            "ConflictingOption",
            "ConflictingOption",
            "ConflictingOption"
        ]
    );
    let pairs: Vec<Vec<String>> = diagnostics
        .iter()
        .map(|d| match &d.kind {
            DiagnosticKind::ConflictingOption { options, .. } => options.clone(),
            _ => panic!(),
        })
        .collect();
    assert_eq!(pairs[0], ["average", "count"]);
    assert_eq!(pairs[1], ["range_average", "range_count"]);
    assert_eq!(pairs[2], ["range_average", "range_sum"]);
    assert_eq!(pairs[3], ["average", "count"]);
}

#[test]
fn should_promote_omitted_options_but_keep_explicit_true_next_to_average_sugar() {
    let spec = CollectionSpec::documents(collection("c"))
        .document_id_field("id")
        .field(FieldSpec::new(property("class"), 0, FieldType::string(8)))
        .field(FieldSpec::new(
            property("points"),
            1,
            FieldType::integer(IntegerWidth::I64),
        ))
        .count(true)
        .average(property("points"))
        .index(
            IndexSpec::new(index_name("i"), vec![path("class")])
                .range_count(true)
                .average(property("points"))
                .range_average(true),
        );
    let manifest = expect_manifest(&ContractDeclaration::new().collection(spec));
    let c = manifest.collection("c").unwrap();
    assert!(c.count && !c.range_count && !c.range_sum);
    let i = c.index("i").unwrap();
    assert_eq!(i.count, Countability::Countable);
    assert!(i.range_count && i.range_sum);
}

#[test]
fn should_report_conflicting_option_for_range_average_without_average() {
    let spec = minimal("c")
        .range_average(true)
        .index(IndexSpec::new(index_name("i"), vec![path("a")]).range_average(true));
    let diagnostics = expect_diagnostics(&ContractDeclaration::new().collection(spec));
    assert_eq!(
        kinds(&diagnostics),
        ["ConflictingOption", "ConflictingOption"]
    );
}

// Order independence and identity stability

fn reversed(declaration: &ContractDeclaration) -> ContractDeclaration {
    let mut reversed = declaration.clone();
    reversed.modules.reverse();
    reversed.interfaces.reverse();
    reversed.collections.reverse();
    reversed.typed_collections.reverse();
    reversed.entries.reverse();
    reversed.rules.reverse();
    reversed.capabilities.reverse();
    for collection in &mut reversed.collections {
        collection.fields.reverse();
        collection.indexes.reverse();
        collection.token_costs.reverse();
        for index in &mut collection.indexes {
            if let RankedCount::At(levels) = &mut index.ranked.count {
                levels.reverse();
            }
            if let Some(contested) = &mut index.contested {
                contested.field_matches.reverse();
            }
        }
    }
    for rule in &mut reversed.rules {
        rule.actions.reverse();
    }
    for module in &mut reversed.modules {
        module.uses.reverse();
    }
    reversed
}

fn rich_declaration() -> ContractDeclaration {
    let scores = score_collection(WritePolicy::Owner)
        .token_cost(ActionScope::Create, TokenCost::new(0, 5))
        .token_cost(ActionScope::Delete, TokenCost::new(1, 3))
        .index(
            IndexSpec::new(index_name("prefix"), vec![path("class"), path("owner")])
                .count()
                .range_count(true)
                .ranked_count_at(vec![path("owner"), path("class")])
                .contested(ContestedSpec::masternode_vote(vec![
                    (path("owner"), "^a".to_string()),
                    (path("class"), "^b".to_string()),
                ])),
        );
    let mut declaration = ContractDeclaration::new()
        .module(
            ModuleSpec::new(module("main"))
                .uses(interface("math"))
                .uses(interface("text")),
        )
        .module(ModuleSpec::new(module("helpers")))
        .interface(InterfaceSpec::new(interface("math"), module("helpers")))
        .interface(InterfaceSpec::new(interface("text"), module("helpers")))
        .collection(scores)
        .collection(minimal("audit"))
        .typed_collection(TypedCollectionSpec::new(
            collection("totals"),
            TypedCollectionKind::Sum,
            ValueType::Identifier,
            ValueType::Integer(IntegerWidth::I64),
        ))
        .rule(RuleSpec::guard(
            collection("scores"),
            rule("monotonic"),
            vec![ActionScope::Replace, ActionScope::Create],
            GuardExpr::field(FieldContext::New, path("points"))
                .ge(GuardExpr::field(FieldContext::Old, path("points"))),
        ))
        .rule(RuleSpec::guard(
            collection("scores"),
            rule("bounded"),
            vec![ActionScope::Create],
            GuardExpr::field(FieldContext::New, path("points")).le(GuardExpr::integer(100)),
        ))
        .rule(RuleSpec::predicate(
            collection("scores"),
            rule("checked"),
            vec![ActionScope::Delete],
            module("helpers"),
            "may_delete",
        ))
        .require(CapabilityRequirement::Randomness)
        .require(CapabilityRequirement::Acl);
    for entry in score_entries() {
        declaration = declaration.entry(entry.module(module("main")));
    }
    declaration
}

#[test]
fn should_produce_the_same_manifest_regardless_of_declaration_order() {
    let declaration = rich_declaration();
    let forward = expect_manifest(&declaration);
    let backward = expect_manifest(&reversed(&declaration));
    assert_eq!(forward, backward);
    let rules: Vec<&str> = forward.rules.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(rules, ["bounded", "checked", "monotonic"]);
    let costs: Vec<ActionScope> = forward
        .collection("scores")
        .unwrap()
        .token_costs
        .iter()
        .map(|c| c.action)
        .collect();
    assert_eq!(costs, [ActionScope::Create, ActionScope::Delete]);
}

#[test]
fn should_produce_the_same_manifest_regardless_of_attribute_or_builder_origin() {
    let declaration = rich_declaration();
    let mut swapped = declaration.clone();
    for collection in &mut swapped.collections {
        collection.origin = DeclarationOrigin::Builder;
        for index in &mut collection.indexes {
            index.origin = DeclarationOrigin::Builder;
        }
    }
    for entry in &mut swapped.entries {
        entry.origin = DeclarationOrigin::Builder;
    }
    assert_eq!(expect_manifest(&declaration), expect_manifest(&swapped));
}

#[test]
fn should_keep_method_identity_when_an_entry_moves_between_modules() {
    let declaration = rich_declaration();
    let before = expect_manifest(&declaration);
    let mut moved = declaration.clone();
    for entry in &mut moved.entries {
        if entry.name.as_str() == "score.total" {
            entry.module = Some(module("helpers"));
        }
    }
    let after = expect_manifest(&moved);
    let names_before: Vec<&MethodName> = before.methods.names().collect();
    let names_after: Vec<&MethodName> = after.methods.names().collect();
    assert_eq!(names_before, names_after);
    for (a, b) in before.methods.entries.iter().zip(&after.methods.entries) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.export, b.export);
        assert_eq!(a.params, b.params);
        assert_eq!(a.returns, b.returns);
        if a.name.as_str() == "score.total" {
            assert_eq!(a.module.as_str(), "main");
            assert_eq!(b.module.as_str(), "helpers");
        } else {
            assert_eq!(a.module, b.module);
        }
    }
    let mut after_rebound = after.clone();
    for entry in &mut after_rebound.methods.entries {
        if entry.name.as_str() == "score.total" {
            entry.module = module("main");
        }
    }
    assert_eq!(before, after_rebound);
}

#[test]
fn should_collect_every_diagnostic_instead_of_stopping_at_the_first() {
    let declaration = ContractDeclaration::new()
        .collection(minimal("c").schema_revision(0))
        .entry(EntrySpec::new(method("f")).receiver(Receiver::Ref(collection("ghost"))))
        .rule(RuleSpec::guard(
            collection("ghost"),
            rule("r"),
            vec![],
            GuardExpr::boolean(true),
        ));
    let diagnostics = expect_diagnostics(&declaration);
    assert_eq!(
        kinds(&diagnostics),
        [
            "InvalidSchemaRevision",
            "ReceiverCollectionUnknown",
            "RuleCollectionUnknown",
            "RuleWithoutActions"
        ]
    );
}
