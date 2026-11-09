use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::{
    AppliedRandomness, BlockExecutionContextV0Getters, BlockExecutionContextV0Setters,
};
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods,
};
use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::epoch::Epoch;
use drive::error::Error as DriveError;
use drive::grovedb::Transaction;
use tenderdash_abci::proto::abci as proto;

/// Applies the random beacon of a block to the state this node's `ProcessProposal` left for it
/// in the request's round, and returns the app hash the round's precommits sign.
///
/// Tenderdash calls it from consensus version 2 (protocol version 15): on a validator once the
/// round's prevotes combine into the beacon, before it precommits, and on every path that
/// finalizes a block from its commit (block sync, replay, a node that did not vote) before
/// `FinalizeBlock`, with the commit's beacon and round. Tenderdash processes the block in that
/// round first, so the block execution context must be that block's. It may ask again for the
/// same round, and is answered with the same app hash.
pub fn apply_block_randomness<'a, A, C>(
    app: &A,
    request: proto::RequestApplyBlockRandomness,
) -> Result<proto::ResponseApplyBlockRandomness, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("apply_block_randomness");

    let proto::RequestApplyBlockRandomness {
        hash: block_hash,
        height,
        round,
        beacon,
    } = request;

    let height = u64::try_from(height).map_err(|_| {
        AbciError::BadRequest("height is negative in apply block randomness request".to_string())
    })?;
    let round = u32::try_from(round).map_err(|_| {
        AbciError::BadRequest("round is negative in apply block randomness request".to_string())
    })?;

    // The transaction first, then the block execution context, the order every handler takes
    // them in
    let transaction_guard = app
        .transaction()
        .read()
        .expect("poisoned only after a panic, which stops the node");
    let mut block_execution_context_guard = app
        .block_execution_context()
        .write()
        .expect("poisoned only after a panic, which stops the node");

    let Some(block_execution_context) = block_execution_context_guard.as_mut() else {
        return Err(AbciError::RequestForWrongBlockReceived(format!(
            "received an apply block randomness request for height: {} round: {}, block: {}, but no block was processed",
            height,
            round,
            hex::encode(&block_hash),
        ))
        .into());
    };

    let block_state_info = block_execution_context.block_state_info();
    if !block_state_info.matches_current_block(height, round, block_hash.clone())? {
        return Err(AbciError::RequestForWrongBlockReceived(format!(
            "received an apply block randomness request for height: {} round: {}, block: {}, but the block processed last is height: {} round: {}, block: {}",
            height,
            round,
            hex::encode(&block_hash),
            block_state_info.height(),
            block_state_info.round(),
            block_state_info
                .block_hash()
                .map(hex::encode)
                .unwrap_or("None".to_string()),
        ))
        .into());
    }

    let transaction =
        transaction_guard
            .as_ref()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "applying a block's randomness without a current transaction",
            )))?;

    let app_hash = apply_randomness(
        app.platform(),
        block_execution_context,
        round,
        &beacon,
        transaction,
    )?;

    Ok(proto::ResponseApplyBlockRandomness {
        app_hash: app_hash.to_vec(),
    })
}

/// Applies `beacon`, the random beacon of `round`, to the block `block_execution_context`
/// holds, in `transaction`, the block's transaction, and returns the app hash after the draws:
/// the one the round's precommits sign and the block commits. Asked again for the same round
/// and beacon, it returns that app hash without drawing again; another beacon is refused, as a
/// round's prevotes combine into exactly one. When the block's protocol version draws nothing
/// (`apply_random_beacon` is `None`, before 15), nothing is applied and the app hash is the
/// block's own.
///
/// Drive does not verify the beacon's BLS signature: the request carries no part-set header,
/// so the prevote sign bytes the beacon signs cannot be rebuilt here. Tenderdash verified it
/// against the quorum's threshold public key when it recovered it from the prevotes, and
/// verifies it again in every commit it accepts.
pub(super) fn apply_randomness<C>(
    platform: &Platform<C>,
    block_execution_context: &mut BlockExecutionContext,
    round: u32,
    beacon: &[u8],
    transaction: &Transaction,
) -> Result<[u8; 32], Error>
where
    C: CoreRPCLike,
{
    if let Some(applied) = block_execution_context.applied_randomness() {
        if applied.round == round && applied.beacon == beacon {
            return Ok(applied.app_hash);
        }
        return Err(AbciError::BadRequest(format!(
            "the randomness of round {} was applied to the block at height {} already; the beacon {} of round {} differs",
            applied.round,
            block_execution_context.block_state_info().height(),
            hex::encode(beacon),
            round,
        ))
        .into());
    }

    let platform_version = block_execution_context
        .block_platform_state()
        .current_platform_version()?;

    if platform_version
        .drive_abci
        .methods
        .block_end
        .apply_random_beacon
        .is_some()
    {
        let epoch = Epoch::new(block_execution_context.epoch_info().current_epoch_index())
            .map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "the epoch of an executed block is in range",
                ))
            })?;
        let block_info = block_execution_context
            .block_state_info()
            .to_block_info(epoch);
        platform.apply_random_beacon(
            beacon,
            block_execution_context.random_requests(),
            &block_info,
            transaction,
            platform_version,
        )?;
    }

    let app_hash = platform
        .drive
        .grove
        .root_hash(Some(transaction), &platform_version.drive.grove_version)
        .unwrap()
        .map_err(|e| Error::Drive(DriveError::from(e)))?; // GroveDb errors are system errors

    block_execution_context.set_applied_randomness(Some(AppliedRandomness {
        round,
        beacon: beacon.to_vec(),
        app_hash,
    }));

    Ok(app_hash)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::abci::app::FullAbciApplication;
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_random_requests::BlockRandomRequests;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::balances::credits::TokenAmount;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use drive::drive::document::draw_random_document_properties::RandomDocumentRequest;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::drive::tokens::settle_random_token_amounts::{
        RandomTokenRequest, RandomTokenRequestKind,
    };
    use drive::query::DriveDocumentQuery;
    use drive::util::batch::drive_op_batch::TokenOperationType;
    use drive::util::batch::DriveOperation;
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use drive::util::storage_flags::StorageFlags;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    pub(in crate::abci::handler) const OWNER: [u8; 32] = [42; 32];
    pub(in crate::abci::handler) const BEACON: [u8; 96] = [7; 96];
    const BLOCK_HASH: [u8; 32] = [0xAA; 32];
    const HEIGHT: u64 = 10;
    /// The rarity the card draws from [`BEACON`], pinned so a skipped draw, which leaves the
    /// placeholder 1, fails
    const DRAWN_RARITY: i128 = 26;
    const TOKEN: [u8; 32] = [5; 32];
    pub(in crate::abci::handler) const RECIPIENT: [u8; 32] = [8; 32];
    pub(in crate::abci::handler) const TOKEN_SUPPLY: TokenAmount = 10_000;

    /// A contract with an immutable `card` type, its `rarity` drawn from 1 to 100 and indexed.
    pub(in crate::abci::handler) fn card_contract() -> DataContract {
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("expected a contract factory")
            .create_with_value_config(
                Identifier::from(OWNER),
                0,
                platform_value!({
                    "card": {
                        "type": "object",
                        "documentsMutable": false,
                        "properties": {
                            "name": { "type": "string", "maxLength": 32, "position": 0 },
                            "rarity": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "generatedFrom": { "function": "sys.random.uniform" },
                                "position": 1
                            }
                        },
                        "indices": [{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }],
                        "required": ["name", "rarity"],
                        "additionalProperties": false
                    }
                }),
                None,
                None,
            )
            .expect("expected the card contract")
            .data_contract_owned()
    }

    /// Applies the card contract to committed state.
    pub(in crate::abci::handler) fn apply_card_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
    ) -> DataContract {
        let contract = card_contract();
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to apply the card contract");
        contract
    }

    /// Inserts a `card` holding the placeholder rarity, the way a create transition writes it,
    /// in `transaction`, and returns the request the block records for it.
    pub(in crate::abci::handler) fn insert_placeholder_card(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        transaction: &Transaction,
    ) -> RandomDocumentRequest {
        let id = Identifier::from([1; 32]);
        let document = Document::V0(DocumentV0 {
            id,
            owner_id: Identifier::from(OWNER),
            properties: BTreeMap::from([
                ("name".to_string(), Value::Text("Ace".to_string())),
                ("rarity".to_string(), Value::U8(1)),
            ]),
            ..Default::default()
        });
        platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(OWNER)))),
                        )),
                        owner_id: Some(OWNER),
                    },
                    contract,
                    document_type: contract
                        .document_type_for_name("card")
                        .expect("expected the card type"),
                },
                false,
                BlockInfo::default(),
                true,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected the card to insert");
        RandomDocumentRequest {
            contract_id: contract.id(),
            document_type_name: "card".to_string(),
            document_id: id,
        }
    }

    /// The rarity the card holds in `transaction`.
    pub(in crate::abci::handler) fn stored_rarity(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        transaction: &Transaction,
    ) -> i128 {
        let platform_version = PlatformVersion::latest();
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from card",
            contract,
            Some(&platform.config.drive),
            platform_version,
        )
        .expect("expected a card query");
        let documents = platform
            .drive
            .query_documents(
                query,
                None,
                false,
                Some(transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected the card query to run")
            .documents()
            .to_vec();
        assert_eq!(documents.len(), 1, "one card is stored");
        documents[0]
            .get("rarity")
            .and_then(|rarity| rarity.to_integer::<i128>().ok())
            .expect("expected the card to hold a rarity")
    }

    /// The context `ProcessProposal` leaves for the block at [`HEIGHT`] in `round` at
    /// `protocol_version`, which created the cards `documents` names.
    pub(in crate::abci::handler) fn block_execution_context(
        platform: &TempPlatform<MockCoreRPCLike>,
        protocol_version: u32,
        round: u32,
        block_hash: [u8; 32],
        documents: Vec<RandomDocumentRequest>,
    ) -> BlockExecutionContext {
        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfoV0 {
                height: HEIGHT,
                round,
                block_time_ms: 1_700_000_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: Some(block_hash),
                app_hash: None,
            }
            .into(),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state: PlatformState::default_with_protocol_versions(
                protocol_version,
                protocol_version,
                &platform.config,
            )
            .expect("expected a platform state"),
            proposer_results: None,
            random_requests: BlockRandomRequests {
                documents,
                tokens: Vec::new(),
            },
            applied_randomness: None,
        })
    }

    fn request(
        round: i32,
        block_hash: [u8; 32],
        beacon: &[u8],
    ) -> proto::RequestApplyBlockRandomness {
        proto::RequestApplyBlockRandomness {
            hash: block_hash.to_vec(),
            height: HEIGHT as i64,
            round,
            beacon: beacon.to_vec(),
        }
    }

    fn transaction_root(app: &FullAbciApplication<MockCoreRPCLike>) -> [u8; 32] {
        let transaction_guard = app.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().expect("expected a transaction");
        app.platform
            .drive
            .grove
            .root_hash(
                Some(transaction),
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .expect("expected a root hash")
    }

    fn rarity_in_block(
        platform: &TempPlatform<MockCoreRPCLike>,
        app: &FullAbciApplication<MockCoreRPCLike>,
        contract: &DataContract,
    ) -> i128 {
        let transaction_guard = app.transaction.read().unwrap();
        stored_rarity(
            platform,
            contract,
            transaction_guard.as_ref().expect("expected a transaction"),
        )
    }

    /// Reserves, in `transaction`, a random transfer of 100 to 1,000 of `OWNER`'s tokens to
    /// [`RECIPIENT`], as the execution of a random transfer does, and returns the request the
    /// block records for it.
    pub(in crate::abci::handler) fn reserve_random_transfer_in(
        platform: &TempPlatform<MockCoreRPCLike>,
        transaction: &Transaction,
    ) -> RandomTokenRequest {
        let platform_version = PlatformVersion::latest();
        let request = RandomTokenRequest {
            token_id: Identifier::from(TOKEN),
            owner_id: Identifier::from(OWNER),
            identity_contract_nonce: 1,
            minimum: 100,
            maximum: 1_000,
            kind: RandomTokenRequestKind::Transfer {
                sender_id: Identifier::from(OWNER),
                recipient_id: Identifier::from(RECIPIENT),
            },
        };
        platform
            .drive
            .create_token_trees(
                Identifier::from([4; 32]),
                0,
                TOKEN,
                false,
                false,
                &BlockInfo::default(),
                true,
                Some(transaction),
                platform_version,
            )
            .expect("expected to create the token trees");
        platform
            .drive
            .token_mint(
                TOKEN,
                OWNER,
                TOKEN_SUPPLY,
                false,
                false,
                &BlockInfo::default(),
                true,
                Some(transaction),
                platform_version,
            )
            .expect("expected to mint the supply");
        platform
            .drive
            .apply_drive_operations(
                vec![DriveOperation::TokenOperation(
                    TokenOperationType::TokenReserveRandomTransfer {
                        token_id: request.token_id,
                        sender_id: request.owner_id,
                        recipient_id: Identifier::from(RECIPIENT),
                        identity_contract_nonce: request.identity_contract_nonce,
                        maximum: request.maximum,
                    },
                )],
                true,
                &BlockInfo::default(),
                Some(transaction),
                platform_version,
            )
            .expect("expected the reservation to apply");
        request
    }

    /// [`reserve_random_transfer_in`] the block's transaction, recording the request in the
    /// block's context.
    fn reserve_random_transfer(
        platform: &TempPlatform<MockCoreRPCLike>,
        app: &FullAbciApplication<MockCoreRPCLike>,
    ) -> RandomTokenRequest {
        let request = {
            let transaction_guard = app.transaction.read().unwrap();
            reserve_random_transfer_in(
                platform,
                transaction_guard.as_ref().expect("expected a transaction"),
            )
        };
        let mut context_guard = app.block_execution_context.write().unwrap();
        let Some(BlockExecutionContext::V0(context)) = context_guard.as_mut() else {
            panic!("expected the block's context");
        };
        context.random_requests.tokens.push(request.clone());
        request
    }

    /// The token balance of `identity_id` and what the escrow holds under `request`'s
    /// reservation, in the block's transaction.
    fn token_state_in_block(
        platform: &TempPlatform<MockCoreRPCLike>,
        app: &FullAbciApplication<MockCoreRPCLike>,
        identity_id: [u8; 32],
        request: &RandomTokenRequest,
    ) -> (Option<TokenAmount>, Option<TokenAmount>) {
        let platform_version = PlatformVersion::latest();
        let transaction_guard = app.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().expect("expected a transaction");
        let balance = platform
            .drive
            .fetch_identity_token_balance(TOKEN, identity_id, Some(transaction), platform_version)
            .expect("expected to fetch a token balance");
        let reserved = platform
            .drive
            .fetch_token_order_escrow(
                TOKEN,
                request.reservation_id(),
                Some(transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to fetch the reservation");
        (balance, reserved)
    }

    /// A platform holding the card contract in committed state.
    fn setup() -> (TempPlatform<MockCoreRPCLike>, DataContract) {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let contract = apply_card_contract(&platform);
        (platform, contract)
    }

    /// An application whose transaction holds a placeholder card, and whose context is the
    /// block at `protocol_version` that created it, processed in round 0.
    fn start_block<'a>(
        platform: &'a TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        protocol_version: u32,
    ) -> FullAbciApplication<'a, MockCoreRPCLike> {
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        app.start_transaction();
        let card = {
            let transaction_guard = app.transaction.read().unwrap();
            insert_placeholder_card(
                platform,
                contract,
                transaction_guard.as_ref().expect("expected a transaction"),
            )
        };
        app.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context(
                platform,
                protocol_version,
                0,
                BLOCK_HASH,
                vec![card],
            ));
        app
    }

    #[test]
    fn should_draw_the_block_and_return_the_root_after_the_draws() {
        let latest = PlatformVersion::latest().protocol_version;
        let (platform, contract) = setup();
        let app = start_block(&platform, &contract, latest);
        let root_before = transaction_root(&app);
        assert_eq!(rarity_in_block(&platform, &app, &contract), 1);

        let response =
            apply_block_randomness::<_, MockCoreRPCLike>(&app, request(0, BLOCK_HASH, &BEACON))
                .expect("expected the randomness to apply");

        let root_after = transaction_root(&app);
        assert_eq!(response.app_hash, root_after.to_vec());
        assert_eq!(rarity_in_block(&platform, &app, &contract), DRAWN_RARITY);
        assert_ne!(root_before, root_after, "the drawn value moved the root");
        assert_eq!(
            app.block_execution_context
                .read()
                .unwrap()
                .as_ref()
                .expect("expected the context")
                .applied_randomness(),
            Some(&AppliedRandomness {
                round: 0,
                beacon: BEACON.to_vec(),
                app_hash: root_after,
            })
        );
    }

    #[test]
    fn should_answer_the_same_round_again_without_drawing_again() {
        let latest = PlatformVersion::latest().protocol_version;
        let (platform, contract) = setup();
        let app = start_block(&platform, &contract, latest);
        // A settlement removes its reservation, and a second one of the same request is
        // refused, so a settlement run twice would fail the second answer
        let transfer = reserve_random_transfer(&platform, &app);
        assert_eq!(
            token_state_in_block(&platform, &app, RECIPIENT, &transfer),
            (Some(0), Some(transfer.maximum))
        );

        let first =
            apply_block_randomness::<_, MockCoreRPCLike>(&app, request(0, BLOCK_HASH, &BEACON))
                .expect("expected the randomness to apply");
        let drawn = rarity_in_block(&platform, &app, &contract);
        let (paid, reserved) = token_state_in_block(&platform, &app, RECIPIENT, &transfer);
        let paid = paid.expect("expected the recipient to hold the drawn amount");
        assert!((transfer.minimum..=transfer.maximum).contains(&paid));
        assert_eq!(reserved, None, "the settlement removed the reservation");
        assert_eq!(
            token_state_in_block(&platform, &app, OWNER, &transfer).0,
            Some(TOKEN_SUPPLY - paid)
        );

        let second =
            apply_block_randomness::<_, MockCoreRPCLike>(&app, request(0, BLOCK_HASH, &BEACON))
                .expect("expected the same round to be answered again");
        assert_eq!(first, second);
        assert_eq!(second.app_hash, transaction_root(&app).to_vec());
        assert_eq!(rarity_in_block(&platform, &app, &contract), drawn);
        assert_eq!(
            token_state_in_block(&platform, &app, RECIPIENT, &transfer),
            (Some(paid), None)
        );

        // A round's prevotes combine into one beacon: another one for the same round is refused
        let result =
            apply_block_randomness::<_, MockCoreRPCLike>(&app, request(0, BLOCK_HASH, &[8u8; 96]));
        assert!(
            matches!(result, Err(Error::Abci(AbciError::BadRequest(_)))),
            "{result:?}"
        );
        assert_eq!(rarity_in_block(&platform, &app, &contract), drawn);
    }

    #[test]
    fn should_refuse_a_block_this_node_did_not_process_in_the_round() {
        let latest = PlatformVersion::latest().protocol_version;
        let (platform, contract) = setup();

        // No block processed
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        app.start_transaction();
        let result =
            apply_block_randomness::<_, MockCoreRPCLike>(&app, request(0, BLOCK_HASH, &BEACON));
        assert!(
            matches!(
                result,
                Err(Error::Abci(AbciError::RequestForWrongBlockReceived(_)))
            ),
            "{result:?}"
        );

        let app = start_block(&platform, &contract, latest);
        let root_before = transaction_root(&app);
        for (round, block_hash, height) in [
            (0, [0xBB; 32], HEIGHT),
            (1, BLOCK_HASH, HEIGHT),
            (0, BLOCK_HASH, HEIGHT + 1),
        ] {
            let result = apply_block_randomness::<_, MockCoreRPCLike>(
                &app,
                proto::RequestApplyBlockRandomness {
                    hash: block_hash.to_vec(),
                    height: height as i64,
                    round,
                    beacon: BEACON.to_vec(),
                },
            );
            assert!(
                matches!(
                    result,
                    Err(Error::Abci(AbciError::RequestForWrongBlockReceived(_)))
                ),
                "round {round}: {result:?}"
            );
        }
        assert_eq!(transaction_root(&app), root_before);
        assert_eq!(rarity_in_block(&platform, &app, &contract), 1);
    }

    #[test]
    fn should_draw_nothing_before_protocol_version_15() {
        let (platform, contract) = setup();
        let app = start_block(&platform, &contract, 14);
        let root_before = transaction_root(&app);

        let response =
            apply_block_randomness::<_, MockCoreRPCLike>(&app, request(0, BLOCK_HASH, &BEACON))
                .expect("expected the request to be answered");

        assert_eq!(response.app_hash, root_before.to_vec());
        assert_eq!(transaction_root(&app), root_before);
        assert_eq!(rarity_in_block(&platform, &app, &contract), 1);
    }
}
