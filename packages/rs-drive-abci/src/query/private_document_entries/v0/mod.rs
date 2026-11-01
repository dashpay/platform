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
use drive::error::drive::DriveError;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_private_document_entries_v0(
        &self,
        GetPrivateDocumentEntriesRequestV0 {
            data_contract_id,
            document_type_name,
            start_position,
            count,
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
            return Err(Error::Drive(drive::error::Error::Drive(
                DriveError::VersionNotActive {
                    method: method.to_string(),
                    known_versions: vec![0],
                },
            )));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "data_contract_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let maybe_contract_fetch_info = self
            .drive
            .fetch_contract(contract_id.to_buffer(), None, None, None, platform_version)
            .unwrap()?;
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

        // Drive settles the page (a chunk boundary of the store, at most a chunk) and refuses a
        // type that is not private, for a read and a proof alike.
        let response = if prove {
            let proof = self.drive.prove_private_document_entries(
                contract_id.to_buffer(),
                document_type,
                start_position,
                count,
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
                count,
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
    use crate::query::tests::setup_platform;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::tests::json_document::json_document_to_contract;
    use drive::drive::Drive;
    use drive::util::storage_flags::StorageFlags;

    /// Platform at the latest protocol version with the contract declaring the private
    /// `secret` type (512-byte entries, so 512 entries to a chunk of its store) and three
    /// entries appended to that store, entry `i` behind the header of block `10 + i` at time
    /// `500 * i` and filled with byte `i`.
    fn platform_with_private_entries() -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        std::sync::Arc<PlatformState>,
        &'static PlatformVersion,
        dpp::prelude::DataContract,
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
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                version,
            )
            .expect("apply contract");
        let contract_id = contract.id().to_buffer();
        let type_path: [&[u8]; 4] = [&[64], &contract_id, &[1], b"secret"];
        for i in 0..3u8 {
            let mut stored = (10 + u64::from(i)).to_le_bytes().to_vec();
            stored.extend_from_slice(&(500 * u64::from(i)).to_le_bytes());
            stored.extend_from_slice(&[i; 512]);
            platform
                .drive
                .grove
                .private_document_store_insert(
                    type_path.as_slice(),
                    &[0],
                    stored,
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
            count: 0,
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

    #[test]
    fn should_refuse_a_misaligned_page_and_a_missing_contract() {
        let (platform, state, version, contract) = platform_with_private_entries();
        for prove in [false, true] {
            let result = platform.query_private_document_entries_v0(
                request(contract.id().to_vec(), "secret", 5, prove),
                &state,
                version,
            );
            assert!(
                matches!(
                    result,
                    Err(Error::Drive(drive::error::Error::Query(
                        drive::error::query::QuerySyntaxError::InvalidParameter(ref message)
                    ))) if message.contains("chunk boundary")
                ),
                "prove {prove}: {result:?}"
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
