//! Verified **having-range**
//! (`GROUP BY … HAVING <aggregate> <op> <value> LIMIT n`) document
//! results.
//!
//! A having-range query answers "which groups' aggregate falls inside a
//! value bound?" — `SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100
//! LIMIT 100`. The answer is a value-bounded range read of the same
//! per-axis *secondary* Merk the ranked query walks, so it costs
//! `O(log n + k)` and comes with a proof that commits to exactly the
//! returned `(aggregate, group key)` pairs **and their completeness**:
//! the Merk range proof commits its boundaries, so an in-range group the
//! node omitted fails verification.
//!
//! This module holds the client-facing result type
//! ([`DocumentHavingEntries`]), the tenderdash-composition wrapper
//! around rs-drive's merk-level verifier
//! ([`verify_having_range_proof`]), and the decoder for the unproven
//! wire payload ([`DocumentHavingEntries::from_unproved_response`]) —
//! which rides the same `ResultData.ranked` variant the ranked surface
//! uses, since a having page is the same "group key + aggregate value"
//! entry list.
//!
//! Per-shape routing (which index covers the axis, which bounds the
//! clause translates to) lives in rs-sdk's `having_proof_helpers`,
//! exactly as the ranked equivalents live in `ranked_proof_helpers` —
//! it needs the data contract, which this crate does not carry.

use crate::error::MapGroveDbError;
use crate::proof::document_ranked::{ranked_entry_from_proto, result_variant_name};
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    result_data, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::{DriveDocumentHavingQuery, DriveDocumentQuery, RankedEntry};
use drive::verify::RootHash;

/// One page of a `GROUP BY … HAVING <aggregate> <op> <value> LIMIT n`
/// query: the groups whose aggregate falls inside the bound.
///
/// **Entry order is axis order in the walk direction** — ascending by
/// default, descending when the request ordered by the aggregate
/// descending. Callers must not re-sort; ties (groups with equal
/// aggregates) come back in group-key order in the direction of the
/// walk, same as on the ranked surface.
///
/// Fewer than `n` entries means fewer groups matched — not an error.
/// **Exactly `n` entries may mean the match set was cut at the limit**;
/// nothing in the page marks the cut. Tightening the bound past the
/// last aggregate value seen continues past *distinct* values only — a
/// cut inside a tie (several groups sharing the boundary aggregate)
/// cannot be continued, so size the limit above the widest expected
/// tie.
///
/// Entry semantics ([`RankedEntry`]) are identical to the ranked
/// surface's, including the fixed-point average scaling and the
/// exact-on-the-proved-path-only caveat.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentHavingEntries {
    /// The matching groups, in axis order in the walk direction.
    pub entries: Vec<RankedEntry>,
}

impl DocumentHavingEntries {
    /// Build a [`DocumentHavingEntries`] from the verifier-side entry
    /// list — the shape rs-drive's merk-level verifier returns.
    pub fn from_verified(entries: Vec<RankedEntry>) -> Self {
        DocumentHavingEntries { entries }
    }

    /// Decode the **unproven** having-range payload of a `getDocuments`
    /// response — the `ResultData.ranked` variant a node returns for a
    /// having-range request sent with `prove = false`. (The wire reuses
    /// the ranked entries message; a having page leaves its `skipped`
    /// field unset, and this decoder ignores it either way, because a
    /// value-bounded page has no rank base for it to describe.)
    ///
    /// Order is preserved verbatim. This is a plain wire decode with
    /// **no cryptographic guarantee whatsoever** — and unlike the ranked
    /// surface the missing guarantee here includes *completeness*: an
    /// unproven page is free to omit matching groups, which for a
    /// spam-resistance query is precisely the interesting attack. Prefer
    /// [`verify_having_range_proof`] (via rs-sdk's
    /// `DocumentHavingEntries::fetch`) unless you deliberately trust the
    /// node.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyVersion`] when the response carries no version.
    /// - [`Error::ResponseDecodeError`] when the response is a V0
    ///   response, carries a proof rather than data, carries a
    ///   non-ranked `ResultData` variant, or an entry's `value` oneof is
    ///   unset / out of domain.
    pub fn from_unproved_response(
        response: &GetDocumentsResponse,
    ) -> Result<(Self, ResponseMetadata), Error> {
        let version = response.version.as_ref().ok_or(Error::EmptyVersion)?;
        let ResponseVersion::V1(v1) = version else {
            return Err(Error::ResponseDecodeError {
                error: "having-range results are a V1-only response shape; got a V0 \
                        getDocuments response. Having-range queries require protocol \
                        version 14+."
                    .to_string(),
            });
        };
        let metadata = v1.metadata.clone().ok_or(Error::EmptyResponseMetadata)?;
        let entries = match v1.result.as_ref() {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Ranked(ranked)),
            })) => ranked
                .entries
                .iter()
                .map(ranked_entry_from_proto)
                .collect::<Result<Vec<_>, _>>()?,
            Some(get_documents_response_v1::Result::Proof(_)) => {
                return Err(Error::ResponseDecodeError {
                    error: "the response carries a proof, not unproven having-range entries; \
                            verify it with `verify_having_range_proof` instead of decoding it"
                        .to_string(),
                });
            }
            other => {
                return Err(Error::ResponseDecodeError {
                    error: format!(
                        "expected a `ResultData.ranked` payload for a having-range request, \
                         got {}. A response on another variant means the node routed \
                         the request to a different executor — check that the request \
                         carries a `group_by` and exactly one `having` clause bounding the \
                         single `select`'s aggregate.",
                        result_variant_name(other)
                    ),
                });
            }
        };
        Ok((DocumentHavingEntries { entries }, metadata))
    }
}

/// Verify a grovedb indexed-axis range proof **and the surrounding
/// tenderdash commit**, returning the reconstructed root hash and the
/// matching groups it commits to.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentHavingQuery::verify_having_range_proof`] in rs-drive
/// (which does the merk-level verification). Both sides derive the
/// proved subtree from the same
/// `DriveDocumentHavingQuery::indexed_property_name_tree_path` and the
/// secondary query from the same `AxisRangeBounds::merk_query`, so
/// prover and verifier cannot drift on *which bound over which tree* is
/// being checked, and grovedb re-checks the echoed query and limit — a
/// proof of one bound does not verify as another.
///
/// ## The root hash is the whole point
///
/// Same as on the ranked surface: the merk-level verifier returning
/// `Ok` is not by itself evidence of anything — the binding to the
/// quorum-signed app hash in [`verify_tenderdash_proof`] is what makes
/// the entries (and their completeness) attested facts. This function
/// exists so that composition can never be skipped by accident.
pub fn verify_having_range_proof(
    query: &DriveDocumentHavingQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, Vec<RankedEntry>), Error> {
    let (root_hash, entries) = query
        .verify_having_range_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((root_hash, entries))
}

/// Reject the generic [`FromProof`] entry point for
/// [`DocumentHavingEntries`] — same guard rail, same rationale as the
/// [`crate::DocumentRankedEntries`] blanket impl: the generic
/// `FromProof<Q: TryInto<DriveDocumentQuery>>` path carries neither the
/// bounds nor the covering index, so it errors out explicitly rather
/// than verifying the wrong thing.
impl<'dq, Q> FromProof<Q> for DocumentHavingEntries
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        _response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
        _provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        Err(Error::RequestError {
            error: "DocumentHavingEntries can't be verified via the generic FromProof path; \
                 call DocumentHavingEntries::fetch on a DocumentQuery carrying \
                 .with_select(<aggregate>), .with_group_by(<property>), \
                 .with_having(<one clause bounding the selected aggregate>) and \
                 .with_limit(n), which resolves the bounds and the covering index from \
                 the data contract"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the unproven decode and the response-shape
    //! rejections, plus the [`trust_boundary`] suite exercising this
    //! module's [`verify_having_range_proof`] wrapper — a real proof
    //! from a real Drive, verified against a signed tenderdash commit.
    //! The merk-level verification alone is exercised end-to-end by
    //! rs-drive's `drive_document_having_query::tests` (prover and
    //! verifier against a real Drive) and rs-drive-abci's
    //! `having_range_tests` (wire encoding of the same values).
    use super::*;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
        ranked_entry, Documents, RankedEntries, RankedEntry as ProtoRankedEntry,
    };
    use dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV1;
    use drive::query::RankedEntryValue;

    fn count_entry(key: &str, count: u64) -> ProtoRankedEntry {
        ProtoRankedEntry {
            key: key.as_bytes().to_vec(),
            value: Some(ranked_entry::Value::Count(count)),
        }
    }

    fn response_with(result: get_documents_response_v1::Result) -> GetDocumentsResponse {
        GetDocumentsResponse {
            version: Some(ResponseVersion::V1(GetDocumentsResponseV1 {
                result: Some(result),
                metadata: Some(ResponseMetadata {
                    height: 42,
                    ..Default::default()
                }),
            })),
        }
    }

    fn having_response(
        entries: Vec<ProtoRankedEntry>,
        skipped: Option<u64>,
    ) -> GetDocumentsResponse {
        response_with(get_documents_response_v1::Result::Data(ResultData {
            variant: Some(result_data::Variant::Ranked(RankedEntries {
                entries,
                skipped,
            })),
        }))
    }

    /// The headline decode: `HAVING $count > 100`-shaped entries come
    /// back in axis order, untouched, with `skipped` (unset on a
    /// having page) ignored.
    #[test]
    fn decodes_entries_preserving_axis_order() {
        let response = having_response(
            vec![count_entry("dash", 101), count_entry("evo", 250)],
            None,
        );
        let (decoded, metadata) = DocumentHavingEntries::from_unproved_response(&response)
            .expect("a well-formed having payload decodes");
        assert_eq!(metadata.height, 42);
        assert_eq!(
            decoded.entries.iter().map(|e| e.value).collect::<Vec<_>>(),
            vec![RankedEntryValue::Count(101), RankedEntryValue::Count(250)]
        );
    }

    /// A stray `skipped` from a non-conforming node is ignored, not a
    /// decode failure: the field cannot describe anything on a
    /// value-bounded page, and failing on it would break against a
    /// node that reused its ranked encoder wholesale.
    #[test]
    fn a_stray_skipped_field_is_ignored() {
        let response = having_response(vec![count_entry("dash", 101)], Some(7));
        let (decoded, _) = DocumentHavingEntries::from_unproved_response(&response)
            .expect("a stray skipped is not a decode failure");
        assert_eq!(decoded.entries.len(), 1);
    }

    /// No groups matching the bound is a legitimate answer.
    #[test]
    fn decodes_an_empty_match_set() {
        let (decoded, _) =
            DocumentHavingEntries::from_unproved_response(&having_response(vec![], None))
                .expect("an empty match set is well-formed");
        assert!(decoded.entries.is_empty());
    }

    /// Same caller-mistake guard as the ranked decoder: a proof must
    /// be verified, not decoded.
    #[test]
    fn rejects_a_proof_response() {
        let response = response_with(get_documents_response_v1::Result::Proof(Proof::default()));
        let err = DocumentHavingEntries::from_unproved_response(&response)
            .expect_err("a proof is not an unproven having payload");
        assert!(format!("{err}").contains("verify_having_range_proof"));
    }

    /// A response on another variant means the node routed the request
    /// somewhere else entirely.
    #[test]
    fn rejects_a_non_ranked_result_variant() {
        let response = response_with(get_documents_response_v1::Result::Data(ResultData {
            variant: Some(result_data::Variant::Documents(Documents {
                documents: Vec::new(),
            })),
        }));
        let err = DocumentHavingEntries::from_unproved_response(&response)
            .expect_err("a documents payload is not a having one");
        assert!(format!("{err}").contains("ResultData.ranked"));
    }

    /// V0 predates the SQL-shaped surface entirely.
    #[test]
    fn rejects_a_v0_response() {
        let response = GetDocumentsResponse {
            version: Some(ResponseVersion::V0(Default::default())),
        };
        let err = DocumentHavingEntries::from_unproved_response(&response)
            .expect_err("V0 has no having shape");
        assert!(format!("{err}").contains("V1-only"));
    }

    mod trust_boundary {
        //! The security-critical composition: a grovedb-valid proof is
        //! only an authenticated platform result once its reconstructed
        //! root hash is bound to the quorum-signed app hash. These tests
        //! run [`verify_having_range_proof`] — the module-level wrapper,
        //! not the merk-level verifier — against a real proof generated
        //! by a real Drive and a commit signed with a test quorum key:
        //! the correctly signed root verifies, and a commit over a
        //! different app hash, tampered response metadata, or a wrong
        //! quorum key each fail, so the tenderdash binding cannot be
        //! skipped or miswired without a test going red.

        use super::super::verify_having_range_proof;
        use crate::ContextProvider;
        use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
        use dash_context_provider::ContextProviderError;
        use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::document_type::random_document::CreateRandomDocument;
        use dpp::data_contract::TokenConfiguration;
        use dpp::document::{Document, DocumentV0Setters};
        use dpp::platform_value::Value;
        use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
        use dpp::tests::json_document::json_document_to_contract;
        use dpp::version::PlatformVersion;
        use drive::drive::Drive;
        use drive::query::drive_document_having_query::drive_dispatcher::{
            DocumentHavingRequest, DocumentHavingResponse,
        };
        use drive::query::drive_document_having_query::mode_detection::detect_having_mode;
        use drive::query::drive_document_ranked_query::index_picker::find_ranked_index_for_axis;
        use drive::query::having::{
            HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator,
            HavingRightOperand,
        };
        use drive::query::projection::SelectProjection;
        use drive::query::{DriveDocumentHavingQuery, RankedPaginationInputs};
        use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
        use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
        use drive::util::storage_flags::StorageFlags;
        use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use std::sync::Arc;
        use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
        use tenderdash_abci::signatures::{Hashable, Signable};

        const CHAIN_ID: &str = "test-having-chain";
        const HEIGHT: u64 = 4242;
        const ROUND: u32 = 0;
        const QUORUM_TYPE: u32 = 1; // LLMQ_50_60
        const CORE_LOCKED_HEIGHT: u32 = 1200;
        const TIME_MS: u64 = 1_755_000_000_000;

        /// Provider that knows exactly one quorum key — the test one.
        struct TestQuorumProvider {
            pubkey: [u8; 48],
        }

        impl ContextProvider for TestQuorumProvider {
            fn get_data_contract(
                &self,
                _id: &Identifier,
                _platform_version: &PlatformVersion,
            ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
                Ok(None)
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
                Ok(self.pubkey)
            }

            fn get_platform_activation_height(
                &self,
            ) -> Result<CoreBlockHeight, ContextProviderError> {
                Ok(1)
            }
        }

        fn platform_version() -> &'static PlatformVersion {
            PlatformVersion::latest()
        }

        /// A deterministic, valid BLS scalar — no RNG dependency.
        fn quorum_secret_key() -> SecretKey<Bls12381G2Impl> {
            let mut bytes = [0u8; 32];
            bytes[31] = 42;
            SecretKey::<Bls12381G2Impl>::from_be_bytes(&bytes)
                .into_option()
                .expect("a small nonzero scalar is a valid secret key")
        }

        /// Real Drive, the ranked grades fixture, and a few grade
        /// documents so the axis secondary has content to prove over.
        fn setup_drive_with_grades() -> (Drive, DataContract) {
            let drive = setup_drive_with_initial_state_structure(None);
            let pv = platform_version();
            let contract = json_document_to_contract(
                "../rs-drive/tests/supporting_files/contract/grades/grades-ranked-contract.json",
                false,
                pv,
            )
            .expect("expected to parse the ranked grades contract");
            drive
                .apply_contract(
                    &contract,
                    Default::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    pv,
                )
                .expect("expected to apply the ranked grades contract");

            let document_type = contract
                .document_type_for_name("grade")
                .expect("grade doctype exists");
            let rows: [([u8; 32], i64); 4] = [
                ([1u8; 32], 70),
                ([1u8; 32], 80),
                ([2u8; 32], 85),
                ([2u8; 32], 95),
            ];
            for (i, (identity, grade)) in rows.iter().enumerate() {
                let mut doc: Document = document_type
                    .random_document(Some(9000 + i as u64), pv)
                    .expect("random document");
                let mut props = std::collections::BTreeMap::new();
                props.insert("identityId".to_string(), Value::Identifier(*identity));
                props.insert("grade".to_string(), Value::I64(*grade));
                doc.set_properties(props);
                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentRefInfo((&doc, None)),
                                owner_id: None,
                            },
                            contract: &contract,
                            document_type,
                        },
                        false,
                        Default::default(),
                        true,
                        None,
                        pv,
                        None,
                    )
                    .expect("expected to insert a grade document");
            }
            (drive, contract)
        }

        /// `AVG(grade) > 80 LIMIT 10` — matches identity `[2; 32]`
        /// (average 90) and excludes identity `[1; 32]` (average 75).
        fn having_clause() -> HavingClause {
            HavingClause {
                aggregate: HavingAggregate {
                    function: HavingAggregateFunction::Avg,
                    field: "grade".to_string(),
                },
                operator: HavingOperator::GreaterThan,
                right: HavingRightOperand::Value(Value::U64(80)),
            }
        }

        fn client_side_query(contract: &DataContract) -> DriveDocumentHavingQuery<'_> {
            let group_by = vec!["identityId".to_string()];
            let having = vec![having_clause()];
            let mode = detect_having_mode(
                &SelectProjection::avg("grade"),
                &group_by,
                &having,
                &[],
                &[],
                RankedPaginationInputs {
                    limit: Some(10),
                    offset: None,
                    has_start_at: false,
                },
                platform_version(),
            )
            .expect("the case is well-formed");
            let index = find_ranked_index_for_axis(
                contract
                    .document_types()
                    .get("grade")
                    .expect("grade doctype exists")
                    .indexes(),
                &mode.group_by_property,
                mode.bounds.axis(),
                &mode.aggregate_field,
            )
            .expect("the fixture declares the avg axis");
            DriveDocumentHavingQuery {
                document_type: contract
                    .document_type_for_name("grade")
                    .expect("grade doctype exists"),
                contract_id: contract.id_ref().to_buffer(),
                document_type_name: "grade".to_string(),
                index,
                bounds: mode.bounds,
                descending: mode.descending,
                limit: mode.limit,
            }
        }

        /// Prove the having request against the live Drive and return
        /// `(grovedb proof bytes, live root hash)`.
        fn prove(drive: &Drive, contract: &DataContract) -> (Vec<u8>, [u8; 32]) {
            let group_by = vec!["identityId".to_string()];
            let having = vec![having_clause()];
            let response = drive
                .execute_document_having_request(
                    DocumentHavingRequest {
                        contract,
                        document_type: contract
                            .document_type_for_name("grade")
                            .expect("grade doctype exists"),
                        group_by: &group_by,
                        select: SelectProjection::avg("grade"),
                        having: &having,
                        order_by: &[],
                        where_clauses: &[],
                        limit: Some(10),
                        offset: None,
                        has_start_at: false,
                        prove: true,
                    },
                    None,
                    platform_version(),
                )
                .expect("the prove request must execute");
            let proof_bytes = match response {
                DocumentHavingResponse::Proof(proof) => proof,
                DocumentHavingResponse::Entries(_) => panic!("expected a proof, got entries"),
            };
            let root_hash = drive
                .grove
                .root_hash(None, &platform_version().drive.grove_version)
                .unwrap()
                .expect("root hash must be readable");
            (proof_bytes, root_hash)
        }

        fn metadata() -> ResponseMetadata {
            ResponseMetadata {
                height: HEIGHT,
                core_chain_locked_height: CORE_LOCKED_HEIGHT,
                epoch: 0,
                time_ms: TIME_MS,
                protocol_version: platform_version().protocol_version,
                chain_id: CHAIN_ID.to_string(),
            }
        }

        /// Sign a tenderdash precommit whose state id carries
        /// `app_hash` — the same canonical construction
        /// `verify_tenderdash_proof` rebuilds on the verify side.
        fn signed_proof(
            grovedb_proof: Vec<u8>,
            app_hash: &[u8; 32],
            mtd: &ResponseMetadata,
            secret_key: &SecretKey<Bls12381G2Impl>,
            quorum_hash: [u8; 32],
        ) -> Proof {
            let block_id_hash = [7u8; 32].to_vec();
            let state_id = StateId {
                app_version: mtd.protocol_version as u64,
                core_chain_locked_height: mtd.core_chain_locked_height,
                time: mtd.time_ms,
                app_hash: app_hash.to_vec(),
                height: mtd.height,
            };
            let state_id_hash = state_id
                .calculate_msg_hash(&mtd.chain_id, mtd.height as i64, ROUND as i32)
                .expect("state id hash");
            let commit = CanonicalVote {
                r#type: SignedMsgType::Precommit.into(),
                block_id: block_id_hash.clone(),
                chain_id: mtd.chain_id.clone(),
                height: mtd.height as i64,
                round: ROUND as i64,
                state_id: state_id_hash,
            };
            let sign_digest = commit
                .calculate_sign_hash(
                    &mtd.chain_id,
                    QUORUM_TYPE.try_into().expect("valid quorum type"),
                    &quorum_hash,
                    mtd.height as i64,
                    ROUND as i32,
                )
                .expect("sign digest");
            let signature = secret_key
                .sign(SignatureSchemes::Basic, &sign_digest)
                .expect("signing with a valid key succeeds")
                .as_raw_value()
                .to_compressed()
                .to_vec();
            Proof {
                grovedb_proof,
                quorum_hash: quorum_hash.to_vec(),
                signature,
                round: ROUND,
                block_id_hash,
                quorum_type: QUORUM_TYPE,
            }
        }

        /// The full composition succeeds end to end: merk verification
        /// reconstructs the root, the tenderdash commit over that root
        /// verifies against the quorum key, and the verified entries
        /// are exactly the matching groups.
        #[test]
        fn a_correctly_signed_root_verifies_and_returns_the_matches() {
            let (drive, contract) = setup_drive_with_grades();
            let (grovedb_proof, root_hash) = prove(&drive, &contract);
            let secret_key = quorum_secret_key();
            let quorum_hash = [3u8; 32];
            let mtd = metadata();
            let proof = signed_proof(grovedb_proof, &root_hash, &mtd, &secret_key, quorum_hash);
            let provider = TestQuorumProvider {
                pubkey: secret_key.public_key().0.to_compressed(),
            };

            let query = client_side_query(&contract);
            let (verified_root, entries) =
                verify_having_range_proof(&query, &proof, &mtd, platform_version(), &provider)
                    .expect("a correctly signed root must verify");

            assert_eq!(verified_root, root_hash);
            assert_eq!(
                entries.iter().map(|e| e.key.clone()).collect::<Vec<_>>(),
                vec![[2u8; 32].to_vec()],
                "only the identity averaging 90 clears the > 80 bound"
            );
        }

        /// A commit signed over a *different* app hash must not verify:
        /// the node's grovedb proof reconstructs the true root, and the
        /// tenderdash binding is what catches the mismatch.
        #[test]
        fn a_commit_over_a_different_app_hash_is_rejected() {
            let (drive, contract) = setup_drive_with_grades();
            let (grovedb_proof, _root_hash) = prove(&drive, &contract);
            let secret_key = quorum_secret_key();
            let quorum_hash = [3u8; 32];
            let mtd = metadata();
            let wrong_app_hash = [0xAA; 32];
            let proof = signed_proof(
                grovedb_proof,
                &wrong_app_hash,
                &mtd,
                &secret_key,
                quorum_hash,
            );
            let provider = TestQuorumProvider {
                pubkey: secret_key.public_key().0.to_compressed(),
            };

            let query = client_side_query(&contract);
            let error =
                verify_having_range_proof(&query, &proof, &mtd, platform_version(), &provider)
                    .expect_err("a commit over a different app hash must be rejected");
            assert!(
                matches!(error, crate::Error::InvalidSignature { .. }),
                "the rejection must be the signature binding, got: {error:?}"
            );
        }

        /// Tampered response metadata changes the canonical state id,
        /// so a signature over the honest metadata stops verifying.
        #[test]
        fn tampered_metadata_is_rejected() {
            let (drive, contract) = setup_drive_with_grades();
            let (grovedb_proof, root_hash) = prove(&drive, &contract);
            let secret_key = quorum_secret_key();
            let quorum_hash = [3u8; 32];
            let mtd = metadata();
            let proof = signed_proof(grovedb_proof, &root_hash, &mtd, &secret_key, quorum_hash);
            let provider = TestQuorumProvider {
                pubkey: secret_key.public_key().0.to_compressed(),
            };

            let mut tampered = mtd;
            tampered.height += 1;

            let query = client_side_query(&contract);
            let error =
                verify_having_range_proof(&query, &proof, &tampered, platform_version(), &provider)
                    .expect_err("tampered metadata must be rejected");
            assert!(
                matches!(error, crate::Error::InvalidSignature { .. }),
                "the rejection must be the signature binding, got: {error:?}"
            );
        }

        /// A provider vending a different quorum key models a signer
        /// outside the expected quorum: the commit must not verify.
        #[test]
        fn a_wrong_quorum_key_is_rejected() {
            let (drive, contract) = setup_drive_with_grades();
            let (grovedb_proof, root_hash) = prove(&drive, &contract);
            let secret_key = quorum_secret_key();
            let quorum_hash = [3u8; 32];
            let mtd = metadata();
            let proof = signed_proof(grovedb_proof, &root_hash, &mtd, &secret_key, quorum_hash);

            let mut other_bytes = [0u8; 32];
            other_bytes[31] = 43;
            let other_key = SecretKey::<Bls12381G2Impl>::from_be_bytes(&other_bytes)
                .into_option()
                .expect("valid scalar");
            let provider = TestQuorumProvider {
                pubkey: other_key.public_key().0.to_compressed(),
            };

            let query = client_side_query(&contract);
            let error =
                verify_having_range_proof(&query, &proof, &mtd, platform_version(), &provider)
                    .expect_err("a commit signed outside the expected quorum must be rejected");
            assert!(
                matches!(error, crate::Error::InvalidSignature { .. }),
                "the rejection must be the signature binding, got: {error:?}"
            );
        }
    }
}
