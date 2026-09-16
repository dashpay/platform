use crate::drive::document::insert_contested::award_contested_document_vote_poll::ContestedDocumentVotePollAwardOutcome;
use crate::drive::votes::paths::vote_contested_resource_end_date_queries_at_time_tree_path_vec;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    FinalizedContestedDocumentVotePollDriveQueryExecutionResult,
};
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentAndSerialization;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;
use itertools::Itertools;

/// How many of the contenders sharing the top tally are deserialized for the tie-break.
/// The same bound the block executor applied before the selection moved here.
const MAX_TOP_CONTENDERS_TO_TIE_BREAK: usize = 100;

impl Drive {
    /// Selects and awards the winner of an ended contested resource vote poll.
    ///
    /// The checks run in order and return before anything is written:
    ///
    /// 0. The contract the poll names exists in state. It is fetched by id here, and its
    ///    document type and index definition are the ones every later step reads: the
    ///    caller supplies the poll's identity and nothing about the contract.
    /// 1. The poll's stored info exists and its status is `Started`. A poll that was already
    ///    awarded or locked, or that never started, is not awardable; this is the same
    ///    demand the record keeper makes when it finalizes the stored info afterwards.
    /// 2. The block time has reached `end_date`, and the poll's unique id is present in the
    ///    end-date queue under `end_date`. The queue entry is written once when the contest
    ///    starts, with the real end date, and removed by the cleanup after a legitimate
    ///    award, so a caller can neither award early, nor name an arbitrary end date, nor
    ///    award again after cleanup.
    /// 3. The winner is derived from the vote state: contenders with their tallies, sorted
    ///    by tally descending, the top tally group tie-broken by the greatest `$createdAt`,
    ///    then the greatest creation block height, then the greatest creation core height,
    ///    then the greatest document id, the rule the block executor applied before the
    ///    selection moved here. No contenders is `NoWinner`; a lock tally strictly above
    ///    the top tally is `Locked`; otherwise the winner is awarded.
    ///
    /// Only then is the winner's stored document inserted, through the same generic
    /// document insert the block executor used before this operation existed, so a
    /// legitimate award writes the same bytes as before.
    #[inline(always)]
    pub(super) fn award_contested_document_vote_poll_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePoll,
        end_date: TimestampMillis,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentVotePollAwardOutcome, Error> {
        // 0. The contract comes from state, never from the caller.
        let CostContext {
            value: maybe_contract_fetch_info,
            ..
        } = self.fetch_contract(
            vote_poll.contract_id.to_buffer(),
            None,
            None,
            transaction,
            platform_version,
        );

        let Some(contract_fetch_info) = maybe_contract_fetch_info? else {
            return Err(Error::Drive(DriveError::ContestedAwardRejected(format!(
                "vote poll names a contract that is not in state: {}",
                vote_poll.contract_id
            ))));
        };

        let vote_poll =
            vote_poll.resolve_with_provided_arc_contract_fetch_info(contract_fetch_info)?;
        let vote_poll = &vote_poll;

        // 1. The poll must be a started contest.
        let (_, stored_info) = self.fetch_contested_document_vote_poll_stored_info(
            vote_poll,
            None,
            transaction,
            platform_version,
        )?;

        match stored_info.map(|stored_info| stored_info.vote_poll_status()) {
            Some(ContestedDocumentVotePollStatus::Started(_)) => {}
            Some(status) => {
                return Err(Error::Drive(DriveError::ContestedAwardRejected(format!(
                    "vote poll is not a started contest, its status is {status}"
                ))));
            }
            None => {
                return Err(Error::Drive(DriveError::ContestedAwardRejected(
                    "vote poll is not a started contest, it has no stored info".to_string(),
                )));
            }
        }

        // 2. The poll must have ended and still be queued for finalization at that end date.
        if end_date > block_info.time_ms {
            return Err(Error::Drive(DriveError::ContestedAwardRejected(format!(
                "vote poll has not ended: end date {end_date} is after block time {}",
                block_info.time_ms
            ))));
        }

        let end_date_queue_path =
            vote_contested_resource_end_date_queries_at_time_tree_path_vec(end_date);
        let unique_id = vote_poll.unique_id()?;

        let queued_at_end_date = self.grove_has_raw(
            end_date_queue_path.as_slice().into(),
            unique_id.as_bytes(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        if !queued_at_end_date {
            return Err(Error::Drive(DriveError::ContestedAwardRejected(format!(
                "vote poll is not queued for finalization at end date {end_date}"
            ))));
        }

        // 3. Native selection.
        let document_type = vote_poll.document_type()?;

        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll: vote_poll.into(),
            result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            offset: None,
            limit: Some(
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .event_constants
                    .maximum_contenders_to_consider,
            ),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: true,
        };

        let FinalizedContestedDocumentVotePollDriveQueryExecutionResult {
            contenders,
            locked_vote_tally,
            ..
        } = query
            .execute_no_proof(self, transaction, &mut vec![], platform_version)?
            .try_into()?;

        let sorted_contenders: Vec<_> = contenders
            .into_iter()
            .sorted_by(|a, b| Ord::cmp(&b.final_vote_tally, &a.final_vote_tally))
            .collect();

        let highest_vote_tally = sorted_contenders
            .first()
            .map(|top| top.final_vote_tally)
            .unwrap_or_default();

        let top_contenders = sorted_contenders
            .iter()
            .filter(|contender| contender.final_vote_tally == highest_vote_tally)
            .take(MAX_TOP_CONTENDERS_TO_TIE_BREAK)
            .cloned()
            .map(|contender| {
                FinalizedContender::try_from_contender_with_serialized_document(
                    contender,
                    document_type,
                    platform_version,
                )
                .map_err(Error::from)
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let maybe_top_contender = top_contenders.into_iter().max_by(|a, b| {
            a.document
                .created_at()
                .cmp(&b.document.created_at())
                .then_with(|| {
                    a.document
                        .created_at_block_height()
                        .cmp(&b.document.created_at_block_height())
                })
                .then_with(|| {
                    a.document
                        .created_at_core_block_height()
                        .cmp(&b.document.created_at_core_block_height())
                })
                .then_with(|| a.document.id().cmp(&b.document.id()))
        });

        let Some(top_contender) = maybe_top_contender else {
            return Ok(ContestedDocumentVotePollAwardOutcome {
                vote_poll: vote_poll.clone(),
                winner: ContestedDocumentVotePollWinnerInfo::NoWinner,
                contenders: sorted_contenders,
            });
        };

        // A lock tied with the top contender does not lock: the top contender gets it.
        if locked_vote_tally > top_contender.final_vote_tally {
            return Ok(ContestedDocumentVotePollAwardOutcome {
                vote_poll: vote_poll.clone(),
                winner: ContestedDocumentVotePollWinnerInfo::Locked,
                contenders: sorted_contenders,
            });
        }

        // 4. Award: insert the stored contender document as the winner's document.
        let FinalizedContender {
            identity_id,
            document,
            serialized_document,
            ..
        } = top_contender;

        let owned_document_info = OwnedDocumentInfo {
            document_info: DocumentAndSerialization((document, serialized_document, None)),
            owner_id: Some(identity_id.to_buffer()),
        };

        // The award has no payer: the fee result is not used.
        self.add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info,
                contract: vote_poll.contract.as_ref(),
                document_type,
            },
            false,
            *block_info,
            true,
            transaction,
            platform_version,
            None,
        )?;

        Ok(ContestedDocumentVotePollAwardOutcome {
            vote_poll: vote_poll.clone(),
            winner: ContestedDocumentVotePollWinnerInfo::WonByIdentity(identity_id),
            contenders: sorted_contenders,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DriveConfig;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::setup_contract;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::DataContract;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{Bytes32, Value};
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    const CONTESTED_DPNS: &str =
        "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json";
    const LABEL: &str = "quantum";
    /// The block the contest starts in.
    const START_TIME_MS: TimestampMillis = 1_000;

    /// A contest on the contested DPNS fixture between two contenders, plus the votes
    /// each test needs, ready to be awarded.
    struct Contest {
        drive: Drive,
        contract: DataContract,
        /// The poll resolved against the applied contract, for the setup helpers that need a
        /// resolved poll (votes, stored info); the award itself only ever sees `poll_id()`.
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        /// The end date the contest was queued under.
        end_date: TimestampMillis,
        /// The two contenders, sorted by identity id, with the document each one submitted.
        contenders: [(Identifier, Document); 2],
    }

    impl Contest {
        /// The awarded documents of the domain type, by id.
        fn awarded_documents(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from domain",
                &self.contract,
                Some(&DriveConfig::default()),
                platform_version,
            )
            .expect("should build query");
            let (documents, _, _) = query
                .execute_raw_results_no_proof(&self.drive, None, None, platform_version)
                .expect("expected to execute query");
            let domain = self
                .contract
                .document_type_for_name("domain")
                .expect("domain document type");
            documents
                .into_iter()
                .map(|bytes| {
                    Document::from_bytes(&bytes, domain, platform_version)
                        .expect("stored bytes decode")
                })
                .collect()
        }

        /// The awarded documents reachable through the unique `parentNameAndLabel` index.
        fn awarded_documents_by_name(&self) -> Vec<Vec<u8>> {
            self.query_raw(&format!(
                "select * from domain where normalizedParentDomainName = 'dash' and \
                 normalizedLabel = '{LABEL}'"
            ))
        }

        /// The awarded documents reachable through the `identityId` index for an identity.
        /// The index is on the nested `records.identity`, which the SQL grammar cannot
        /// name, so the query is built directly.
        fn awarded_documents_by_identity(&self, identity_id: Identifier) -> Vec<Vec<u8>> {
            let platform_version = PlatformVersion::latest();
            let domain = self
                .contract
                .document_type_for_name("domain")
                .expect("domain document type");
            let query = DriveDocumentQuery {
                contract: &self.contract,
                document_type: domain,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clauses: Vec::new(),
                    range_clause: None,
                    equal_clauses: BTreeMap::from([(
                        "records.identity".to_string(),
                        WhereClause {
                            field: "records.identity".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Identifier(identity_id.to_buffer()),
                        },
                    )]),
                },
                offset: None,
                limit: None,
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
                resolved_time_ranges: vec![],
                sub_queries: vec![],
            };
            let (documents, _, _) = query
                .execute_raw_results_no_proof(&self.drive, None, None, platform_version)
                .expect("expected to execute query");
            documents
        }

        fn query_raw(&self, sql: &str) -> Vec<Vec<u8>> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                sql,
                &self.contract,
                Some(&DriveConfig::default()),
                platform_version,
            )
            .expect("should build query");
            let (documents, _, _) = query
                .execute_raw_results_no_proof(&self.drive, None, None, platform_version)
                .expect("expected to execute query");
            documents
        }

        fn root_hash(&self) -> [u8; 32] {
            let platform_version = PlatformVersion::latest();
            self.drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected a root hash")
        }

        fn stored_status(&self) -> ContestedDocumentVotePollStatus {
            let platform_version = PlatformVersion::latest();
            let (_, stored_info) = self
                .drive
                .fetch_contested_document_vote_poll_stored_info(
                    &self.vote_poll,
                    None,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the stored info");
            stored_info
                .expect("expected the contest to have stored info")
                .vote_poll_status()
        }

        /// The poll's identity, which is all the award accepts.
        fn poll_id(&self) -> ContestedDocumentResourceVotePoll {
            (&self.vote_poll).into()
        }

        /// Awards at the contest's end date, exactly as the finalization sweep would.
        fn award(&self) -> Result<ContestedDocumentVotePollAwardOutcome, Error> {
            self.award_at(self.end_date, self.end_date)
        }

        fn award_at(
            &self,
            end_date: TimestampMillis,
            block_time_ms: TimestampMillis,
        ) -> Result<ContestedDocumentVotePollAwardOutcome, Error> {
            let platform_version = PlatformVersion::latest();
            self.drive.award_contested_document_vote_poll(
                &self.poll_id(),
                end_date,
                &BlockInfo::default_with_time(block_time_ms),
                None,
                platform_version,
            )
        }

        /// Registers `count` votes of strength 1 from distinct masternodes for a choice.
        fn vote(&self, choice: ResourceVoteChoice, count: u8, seed: u8) {
            let platform_version = PlatformVersion::latest();
            for i in 0..count {
                let mut pro_tx_hash = [seed; 32];
                pro_tx_hash[0] = i;
                self.drive
                    .register_contested_resource_identity_vote(
                        pro_tx_hash,
                        1,
                        self.vote_poll.clone(),
                        choice,
                        None,
                        &BlockInfo::default_with_time(START_TIME_MS + 1),
                        None,
                        platform_version,
                    )
                    .expect("expected to register the vote");
            }
        }

        /// Flips the stored info to the state the record keeper leaves behind a
        /// finalization with `winner`.
        fn finalize_stored_info(&self, winner: ContestedDocumentVotePollWinnerInfo) {
            let platform_version = PlatformVersion::latest();
            let (_, stored_info) = self
                .drive
                .fetch_contested_document_vote_poll_stored_info(
                    &self.vote_poll,
                    None,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the stored info");
            let mut stored_info = stored_info.expect("expected stored info");
            stored_info
                .finalize_vote_poll(vec![], BlockInfo::default_with_time(self.end_date), winner)
                .expect("expected to finalize the stored info");
            self.drive
                .insert_stored_info_for_contested_resource_vote_poll(
                    &self.vote_poll,
                    stored_info,
                    None,
                    platform_version,
                )
                .expect("expected to store the finalized info");
        }
    }

    /// Starts a contest between two contenders on the contested DPNS fixture. The
    /// contenders join at `START_TIME_MS`, `created_at_offsets` apart, so a test can
    /// choose which one was created first.
    fn setup_contest(created_at_offsets: [TimestampMillis; 2]) -> Contest {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = setup_contract(
            &drive,
            CONTESTED_DPNS,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        let domain = contract
            .document_type_for_name("domain")
            .expect("domain document type");

        let mut rng = StdRng::seed_from_u64(0xF1);
        let mut identities = [
            Identifier::random_with_rng(&mut rng),
            Identifier::random_with_rng(&mut rng),
        ];
        identities.sort();

        let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text(LABEL.to_string()),
            ],
        };

        // The Drive test config is mainnet, so the poll runs for the mainnet duration.
        assert_eq!(drive.config.network, Network::Mainnet);
        let end_date = START_TIME_MS
            + platform_version
                .dpp
                .voting_versions
                .default_vote_poll_time_duration_mainnet_ms;

        let mut contenders = Vec::with_capacity(2);
        for (position, identity_id) in identities.into_iter().enumerate() {
            let created_at = START_TIME_MS + created_at_offsets[position];
            let mut document = domain
                .random_document_with_params(
                    identity_id,
                    Bytes32::random_with_rng(&mut rng),
                    Some(created_at),
                    Some(1),
                    Some(1),
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    &mut rng,
                    platform_version,
                )
                .expect("random domain document");
            document.set("parentDomainName", "dash".into());
            document.set("normalizedParentDomainName", "dash".into());
            document.set("label", LABEL.into());
            document.set("normalizedLabel", LABEL.into());
            document.set("records.identity", identity_id.into());
            document.set("subdomainRules.allowSubdomains", false.into());

            // The first contender starts the contest and stores its info; the second
            // joins it, as the create transition does for a contest that already exists.
            let stored_info = (position == 0).then(|| {
                ContestedDocumentVotePollStoredInfo::new(
                    BlockInfo::default_with_time(START_TIME_MS),
                    platform_version,
                )
                .expect("stored info")
            });

            drive
                .add_contested_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((
                                &document,
                                StorageFlags::optional_default_as_cow(),
                            )),
                            owner_id: Some(identity_id.to_buffer()),
                        },
                        contract: &contract,
                        document_type: domain,
                    },
                    vote_poll.clone(),
                    false,
                    BlockInfo::default_with_time(START_TIME_MS),
                    true,
                    stored_info,
                    None,
                    platform_version,
                )
                .expect("expected to add the contender");

            contenders.push((identity_id, document));
        }

        let contenders = contenders
            .try_into()
            .unwrap_or_else(|_| panic!("exactly two contenders"));

        Contest {
            drive,
            contract,
            vote_poll,
            end_date,
            contenders,
        }
    }

    fn assert_awarded(contest: &Contest, winner: Identifier, winner_document: &Document) {
        let awarded = contest.awarded_documents();
        assert_eq!(awarded.len(), 1, "exactly one domain document is awarded");
        let awarded = &awarded[0];
        assert_eq!(awarded.id(), winner_document.id());
        assert_eq!(awarded.owner_id(), winner);

        // Atomic index effects: the document is reachable through the unique index and the
        // secondary index, and only the winner's identity finds it.
        assert_eq!(contest.awarded_documents_by_name().len(), 1);
        assert_eq!(contest.awarded_documents_by_identity(winner).len(), 1);
        for (identity_id, _) in &contest.contenders {
            if *identity_id != winner {
                assert!(contest
                    .awarded_documents_by_identity(*identity_id)
                    .is_empty());
            }
        }
    }

    #[test]
    fn should_award_the_contender_with_the_highest_tally() {
        let contest = setup_contest([0, 0]);
        let [(first, first_document), (second, _)] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        contest.vote(ResourceVoteChoice::TowardsIdentity(*second), 2, 0xB0);

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(
            outcome.winner,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(*first)
        );
        assert_eq!(outcome.contenders.len(), 2);
        assert_eq!(outcome.contenders[0].identity_id, *first);
        assert_eq!(outcome.contenders[0].final_vote_tally, 5);
        assert_eq!(outcome.contenders[1].identity_id, *second);
        assert_eq!(outcome.contenders[1].final_vote_tally, 2);
        assert_awarded(&contest, *first, first_document);
    }

    /// The winner comes from the tallies, never from the caller: with the votes swapped the
    /// other contender is awarded from an identical call.
    #[test]
    fn should_award_the_other_contender_when_the_tallies_are_swapped() {
        let contest = setup_contest([0, 0]);
        let [(first, _), (second, second_document)] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 2, 0xA0);
        contest.vote(ResourceVoteChoice::TowardsIdentity(*second), 5, 0xB0);

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(
            outcome.winner,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(*second)
        );
        assert_awarded(&contest, *second, second_document);
    }

    /// A tie on the tally goes to the contender whose document carries the greatest
    /// `$createdAt`: the native tie-break the block executor always applied.
    #[test]
    fn should_break_a_tie_by_the_greatest_creation_time() {
        let contest = setup_contest([0, 500]);
        let [(first, _), (second, second_document)] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 3, 0xA0);
        contest.vote(ResourceVoteChoice::TowardsIdentity(*second), 3, 0xB0);

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(
            outcome.winner,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(*second)
        );
        assert_awarded(&contest, *second, second_document);
    }

    /// A tie on the tally and on creation goes to the greatest document id.
    #[test]
    fn should_break_a_full_tie_by_the_greatest_document_id() {
        let contest = setup_contest([0, 0]);
        let [(first, first_document), (second, second_document)] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 3, 0xA0);
        contest.vote(ResourceVoteChoice::TowardsIdentity(*second), 3, 0xB0);

        let (expected_winner, expected_document) = if first_document.id() > second_document.id() {
            (*first, first_document)
        } else {
            (*second, second_document)
        };

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(
            outcome.winner,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(expected_winner)
        );
        assert_awarded(&contest, expected_winner, expected_document);
    }

    #[test]
    fn should_lock_when_lock_votes_exceed_the_top_tally() {
        let contest = setup_contest([0, 0]);
        let [(first, _), (second, _)] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 2, 0xA0);
        contest.vote(ResourceVoteChoice::TowardsIdentity(*second), 1, 0xB0);
        contest.vote(ResourceVoteChoice::Lock, 3, 0xC0);
        let root_hash_before = contest.root_hash();

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(outcome.winner, ContestedDocumentVotePollWinnerInfo::Locked);
        assert_eq!(outcome.contenders.len(), 2);
        assert!(contest.awarded_documents().is_empty());
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    /// A lock tally equal to the top tally does not lock: the top contender is awarded.
    #[test]
    fn should_award_when_lock_votes_only_tie_the_top_tally() {
        let contest = setup_contest([0, 0]);
        let [(first, first_document), (second, _)] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 3, 0xA0);
        contest.vote(ResourceVoteChoice::TowardsIdentity(*second), 1, 0xB0);
        contest.vote(ResourceVoteChoice::Lock, 3, 0xC0);

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(
            outcome.winner,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(*first)
        );
        assert_awarded(&contest, *first, first_document);
    }

    #[test]
    fn should_award_by_the_tie_break_when_nobody_voted() {
        // With no votes at all both contenders tie at zero and the tie-break decides; a
        // poll cannot start without a contender, so the no-winner outcome is unreachable
        // through a started contest. This pins that zero votes still award rather than
        // stall the poll.
        let contest = setup_contest([0, 100]);
        let [_, (second, second_document)] = &contest.contenders;

        let outcome = contest.award().expect("expected the award to succeed");

        assert_eq!(
            outcome.winner,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(*second)
        );
        assert_awarded(&contest, *second, second_document);
    }

    #[test]
    fn should_reject_an_award_before_the_end_date() {
        let contest = setup_contest([0, 0]);
        let [(first, _), _] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        let root_hash_before = contest.root_hash();

        let error = contest
            .award_at(contest.end_date, contest.end_date - 1)
            .expect_err("an award before the end date must be rejected");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.starts_with("vote poll has not ended")),
            "unexpected error: {error:?}"
        );
        assert!(contest.awarded_documents().is_empty());
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    /// A caller naming an end date the poll was never queued under (an earlier one, so the
    /// block time check passes) is rejected: the timing is read from the queue, not the
    /// argument.
    #[test]
    fn should_reject_an_award_at_an_end_date_the_poll_is_not_queued_under() {
        let contest = setup_contest([0, 0]);
        let [(first, _), _] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        let root_hash_before = contest.root_hash();

        let forged_end_date = START_TIME_MS + 1;
        let error = contest
            .award_at(forged_end_date, contest.end_date)
            .expect_err("an award at a forged end date must be rejected");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.starts_with("vote poll is not queued for finalization")),
            "unexpected error: {error:?}"
        );
        assert!(contest.awarded_documents().is_empty());
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    #[test]
    fn should_reject_an_award_of_a_poll_that_was_already_awarded() {
        let contest = setup_contest([0, 0]);
        let [(first, _), _] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        contest.finalize_stored_info(ContestedDocumentVotePollWinnerInfo::WonByIdentity(*first));
        assert!(matches!(
            contest.stored_status(),
            ContestedDocumentVotePollStatus::Awarded(_)
        ));
        let root_hash_before = contest.root_hash();

        let error = contest
            .award()
            .expect_err("an awarded poll must not be awarded again");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.starts_with("vote poll is not a started contest")),
            "unexpected error: {error:?}"
        );
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    #[test]
    fn should_reject_an_award_of_a_locked_poll() {
        let contest = setup_contest([0, 0]);
        contest.finalize_stored_info(ContestedDocumentVotePollWinnerInfo::Locked);
        assert!(matches!(
            contest.stored_status(),
            ContestedDocumentVotePollStatus::Locked
        ));
        let root_hash_before = contest.root_hash();

        let error = contest
            .award()
            .expect_err("a locked poll must not be awarded");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.starts_with("vote poll is not a started contest")),
            "unexpected error: {error:?}"
        );
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    #[test]
    fn should_reject_an_award_of_a_poll_that_has_not_started() {
        let contest = setup_contest([0, 0]);
        contest.finalize_stored_info(ContestedDocumentVotePollWinnerInfo::NoWinner);
        assert!(matches!(
            contest.stored_status(),
            ContestedDocumentVotePollStatus::NotStarted
        ));
        let root_hash_before = contest.root_hash();

        let error = contest
            .award()
            .expect_err("a poll that has not started must not be awarded");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.starts_with("vote poll is not a started contest")),
            "unexpected error: {error:?}"
        );
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    #[test]
    fn should_reject_an_award_of_a_poll_without_stored_info() {
        let platform_version = PlatformVersion::latest();
        let contest = setup_contest([0, 0]);
        // A poll for a name nobody contested: it resolves against the same contract but has
        // no stored info, no queue entry and no contenders.
        let unknown_poll = ContestedDocumentResourceVotePoll {
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("nobody".to_string()),
            ],
            ..contest.poll_id()
        };
        let root_hash_before = contest.root_hash();

        let error = contest
            .drive
            .award_contested_document_vote_poll(
                &unknown_poll,
                contest.end_date,
                &BlockInfo::default_with_time(contest.end_date),
                None,
                platform_version,
            )
            .expect_err("a poll without stored info must not be awarded");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.ends_with("it has no stored info")),
            "unexpected error: {error:?}"
        );
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    /// The contract is read from state by the id the poll names, so a caller cannot route
    /// the award through metadata of its own: a poll naming a contract id that is not in
    /// state is rejected before anything else is read, with nothing applied.
    #[test]
    fn should_reject_an_award_naming_a_contract_that_is_not_in_state() {
        let platform_version = PlatformVersion::latest();
        let contest = setup_contest([0, 0]);
        let [(first, _), _] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        let root_hash_before = contest.root_hash();

        let forged_contract_poll = ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([0xFE; 32]),
            ..contest.poll_id()
        };

        let error = contest
            .drive
            .award_contested_document_vote_poll(
                &forged_contract_poll,
                contest.end_date,
                &BlockInfo::default_with_time(contest.end_date),
                None,
                platform_version,
            )
            .expect_err("a poll naming an unknown contract must not be awarded");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedAwardRejected(ref reason)) if reason.starts_with("vote poll names a contract that is not in state")),
            "unexpected error: {error:?}"
        );
        assert!(contest.awarded_documents().is_empty());
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    /// The document type and index the award uses are the committed contract's, so a poll
    /// naming an index the committed contract does not define is a typed rejection with
    /// nothing applied, whatever the caller believes the index to be.
    #[test]
    fn should_reject_an_award_naming_an_index_the_committed_contract_does_not_define() {
        let platform_version = PlatformVersion::latest();
        let contest = setup_contest([0, 0]);
        let [(first, _), _] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        let root_hash_before = contest.root_hash();

        let forged_index_poll = ContestedDocumentResourceVotePoll {
            index_name: "forgedIndex".to_string(),
            ..contest.poll_id()
        };

        let error = contest
            .drive
            .award_contested_document_vote_poll(
                &forged_index_poll,
                contest.end_date,
                &BlockInfo::default_with_time(contest.end_date),
                None,
                platform_version,
            )
            .expect_err("a poll naming an index the contract does not define must not be awarded");

        assert!(
            matches!(error, Error::Drive(DriveError::ContestedIndexNotFound(_))),
            "unexpected error: {error:?}"
        );
        assert!(contest.awarded_documents().is_empty());
        assert_eq!(contest.root_hash(), root_hash_before);
    }

    /// Between a legitimate award and the record keeper's status flip, a second call cannot
    /// produce a divergent award: the winner's document already exists in primary storage.
    #[test]
    fn should_fail_a_second_award_before_the_status_flips() {
        let contest = setup_contest([0, 0]);
        let [(first, first_document), _] = &contest.contenders;
        contest.vote(ResourceVoteChoice::TowardsIdentity(*first), 5, 0xA0);
        contest
            .award()
            .expect("expected the first award to succeed");
        let root_hash_after_award = contest.root_hash();

        let error = contest
            .award()
            .expect_err("a second award of the same poll must fail");

        assert!(
            matches!(
                error,
                Error::Drive(DriveError::CorruptedDocumentAlreadyExists(_))
            ),
            "unexpected error: {error:?}"
        );
        assert_awarded(&contest, *first, first_document);
        assert_eq!(contest.root_hash(), root_hash_after_award);
    }
}
