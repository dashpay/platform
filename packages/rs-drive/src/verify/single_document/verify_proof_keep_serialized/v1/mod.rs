use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::verify::RootHash;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl SingleDocumentDriveQuery {
    /// Generation 0, except that two kinds of lookup it refused are verified: a document that
    /// may be contested, and a document at a block time in a type that keeps history.
    ///
    /// Generation 0 asks GroveDB to name the keys that are absent. That needs an overall limit
    /// and keys that can be listed. A query for a document that may be contested looks in two
    /// places, among the documents of its type and in the contest's area, each under a cap of
    /// its own, so the merged query has no overall limit. A lookup at a block time ranges over
    /// the times a version was stored at, which can not be listed. Here the proof of either is
    /// verified against the query as it is: it answers with the one document the query finds,
    /// and with nothing when there is none, which the proof then shows.
    #[inline(always)]
    pub(super) fn verify_proof_keep_serialized_v1(
        &self,
        is_subset: bool,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Vec<u8>>), Error> {
        let verified_by_generation_0 = match self.contested_status {
            SingleDocumentDriveQueryContestedStatus::NotContested => {
                !(self.document_type_keeps_history && self.block_time_ms.is_some())
            }
            SingleDocumentDriveQueryContestedStatus::MaybeContested => false,
            // A contest's area keeps no history, so its query has no block time
            SingleDocumentDriveQueryContestedStatus::Contested => true,
        };
        if verified_by_generation_0 {
            return self.verify_proof_keep_serialized_v0(is_subset, proof, platform_version);
        }
        let path_query = self.construct_path_query(platform_version)?;
        let (root_hash, mut proved_key_values) = if is_subset {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        // A document is stored among the documents of its type or waits in a contest, never
        // both, and a lookup in its history is capped at one version
        if proved_key_values.len() > 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(format!(
                "a single document query finds one document at most, we got {} elements",
                proved_key_values.len()
            ))));
        }

        let serialized = proved_key_values
            .pop()
            .and_then(|(_, _, element)| element)
            .map(|element| element.into_item_bytes().map_err(Error::from))
            .transpose()?;

        Ok((root_hash, serialized))
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::error::proof::ProofError;
    use crate::error::Error;
    use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{
        DataContractOwnedResolvedInfo, DocumentAndContractInfo, OwnedDocumentInfo,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::setup_contract;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContract;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value::Value;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;

    const DPNS: &str =
        "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json";
    const FAMILY_WITH_HISTORY: &str =
        "tests/supporting_files/contract/family/family-contract-with-history-only-message-index.json";

    fn drive_with_contract(path: &str) -> (Drive, DataContract) {
        drive_with_contract_at(path, PlatformVersion::latest())
    }

    fn drive_with_contract_at(
        path: &str,
        platform_version: &PlatformVersion,
    ) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            path,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        (drive, contract)
    }

    fn info<'a>(
        document: &'a Document,
        contract: &'a DataContract,
        document_type_name: &str,
    ) -> DocumentAndContractInfo<'a> {
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, None)),
                owner_id: Some(document.owner_id().to_buffer()),
            },
            contract,
            document_type: contract
                .document_type_for_name(document_type_name)
                .expect("expected the document type"),
        }
    }

    fn maybe_contested(
        contract: &DataContract,
        document_type_name: &str,
        document_id: [u8; 32],
        block_time_ms: Option<u64>,
    ) -> SingleDocumentDriveQuery {
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: document_type_name.to_string(),
            document_type_keeps_history: document_type.documents_keep_history(),
            document_id,
            block_time_ms,
            contested_status: SingleDocumentDriveQueryContestedStatus::MaybeContested,
        }
    }

    /// The document the query proves, verified as the whole proof and as a part of one.
    fn prove_and_verify(
        drive: &Drive,
        contract: &DataContract,
        query: &SingleDocumentDriveQuery,
    ) -> Option<Document> {
        let platform_version = PlatformVersion::latest();
        let path_query = query
            .construct_path_query(platform_version)
            .expect("expected to build the query");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected a proof");
        let document_type = contract
            .document_type_for_name(query.document_type_name.as_str())
            .expect("expected the document type");
        let (root_hash, document) = query
            .verify_proof(false, proof.as_slice(), document_type, platform_version)
            .expect("expected the proof to verify");
        let (subset_root_hash, subset_document) = query
            .verify_proof(true, proof.as_slice(), document_type, platform_version)
            .expect("expected the proof to verify as a subset");
        assert_eq!(root_hash, subset_root_hash);
        assert_eq!(document, subset_document);
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected a root hash"),
            "the proof is of the stored state"
        );
        document
    }

    fn domain(contract: &DataContract, seed: u64, label: &str) -> Document {
        let mut document = contract
            .document_type_for_name("domain")
            .expect("expected the domain document type")
            .random_document(Some(seed), PlatformVersion::latest())
            .expect("expected a random document");
        document.set(
            "normalizedParentDomainName",
            Value::Text("dash".to_string()),
        );
        document.set("normalizedLabel", Value::Text(label.to_string()));
        document
    }

    /// Stores `document` as a contender for the name "quantum", in the contest's area.
    fn add_to_contest(drive: &Drive, contract: &DataContract, document: &Document) {
        let platform_version = PlatformVersion::latest();
        drive
            .add_contested_document_for_contract(
                info(document, contract, "domain"),
                ContestedDocumentResourceVotePollWithContractInfo {
                    contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text("quantum".to_string()),
                    ],
                },
                false,
                BlockInfo::default(),
                true,
                Some(
                    ContestedDocumentVotePollStoredInfo::new(
                        BlockInfo::default(),
                        platform_version,
                    )
                    .expect("expected the stored info of a new poll"),
                ),
                None,
                platform_version,
            )
            .expect("expected to add the contested document");
    }

    #[test]
    fn should_find_a_document_stored_among_its_type_when_it_may_be_contested() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = drive_with_contract(DPNS);
        let document = domain(&contract, 1, "quantum");
        drive
            .add_document_for_contract(
                info(&document, &contract, "domain"),
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the document");

        let found = prove_and_verify(
            &drive,
            &contract,
            &maybe_contested(&contract, "domain", document.id().to_buffer(), None),
        );

        assert_eq!(found.map(|found| found.id()), Some(document.id()));
    }

    #[test]
    fn should_find_a_document_waiting_in_a_contest_when_it_may_be_contested() {
        let (drive, contract) = drive_with_contract(DPNS);
        let document = domain(&contract, 1, "quantum");
        add_to_contest(&drive, &contract, &document);

        let found = prove_and_verify(
            &drive,
            &contract,
            &maybe_contested(&contract, "domain", document.id().to_buffer(), None),
        );

        assert_eq!(found.map(|found| found.id()), Some(document.id()));
    }

    #[test]
    fn should_prove_a_document_is_in_neither_place_when_it_may_be_contested() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = drive_with_contract(DPNS);
        // Another document, so both places hold something the proof must tell apart
        let other = domain(&contract, 1, "quantum");
        drive
            .add_document_for_contract(
                info(&other, &contract, "domain"),
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the document");

        let found = prove_and_verify(
            &drive,
            &contract,
            &maybe_contested(&contract, "domain", [9; 32], None),
        );

        assert_eq!(found, None);
    }

    /// A person stored at time 1000 and changed at time 2000, with the two versions.
    fn person_with_two_versions(drive: &Drive, contract: &DataContract) -> (Document, Document) {
        person_with_two_versions_at(drive, contract, PlatformVersion::latest())
    }

    fn person_with_two_versions_at(
        drive: &Drive,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> (Document, Document) {
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected the person document type");
        let mut first = document_type
            .random_document(Some(7), platform_version)
            .expect("expected a random document");
        first.set("age", Value::U8(33));
        drive
            .add_document_for_contract(
                info(&first, contract, "person"),
                false,
                BlockInfo::default_with_time(1000),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the document");
        let mut second = first.clone();
        second.set("age", Value::U8(34));
        drive
            .update_document_for_contract(
                &second,
                contract,
                document_type,
                Some(second.owner_id().to_buffer()),
                BlockInfo::default_with_time(2000),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to update the document");
        (first, second)
    }

    fn serialized(document: &Document, contract: &DataContract) -> Vec<u8> {
        document
            .serialize(
                contract
                    .document_type_for_name("person")
                    .expect("expected the person document type"),
                contract,
                PlatformVersion::latest(),
            )
            .expect("expected to serialize the document")
    }

    #[test]
    fn should_find_the_latest_version_of_a_document_with_history_when_it_may_be_contested() {
        let (drive, contract) = drive_with_contract(FAMILY_WITH_HISTORY);
        let (_, second) = person_with_two_versions(&drive, &contract);

        let found = prove_and_verify(
            &drive,
            &contract,
            &maybe_contested(&contract, "person", second.id().to_buffer(), None),
        )
        .expect("expected the document");

        assert_eq!(
            serialized(&found, &contract),
            serialized(&second, &contract)
        );
    }

    /// The query of the person `document_id` as it stood at `block_time_ms`.
    fn person_at_block_time(
        contract: &DataContract,
        document_id: [u8; 32],
        block_time_ms: u64,
        contested_status: SingleDocumentDriveQueryContestedStatus,
    ) -> SingleDocumentDriveQuery {
        SingleDocumentDriveQuery {
            contested_status,
            ..maybe_contested(contract, "person", document_id, Some(block_time_ms))
        }
    }

    #[test]
    fn should_find_the_version_of_a_document_stored_at_or_before_a_block_time() {
        let (drive, contract) = drive_with_contract(FAMILY_WITH_HISTORY);
        let (first, second) = person_with_two_versions(&drive, &contract);

        for contested_status in [
            SingleDocumentDriveQueryContestedStatus::NotContested,
            SingleDocumentDriveQueryContestedStatus::MaybeContested,
        ] {
            for (block_time_ms, expected) in [
                // Before the document was first stored there is no version of it
                (0, None),
                (999, None),
                (1000, Some(&first)),
                (1500, Some(&first)),
                (1999, Some(&first)),
                (2000, Some(&second)),
                (2500, Some(&second)),
                (u64::MAX, Some(&second)),
            ] {
                let found = prove_and_verify(
                    &drive,
                    &contract,
                    &person_at_block_time(
                        &contract,
                        first.id().to_buffer(),
                        block_time_ms,
                        contested_status.clone(),
                    ),
                );

                assert_eq!(
                    found.map(|found| serialized(&found, &contract)),
                    expected.map(|expected| serialized(expected, &contract)),
                    "{contested_status:?} at block time {block_time_ms}"
                );
            }
        }
    }

    #[test]
    fn should_prove_a_document_that_was_never_stored_is_absent_at_a_block_time() {
        let (drive, contract) = drive_with_contract(FAMILY_WITH_HISTORY);
        // Another document, so the type holds a history the proof must tell apart
        person_with_two_versions(&drive, &contract);

        for contested_status in [
            SingleDocumentDriveQueryContestedStatus::NotContested,
            SingleDocumentDriveQueryContestedStatus::MaybeContested,
        ] {
            let found = prove_and_verify(
                &drive,
                &contract,
                &person_at_block_time(&contract, [9; 32], 1500, contested_status.clone()),
            );

            assert_eq!(found, None, "{contested_status:?}");
        }
    }

    #[test]
    fn should_refuse_a_proof_made_for_another_block_time() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = drive_with_contract(FAMILY_WITH_HISTORY);
        let (first, _) = person_with_two_versions(&drive, &contract);

        for contested_status in [
            SingleDocumentDriveQueryContestedStatus::NotContested,
            SingleDocumentDriveQueryContestedStatus::MaybeContested,
        ] {
            // Each pair of times answers with a different version, or with none for one of them
            for (proved_block_time_ms, verified_block_time_ms) in
                [(1500, 2500), (2500, 1500), (500, 1500), (1500, 500)]
            {
                let proved = person_at_block_time(
                    &contract,
                    first.id().to_buffer(),
                    proved_block_time_ms,
                    contested_status.clone(),
                );
                let proof = drive
                    .grove_get_proved_path_query(
                        &proved
                            .construct_path_query(platform_version)
                            .expect("expected to build the query"),
                        None,
                        &mut vec![],
                        &platform_version.drive,
                    )
                    .expect("expected a proof");
                let verified = SingleDocumentDriveQuery {
                    block_time_ms: Some(verified_block_time_ms),
                    ..proved
                };

                for is_subset in [false, true] {
                    assert!(
                        verified
                            .verify_proof_keep_serialized(
                                is_subset,
                                proof.as_slice(),
                                platform_version
                            )
                            .is_err(),
                        "{contested_status:?}, a proof at {proved_block_time_ms} verified at \
                         {verified_block_time_ms}, is_subset {is_subset}"
                    );
                }
            }
        }
    }

    /// Protocol version 14 selects generation 0 of the query and of this verifier, which a
    /// lookup at a block time never passed.
    #[test]
    fn should_leave_a_lookup_at_a_block_time_refused_at_protocol_version_14() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        let (drive, contract) = drive_with_contract_at(FAMILY_WITH_HISTORY, protocol_version_14);
        let (first, _) = person_with_two_versions_at(&drive, &contract, protocol_version_14);
        let query = person_at_block_time(
            &contract,
            first.id().to_buffer(),
            1500,
            SingleDocumentDriveQueryContestedStatus::NotContested,
        );
        let proof = drive
            .grove_get_proved_path_query(
                &query
                    .construct_path_query(protocol_version_14)
                    .expect("expected to build the query"),
                None,
                &mut vec![],
                &protocol_version_14.drive,
            )
            .expect("expected a proof");

        for is_subset in [false, true] {
            let result = query.verify_proof_keep_serialized(
                is_subset,
                proof.as_slice(),
                protocol_version_14,
            );

            assert!(
                matches!(result, Err(Error::GroveDB(_))),
                "is_subset {is_subset}: {result:?}"
            );
        }
    }

    /// A proof of the document's query in `proved_status`, which looks in one place only.
    fn proof_of_one_place(
        drive: &Drive,
        query: &SingleDocumentDriveQuery,
        proved_status: SingleDocumentDriveQueryContestedStatus,
    ) -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let one_place = SingleDocumentDriveQuery {
            contested_status: proved_status,
            ..query.clone()
        };
        drive
            .grove_get_proved_path_query(
                &one_place
                    .construct_path_query(platform_version)
                    .expect("expected to build the query"),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected a proof")
    }

    #[test]
    fn should_refuse_a_proof_that_covers_only_one_of_the_two_places() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = drive_with_contract(DPNS);
        let document = domain(&contract, 1, "quantum");
        drive
            .add_document_for_contract(
                info(&document, &contract, "domain"),
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the document");
        add_to_contest(&drive, &contract, &domain(&contract, 2, "quantum"));
        let query = maybe_contested(&contract, "domain", document.id().to_buffer(), None);

        for proved_status in [
            SingleDocumentDriveQueryContestedStatus::NotContested,
            SingleDocumentDriveQueryContestedStatus::Contested,
        ] {
            let proof = proof_of_one_place(&drive, &query, proved_status.clone());
            for is_subset in [false, true] {
                assert!(
                    query
                        .verify_proof_keep_serialized(is_subset, proof.as_slice(), platform_version)
                        .is_err(),
                    "a proof of the {proved_status:?} place alone, is_subset {is_subset}"
                );
            }
        }
    }

    #[test]
    fn should_refuse_a_document_proved_to_be_in_both_places() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = drive_with_contract(DPNS);
        let document = domain(&contract, 1, "quantum");
        drive
            .add_document_for_contract(
                info(&document, &contract, "domain"),
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the document");
        add_to_contest(&drive, &contract, &document);
        let query = maybe_contested(&contract, "domain", document.id().to_buffer(), None);
        let proof = drive
            .grove_get_proved_path_query(
                &query
                    .construct_path_query(platform_version)
                    .expect("expected to build the query"),
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected a proof");

        let result = query.verify_proof_keep_serialized(false, proof.as_slice(), platform_version);

        assert!(
            matches!(result, Err(Error::Proof(ProofError::CorruptedProof(_)))),
            "{result:?}"
        );
    }
}
