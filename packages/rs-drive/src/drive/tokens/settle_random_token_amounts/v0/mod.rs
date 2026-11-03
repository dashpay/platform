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
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
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

    /// Applies what a random transfer or mint gathers when the block executes it: the
    /// reservation, and the history document holding the range's minimum when `with_history`.
    fn reserve(drive: &Drive, request: &RandomTokenRequest, with_history: bool) {
        let token_id = request.token_id;
        let (reservation, event) = match request.kind {
            RandomTokenRequestKind::Transfer {
                sender_id,
                recipient_id,
            } => (
                TokenOperationType::TokenReserveRandomTransfer {
                    token_id,
                    sender_id,
                    recipient_id,
                    identity_contract_nonce: request.identity_contract_nonce,
                    maximum: request.maximum,
                },
                TokenEvent::Transfer(recipient_id, None, None, None, request.minimum),
            ),
            RandomTokenRequestKind::Mint { recipient_id } => (
                TokenOperationType::TokenReserveRandomMint {
                    token_id,
                    owner_id: request.owner_id,
                    recipient_id,
                    identity_contract_nonce: request.identity_contract_nonce,
                    maximum: request.maximum,
                    allow_first_mint: false,
                },
                TokenEvent::Mint(request.minimum, recipient_id, None),
            ),
        };
        let mut operations = vec![DriveOperation::TokenOperation(reservation)];
        if with_history {
            operations.push(DriveOperation::TokenOperation(
                TokenOperationType::TokenRandomAmountHistory {
                    token_id,
                    owner_id: request.owner_id,
                    nonce: request.identity_contract_nonce,
                    event,
                    minimum: request.minimum,
                    maximum: request.maximum,
                },
            ));
        }
        drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("expected the reservation to apply");
    }

    fn settle(drive: &Drive, requests: &[RandomTokenRequest]) -> u32 {
        drive
            .settle_random_token_amounts(
                &BEACON,
                requests,
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected the settlement to run")
    }

    fn balance(drive: &Drive, identity_id: [u8; 32]) -> Option<TokenAmount> {
        drive
            .fetch_identity_token_balance(TOKEN, identity_id, None, PlatformVersion::latest())
            .expect("expected to fetch a token balance")
    }

    fn supply(drive: &Drive) -> Option<TokenAmount> {
        drive
            .fetch_token_total_supply(TOKEN, None, PlatformVersion::latest())
            .expect("expected to fetch the total supply")
    }

    /// What the token marketplace escrow holds under `request`'s reservation.
    fn reserved(drive: &Drive, request: &RandomTokenRequest) -> Option<TokenAmount> {
        drive
            .fetch_token_order_escrow(
                TOKEN,
                request.reservation_id(),
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the reservation")
    }

    /// The identity balances, the shielded pools and the escrow add up to the total supplies.
    fn assert_tokens_balanced(drive: &Drive) {
        let total = drive
            .calculate_total_tokens_balance(None, PlatformVersion::latest())
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
                None,
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
