use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_compacted_address_balance_changes_request::GetRecentCompactedAddressBalanceChangesRequestV0;
use dapi_grpc::platform::v0::get_recent_compacted_address_balance_changes_response::{
    get_recent_compacted_address_balance_changes_response_v0,
    GetRecentCompactedAddressBalanceChangesResponseV0,
};
use dapi_grpc::platform::v0::{
    compacted_address_balance_change, AddToCreditsOperations, BlockHeightCreditEntry,
    CompactedAddressBalanceChange, CompactedAddressBalanceUpdateEntries,
    CompactedBlockAddressBalanceChanges,
};
use dpp::balances::credits::BlockAwareCreditOperation;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_recent_compacted_address_balance_changes_v0(
        &self,
        GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height,
            prove,
        }: GetRecentCompactedAddressBalanceChangesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentCompactedAddressBalanceChangesResponseV0>, Error>
    {
        // Limit the number of compacted entries we return (max 25 to stay within proof size limits)
        // Ensure it matches limit set in rs-drive-proof-verifier/src/proof.rs for RecentCompactedAddressBalanceChanges
        let limit = Some(25u16);

        let response = if prove {
            let proof = self.drive.prove_compacted_address_balance_changes(
                start_block_height,
                limit,
                None,
                platform_version,
            )?;

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetRecentCompactedAddressBalanceChangesResponseV0 {
                result: Some(
                    get_recent_compacted_address_balance_changes_response_v0::Result::Proof(proof),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let compacted_address_balance_changes =
                self.drive.fetch_compacted_address_balance_changes(
                    start_block_height,
                    limit,
                    None,
                    platform_version,
                )?;

            // Convert the fetched data to proto format
            let compacted_block_changes: Vec<CompactedBlockAddressBalanceChanges> =
                compacted_address_balance_changes
                    .into_iter()
                    .map(|(start_block, end_block, changes)| {
                        let address_changes: Vec<CompactedAddressBalanceChange> = changes
                            .into_iter()
                            .map(|(address, operation)| {
                                let op = match operation {
                                    BlockAwareCreditOperation::SetCredits(credits) => {
                                        compacted_address_balance_change::Operation::SetCredits(
                                            credits,
                                        )
                                    }
                                    BlockAwareCreditOperation::AddToCreditsOperations(
                                        block_credits_map,
                                    ) => {
                                        let entries: Vec<BlockHeightCreditEntry> =
                                            block_credits_map
                                                .into_iter()
                                                .map(|(block_height, credits)| {
                                                    BlockHeightCreditEntry {
                                                        block_height,
                                                        credits,
                                                    }
                                                })
                                                .collect();
                                        compacted_address_balance_change::Operation::AddToCreditsOperations(
                                            AddToCreditsOperations { entries },
                                        )
                                    }
                                };
                                CompactedAddressBalanceChange {
                                    address: address.to_bytes(),
                                    operation: Some(op),
                                }
                            })
                            .collect();

                        CompactedBlockAddressBalanceChanges {
                            start_block_height: start_block,
                            end_block_height: end_block,
                            changes: address_changes,
                        }
                    })
                    .collect();

            GetRecentCompactedAddressBalanceChangesResponseV0 {
                result: Some(
                    get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(
                        CompactedAddressBalanceUpdateEntries { compacted_block_changes },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
