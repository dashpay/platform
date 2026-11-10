use crate::drive::document::paths::contract_documents_primary_key_path;
use crate::drive::random_beacon::{draw_from_beacon_v0, TOKEN_AMOUNT_DRAW};
use crate::drive::tokens::settle_random_token_amounts::{
    RandomTokenRequest, RandomTokenRequestKind,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{integer_value_for_type, Index, RandomRange};
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{DataContract, Identifier, IdentityNonce};
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_history_document_id;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use std::borrow::Cow;

/// The property of a transfer or mint history document holding the amount.
const HISTORY_AMOUNT_PROPERTY: &str = "amount";

impl Drive {
    #[inline(always)]
    pub(super) fn settle_random_token_amounts_v0(
        &self,
        beacon: &[u8],
        requests: &[RandomTokenRequest],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u32, Error> {
        let mut settled = 0u32;
        for request in requests {
            self.settle_random_token_amount_v0(
                beacon,
                request,
                block_info,
                transaction,
                platform_version,
            )?;
            settled = settled.saturating_add(1);
        }
        Ok(settled)
    }

    /// Draws the amount of `request` and settles what its transition reserved. Each request is
    /// applied before the next is read, so two requests of a block crediting one balance each
    /// add to what the other left.
    fn settle_random_token_amount_v0(
        &self,
        beacon: &[u8],
        request: &RandomTokenRequest,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drawn = draw_token_amount_v0(beacon, request).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "structure validation keeps a random token amount's minimum below its maximum",
            ),
        ))?;
        // What the reservation holds beyond the drawn amount
        let released = request.maximum.checked_sub(drawn).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "a drawn token amount is at most the maximum it is drawn up to",
            ),
        ))?;
        let token_id = request.token_id.to_buffer();

        // The reservation holds exactly the maximum: the block's execution of the transition
        // wrote it, and nothing else writes under a reservation id. Anything else is a request
        // no executed transition reserved, which is never paid out.
        let reservation_id = request.reservation_id();
        let reserved = self.fetch_token_order_escrow(
            token_id,
            reservation_id,
            transaction,
            &mut vec![],
            platform_version,
        )?;
        if reserved != Some(request.maximum) {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a random token amount is settled only from the reservation its execution wrote",
            )));
        }
        let mut operations = self.remove_token_order_escrow_operations(
            token_id,
            reservation_id,
            &mut None,
            platform_version,
        )?;

        let history_document_type_name = match request.kind {
            RandomTokenRequestKind::Transfer {
                sender_id,
                recipient_id,
            } => {
                operations.extend(self.add_to_identity_token_balance_operations(
                    token_id,
                    recipient_id.to_buffer(),
                    drawn,
                    &mut None,
                    transaction,
                    platform_version,
                )?);
                if released > 0 {
                    // The sender and the recipient differ (structure validation refuses a
                    // transfer to oneself), so the two writes are to two balances
                    operations.extend(self.add_to_identity_token_balance_operations(
                        token_id,
                        sender_id.to_buffer(),
                        released,
                        &mut None,
                        transaction,
                        platform_version,
                    )?);
                }
                "transfer"
            }
            RandomTokenRequestKind::Mint { recipient_id } => {
                operations.extend(self.add_to_identity_token_balance_operations(
                    token_id,
                    recipient_id.to_buffer(),
                    drawn,
                    &mut None,
                    transaction,
                    platform_version,
                )?);
                if released > 0 {
                    operations.extend(self.remove_from_token_total_supply_operations(
                        token_id,
                        released,
                        &mut None,
                        transaction,
                        platform_version,
                    )?);
                }
                "mint"
            }
        };
        operations.extend(self.random_token_history_amount_operations_v0(
            request,
            history_document_type_name,
            drawn,
            block_info,
            transaction,
            platform_version,
        )?);

        // No fee: the transition paid for its execution, which wrote the recipient's balance
        // and the history document the settlement only replaces, and the reservation it removes
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    /// Generation 0 of [`Self::prepay_random_token_history_entries`]: the entries of the
    /// history document `event` builds, as `add_token_transaction_history_operations` writes
    /// it (without storage flags), in the indexes of its type holding the amount
    /// (`Drive::prepay_drawn_index_entries_v0`).
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn prepay_random_token_history_entries_v0(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        identity_contract_nonce: IdentityNonce,
        event: TokenEvent,
        random_range: (TokenAmount, TokenAmount),
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let token_history_contract = self
            .cache
            .system_data_contracts
            .load_token_history(platform_version)?;
        let contract: &DataContract = &token_history_contract;
        let document_type = event.associated_document_type(contract)?;
        let document = event.build_historical_document_owned(
            token_id,
            owner_id,
            identity_contract_nonce,
            Some(random_range),
            block_info,
            platform_version,
        )?;
        self.prepay_drawn_index_entries_v0(
            &document,
            owner_id,
            None,
            contract,
            document_type,
            |index: &Index| {
                index
                    .properties
                    .iter()
                    .any(|property| property.name == HISTORY_AMOUNT_PROPERTY)
            },
            block_info,
            transaction,
            platform_version,
        )
    }

    /// The operations rewriting the history document of `request`'s transition to hold
    /// `amount`, none when the token keeps no history of the kind: the transition then wrote no
    /// history document. The document is found by the id it was written under, which hashes the
    /// request's range (`token_history_document_id`), and rewritten in place with its own
    /// storage flags, its `byAmount` index entry moved to the drawn amount.
    fn random_token_history_amount_operations_v0(
        &self,
        request: &RandomTokenRequest,
        document_type_name: &str,
        amount: TokenAmount,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let token_history_contract = self
            .cache
            .system_data_contracts
            .load_token_history(platform_version)?;
        let contract: &DataContract = &token_history_contract;
        let document_type = contract
            .document_type_for_name(document_type_name)
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "the token history contract defines the transfer and mint types",
                ))
            })?;
        let document_id = token_history_document_id(
            &request.token_id,
            &request.owner_id,
            document_type_name,
            request.identity_contract_nonce,
            Some((request.minimum, request.maximum)),
        );

        let primary_key_path =
            contract_documents_primary_key_path(contract.id_ref().as_bytes(), document_type_name);
        let element = self.grove_get_raw_optional(
            (&primary_key_path).into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        let (serialized, element_flags) = match element {
            Some(Element::Item(serialized, flags))
            | Some(Element::ItemWithSumItem(serialized, _, flags)) => (serialized, flags),
            // The token keeps no history of the kind
            None => return Ok(vec![]),
            Some(_) => {
                return Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                    "a token history document is stored as an item",
                )))
            }
        };
        let mut document = Document::from_bytes(&serialized, document_type, platform_version)?;
        let storage_flags = StorageFlags::map_some_element_flags_ref(&element_flags)?;

        let amount_type = &document_type
            .flattened_properties()
            .get(HISTORY_AMOUNT_PROPERTY)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "a token history transfer or mint document has an amount",
            )))?
            .property_type;
        let value = integer_value_for_type(amount_type, i128::from(amount)).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "a token history amount is an integer holding every token amount",
            ),
        ))?;
        document
            .properties_mut()
            .insert(HISTORY_AMOUNT_PROPERTY.to_string(), value);

        let owner_id = document.owner_id().to_buffer();
        self.update_stored_document_operations_v1(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentOwnedInfo((document, storage_flags.map(Cow::Owned))),
                    owner_id: Some(owner_id),
                },
                contract,
                document_type,
            },
            true,
            false,
            block_info,
            &mut None,
            &mut None,
            transaction,
            platform_version,
        )
    }
}

/// The amount `beacon` draws for `request` (`draw_from_beacon_v0`), from its minimum to its
/// maximum, under the request key of a token amount: what the draw is for, the token, the owner
/// and the identity contract nonce (a big-endian `u64`). The nonce is used once per owner and
/// contract, so the requests of a block never share a key. `None` only for a minimum above the
/// maximum, which structure validation rules out.
fn draw_token_amount_v0(beacon: &[u8], request: &RandomTokenRequest) -> Option<TokenAmount> {
    let mut request_key = Vec::with_capacity(1 + 32 + 32 + 8);
    request_key.push(TOKEN_AMOUNT_DRAW);
    request_key.extend_from_slice(request.token_id.as_slice());
    request_key.extend_from_slice(request.owner_id.as_slice());
    request_key.extend_from_slice(&request.identity_contract_nonce.to_be_bytes());
    let range = RandomRange::new(i128::from(request.minimum), i128::from(request.maximum))?;
    let drawn = draw_from_beacon_v0(beacon, &request_key, range)?;
    TokenAmount::try_from(drawn).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::tokens::settle_random_token_amounts::random_token_reservation_id;
    use crate::util::batch::drive_op_batch::TokenOperationType;
    use crate::util::batch::DriveOperation;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{
        TokenMintTransitionAction, TokenMintTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::{
        TokenTransferTransitionAction, TokenTransferTransitionActionV0,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::v1::DataContractV1;
    use grovedb_costs::OperationCost;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use dpp::platform_value::Value;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

    const BEACON: [u8; 96] = [7; 96];
    const TOKEN: [u8; 32] = [5; 32];
    const SENDER: [u8; 32] = [6; 32];
    const RECIPIENT: [u8; 32] = [8; 32];
    const NONCE: u64 = 3;
    const BASE_SUPPLY: TokenAmount = 10_000;
    const MINIMUM: TokenAmount = 100;
    const MAXIMUM: TokenAmount = 1_000;

    /// A drive holding the token history contract and a token whose whole base supply the
    /// sender holds.
    fn setup() -> Drive {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_history =
            load_system_data_contract(SystemDataContract::TokenHistory, platform_version)
                .expect("expected the token history contract");
        drive
            .apply_contract(
                &token_history,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to store the token history contract");
        drive
            .create_token_trees(
                Identifier::from([4; 32]),
                0,
                TOKEN,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create the token trees");
        drive
            .token_mint(
                TOKEN,
                SENDER,
                BASE_SUPPLY,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint the base supply");
        drive
    }

    fn request(kind: RandomTokenRequestKind) -> RandomTokenRequest {
        RandomTokenRequest {
            token_id: Identifier::from(TOKEN),
            owner_id: Identifier::from(SENDER),
            identity_contract_nonce: NONCE,
            minimum: MINIMUM,
            maximum: MAXIMUM,
            kind,
        }
    }

    fn transfer() -> RandomTokenRequest {
        request(RandomTokenRequestKind::Transfer {
            sender_id: Identifier::from(SENDER),
            recipient_id: Identifier::from(RECIPIENT),
        })
    }

    fn mint() -> RandomTokenRequest {
        request(RandomTokenRequestKind::Mint {
            recipient_id: Identifier::from(RECIPIENT),
        })
    }

    /// A contract whose token, at position 0, keeps every kind of history when
    /// `with_history`, and none otherwise.
    fn token_contract(with_history: bool) -> Arc<DataContractFetchInfo> {
        let mut token = TokenConfigurationV0::default_most_restrictive();
        token.keeps_history =
            TokenKeepsHistoryRulesV0::default_for_keeping_all_history(with_history).into();
        Arc::new(DataContractFetchInfo {
            contract: DataContract::V1(DataContractV1 {
                id: Identifier::from([4; 32]),
                version: 0,
                owner_id: Identifier::from(SENDER),
                document_types: Default::default(),
                config: DataContractConfigV0::default().into(),
                schema_defs: None,
                created_at: None,
                updated_at: None,
                created_at_block_height: None,
                updated_at_block_height: None,
                created_at_epoch: None,
                updated_at_epoch: None,
                groups: Default::default(),
                tokens: BTreeMap::from([(0, TokenConfiguration::V0(token))]),
                keywords: Vec::new(),
                description: None,
            }),
            storage_flags: None,
            cost: OperationCost::default(),
            fee: None,
        })
    }

    /// The operations the block's execution of the random transfer or mint `request`
    /// describes gathers: its action, as its transformer builds it, converted
    /// (`into_high_level_batch_drive_operations`), with its token keeping history when
    /// `with_history`.
    fn random_operations(
        request: &RandomTokenRequest,
        with_history: bool,
    ) -> Vec<DriveOperation<'static>> {
        let base = TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: request.token_id,
            identity_contract_nonce: request.identity_contract_nonce,
            token_contract_position: 0,
            data_contract: token_contract(with_history),
            store_in_group: None,
            perform_action: true,
        });
        let epoch = BlockInfo::default().epoch;
        let platform_version = PlatformVersion::latest();
        match request.kind {
            RandomTokenRequestKind::Transfer { recipient_id, .. } => {
                TokenTransferTransitionAction::V0(TokenTransferTransitionActionV0 {
                    base,
                    amount: request.maximum,
                    recipient_id,
                    public_note: None,
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                    random_minimum_amount: Some(request.minimum),
                })
                .into_high_level_batch_drive_operations(
                    &epoch,
                    request.owner_id,
                    platform_version,
                )
            }
            RandomTokenRequestKind::Mint { recipient_id } => {
                TokenMintTransitionAction::V0(TokenMintTransitionActionV0 {
                    base,
                    mint_amount: request.maximum,
                    identity_balance_holder_id: recipient_id,
                    public_note: None,
                    random_minimum_amount: Some(request.minimum),
                })
                .into_high_level_batch_drive_operations(
                    &epoch,
                    request.owner_id,
                    platform_version,
                )
            }
        }
        .expect("expected the random action to convert")
    }

    /// Applies what the block's execution of `request` writes ([`random_operations`]): the
    /// reservation, and the history document holding the range's minimum when `with_history`.
    /// The identity contract nonce update is left out, as these tests keep no identities.
    fn reserve(drive: &Drive, request: &RandomTokenRequest, with_history: bool) {
        reserve_in(drive, request, with_history, None)
    }

    /// [`reserve`], inside `transaction`.
    fn reserve_in(
        drive: &Drive,
        request: &RandomTokenRequest,
        with_history: bool,
        transaction: TransactionArg,
    ) {
        let operations = random_operations(request, with_history)
            .into_iter()
            .filter(|operation| matches!(operation, DriveOperation::TokenOperation(_)))
            .collect();
        drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected the reservation to apply");
    }

    fn settle(drive: &Drive, requests: &[RandomTokenRequest]) -> u32 {
        settle_in(drive, requests, None)
    }

    /// [`settle`], inside `transaction`.
    fn settle_in(
        drive: &Drive,
        requests: &[RandomTokenRequest],
        transaction: TransactionArg,
    ) -> u32 {
        drive
            .settle_random_token_amounts(
                &BEACON,
                requests,
                &BlockInfo::default(),
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected the settlement to run")
    }

    fn balance(drive: &Drive, identity_id: [u8; 32]) -> Option<TokenAmount> {
        balance_in(drive, identity_id, None)
    }

    /// [`balance`], inside `transaction`.
    fn balance_in(
        drive: &Drive,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Option<TokenAmount> {
        drive
            .fetch_identity_token_balance(
                TOKEN,
                identity_id,
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch a token balance")
    }

    fn supply(drive: &Drive) -> Option<TokenAmount> {
        supply_in(drive, None)
    }

    /// [`supply`], inside `transaction`.
    fn supply_in(drive: &Drive, transaction: TransactionArg) -> Option<TokenAmount> {
        drive
            .fetch_token_total_supply(TOKEN, transaction, PlatformVersion::latest())
            .expect("expected to fetch the total supply")
    }

    /// What the token marketplace escrow holds under `request`'s reservation.
    fn reserved(drive: &Drive, request: &RandomTokenRequest) -> Option<TokenAmount> {
        reserved_in(drive, request, None)
    }

    /// [`reserved`], inside `transaction`.
    fn reserved_in(
        drive: &Drive,
        request: &RandomTokenRequest,
        transaction: TransactionArg,
    ) -> Option<TokenAmount> {
        drive
            .fetch_token_order_escrow(
                TOKEN,
                request.reservation_id(),
                transaction,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the reservation")
    }

    /// The identity balances, the shielded pools and the escrow add up to the total supplies.
    fn assert_tokens_balanced(drive: &Drive) {
        assert_tokens_balanced_in(drive, None)
    }

    /// [`assert_tokens_balanced`], inside `transaction`.
    fn assert_tokens_balanced_in(drive: &Drive, transaction: TransactionArg) {
        let total = drive
            .calculate_total_tokens_balance(transaction, PlatformVersion::latest())
            .expect("expected to sum the token balances");
        assert!(
            total.ok().expect("expected the sums to compare"),
            "tokens are not balanced: {total}"
        );
    }

    /// The amount the history document of `request`'s transition holds.
    fn history_amount(
        drive: &Drive,
        request: &RandomTokenRequest,
        document_type_name: &str,
    ) -> Option<TokenAmount> {
        history_amount_in(drive, request, document_type_name, None)
    }

    /// [`history_amount`], inside `transaction`.
    fn history_amount_in(
        drive: &Drive,
        request: &RandomTokenRequest,
        document_type_name: &str,
        transaction: TransactionArg,
    ) -> Option<TokenAmount> {
        let platform_version = PlatformVersion::latest();
        let contract = drive
            .cache
            .system_data_contracts
            .load_token_history(platform_version)
            .expect("expected the token history contract");
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected the history type");
        let document_id = token_history_document_id(
            &request.token_id,
            &request.owner_id,
            document_type_name,
            request.identity_contract_nonce,
            Some((request.minimum, request.maximum)),
        );
        let path =
            contract_documents_primary_key_path(contract.id_ref().as_bytes(), document_type_name);
        let element = drive
            .grove_get_raw_optional(
                (&path).into(),
                document_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to read the history document")?;
        let Element::Item(serialized, _) = element else {
            panic!("expected the history document to be an item");
        };
        let document = Document::from_bytes(&serialized, document_type, platform_version)
            .expect("expected the history document to decode");
        document
            .properties()
            .get(HISTORY_AMOUNT_PROPERTY)
            .map(|amount: &Value| amount.to_integer::<u64>().expect("an integer amount"))
    }

    /// The history write of `request`'s transition, as its conversion gathers it
    /// ([`random_operations`]), its token keeping history.
    fn random_history_write(request: &RandomTokenRequest) -> TokenOperationType {
        random_operations(request, true)
            .into_iter()
            .find_map(|operation| match operation {
                DriveOperation::TokenOperation(
                    history @ TokenOperationType::TokenRandomAmountHistory { .. },
                ) => Some(history),
                _ => None,
            })
            .expect("expected a range-bound history write")
    }

    /// What applying `history` on a fresh drive bills, or what estimating it does when not
    /// `apply`.
    fn history_write_fee(history: TokenOperationType, apply: bool) -> FeeResult {
        setup()
            .apply_drive_operations(
                vec![DriveOperation::TokenOperation(history)],
                apply,
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected the history write to apply")
    }

    #[test]
    fn should_prepay_the_settlement_once_with_the_history_write() {
        let platform_version = PlatformVersion::latest();
        let drive = setup();
        for request in [transfer(), mint()] {
            let random = random_history_write(&request);
            let TokenOperationType::TokenRandomAmountHistory {
                token_id,
                owner_id,
                nonce,
                event,
                minimum,
                maximum,
            } = random.clone()
            else {
                unreachable!("a range-bound history write");
            };
            let prepayment = drive
                .prepay_random_token_history_entries(
                    token_id,
                    owner_id,
                    nonce,
                    event.clone(),
                    (minimum, maximum),
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
                .expect("expected the prepayment to be priced");
            assert!(
                prepayment.storage_fee > 0 && prepayment.processing_fee > 0,
                "expected the settlement's entries to cost something: {prepayment:?}"
            );
            // The same event written as the history of a fixed amount: a document of the same
            // size under an id of the same size, with no settlement to prepay
            let fixed = TokenOperationType::TokenHistory {
                token_id,
                owner_id,
                nonce,
                event,
            };
            for apply in [false, true] {
                let random_fee = history_write_fee(random.clone(), apply);
                let fixed_fee = history_write_fee(fixed.clone(), apply);
                assert_eq!(
                    (
                        random_fee.storage_fee - fixed_fee.storage_fee,
                        random_fee.processing_fee - fixed_fee.processing_fee
                    ),
                    (prepayment.storage_fee, prepayment.processing_fee),
                    "expected the history write to prepay once (applied: {apply})"
                );
            }
        }
    }

    #[test]
    fn should_convert_a_random_transfer_or_mint_into_a_reservation_and_a_range_bound_history() {
        for request in [transfer(), mint()] {
            for with_history in [true, false] {
                let operations = random_operations(&request, with_history);
                let token_operations: Vec<&TokenOperationType> = operations
                    .iter()
                    .filter_map(|operation| match operation {
                        DriveOperation::TokenOperation(operation) => Some(operation),
                        _ => None,
                    })
                    .collect();
                // The maximum is reserved, and no fixed amount moves
                let reserved: Vec<TokenAmount> = token_operations
                    .iter()
                    .filter_map(|operation| match operation {
                        TokenOperationType::TokenReserveRandomTransfer { maximum, .. }
                        | TokenOperationType::TokenReserveRandomMint { maximum, .. } => {
                            Some(*maximum)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(reserved, vec![MAXIMUM]);
                assert!(!token_operations.iter().any(|operation| matches!(
                    operation,
                    TokenOperationType::TokenTransfer { .. }
                        | TokenOperationType::TokenMint { .. }
                        | TokenOperationType::TokenHistory { .. }
                )));
                // The history, when kept, holds the minimum under the id bound to the range
                let histories: Vec<(TokenAmount, TokenAmount, TokenAmount)> = token_operations
                    .iter()
                    .filter_map(|operation| match operation {
                        TokenOperationType::TokenRandomAmountHistory {
                            event,
                            minimum,
                            maximum,
                            ..
                        } => {
                            let amount = match event {
                                TokenEvent::Transfer(_, _, _, _, amount)
                                | TokenEvent::Mint(amount, _, _) => *amount,
                                other => panic!("expected a transfer or mint event, got {other:?}"),
                            };
                            Some((amount, *minimum, *maximum))
                        }
                        _ => None,
                    })
                    .collect();
                if with_history {
                    assert_eq!(histories, vec![(MINIMUM, MINIMUM, MAXIMUM)]);
                } else {
                    assert!(histories.is_empty());
                }
            }
        }
    }

    #[test]
    fn should_settle_a_random_transfer_as_a_transfer_of_the_drawn_amount() {
        let drive = setup();
        let request = transfer();
        let drawn = draw_token_amount_v0(&BEACON, &request).expect("draws");
        assert!((MINIMUM..=MAXIMUM).contains(&drawn));

        reserve(&drive, &request, true);
        assert_eq!(balance(&drive, SENDER), Some(BASE_SUPPLY - MAXIMUM));
        assert_eq!(
            balance(&drive, RECIPIENT),
            Some(0),
            "the recipient's balance is written, and waits for the draw"
        );
        assert_eq!(reserved(&drive, &request), Some(MAXIMUM));
        assert_eq!(history_amount(&drive, &request, "transfer"), Some(MINIMUM));
        assert_eq!(supply(&drive), Some(BASE_SUPPLY));
        assert_tokens_balanced(&drive);

        assert_eq!(settle(&drive, std::slice::from_ref(&request)), 1);
        assert_eq!(reserved(&drive, &request), None);

        // What a plain transfer of the drawn amount leaves
        let plain = setup();
        plain
            .token_transfer(
                TOKEN,
                SENDER,
                RECIPIENT,
                drawn,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected the plain transfer");
        assert_eq!(balance(&drive, SENDER), balance(&plain, SENDER));
        assert_eq!(balance(&drive, RECIPIENT), balance(&plain, RECIPIENT));
        assert_eq!(balance(&drive, RECIPIENT), Some(drawn));
        assert_eq!(supply(&drive), supply(&plain));
        assert_eq!(supply(&drive), Some(BASE_SUPPLY));
        assert_eq!(history_amount(&drive, &request, "transfer"), Some(drawn));
        assert_tokens_balanced(&drive);
    }

    #[test]
    fn should_settle_a_random_mint_as_a_mint_of_the_drawn_amount() {
        let drive = setup();
        let request = mint();
        let drawn = draw_token_amount_v0(&BEACON, &request).expect("draws");

        reserve(&drive, &request, true);
        assert_eq!(supply(&drive), Some(BASE_SUPPLY + MAXIMUM));
        assert_eq!(
            balance(&drive, RECIPIENT),
            Some(0),
            "the recipient's balance is written, and waits for the draw"
        );
        assert_eq!(reserved(&drive, &request), Some(MAXIMUM));
        assert_eq!(history_amount(&drive, &request, "mint"), Some(MINIMUM));
        assert_tokens_balanced(&drive);

        assert_eq!(settle(&drive, std::slice::from_ref(&request)), 1);
        assert_eq!(reserved(&drive, &request), None);

        // What a plain mint of the drawn amount leaves
        let plain = setup();
        plain
            .token_mint(
                TOKEN,
                RECIPIENT,
                drawn,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected the plain mint");
        assert_eq!(balance(&drive, RECIPIENT), balance(&plain, RECIPIENT));
        assert_eq!(balance(&drive, RECIPIENT), Some(drawn));
        assert_eq!(balance(&drive, SENDER), balance(&plain, SENDER));
        assert_eq!(supply(&drive), supply(&plain));
        assert_eq!(supply(&drive), Some(BASE_SUPPLY + drawn));
        assert_eq!(history_amount(&drive, &request, "mint"), Some(drawn));
        assert_tokens_balanced(&drive);
    }

    #[test]
    fn should_settle_without_a_history_document_when_the_token_keeps_none() {
        let drive = setup();
        let request = transfer();
        let drawn = draw_token_amount_v0(&BEACON, &request).expect("draws");

        reserve(&drive, &request, false);
        assert_tokens_balanced(&drive);
        assert_eq!(settle(&drive, std::slice::from_ref(&request)), 1);
        assert_eq!(balance(&drive, RECIPIENT), Some(drawn));
        assert_eq!(balance(&drive, SENDER), Some(BASE_SUPPLY - drawn));
        assert_eq!(history_amount(&drive, &request, "transfer"), None);
        assert_tokens_balanced(&drive);
    }

    #[test]
    fn should_refuse_a_batch_reserving_two_random_amounts_of_one_token() {
        // Two random transfers of the token with disjoint balances, in one batch: each would
        // create the token's escrow tree, which it does not have yet, and GroveDB's consistency
        // check refuses the creation queued twice, so the batch is refused before it writes
        let drive = setup();
        assert!(drive.config.batching_consistency_verification);
        let other_sender = [9; 32];
        drive
            .token_mint(
                TOKEN,
                other_sender,
                MAXIMUM,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to mint the other sender's balance");
        let first = transfer();
        let second = RandomTokenRequest {
            owner_id: Identifier::from(other_sender),
            kind: RandomTokenRequestKind::Transfer {
                sender_id: Identifier::from(other_sender),
                recipient_id: Identifier::from([10; 32]),
            },
            ..transfer()
        };
        assert_eq!(reserved(&drive, &first), None);

        let operations = random_operations(&first, false)
            .into_iter()
            .chain(random_operations(&second, false))
            .filter(|operation| matches!(operation, DriveOperation::TokenOperation(_)))
            .collect();
        let result = drive.apply_drive_operations(
            operations,
            true,
            &BlockInfo::default(),
            None,
            PlatformVersion::latest(),
        );
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
            ),
            "{result:?}"
        );
        assert_eq!(reserved(&drive, &first), None);
        assert_eq!(reserved(&drive, &second), None);
        assert_eq!(balance(&drive, SENDER), Some(BASE_SUPPLY));
        assert_eq!(balance(&drive, other_sender), Some(MAXIMUM));
        assert_tokens_balanced(&drive);

        // Batch by batch, the first creates the escrow tree and the second finds it
        reserve(&drive, &first, false);
        reserve(&drive, &second, false);
        assert_eq!(reserved(&drive, &first), Some(MAXIMUM));
        assert_eq!(reserved(&drive, &second), Some(MAXIMUM));
        assert_tokens_balanced(&drive);
    }

    #[test]
    fn should_settle_two_requests_crediting_one_balance() {
        let drive = setup();
        let first = transfer();
        let second = RandomTokenRequest {
            identity_contract_nonce: NONCE + 1,
            ..transfer()
        };
        let first_drawn = draw_token_amount_v0(&BEACON, &first).expect("draws");
        let second_drawn = draw_token_amount_v0(&BEACON, &second).expect("draws");

        reserve(&drive, &first, true);
        reserve(&drive, &second, true);
        assert_ne!(first.reservation_id(), second.reservation_id());
        assert_eq!(reserved(&drive, &first), Some(MAXIMUM));
        assert_eq!(reserved(&drive, &second), Some(MAXIMUM));
        assert_tokens_balanced(&drive);
        assert_eq!(settle(&drive, &[first.clone(), second.clone()]), 2);
        assert_eq!(balance(&drive, RECIPIENT), Some(first_drawn + second_drawn));
        assert_eq!(
            balance(&drive, SENDER),
            Some(BASE_SUPPLY - first_drawn - second_drawn)
        );
        assert_eq!(
            history_amount(&drive, &first, "transfer"),
            Some(first_drawn)
        );
        assert_eq!(
            history_amount(&drive, &second, "transfer"),
            Some(second_drawn)
        );
        assert_tokens_balanced(&drive);
    }

    #[test]
    fn should_reserve_and_settle_overlapping_requests_inside_one_uncommitted_transaction() {
        let drive = setup();
        let first = transfer();
        let second = RandomTokenRequest {
            identity_contract_nonce: NONCE + 1,
            ..transfer()
        };
        let third = RandomTokenRequest {
            identity_contract_nonce: NONCE + 2,
            ..mint()
        };
        let requests = [first.clone(), second.clone(), third.clone()];
        let first_drawn = draw_token_amount_v0(&BEACON, &first).expect("draws");
        let second_drawn = draw_token_amount_v0(&BEACON, &second).expect("draws");
        let third_drawn = draw_token_amount_v0(&BEACON, &third).expect("draws");

        let assert_nothing_committed = || {
            assert_eq!(balance(&drive, SENDER), Some(BASE_SUPPLY));
            assert_eq!(balance(&drive, RECIPIENT), None);
            assert_eq!(supply(&drive), Some(BASE_SUPPLY));
            for request in &requests {
                assert_eq!(reserved(&drive, request), None);
            }
            assert_eq!(history_amount(&drive, &first, "transfer"), None);
            assert_eq!(history_amount(&drive, &second, "transfer"), None);
            assert_eq!(history_amount(&drive, &third, "mint"), None);
            assert_tokens_balanced(&drive);
        };

        let transaction = drive.grove.start_transaction();
        let block = Some(&transaction);

        for request in &requests {
            reserve_in(&drive, request, true, block);
        }
        for request in &requests {
            assert_eq!(reserved_in(&drive, request, block), Some(MAXIMUM));
        }
        assert_eq!(
            balance_in(&drive, SENDER, block),
            Some(BASE_SUPPLY - 2 * MAXIMUM)
        );
        assert_eq!(balance_in(&drive, RECIPIENT, block), Some(0));
        assert_eq!(supply_in(&drive, block), Some(BASE_SUPPLY + MAXIMUM));
        assert_eq!(
            history_amount_in(&drive, &first, "transfer", block),
            Some(MINIMUM)
        );
        assert_eq!(
            history_amount_in(&drive, &second, "transfer", block),
            Some(MINIMUM)
        );
        assert_eq!(
            history_amount_in(&drive, &third, "mint", block),
            Some(MINIMUM)
        );
        assert_tokens_balanced_in(&drive, block);
        assert_nothing_committed();

        // Each settlement reads what the one before it left in the transaction
        assert_eq!(settle_in(&drive, &requests, block), 3);
        for request in &requests {
            assert_eq!(reserved_in(&drive, request, block), None);
        }
        assert_eq!(
            balance_in(&drive, SENDER, block),
            Some(BASE_SUPPLY - first_drawn - second_drawn)
        );
        assert_eq!(
            balance_in(&drive, RECIPIENT, block),
            Some(first_drawn + second_drawn + third_drawn)
        );
        assert_eq!(supply_in(&drive, block), Some(BASE_SUPPLY + third_drawn));
        assert_eq!(
            history_amount_in(&drive, &first, "transfer", block),
            Some(first_drawn)
        );
        assert_eq!(
            history_amount_in(&drive, &second, "transfer", block),
            Some(second_drawn)
        );
        assert_eq!(
            history_amount_in(&drive, &third, "mint", block),
            Some(third_drawn)
        );
        assert_tokens_balanced_in(&drive, block);
        assert_nothing_committed();

        drive
            .grove
            .rollback_transaction(&transaction)
            .expect("expected to roll the block back");
        assert_nothing_committed();
        assert_eq!(balance_in(&drive, SENDER, block), Some(BASE_SUPPLY));
        assert_eq!(balance_in(&drive, RECIPIENT, block), None);
        assert_eq!(supply_in(&drive, block), Some(BASE_SUPPLY));
    }

    #[test]
    fn should_refuse_to_settle_a_request_nothing_reserved() {
        let drive = setup();
        let request = transfer();
        let result = drive.settle_random_token_amounts(
            &BEACON,
            std::slice::from_ref(&request),
            &BlockInfo::default(),
            None,
            PlatformVersion::latest(),
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
        assert_eq!(balance(&drive, RECIPIENT), None, "nothing is paid out");
        assert_eq!(balance(&drive, SENDER), Some(BASE_SUPPLY));

        // A reservation of another maximum is not this request's either
        reserve(&drive, &request, false);
        let result = drive.settle_random_token_amounts(
            &BEACON,
            &[RandomTokenRequest {
                maximum: MAXIMUM + 1,
                ..transfer()
            }],
            &BlockInfo::default(),
            None,
            PlatformVersion::latest(),
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_key_each_reservation_by_token_owner_and_nonce() {
        let token_id = Identifier::from(TOKEN);
        let owner_id = Identifier::from(SENDER);
        let reservation = random_token_reservation_id(&token_id, &owner_id, NONCE);
        assert_eq!(transfer().reservation_id(), reservation);
        assert_eq!(mint().reservation_id(), reservation);
        assert_ne!(
            random_token_reservation_id(&token_id, &owner_id, NONCE + 1),
            reservation
        );
        assert_ne!(
            random_token_reservation_id(&Identifier::from([9; 32]), &owner_id, NONCE),
            reservation
        );
        assert_ne!(
            random_token_reservation_id(&token_id, &Identifier::from([9; 32]), NONCE),
            reservation
        );
    }

    #[test]
    fn should_draw_each_token_amount_reproducibly() {
        let request = transfer();
        let widest = RandomTokenRequest {
            minimum: 0,
            maximum: u64::MAX,
            ..transfer()
        };
        assert_eq!(
            draw_token_amount_v0(&BEACON, &request),
            draw_token_amount_v0(&BEACON, &request)
        );
        assert_ne!(
            draw_token_amount_v0(&BEACON, &widest),
            draw_token_amount_v0(
                &BEACON,
                &RandomTokenRequest {
                    identity_contract_nonce: NONCE + 1,
                    ..widest.clone()
                }
            )
        );
        assert_ne!(
            draw_token_amount_v0(&BEACON, &widest),
            draw_token_amount_v0(&[8; 96], &widest)
        );
        // A transfer and a mint of one owner, token and nonce draw alike: one transition is
        // either, never both
        assert_eq!(
            draw_token_amount_v0(&BEACON, &request),
            draw_token_amount_v0(&BEACON, &mint())
        );
        // The draw is consensus: every node must compute these values forever
        assert_eq!(
            draw_token_amount_v0(&BEACON, &widest),
            Some(TOKEN_DRAW_VECTOR)
        );
        assert_eq!(
            draw_token_amount_v0(&BEACON, &request),
            Some(TOKEN_RANGE_DRAW_VECTOR)
        );
        assert_eq!(
            draw_token_amount_v0(
                &BEACON,
                &RandomTokenRequest {
                    minimum: 2,
                    maximum: 1,
                    ..transfer()
                }
            ),
            None
        );
    }

    /// The draw of [`transfer`] over the widest range, from 0 to `u64::MAX`.
    const TOKEN_DRAW_VECTOR: TokenAmount = 10_714_275_481_292_403_667;
    /// The draw of [`transfer`] over its range, from 100 to 1_000.
    const TOKEN_RANGE_DRAW_VECTOR: TokenAmount = 298;

    #[test]
    fn should_not_settle_before_protocol_version_15() {
        let drive = setup();
        let result = drive.settle_random_token_amounts(
            &BEACON,
            &[],
            &BlockInfo::default(),
            None,
            PlatformVersion::get(14).expect("protocol version 14"),
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}
