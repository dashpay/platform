//! The declaration model on the guest profile: this test is compiled by the
//! CI guest cut with `--no-default-features`, so it exercises the builders,
//! the validator and the manifest without `std`. Under default features it
//! runs the same code with `std` present.

use dash_sdk_contract::prelude::*;

fn scores() -> CollectionSpec {
    CollectionSpec::documents(CollectionName::new("scores").unwrap())
        .document_id_field("id")
        .field(FieldSpec::new(
            PropertyName::new("class").unwrap(),
            0,
            FieldType::string(64),
        ))
        .field(FieldSpec::new(
            PropertyName::new("points").unwrap(),
            1,
            FieldType::bounded_integer(IntegerWidth::I64, 0, 1_000_000),
        ))
        .index(
            IndexSpec::new(
                IndexName::new("by_class").unwrap(),
                vec![PropertyPath::new("class").unwrap()],
            )
            .count()
            .sum(PropertyName::new("points").unwrap()),
        )
}

#[test]
fn should_build_the_sketch_manifest_through_builders() {
    let declaration = ContractDeclaration::new().collection(scores()).entry(
        EntrySpec::new(MethodName::new("score.add").unwrap())
            .receiver(Receiver::Mut(CollectionName::new("scores").unwrap()))
            .param("delta", ValueType::Integer(IntegerWidth::I64)),
    );
    let manifest = validate(&declaration).expect("the sketch validates");
    assert_eq!(manifest.collections().len(), 1);
    let add = manifest.method("score.add").expect("entry present");
    assert_eq!(add.export, entry_export_symbol(&add.name));
    assert_eq!(
        StagingPoint::for_receiver(&add.receiver),
        Some(StagingPoint::MutReceiverOnOk)
    );
}

#[test]
fn should_report_diagnostics_with_stable_codes() {
    let declaration = ContractDeclaration::new()
        .collection(scores().mutable(false))
        .entry(
            EntrySpec::new(MethodName::new("score.add").unwrap())
                .receiver(Receiver::Mut(CollectionName::new("scores").unwrap())),
        );
    let diagnostics = validate(&declaration).expect_err("immutable receiver is rejected");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].kind.name(),
        "MutableReceiverOnImmutableCollection"
    );
    assert!(diagnostics[0].code().starts_with("DSC"));
}
