mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::tokens::token_event::TokenEvent;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// The domain of a random token reservation's id.
const RANDOM_TOKEN_RESERVATION_DOMAIN: &[u8] = b"random_token_reservation";

/// The tag of a transfer in a random token reservation's id.
const RANDOM_TOKEN_TRANSFER_TAG: u8 = 0;

/// The tag of a mint in a random token reservation's id.
const RANDOM_TOKEN_MINT_TAG: u8 = 1;

/// What a random token amount pays out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomTokenRequestKind {
    /// A random token transfer: the block moved the request's maximum from the sender, the
    /// request's owner, into the reservation, and credited the recipient zero.
    Transfer {
        /// The identity receiving the drawn amount
        recipient_id: Identifier,
    },
    /// A random token mint: the block added the request's maximum to the token's total supply
    /// and held it in the reservation, and credited the recipient zero.
    Mint {
        /// The identity receiving the drawn amount
        recipient_id: Identifier,
    },
}

/// A token transfer or mint of a block that moves a random amount (protocol version 15), whose
/// maximum the block reserved when it executed the transition
/// (`TokenOperationType::TokenReserveRandomTransfer` and `TokenReserveRandomMint`): it holds
/// the maximum in the token marketplace escrow of the token, under
/// [`RandomTokenRequest::reservation_id`], and gave the recipient a balance (of zero if it had
/// none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomTokenRequest {
    /// The token
    pub token_id: Identifier,
    /// The identity that sent the transition, and a transfer's sender
    pub owner_id: Identifier,
    /// The identity contract nonce of the transition, which with the token and the owner keys
    /// the draw and the transition's history document
    pub identity_contract_nonce: IdentityNonce,
    /// The least the transition moves
    pub minimum: TokenAmount,
    /// The most the transition moves, which the block reserved
    pub maximum: TokenAmount,
    /// What the drawn amount pays out
    pub kind: RandomTokenRequestKind,
}

impl RandomTokenRequest {
    /// The key under which the token marketplace escrow of the token
    /// (`[Tokens, TOKEN_ORDER_ESCROW_KEY, token id]`) holds the request's reservation:
    /// `hash_double("random_token_reservation" || token_id || owner_id ||
    /// identity_contract_nonce || minimum || maximum || kind || recipient_id)`, the numbers
    /// big-endian `u64`s and the kind 0 for a transfer, 1 for a mint. The block executing the
    /// transition writes the entry (`TokenOperationType::TokenReserveRandomTransfer` and
    /// `TokenReserveRandomMint`) and `Drive::settle_random_token_amounts` removes it.
    ///
    /// It hashes every field of the request, so a settlement finds the reservation only from
    /// the request its transition reserved: one naming another recipient, range or kind finds
    /// none and pays nothing out, and a transfer's sender is the owner it hashes. The domain
    /// keeps it apart from the order ids of the marketplace's listings, and the nonce, used
    /// once per owner and contract, keeps two reservations of a block apart.
    pub fn reservation_id(&self) -> [u8; 32] {
        let (kind_tag, recipient_id) = match self.kind {
            RandomTokenRequestKind::Transfer { recipient_id } => {
                (RANDOM_TOKEN_TRANSFER_TAG, recipient_id)
            }
            RandomTokenRequestKind::Mint { recipient_id } => (RANDOM_TOKEN_MINT_TAG, recipient_id),
        };
        let mut preimage = Vec::with_capacity(
            RANDOM_TOKEN_RESERVATION_DOMAIN.len() + 32 + 32 + 8 + 8 + 8 + 1 + 32,
        );
        preimage.extend_from_slice(RANDOM_TOKEN_RESERVATION_DOMAIN);
        preimage.extend_from_slice(self.token_id.as_slice());
        preimage.extend_from_slice(self.owner_id.as_slice());
        preimage.extend_from_slice(&self.identity_contract_nonce.to_be_bytes());
        preimage.extend_from_slice(&self.minimum.to_be_bytes());
        preimage.extend_from_slice(&self.maximum.to_be_bytes());
        preimage.push(kind_tag);
        preimage.extend_from_slice(recipient_id.as_slice());
        hash_double(preimage)
    }
}

impl Drive {
    /// Pays out the random token transfers and mints of a block (protocol version 15) once
    /// every transition of the block is accepted, from `beacon`, the block's random beacon (the
    /// threshold signature the block's prevotes combine into). For each request it draws an
    /// amount `v` from the request's minimum to its maximum, both included, and settles what
    /// the block reserved. It removes the reservation, the maximum the token marketplace escrow
    /// of the token held, and:
    ///
    /// - a transfer credits the recipient `v` and gives the sender, the request's owner, back
    ///   the rest of the maximum;
    /// - a mint credits the recipient `v` and takes the rest of the maximum off the token's
    ///   total supply.
    ///
    /// The recipient's balance exists, of zero if it had none: the block wrote it when it
    /// executed the transition, which paid for its storage, so the settlement only replaces
    /// values. Token conservation holds before the settlement, with the maximum counted in the
    /// escrow, and after it.
    ///
    /// When the token keeps the history of the kind, the transition's history document, which
    /// holds the range's minimum, is rewritten in place to hold `v`. Settling only pays out or
    /// releases what the block reserved, so it cannot fail for a logical reason. Nobody pays:
    /// the transition paid for its execution when the block ran it, and prepaid the history
    /// index entries the rewrite may create ([`Self::prepay_random_token_history_entries`]), so
    /// the settlement is free.
    ///
    /// # Parameters
    /// - `beacon`: the block's random beacon.
    /// - `requests`: the random token transfers and mints of the block, in the order the block
    ///   executed them.
    /// - `block_info`: the block.
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the method version; `None` before protocol version 15,
    ///   which is the first to decode a random token amount.
    ///
    /// # Returns
    /// How many requests were settled.
    pub fn settle_random_token_amounts(
        &self,
        beacon: &[u8],
        requests: &[RandomTokenRequest],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u32, Error> {
        match platform_version
            .drive
            .methods
            .token
            .settle_random_token_amounts
        {
            Some(0) => self.settle_random_token_amounts_v0(
                beacon,
                requests,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "settle_random_token_amounts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "settle_random_token_amounts".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// What a random token transfer or mint (protocol version 15) whose token keeps the
    /// history of its kind pays in advance for the settlement's rewrite of its history document
    /// ([`Self::settle_random_token_amounts`]). The transition writes the document holding the
    /// range's minimum, and the settlement moves its entry in each index of the document's type
    /// holding `amount` (`byAmount`, under the token) to the drawn amount. The branch it moves
    /// into may not exist yet: the settlement creates it, and is not billed. So the write of the
    /// history document (`TokenOperationType::TokenRandomAmountHistory`) pays for those entries
    /// once more, at what Drive's fee estimation gives for inserting the document's entries
    /// into those indexes without reading state: the average layer information it prices a dry
    /// run with, as if the branches had to be created (the average case, whatever the draw
    /// finds). A transfer or mint refused before its execution writes no history document, so
    /// it pays nothing for them. Selected by the settlement's method version, whose cost model
    /// it is part of.
    ///
    /// # Parameters
    /// - `token_id`: the token.
    /// - `owner_id`: the identity that sent the transition, whose history document it is.
    /// - `identity_contract_nonce`: the transition's identity contract nonce, which keys the
    ///   history document.
    /// - `event`: the history event the transition records, a transfer or a mint of the range's
    ///   minimum.
    /// - `random_range`: the least and the most the transition moves, which the history
    ///   document's id hashes (`token_history_document_id`).
    /// - `block_info`: the block, whose epoch prices the entries.
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the method version; `None` before protocol version 15,
    ///   which is the first to decode a random token amount.
    ///
    /// # Returns
    /// The fee of those entries.
    #[allow(clippy::too_many_arguments)]
    pub fn prepay_random_token_history_entries(
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
        match platform_version
            .drive
            .methods
            .token
            .settle_random_token_amounts
        {
            Some(0) => self.prepay_random_token_history_entries_v0(
                token_id,
                owner_id,
                identity_contract_nonce,
                event,
                random_range,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prepay_random_token_history_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prepay_random_token_history_entries".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
