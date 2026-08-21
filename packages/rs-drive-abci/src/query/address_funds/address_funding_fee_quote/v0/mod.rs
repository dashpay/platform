use crate::error::query::QueryError;
use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_address_funding_fee_quote_request::GetAddressFundingFeeQuoteRequestV0;
use dapi_grpc::platform::v0::get_address_funding_fee_quote_response::GetAddressFundingFeeQuoteResponseV0;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::identity::KeyType;
use dpp::platform_value::Bytes36;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::address_funding_from_asset_lock_transition::calculate_address_funding_min_required_fee_for_counts;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;

/// Clamp bounds and default for the signable-bytes length hint. The hint only
/// sizes the quote's `DoubleSha256` charge (5 000 credits per 64-byte block,
/// well under 1% of the total), and it is clamped so a client cannot
/// understate the fee. The default is the measured signable length of the
/// single-input instant-proof 0-input/1-output funding fixture (390 bytes);
/// wallets whose L1 funding transaction is larger should pass the real
/// length. The calibration test in `address_funding_from_asset_lock/tests.rs`
/// pins the default against the real fixture so a transition format change
/// forces a conscious update here.
pub(crate) const MIN_SIGNABLE_BYTES_LEN_HINT: u32 = 128;
pub(crate) const DEFAULT_SIGNABLE_BYTES_LEN_HINT: u32 = 390;
pub(crate) const MAX_SIGNABLE_BYTES_LEN_HINT: u32 = 8_192;

/// Domain tag for the deterministic placeholder outpoint.
const PLACEHOLDER_OUTPOINT_TAG: &[u8] = b"address_funding_fee_quote_placeholder";

impl<C> Platform<C> {
    /// Version 0 of the address funding fee quote.
    ///
    /// Read-only: prices the exact production operations of a 0-input /
    /// 1-output funding of a FRESH asset lock against committed state
    /// (measured tree depths replace the worst-case layer counts), then adds
    /// the same validation-operation fees `transform_into_action` records and
    /// applies the requested `user_fee_increase`.
    ///
    /// The quoted fee does not depend on the lock amount (sum values are
    /// charged at a fixed width), so the engine is run with the admission
    /// floor as the modeled lock value. The response is a computed value, not
    /// state — it carries metadata but no proof.
    pub(super) fn query_address_funding_fee_quote_v0(
        &self,
        GetAddressFundingFeeQuoteRequestV0 {
            address,
            asset_lock_outpoint,
            user_fee_increase,
            signable_bytes_len_hint,
        }: GetAddressFundingFeeQuoteRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressFundingFeeQuoteResponseV0>, Error> {
        let Ok(recipient) = PlatformAddress::from_bytes(&address) else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "address must be a serialized platform address".to_string(),
                ),
            ));
        };

        let Ok(user_fee_increase): Result<UserFeeIncrease, _> = user_fee_increase.try_into() else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("user_fee_increase must fit in 16 bits".to_string()),
            ));
        };

        let outpoint = if asset_lock_outpoint.is_empty() {
            // Deterministic placeholder: sha256d over a domain tag, the
            // address and the committed height. Outpoint keys are uniformly
            // distributed (txids), so for a fresh lock the placeholder's
            // absence boundary has the same expected search depth as the real
            // key would.
            let mut seed = Vec::with_capacity(
                PLACEHOLDER_OUTPOINT_TAG.len() + address.len() + core::mem::size_of::<u64>(),
            );
            seed.extend_from_slice(PLACEHOLDER_OUTPOINT_TAG);
            seed.extend_from_slice(&address);
            seed.extend_from_slice(&platform_state.last_committed_block_height().to_be_bytes());
            let txid = hash_double(seed);
            let mut outpoint_bytes = [0u8; 36];
            outpoint_bytes[..32].copy_from_slice(&txid);
            // the remaining four bytes are vout 0 in little endian
            Bytes36::new(outpoint_bytes)
        } else {
            let Ok(outpoint_bytes): Result<[u8; 36], _> = asset_lock_outpoint.as_slice().try_into()
            else {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "asset_lock_outpoint must be 36 bytes (txid || vout)".to_string(),
                    ),
                ));
            };
            Bytes36::new(outpoint_bytes)
        };

        let minimum_required_lock_credits =
            calculate_address_funding_min_required_fee_for_counts(0, 1, platform_version)?;

        let block_info = BlockInfo {
            time_ms: platform_state
                .last_committed_block_time_ms()
                .unwrap_or_default(),
            height: platform_state.last_committed_block_height(),
            core_height: platform_state.last_committed_core_height(),
            epoch: platform_state.last_committed_block_epoch(),
        };

        let estimate = match self.drive.estimate_address_funding_fee(
            &recipient,
            outpoint,
            minimum_required_lock_credits,
            &block_info,
            platform_version,
        ) {
            Ok(estimate) => estimate,
            Err(drive::error::Error::Drive(DriveError::AssetLockOutpointAlreadyPresent(_))) => {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "asset_lock_outpoint is already present in the state (spent or partially \
                         used); the quote models a fresh lock"
                            .to_string(),
                    ),
                ));
            }
            Err(error) => return Err(error.into()),
        };
        let mut fee_result = estimate.fee_result;

        // The same validation operations transform_into_action records for a
        // fresh 0-input/1-output funding: hashing the signable bytes and one
        // ECDSA_HASH160 verification of the one-time key signature. The block
        // count deliberately uses the same integer division.
        let signable_bytes_len = if signable_bytes_len_hint == 0 {
            DEFAULT_SIGNABLE_BYTES_LEN_HINT
        } else {
            signable_bytes_len_hint.clamp(MIN_SIGNABLE_BYTES_LEN_HINT, MAX_SIGNABLE_BYTES_LEN_HINT)
        };
        let block_count = signable_bytes_len as u16 / SHA256_BLOCK_SIZE;
        ValidationOperation::add_many_to_fee_result(
            &[
                ValidationOperation::DoubleSha256(block_count),
                ValidationOperation::SignatureVerification(SignatureVerificationOperation::new(
                    KeyType::ECDSA_HASH160,
                )),
            ],
            &mut fee_result,
            platform_version,
        )?;

        fee_result.apply_user_fee_increase(user_fee_increase);

        let response = GetAddressFundingFeeQuoteResponseV0 {
            estimated_fee_credits: fee_result.total_base_fee(),
            minimum_required_lock_credits,
            protocol_version: platform_version.protocol_version,
            state_height: platform_state.last_committed_block_height(),
            metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
