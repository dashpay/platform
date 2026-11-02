use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_private_document_entries_request::GetPrivateDocumentEntriesRequestV0;
use dapi_grpc::platform::v0::get_private_document_entries_response::get_private_document_entries_response_v0::{
    PrivateDocumentEntries, PrivateDocumentEntry,
};
use dapi_grpc::platform::v0::get_private_document_entries_response::{
    get_private_document_entries_response_v0, GetPrivateDocumentEntriesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_private_document_entries_v0(
        &self,
        GetPrivateDocumentEntriesRequestV0 {
            data_contract_id,
            document_type_name,
            start_position,
            prove,
        }: GetPrivateDocumentEntriesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetPrivateDocumentEntriesResponseV0>, Error> {
        // Private document types begin at protocol version 15: before it the read is not
        // active, whatever the request names, so it is refused before anything is looked up.
        let (method, slot) = if prove {
            (
                "prove_private_document_entries",
                platform_version
                    .drive
                    .methods
                    .document
                    .prove_private_document_entries,
            )
        } else {
            (
                "fetch_private_document_entries",
                platform_version
                    .drive
                    .methods
                    .document
                    .fetch_private_document_entries,
            )
        };
        if slot.is_none() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "{method} is not active at protocol version {}",
                    platform_version.protocol_version
                )),
            ));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "data_contract_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let maybe_contract_fetch_info = self.drive.get_contract_with_fetch_info(
            contract_id.to_buffer(),
            false,
            None,
            platform_version,
        )?;
        let contract_fetch_info = check_validation_result_with_data!(maybe_contract_fetch_info
            .ok_or_else(|| {
                QueryError::NotFound(format!("data contract {} not found", contract_id))
            }));
        let contract = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract
            .document_type_for_name(&document_type_name)
            .map_err(|_| QueryError::NotFound(format!(
                "document type {} not found in data contract {}",
                document_type_name, contract_id
            ))));

        // Drive settles the page (from the start to the end of its chunk of the store) and
        // refuses a type that is not private, for a read and a proof alike.
        let response = if prove {
            let proof = self.drive.prove_private_document_entries(
                contract_id.to_buffer(),
                document_type,
                start_position,
                platform_version,
            )?;

            GetPrivateDocumentEntriesResponseV0 {
                result: Some(get_private_document_entries_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let page = self.drive.fetch_private_document_entries(
                contract_id.to_buffer(),
                document_type,
                start_position,
                None,
                platform_version,
            )?;

            GetPrivateDocumentEntriesResponseV0 {
                result: Some(get_private_document_entries_response_v0::Result::Entries(
                    PrivateDocumentEntries {
                        entries: page
                            .entries
                            .into_iter()
                            .map(|entry| PrivateDocumentEntry {
                                position: entry.position,
                                block_height: entry.block_height,
                                block_time_ms: entry.block_time_ms,
                                data: entry.data,
                            })
                            .collect(),
                        total_count: page.total_count,
                    },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{
        assert_invalid_argument_status, quorum_secret_key, setup_platform, signed_proof,
        store_data_contract,
    };
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dapi_grpc::platform::v0::{
        get_private_document_entries_request, get_private_document_entries_response,
        GetPrivateDocumentEntriesRequest, GetPrivateDocumentEntriesResponse, Proof,
        ResponseMetadata,
    };
    use dpp::dashcore::Network;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use dpp::tests::json_document::json_document_to_contract;
    use drive::drive::document::private_document_store::{
        encode_stored_private_document_entry, PRIVATE_DOCUMENT_STORE_KEY,
    };
    use drive::drive::{Drive, RootTree};
    use drive_proof_verifier::types::private_document_entries::PrivateDocumentEntries as VerifiedPrivateDocumentEntries;
    use drive_proof_verifier::{ContextProvider, ContextProviderError, FromProof};
    use std::sync::Arc;

    /// Platform at the latest protocol version with the contract declaring the private
    /// `secret` type (512-byte entries, so 512 entries to a chunk of its store) and three
    /// entries appended to that store, entry `i` behind the header of block `10 + i` at time
    /// `500 * i` and filled with byte `i`.
    fn platform_with_private_entries() -> (
        TempPlatform<MockCoreRPCLike>,
        Arc<PlatformState>,
        &'static PlatformVersion,
        DataContract,
    ) {
        let (platform, state, version) = setup_platform(
            None,
            Network::Testnet,
            Some(PlatformVersion::latest().protocol_version),
        );
        let contract = json_document_to_contract(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/supporting_files/contract/reference-validation/reference-validation-contract-private-foreign.json"
            ),
            false,
            version,
        )
        .expect("expected contract");
        store_data_contract(&platform, &contract, version);
        let contract_id = contract.id().to_buffer();
        let type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &contract_id,
            &[1],
            b"secret",
        ];
        for i in 0..3u8 {
            platform
                .drive
                .grove
                .private_document_store_insert(
                    type_path.as_slice(),
                    &[PRIVATE_DOCUMENT_STORE_KEY],
                    encode_stored_private_document_entry(
                        10 + u64::from(i),
                        500 * u64::from(i),
                        &[i; 512],
                    ),
                    None,
                    &version.drive.grove_version,
                )
                .unwrap()
                .expect("append an entry");
        }
        (platform, state, version, contract)
    }

    fn request(
        contract_id: Vec<u8>,
        document_type_name: &str,
        start_position: u64,
        prove: bool,
    ) -> GetPrivateDocumentEntriesRequestV0 {
        GetPrivateDocumentEntriesRequestV0 {
            data_contract_id: contract_id,
            document_type_name: document_type_name.to_string(),
            start_position,
            prove,
        }
    }

    #[test]
    fn should_return_the_entries_of_a_private_type_with_their_header() {
        let (platform, state, version, contract) = platform_with_private_entries();
        let result = platform
            .query_private_document_entries_v0(
                request(contract.id().to_vec(), "secret", 0, false),
                &state,
                version,
            )
            .expect("query the entries");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let Some(get_private_document_entries_response_v0::Result::Entries(page)) =
            result.data.expect("data").result
        else {
            panic!("expected entries");
        };
        assert_eq!(page.total_count, 3);
        assert_eq!(
            page.entries,
            (0..3u8)
                .map(|i| PrivateDocumentEntry {
                    position: u64::from(i),
                    block_height: 10 + u64::from(i),
                    block_time_ms: 500 * u64::from(i),
                    data: vec![i; 512],
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn should_prove_the_entries_of_a_private_type() {
        let (platform, state, version, contract) = platform_with_private_entries();
        let result = platform
            .query_private_document_entries_v0(
                request(contract.id().to_vec(), "secret", 0, true),
                &state,
                version,
            )
            .expect("query the proof");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let Some(get_private_document_entries_response_v0::Result::Proof(proof)) =
            result.data.expect("data").result
        else {
            panic!("expected a proof");
        };
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let (root_hash, page) = Drive::verify_private_document_entries(
            &proof.grovedb_proof,
            contract.id().to_buffer(),
            secret,
            0,
            version,
        )
        .expect("verify the proof");
        assert_eq!(
            root_hash,
            platform
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .unwrap()
                .expect("root hash")
        );
        assert_eq!(page.total_count, 3);
        assert_eq!(page.entries.len(), 3);
        assert_eq!(page.entries[2].block_height, 12);
        assert_eq!(page.entries[2].data, vec![2; 512]);
    }

    /// The client's context: the contract declaring the private type, and the key of the one
    /// quorum the test signs with.
    struct ClientContext {
        contract: Arc<DataContract>,
        quorum_public_key: [u8; 48],
    }

    impl ContextProvider for ClientContext {
        fn get_data_contract(
            &self,
            id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok((*id == self.contract.id()).then(|| self.contract.clone()))
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok(self.quorum_public_key)
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    /// A proved page goes through the client's `FromProof` end to end: the contract from the
    /// context, the proof checked against Drive, and the root it derives against the quorum's
    /// signature. A correctly signed page verifies; a changed entry, a signature over another
    /// root, a changed signature and changed signed metadata are each refused.
    #[test]
    fn should_verify_a_signed_page_in_the_client_and_refuse_a_tampered_one() {
        let (platform, state, version, contract) = platform_with_private_entries();
        let result = platform
            .query_private_document_entries_v0(
                request(contract.id().to_vec(), "secret", 0, true),
                &state,
                version,
            )
            .expect("query the proof");
        let Some(get_private_document_entries_response_v0::Result::Proof(Proof {
            grovedb_proof,
            ..
        })) = result.data.expect("data").result
        else {
            panic!("expected a proof");
        };
        let root_hash = platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .expect("root hash");
        let metadata = ResponseMetadata {
            height: 4242,
            core_chain_locked_height: 1200,
            epoch: 0,
            time_ms: 1_755_000_000_000,
            protocol_version: version.protocol_version,
            chain_id: "test-private-entries-chain".to_string(),
        };
        let secret_key = quorum_secret_key();
        let quorum_hash = [3u8; 32];
        let context = ClientContext {
            contract: Arc::new(contract.clone()),
            quorum_public_key: secret_key.public_key().to_bytes(),
        };
        let verify = |proof: Proof, metadata: ResponseMetadata| {
            let request = GetPrivateDocumentEntriesRequest {
                version: Some(get_private_document_entries_request::Version::V0(request(
                    contract.id().to_vec(),
                    "secret",
                    0,
                    true,
                ))),
            };
            let response = GetPrivateDocumentEntriesResponse {
                version: Some(get_private_document_entries_response::Version::V0(
                    GetPrivateDocumentEntriesResponseV0 {
                        result: Some(get_private_document_entries_response_v0::Result::Proof(
                            proof,
                        )),
                        metadata: Some(metadata),
                    },
                )),
            };
            <VerifiedPrivateDocumentEntries as FromProof<GetPrivateDocumentEntriesRequest>>::maybe_from_proof_with_metadata(
                request,
                response,
                Network::Testnet,
                version,
                &context,
            )
            .map(|(page, _, _)| page)
        };

        let signed = signed_proof(
            grovedb_proof.clone(),
            &root_hash,
            &metadata,
            &secret_key,
            quorum_hash,
        );
        let page = verify(signed.clone(), metadata.clone())
            .expect("a correctly signed page verifies")
            .expect("a proved page");
        assert_eq!(page.total_count, 3);
        assert_eq!(
            page.entries
                .iter()
                .map(|entry| (
                    entry.position,
                    entry.block_height,
                    entry.block_time_ms,
                    entry.data.clone()
                ))
                .collect::<Vec<_>>(),
            (0..3u8)
                .map(|i| (
                    u64::from(i),
                    10 + u64::from(i),
                    500 * u64::from(i),
                    vec![i; 512]
                ))
                .collect::<Vec<_>>()
        );

        // A changed byte of the last entry, under the signature over the true root
        let entry_at = grovedb_proof
            .windows(512)
            .position(|window| window.iter().all(|byte| *byte == 2))
            .expect("the proof carries the last entry");
        let mut changed_proof = grovedb_proof.clone();
        changed_proof[entry_at] ^= 1;
        let mut changed_entry = signed.clone();
        changed_entry.grovedb_proof = changed_proof;
        assert!(verify(changed_entry, metadata.clone()).is_err());

        // A signature over another root
        let mut other_root = root_hash;
        other_root[0] ^= 1;
        let other_root_signed = signed_proof(
            grovedb_proof.clone(),
            &other_root,
            &metadata,
            &secret_key,
            quorum_hash,
        );
        assert!(matches!(
            verify(other_root_signed, metadata.clone()),
            Err(drive_proof_verifier::Error::InvalidSignature { .. })
        ));

        // A changed signature
        let mut changed_signature = signed.clone();
        changed_signature.signature[10] ^= 1;
        assert!(matches!(
            verify(changed_signature, metadata.clone()),
            Err(drive_proof_verifier::Error::InvalidSignature { .. }
                | drive_proof_verifier::Error::SignatureVerificationError { .. })
        ));

        // Signed metadata changed after signing
        let mut changed_metadata = metadata.clone();
        changed_metadata.time_ms += 1;
        assert!(matches!(
            verify(signed, changed_metadata),
            Err(drive_proof_verifier::Error::InvalidSignature { .. })
        ));
    }

    /// A page starts wherever the request asks and reads to the end of its chunk, read or
    /// proved.
    #[test]
    fn should_read_a_page_from_a_position_inside_a_chunk() {
        let (platform, state, version, contract) = platform_with_private_entries();
        let result = platform
            .query_private_document_entries_v0(
                request(contract.id().to_vec(), "secret", 1, false),
                &state,
                version,
            )
            .expect("query the entries");
        let Some(get_private_document_entries_response_v0::Result::Entries(page)) =
            result.data.expect("data").result
        else {
            panic!("expected entries");
        };
        assert_eq!(
            page.entries
                .iter()
                .map(|entry| entry.position)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );

        let result = platform
            .query_private_document_entries_v0(
                request(contract.id().to_vec(), "secret", 1, true),
                &state,
                version,
            )
            .expect("query the proof");
        let Some(get_private_document_entries_response_v0::Result::Proof(proof)) =
            result.data.expect("data").result
        else {
            panic!("expected a proof");
        };
        let secret = contract
            .document_type_for_name("secret")
            .expect("the secret type");
        let (_, proved_page) = Drive::verify_private_document_entries(
            &proof.grovedb_proof,
            contract.id().to_buffer(),
            secret,
            1,
            version,
        )
        .expect("verify the proof");
        assert_eq!(proved_page.entries.len(), 2);
        assert_eq!(proved_page.total_count, 3);
    }

    /// A type that is not private is refused with INVALID_ARGUMENT, which a client neither
    /// retries nor bans a node for, and a missing contract is not found, read or proved.
    #[test]
    fn should_refuse_a_type_that_is_not_private_and_a_missing_contract() {
        let (platform, state, version, _) = platform_with_private_entries();
        let note_contract = json_document_to_contract(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/supporting_files/contract/note/note-contract-keep-history-and-can-be-deleted.json"
            ),
            false,
            version,
        )
        .expect("expected contract");
        store_data_contract(&platform, &note_contract, version);
        for prove in [false, true] {
            let status =
                assert_invalid_argument_status(platform.query_private_document_entries_v0(
                    request(note_contract.id().to_vec(), "note", 0, prove),
                    &state,
                    version,
                ));
            assert!(
                status.message().contains("not a private document type"),
                "prove {prove}: {}",
                status.message()
            );

            let result = platform
                .query_private_document_entries_v0(
                    request(vec![9; 32], "secret", 0, prove),
                    &state,
                    version,
                )
                .expect("query");
            assert!(
                matches!(result.errors.as_slice(), [QueryError::NotFound(_)]),
                "prove {prove}: {:?}",
                result.errors
            );
        }
    }
}
