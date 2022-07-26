use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use dpp::data_contract::extra::DriveContractExt;
use rs_drive::common::json_document_to_cbor;
use rs_drive::contract::document::Document;
use rs_drive::contract::Contract;
use rs_drive::contract::CreateRandomDocument;
use serde::Serialize;

criterion_main!(serialization, deserialization);
criterion_group!(serialization, test_drive_10_serialization);
criterion_group!(deserialization, test_drive_10_deserialization);

fn test_drive_10_serialization(c: &mut Criterion) {
    let dashpay_cbor = json_document_to_cbor(
        "tests/supporting_files/contract/dashpay/dashpay-contract.json",
        Some(1),
    );
    let contract = <Contract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

    let document_type = contract
        .document_type_for_name("contactRequest")
        .expect("expected to get profile document type");

    let mut group = c.benchmark_group("Serialization");

    group.bench_function("DDSR 10", |b| {
        b.iter_batched(
            || document_type.random_documents(10, Some(3333)),
            |documents| {
                documents.iter().for_each(|document| {
                    document
                        .serialize(document_type)
                        .expect("expected to serialize");
                })
            },
            BatchSize::LargeInput,
        )
    });
    group.bench_function("CBOR 10", |b| {
        b.iter_batched(
            || document_type.random_documents(10, Some(3333)),
            |documents| {
                documents.iter().for_each(|document| {
                    document.to_cbor();
                })
            },
            BatchSize::LargeInput,
        )
    });
    group.bench_function("DDSR Consume 10", |b| {
        b.iter_batched(
            || document_type.random_documents(10, Some(3333)),
            |documents| {
                documents.into_iter().for_each(|document| {
                    document
                        .serialize_consume(document_type)
                        .expect("expected to serialize");
                })
            },
            BatchSize::LargeInput,
        )
    });
}

fn test_drive_10_deserialization(c: &mut Criterion) {
    let dashpay_cbor = json_document_to_cbor(
        "tests/supporting_files/contract/dashpay/dashpay-contract.json",
        Some(1),
    );
    let contract = <Contract as DriveContractExt>::from_cbor(&dashpay_cbor, None).unwrap();

    let document_type = contract
        .document_type_for_name("contactRequest")
        .expect("expected to get profile document type");
    let (serialized_documents, cbor_serialized_documents): (Vec<Vec<u8>>, Vec<Vec<u8>>) =
        document_type
            .random_documents(10, Some(3333))
            .iter()
            .map(|a| (a.serialize(document_type).unwrap(), a.to_cbor()))
            .unzip();

    let mut group = c.benchmark_group("Deserialization");

    group.bench_function("DDSR 10", |b| {
        b.iter(|| {
            serialized_documents.iter().for_each(|serialized_document| {
                Document::from_bytes(serialized_document, document_type)
                    .expect("expected to deserialize");
            })
        })
    });
    group.bench_function("CBOR 10", |b| {
        b.iter(|| {
            cbor_serialized_documents
                .iter()
                .for_each(|serialized_document| {
                    Document::from_cbor(serialized_document, None, None)
                        .expect("expected to deserialize");
                })
        })
    });
}
