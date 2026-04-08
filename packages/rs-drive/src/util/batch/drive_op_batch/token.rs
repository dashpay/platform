use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, TimestampMillis};
use dpp::tokens::status::TokenStatus;
use dpp::tokens::token_event::TokenEvent;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

/// Operations on Tokens
#[derive(Clone, Debug)]
pub enum TokenOperationType {
    /// Burns token from the account issuing the action.
    TokenBurn {
        /// The token id
        token_id: Identifier,
        /// The identity to burn from
        identity_balance_holder_id: Identifier,
        /// The amount to burn
        burn_amount: TokenAmount,
    },
    /// Mints tokens
    TokenMint {
        /// The token id
        token_id: Identifier,
        /// The identity to mint to
        identity_balance_holder_id: Identifier,
        /// The amount to issue
        mint_amount: TokenAmount,
        /// Should we allow this to be the first ever mint
        allow_first_mint: bool,
        /// Should we allow a mint to saturate the upper bounds instead of giving an error?
        /// For example if we were to add 10 to i64::Max - 5 we would get i64::Max
        allow_saturation: bool,
    },
    /// Mints tokens to many recipients
    TokenMintMany {
        /// The token id
        token_id: Identifier,
        /// The identities that will receive this amount along with their weight
        recipients: Vec<(Identifier, u64)>,
        /// The amount to issue
        mint_amount: TokenAmount,
        /// Should we allow this to be the first ever mint
        allow_first_mint: bool,
    },
    /// Marks the perpetual release as distributed
    /// This removes the references in the queue
    TokenMarkPerpetualReleaseAsDistributed {
        /// The token id
        token_id: Identifier,
        /// The recipient of this operation, generally the person making the claim state transition
        recipient_id: Identifier,
        /// The beginning of the current perpetual release cycle.
        /// For example if we pay every 10 blocks, and we are on block 54, this would be 50.
        cycle_start_moment: RewardDistributionMoment,
    },
    /// Marks the pre-programmed release as distributed
    /// This removes the references in the queue
    TokenMarkPreProgrammedReleaseAsDistributed {
        /// The token id
        token_id: Identifier,
        /// The recipient of this operation, generally the person making the state transition
        recipient_id: Identifier,
        /// The last release time, block or epoch
        release_time: TimestampMillis,
    },
    /// Performs a token transfer
    TokenTransfer {
        /// The token id
        token_id: Identifier,
        /// The token id
        sender_id: Identifier,
        /// The recipient of the transfer
        recipient_id: Identifier,
        /// The amount to transfer
        amount: TokenAmount,
    },
    /// Freezes an identity's token balance so money can no longer be sent out.
    TokenFreeze {
        /// The token id
        token_id: Identifier,
        /// The frozen identity id
        frozen_identity_id: Identifier,
    },
    /// Unfreezes an identity's token balance so money can be sent out again.
    TokenUnfreeze {
        /// The token id
        token_id: Identifier,
        /// The frozen identity id
        frozen_identity_id: Identifier,
    },
    /// Sets the status of the token.
    TokenSetStatus {
        /// The token id
        token_id: Identifier,
        /// The status
        status: TokenStatus,
    },
    /// Adds a historical document explaining a token action.
    TokenHistory {
        /// The token id
        token_id: Identifier,
        /// The identity making the event
        owner_id: Identifier,
        /// The nonce
        nonce: IdentityNonce,
        /// The token event
        event: TokenEvent,
    },
    /// Sets the price of a token for direct purchase
    TokenSetPriceForDirectPurchase {
        /// The token id
        token_id: Identifier,
        /// The price we are setting to
        /// None means it's not currently for sale
        price: Option<TokenPricingSchedule>,
    },
}

impl DriveLowLevelOperationConverter for TokenOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            TokenOperationType::TokenBurn {
                token_id,
                identity_balance_holder_id,
                burn_amount,
            } => {
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let identity_id_bytes: [u8; 32] = identity_balance_holder_id.to_buffer();
                let batch_operations = drive.token_burn_operations(
                    token_id_bytes,
                    identity_id_bytes,
                    burn_amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenMint {
                token_id,
                identity_balance_holder_id,
                mint_amount,
                allow_first_mint,
                allow_saturation,
            } => {
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let identity_id_bytes: [u8; 32] = identity_balance_holder_id.to_buffer();
                let batch_operations = drive.token_mint_operations(
                    token_id_bytes,
                    identity_id_bytes,
                    mint_amount,
                    allow_first_mint,
                    allow_saturation,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenMintMany {
                token_id,
                recipients,
                mint_amount,
                allow_first_mint,
            } => {
                let batch_operations = drive.token_mint_many_operations(
                    token_id,
                    recipients,
                    mint_amount,
                    allow_first_mint,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenTransfer {
                token_id,
                sender_id,
                recipient_id,
                amount,
            } => {
                let token_id_bytes: [u8; 32] = token_id.to_buffer();
                let sender_id_bytes: [u8; 32] = sender_id.to_buffer();
                let recipient_id_bytes: [u8; 32] = recipient_id.to_buffer();

                let batch_operations = drive.token_transfer_operations(
                    token_id_bytes,
                    sender_id_bytes,
                    recipient_id_bytes,
                    amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenHistory {
                token_id,
                owner_id,
                nonce,
                event,
            } => {
                let batch_operations = drive.add_token_transaction_history_operations(
                    token_id,
                    owner_id,
                    nonce,
                    event,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenFreeze {
                token_id,
                frozen_identity_id,
            } => {
                let batch_operations = drive.token_freeze_operations(
                    token_id,
                    frozen_identity_id,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenUnfreeze {
                token_id,
                frozen_identity_id,
            } => {
                let batch_operations = drive.token_unfreeze_operations(
                    token_id,
                    frozen_identity_id,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenSetStatus { token_id, status } => {
                let batch_operations = drive.token_apply_status_operations(
                    token_id.to_buffer(),
                    status,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenMarkPerpetualReleaseAsDistributed {
                token_id,
                recipient_id,
                cycle_start_moment,
            } => {
                let batch_operations = drive.mark_perpetual_release_as_distributed_operations(
                    token_id.to_buffer(),
                    recipient_id.to_buffer(),
                    cycle_start_moment,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenMarkPreProgrammedReleaseAsDistributed {
                token_id,
                recipient_id,
                release_time,
            } => {
                let batch_operations = drive
                    .mark_pre_programmed_release_as_distributed_operations(
                        token_id.to_buffer(),
                        recipient_id.to_buffer(),
                        release_time,
                        block_info,
                        estimated_costs_only_with_layer_info,
                        transaction,
                        platform_version,
                    )?;
                Ok(batch_operations)
            }
            TokenOperationType::TokenSetPriceForDirectPurchase { token_id, price } => {
                let batch_operations = drive.token_set_direct_purchase_price_operations(
                    token_id.to_buffer(),
                    price,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_token_id() -> Identifier {
        Identifier::new([1u8; 32])
    }

    fn test_identity_id() -> Identifier {
        Identifier::new([2u8; 32])
    }

    fn test_recipient_id() -> Identifier {
        Identifier::new([3u8; 32])
    }

    // ---------------------------------------------------------------
    // TokenBurn construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_burn_construction() {
        let op = TokenOperationType::TokenBurn {
            token_id: test_token_id(),
            identity_balance_holder_id: test_identity_id(),
            burn_amount: 500,
        };

        match op {
            TokenOperationType::TokenBurn {
                token_id,
                identity_balance_holder_id,
                burn_amount,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(identity_balance_holder_id, test_identity_id());
                assert_eq!(burn_amount, 500);
            }
            _ => panic!("expected TokenBurn variant"),
        }
    }

    // ---------------------------------------------------------------
    // TokenMint construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_mint_construction() {
        let op = TokenOperationType::TokenMint {
            token_id: test_token_id(),
            identity_balance_holder_id: test_identity_id(),
            mint_amount: 1_000_000,
            allow_first_mint: true,
            allow_saturation: false,
        };

        match op {
            TokenOperationType::TokenMint {
                token_id,
                identity_balance_holder_id,
                mint_amount,
                allow_first_mint,
                allow_saturation,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(identity_balance_holder_id, test_identity_id());
                assert_eq!(mint_amount, 1_000_000);
                assert!(allow_first_mint);
                assert!(!allow_saturation);
            }
            _ => panic!("expected TokenMint variant"),
        }
    }

    #[test]
    fn test_token_mint_with_saturation_enabled() {
        let op = TokenOperationType::TokenMint {
            token_id: test_token_id(),
            identity_balance_holder_id: test_identity_id(),
            mint_amount: u64::MAX,
            allow_first_mint: false,
            allow_saturation: true,
        };

        match op {
            TokenOperationType::TokenMint {
                allow_saturation,
                allow_first_mint,
                mint_amount,
                ..
            } => {
                assert!(allow_saturation);
                assert!(!allow_first_mint);
                assert_eq!(mint_amount, u64::MAX);
            }
            _ => panic!("expected TokenMint variant"),
        }
    }

    // ---------------------------------------------------------------
    // TokenMintMany construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_mint_many_construction() {
        let recipients = vec![
            (Identifier::new([10u8; 32]), 50),
            (Identifier::new([11u8; 32]), 30),
            (Identifier::new([12u8; 32]), 20),
        ];
        let op = TokenOperationType::TokenMintMany {
            token_id: test_token_id(),
            recipients: recipients.clone(),
            mint_amount: 100_000,
            allow_first_mint: true,
        };

        match op {
            TokenOperationType::TokenMintMany {
                token_id,
                recipients: r,
                mint_amount,
                allow_first_mint,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(r.len(), 3);
                assert_eq!(r[0].1, 50);
                assert_eq!(r[1].1, 30);
                assert_eq!(r[2].1, 20);
                assert_eq!(mint_amount, 100_000);
                assert!(allow_first_mint);
            }
            _ => panic!("expected TokenMintMany variant"),
        }
    }

    // ---------------------------------------------------------------
    // TokenTransfer construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_transfer_construction() {
        let sender = Identifier::new([4u8; 32]);
        let recipient = Identifier::new([5u8; 32]);
        let op = TokenOperationType::TokenTransfer {
            token_id: test_token_id(),
            sender_id: sender,
            recipient_id: recipient,
            amount: 250,
        };

        match op {
            TokenOperationType::TokenTransfer {
                token_id,
                sender_id,
                recipient_id,
                amount,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(sender_id, Identifier::new([4u8; 32]));
                assert_eq!(recipient_id, Identifier::new([5u8; 32]));
                assert_eq!(amount, 250);
            }
            _ => panic!("expected TokenTransfer variant"),
        }
    }

    // ---------------------------------------------------------------
    // TokenFreeze / TokenUnfreeze construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_freeze_construction() {
        let frozen = Identifier::new([6u8; 32]);
        let op = TokenOperationType::TokenFreeze {
            token_id: test_token_id(),
            frozen_identity_id: frozen,
        };

        match op {
            TokenOperationType::TokenFreeze {
                token_id,
                frozen_identity_id,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(frozen_identity_id, Identifier::new([6u8; 32]));
            }
            _ => panic!("expected TokenFreeze variant"),
        }
    }

    #[test]
    fn test_token_unfreeze_construction() {
        let frozen = Identifier::new([7u8; 32]);
        let op = TokenOperationType::TokenUnfreeze {
            token_id: test_token_id(),
            frozen_identity_id: frozen,
        };

        match op {
            TokenOperationType::TokenUnfreeze {
                token_id,
                frozen_identity_id,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(frozen_identity_id, Identifier::new([7u8; 32]));
            }
            _ => panic!("expected TokenUnfreeze variant"),
        }
    }

    // ---------------------------------------------------------------
    // TokenSetPriceForDirectPurchase construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_set_price_none() {
        let op = TokenOperationType::TokenSetPriceForDirectPurchase {
            token_id: test_token_id(),
            price: None,
        };

        match op {
            TokenOperationType::TokenSetPriceForDirectPurchase { token_id, price } => {
                assert_eq!(token_id, test_token_id());
                assert!(price.is_none());
            }
            _ => panic!("expected TokenSetPriceForDirectPurchase variant"),
        }
    }

    // ---------------------------------------------------------------
    // TokenMarkPreProgrammedReleaseAsDistributed construction
    // ---------------------------------------------------------------

    #[test]
    fn test_token_mark_pre_programmed_release_construction() {
        let op = TokenOperationType::TokenMarkPreProgrammedReleaseAsDistributed {
            token_id: test_token_id(),
            recipient_id: test_recipient_id(),
            release_time: 1_700_000_000_000,
        };

        match op {
            TokenOperationType::TokenMarkPreProgrammedReleaseAsDistributed {
                token_id,
                recipient_id,
                release_time,
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(recipient_id, test_recipient_id());
                assert_eq!(release_time, 1_700_000_000_000);
            }
            _ => panic!("expected TokenMarkPreProgrammedReleaseAsDistributed variant"),
        }
    }

    // ---------------------------------------------------------------
    // Clone behavior
    // ---------------------------------------------------------------

    #[test]
    fn test_token_operation_clone() {
        let op = TokenOperationType::TokenBurn {
            token_id: test_token_id(),
            identity_balance_holder_id: test_identity_id(),
            burn_amount: 100,
        };
        let cloned = op.clone();
        match cloned {
            TokenOperationType::TokenBurn { burn_amount, .. } => {
                assert_eq!(burn_amount, 100);
            }
            _ => panic!("clone should preserve variant"),
        }
    }

    // ---------------------------------------------------------------
    // Debug trait
    // ---------------------------------------------------------------

    #[test]
    fn test_token_set_status_construction() {
        use dpp::tokens::status::v0::TokenStatusV0;
        let op = TokenOperationType::TokenSetStatus {
            token_id: test_token_id(),
            status: TokenStatus::V0(TokenStatusV0 { paused: true }),
        };
        match op {
            TokenOperationType::TokenSetStatus { token_id, .. } => {
                assert_eq!(token_id, test_token_id());
            }
            _ => panic!("expected TokenSetStatus variant"),
        }
    }

    #[test]
    fn test_token_history_construction() {
        use dpp::tokens::token_event::TokenEvent;

        let op = TokenOperationType::TokenHistory {
            token_id: test_token_id(),
            owner_id: test_identity_id(),
            nonce: 42,
            event: TokenEvent::Mint(1000, test_identity_id(), None),
        };
        match op {
            TokenOperationType::TokenHistory {
                token_id,
                owner_id,
                nonce,
                ..
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(owner_id, test_identity_id());
                assert_eq!(nonce, 42);
            }
            _ => panic!("expected TokenHistory variant"),
        }
    }

    #[test]
    fn test_token_mark_perpetual_release_construction() {
        let op = TokenOperationType::TokenMarkPerpetualReleaseAsDistributed {
            token_id: test_token_id(),
            recipient_id: test_recipient_id(),
            cycle_start_moment: RewardDistributionMoment::BlockBasedMoment(100),
        };
        match op {
            TokenOperationType::TokenMarkPerpetualReleaseAsDistributed {
                token_id,
                recipient_id,
                ..
            } => {
                assert_eq!(token_id, test_token_id());
                assert_eq!(recipient_id, test_recipient_id());
            }
            _ => panic!("expected TokenMarkPerpetualReleaseAsDistributed variant"),
        }
    }

    #[test]
    fn test_token_set_price_some() {
        use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

        let pricing = TokenPricingSchedule::SinglePrice(5000);
        let op = TokenOperationType::TokenSetPriceForDirectPurchase {
            token_id: test_token_id(),
            price: Some(pricing),
        };
        match op {
            TokenOperationType::TokenSetPriceForDirectPurchase { token_id, price } => {
                assert_eq!(token_id, test_token_id());
                assert!(price.is_some());
            }
            _ => panic!("expected TokenSetPriceForDirectPurchase variant"),
        }
    }

    #[test]
    fn test_token_operation_debug() {
        let op = TokenOperationType::TokenBurn {
            token_id: test_token_id(),
            identity_balance_holder_id: test_identity_id(),
            burn_amount: 42,
        };
        let debug_str = format!("{:?}", op);
        assert!(debug_str.contains("TokenBurn"));
        assert!(debug_str.contains("42"));
    }
}
