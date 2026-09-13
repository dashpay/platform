//! Collection checks: identity, fields and positions, bounds, indexes and
//! their cross-references, sugar expansion, singleton rules, typed
//! collections.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::declare::{
    CollectionKind, CollectionSpec, ContractDeclaration, FieldSpec, FieldType, IndexSpec,
    IntegerWidth, RankedCount, ReferenceTarget, TypedCollectionSpec, ValueType,
};
use crate::identity::{CollectionName, PropertyPath};
use crate::manifest::{CollectionManifest, IndexManifest, TypedCollectionManifest};
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};
use crate::validate::merge::dedupe;

/// The collections a contract declares, by name and kind, for the entry and
/// rule checks.
pub(super) type CollectionKinds = Vec<(CollectionName, CollectionKind)>;

pub(super) fn validate_collections(
    declaration: &ContractDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<CollectionManifest>, CollectionKinds) {
    let collections = dedupe(
        &declaration.collections,
        |a, b| a.name == b.name,
        |spec| spec.origin,
        |a, b| a.same_shape_ignoring_indexes(b),
        |kept, next| {
            for index in &next.indexes {
                kept.indexes.push(index.clone());
            }
        },
        |spec| DeclarationPath::collection(&spec.name),
        "collection",
        || DiagnosticKind::DuplicateCollection,
        diagnostics,
    );

    let names: Vec<&CollectionSpec> = collections.iter().collect();
    let mut manifests: Vec<CollectionManifest> = collections
        .iter()
        .map(|collection| validate_collection(collection, &names, diagnostics))
        .collect();
    manifests.sort_by(|a, b| a.name.cmp(&b.name));

    let kinds = manifests
        .iter()
        .map(|manifest| (manifest.name.clone(), manifest.kind))
        .collect();
    (manifests, kinds)
}

fn validate_collection(
    collection: &CollectionSpec,
    all: &[&CollectionSpec],
    diagnostics: &mut Vec<Diagnostic>,
) -> CollectionManifest {
    let path = DeclarationPath::collection(&collection.name);

    if collection.schema_revision == 0 {
        diagnostics.push(Diagnostic::new(
            path.clone(),
            DiagnosticKind::InvalidSchemaRevision,
        ));
    }

    match collection.kind {
        CollectionKind::Documents => {
            if collection.document_id_field.is_none() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::MissingDocumentIdField,
                ));
            }
        }
        CollectionKind::Singleton => {
            if collection.document_id_field.is_some() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::SingletonWithDocumentIdField,
                ));
            }
            if !collection.indexes.is_empty() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::SingletonWithIndexes,
                ));
            }
        }
    }

    let fields = validate_fields(&collection.name, "", &collection.fields, all, diagnostics);

    let mut token_costs = collection.token_costs.clone();
    token_costs.sort_by(|a, b| a.action.cmp(&b.action));
    let mut seen_actions = Vec::new();
    for cost in &token_costs {
        if seen_actions.contains(&cost.action) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::DuplicateTokenCostAction {
                    action: cost.action.to_string(),
                },
            ));
        } else {
            seen_actions.push(cost.action);
        }
    }

    // Collection-level average sugar: `average = p` is `count` plus `sum = p`;
    // `range_average` is `range_count` plus `range_sum`. The same conflict
    // rules the native parser applies to `documentsAverageable`.
    let mut count = collection.count;
    let mut range_count = collection.range_count;
    let mut sum = collection.sum.clone();
    let mut range_sum = collection.range_sum;
    if let Some(average) = &collection.average {
        if let Some(existing) = &sum {
            if existing != average {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::ConflictingOption {
                        options: alloc::vec!["average".to_string(), "sum".to_string()],
                        reason: "both name the summed property, so they must agree".to_string(),
                    },
                ));
            }
        }
        count = true;
        sum = Some(average.clone());
        if collection.range_average {
            range_count = true;
            range_sum = true;
        }
    } else if collection.range_average {
        diagnostics.push(Diagnostic::new(
            path.clone(),
            DiagnosticKind::ConflictingOption {
                options: alloc::vec!["range_average".to_string(), "average".to_string()],
                reason: "range_average needs average to name the property".to_string(),
            },
        ));
    }
    if let Some(summed) = &sum {
        if !has_top_level_property(&fields, summed.as_str()) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::SummablePropertyUnknown {
                    property: summed.to_string(),
                },
            ));
        }
    }

    let indexes = dedupe(
        &collection.indexes,
        |a, b| a.name == b.name,
        |spec| spec.origin,
        |a, b| {
            let mut left = a.clone();
            left.origin = b.origin;
            left == *b
        },
        |_, _| {},
        |spec| DeclarationPath::index(&collection.name, &spec.name),
        "index",
        || DiagnosticKind::DuplicateIndex,
        diagnostics,
    );
    let mut index_manifests: Vec<IndexManifest> = indexes
        .iter()
        .map(|index| validate_index(collection, &fields, index, diagnostics))
        .collect();
    index_manifests.sort_by(|a, b| a.name.cmp(&b.name));

    CollectionManifest {
        name: collection.name.clone(),
        kind: collection.kind,
        schema_revision: collection.schema_revision,
        write: collection.write,
        mutable: collection.mutable,
        deletable: collection.deletable,
        keep_history: collection.keep_history,
        keep_transfer_history: collection.keep_transfer_history,
        keep_purchase_history: collection.keep_purchase_history,
        keep_pricing_history: collection.keep_pricing_history,
        transferable: collection.transferable,
        trade: collection.trade,
        security_level: collection.security_level,
        encryption_key: collection.encryption_key,
        decryption_key: collection.decryption_key,
        count,
        range_count,
        sum,
        range_sum,
        index_only: collection.index_only,
        token_costs,
        store: collection.store,
        fields,
        indexes: index_manifests,
    }
}

fn join_path(prefix: &str, name: &str) -> alloc::string::String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        let mut path = prefix.to_string();
        path.push('.');
        path.push_str(name);
        path
    }
}

/// Validates one nesting level of fields and returns them sorted by position.
fn validate_fields(
    collection: &CollectionName,
    prefix: &str,
    fields: &[FieldSpec],
    all: &[&CollectionSpec],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FieldSpec> {
    let mut seen_names: Vec<&str> = Vec::new();
    let mut positions: Vec<u32> = Vec::new();
    let mut sorted = Vec::new();
    for field in fields {
        let dotted = join_path(prefix, field.name.as_str());
        let path = DeclarationPath::field(collection, &dotted);
        if seen_names.contains(&field.name.as_str()) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::DuplicateProperty,
            ));
        } else {
            seen_names.push(field.name.as_str());
        }
        if positions.contains(&field.position) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::DuplicatePosition {
                    position: field.position,
                },
            ));
        }
        positions.push(field.position);

        let ty = validate_field_type(collection, &dotted, fields, &field.ty, all, diagnostics);
        let mut validated = field.clone();
        validated.ty = ty;
        sorted.push(validated);
    }

    let mut unique_positions = positions.clone();
    unique_positions.sort_unstable();
    unique_positions.dedup();
    let contiguous = unique_positions
        .iter()
        .enumerate()
        .all(|(i, &position)| position as usize == i);
    if !contiguous {
        diagnostics.push(Diagnostic::new(
            if prefix.is_empty() {
                DeclarationPath::collection(collection)
            } else {
                DeclarationPath::field(collection, prefix)
            },
            DiagnosticKind::NonContiguousPositions {
                positions: unique_positions,
            },
        ));
    }

    sorted.sort_by(|a, b| a.position.cmp(&b.position).then(a.name.cmp(&b.name)));
    sorted
}

fn validate_field_type(
    collection: &CollectionName,
    dotted: &str,
    siblings: &[FieldSpec],
    ty: &FieldType,
    all: &[&CollectionSpec],
    diagnostics: &mut Vec<Diagnostic>,
) -> FieldType {
    let path = DeclarationPath::field(collection, dotted);
    match ty {
        FieldType::Integer { width, bounds } => {
            check_integer_bounds(&path, *width, bounds.min, bounds.max, diagnostics);
            ty.clone()
        }
        FieldType::String {
            max_chars: None, ..
        }
        | FieldType::Bytes { max_len: None, .. } => {
            diagnostics.push(Diagnostic::new(path, DiagnosticKind::UnboundedField));
            ty.clone()
        }
        FieldType::Reference(target) => {
            validate_reference(&path, target, siblings, all, diagnostics);
            ty.clone()
        }
        FieldType::Object(nested) => FieldType::Object(validate_fields(
            collection,
            dotted,
            nested,
            all,
            diagnostics,
        )),
        FieldType::Bool
        | FieldType::F64
        | FieldType::String { .. }
        | FieldType::Bytes { .. }
        | FieldType::Identifier
        | FieldType::Enum(_) => ty.clone(),
    }
}

pub(super) fn check_integer_bounds(
    path: &DeclarationPath,
    width: IntegerWidth,
    min: Option<i128>,
    max: Option<i128>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let inside = |bound: i128| bound >= width.min_value() && bound <= width.max_value();
    let mut outside = false;
    for bound in [min, max].into_iter().flatten() {
        if !inside(bound) {
            outside = true;
        } else if bound > i64::MAX as i128 {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::IntegerBoundNotNativelyRepresentable { bound },
            ));
        }
    }
    if let (Some(min), Some(max)) = (min, max) {
        if min > max {
            outside = true;
        }
    }
    if outside {
        diagnostics.push(Diagnostic::new(
            path.clone(),
            DiagnosticKind::IntegerBoundsOutsideType {
                width: width.rust_name().to_string(),
            },
        ));
    }
}

fn validate_reference(
    path: &DeclarationPath,
    target: &ReferenceTarget,
    siblings: &[FieldSpec],
    all: &[&CollectionSpec],
    diagnostics: &mut Vec<Diagnostic>,
) {
    match target {
        ReferenceTarget::Identity | ReferenceTarget::Contract | ReferenceTarget::Token => {}
        ReferenceTarget::PermanentDocument {
            contract,
            document_type,
            agreement,
        } => {
            // Only same-contract references can be checked here; another
            // contract's types are validated natively at registration.
            let referenced = all
                .iter()
                .find(|candidate| candidate.name == *document_type);
            if contract.is_none() {
                match referenced {
                    None => diagnostics.push(Diagnostic::new(
                        path.clone(),
                        DiagnosticKind::ReferenceCollectionUnknown {
                            collection: document_type.to_string(),
                        },
                    )),
                    Some(referenced) => {
                        for (_, referenced_property) in agreement {
                            if !has_property_path(&referenced.fields, referenced_property) {
                                diagnostics.push(Diagnostic::new(
                                    path.clone(),
                                    DiagnosticKind::ReferencePropertyUnknown {
                                        property: referenced_property.to_string(),
                                    },
                                ));
                            }
                        }
                    }
                }
            }
            for (referring_property, _) in agreement {
                if !has_property_path(siblings, referring_property) {
                    diagnostics.push(Diagnostic::new(
                        path.clone(),
                        DiagnosticKind::ReferencePropertyUnknown {
                            property: referring_property.to_string(),
                        },
                    ));
                }
            }
        }
        ReferenceTarget::IdentityPublicKey { key_id_field } => {
            if !has_property_path(siblings, key_id_field) {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::ReferencePropertyUnknown {
                        property: key_id_field.to_string(),
                    },
                ));
            }
        }
    }
}

fn has_top_level_property(fields: &[FieldSpec], name: &str) -> bool {
    fields.iter().any(|field| field.name.as_str() == name)
}

/// Resolves a dotted path through nested objects.
pub(super) fn has_property_path(fields: &[FieldSpec], path: &PropertyPath) -> bool {
    if path.is_system() {
        return true;
    }
    let mut current = fields;
    let mut segments = path.segments().peekable();
    while let Some(segment) = segments.next() {
        let Some(field) = current.iter().find(|field| field.name.as_str() == segment) else {
            return false;
        };
        if segments.peek().is_none() {
            return true;
        }
        match &field.ty {
            FieldType::Object(nested) => current = nested,
            _ => return false,
        }
    }
    false
}

fn validate_index(
    collection: &CollectionSpec,
    fields: &[FieldSpec],
    index: &IndexSpec,
    diagnostics: &mut Vec<Diagnostic>,
) -> IndexManifest {
    let path = DeclarationPath::index(&collection.name, &index.name);

    for property in &index.properties {
        if !has_property_path(fields, property) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::IndexPropertyUnknown {
                    property: property.to_string(),
                },
            ));
        }
    }

    // Index-level average sugar, expanded the way the native parser expands
    // `averageable` / `rangeAverageable`.
    let mut count = index.count;
    let mut range_count = index.range_count;
    let mut sum = index.sum.clone();
    let mut range_sum = index.range_sum;
    if let Some(average) = &index.average {
        if let Some(existing) = &sum {
            if existing != average {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::ConflictingOption {
                        options: alloc::vec!["average".to_string(), "sum".to_string()],
                        reason: "both name the summed property, so they must agree".to_string(),
                    },
                ));
            }
        }
        if !count.is_countable() {
            count = crate::declare::Countability::Countable;
        }
        sum = Some(average.clone());
        if index.range_average {
            range_count = true;
            range_sum = true;
        }
    } else if index.range_average {
        diagnostics.push(Diagnostic::new(
            path.clone(),
            DiagnosticKind::ConflictingOption {
                options: alloc::vec!["range_average".to_string(), "average".to_string()],
                reason: "range_average needs average to name the property".to_string(),
            },
        ));
    }
    if let Some(summed) = &sum {
        if !has_top_level_property(fields, summed.as_str()) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::SummablePropertyUnknown {
                    property: summed.to_string(),
                },
            ));
        }
    }

    let mut contested = index.contested.clone();
    if let Some(contested) = contested.as_mut() {
        for (property, _) in &contested.field_matches {
            if !index.properties.contains(property) {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::ContestedFieldNotIndexed {
                        property: property.to_string(),
                    },
                ));
            }
        }
        contested.field_matches.sort_by(|a, b| a.0.cmp(&b.0));
    }

    if let RankedCount::At(levels) = &index.ranked.count {
        for level in levels {
            if !index.properties.contains(level) {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::RankedLevelNotIndexed {
                        property: level.to_string(),
                    },
                ));
            }
        }
    }

    if let Some(time_range) = &index.time_range {
        if index.properties.first() != Some(&time_range.on) {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::TimeRangeSourceNotFirst {
                    property: time_range.on.to_string(),
                },
            ));
        }
    }

    if let Some(options) = &index.index_only {
        if !collection.index_only {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::IndexOnlyOptionOnStoredCollection,
            ));
        }
        if let Some(terminal) = &options.terminal {
            let is_owner = terminal.as_str() == "$ownerId";
            if !is_owner && (terminal.is_system() || !has_property_path(fields, terminal)) {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::TerminalNotAProperty {
                        property: terminal.to_string(),
                    },
                ));
            }
        }
    }

    let mut ranked = index.ranked.clone();
    if let RankedCount::At(levels) = &mut ranked.count {
        levels.sort();
        levels.dedup();
    }

    IndexManifest {
        name: index.name.clone(),
        properties: index.properties.clone(),
        unique: index.unique,
        null_searchable: index.null_searchable,
        contested,
        count,
        range_count,
        sum,
        range_sum,
        ranked,
        time_range: index.time_range.clone(),
        index_only: index.index_only.clone(),
    }
}

pub(super) fn validate_typed_collections(
    declaration: &ContractDeclaration,
    collections: &[CollectionManifest],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TypedCollectionManifest> {
    let typed = dedupe(
        &declaration.typed_collections,
        |a, b| a.id == b.id,
        |spec| spec.origin,
        |a, b| {
            let mut left = a.clone();
            left.origin = b.origin;
            left == *b
        },
        |_, _| {},
        |spec| DeclarationPath::typed_collection(&spec.id),
        "typed collection",
        || DiagnosticKind::DuplicateCollection,
        diagnostics,
    );
    let mut manifests: Vec<TypedCollectionManifest> = typed
        .iter()
        .map(|spec: &TypedCollectionSpec| {
            let path = DeclarationPath::typed_collection(&spec.id);
            if collections
                .iter()
                .any(|collection| collection.name == spec.id)
            {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::DuplicateCollection,
                ));
            }
            check_value_type(&path, &spec.key, diagnostics);
            check_value_type(&path, &spec.element, diagnostics);
            TypedCollectionManifest {
                id: spec.id.clone(),
                kind: spec.kind,
                key: spec.key.clone(),
                element: spec.element.clone(),
                max_elements: spec.max_elements,
            }
        })
        .collect();
    manifests.sort_by(|a, b| a.id.cmp(&b.id));
    manifests
}

/// Every string, byte array and list in a wire type declares a maximum.
pub(super) fn check_value_type(
    path: &DeclarationPath,
    ty: &ValueType,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match ty {
        ValueType::String { max_chars: None } | ValueType::Bytes { max_len: None } => {
            diagnostics.push(Diagnostic::new(
                path.clone(),
                DiagnosticKind::UnboundedField,
            ));
        }
        ValueType::List { max_len, item } => {
            if max_len.is_none() {
                diagnostics.push(Diagnostic::new(
                    path.clone(),
                    DiagnosticKind::UnboundedField,
                ));
            }
            check_value_type(path, item, diagnostics);
        }
        ValueType::Option(inner) => check_value_type(path, inner, diagnostics),
        ValueType::Struct(members) => {
            for (_, member) in members {
                check_value_type(path, member, diagnostics);
            }
        }
        ValueType::Unit
        | ValueType::Bool
        | ValueType::Integer(_)
        | ValueType::F64
        | ValueType::String { .. }
        | ValueType::Bytes { .. }
        | ValueType::Identifier
        | ValueType::DocumentId => {}
    }
}
