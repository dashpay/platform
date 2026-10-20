use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_transactions_broadcasted_path_vec, get_withdrawal_transactions_queue_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::dashcore::consensus::Decodable;
use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::AssetUnlockBaseTransactionInfo;
use dpp::fee::Credits;
use dpp::identity::convert_duffs_to_credits;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_in_flight_withdrawal_amount_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        let mut in_flight: Credits = 0;
        for path in [
            get_withdrawal_transactions_queue_path_vec(),
            get_withdrawal_transactions_broadcasted_path_vec(),
        ] {
            for amount in
                self.fetch_withdrawal_transaction_amounts(path, transaction, platform_version)?
            {
                in_flight = in_flight.checked_add(amount).ok_or(Error::Drive(
                    DriveError::CriticalCorruptedState("in-flight withdrawal amount overflow"),
                ))?;
            }
        }

        Ok(in_flight)
    }

    /// What each untied withdrawal transaction under `path` takes out of Core's credit pool, in
    /// credits: its outputs plus its fee.
    fn fetch_withdrawal_transaction_amounts(
        &self,
        path: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Credits>, Error> {
        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new_unsized(path, query);

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryElementResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let overflow = || {
            Error::Drive(DriveError::CriticalCorruptedState(
                "in-flight withdrawal amount overflow",
            ))
        };

        results
            .to_elements()
            .into_iter()
            .map(|element| {
                let Element::Item(bytes, _) = element else {
                    return Err(Error::Drive(DriveError::CorruptedElementType(
                        "withdrawal transaction is not an item",
                    )));
                };
                // Both trees hold the untied transaction as it was pooled; the request height
                // and quorum signature added when signing do not change what it unlocks.
                let untied =
                    AssetUnlockBaseTransactionInfo::consensus_decode(&mut bytes.as_slice())
                        .map_err(|_| {
                            Error::Drive(DriveError::CorruptedSerialization(
                                "withdrawal transaction cannot be decoded".to_string(),
                            ))
                        })?;

                // What Core counts against its limit: the outputs plus the fee.
                let mut duffs = untied.base_payload.fee as u64;
                for output in &untied.output {
                    duffs = duffs.checked_add(output.value).ok_or_else(overflow)?;
                }

                convert_duffs_to_credits(duffs).map_err(|_| overflow())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::consensus::Encodable;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    };
    use dpp::dashcore::{ScriptBuf, TxOut};
    use dpp::version::PlatformVersion;

    fn untied_transaction(index: u64, payout_duffs: u64, fee_duffs: u32) -> Vec<u8> {
        let transaction = AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![TxOut {
                value: payout_duffs,
                script_pubkey: ScriptBuf::new(),
            }],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index,
                fee: fee_duffs,
            },
        };
        let mut bytes = vec![];
        transaction
            .consensus_encode(&mut bytes)
            .expect("expected to encode");
        bytes
    }

    #[test]
    fn should_read_the_outputs_and_fees_of_queued_and_broadcast_transactions_in_credits() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        assert_eq!(
            drive
                .fetch_in_flight_withdrawal_amount(Some(&transaction), platform_version)
                .expect("expected the amount"),
            0
        );

        let mut drive_operations: Vec<DriveOperation> = vec![];
        drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![
                    (0, untied_transaction(0, 100_000, 2_000)),
                    (1, untied_transaction(1, 50_000, 1_000)),
                ],
                153_000_000,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to enqueue");
        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply");

        // Move one to the broadcast tree, as signing does.
        let mut drive_operations: Vec<DriveOperation> = vec![];
        drive
            .dequeue_untied_withdrawal_transactions(
                1,
                Some(&transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to dequeue");
        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply");

        // Queued: 50,000 + 1,000 duffs; broadcast: 100,000 + 2,000 duffs; in credits.
        assert_eq!(
            drive
                .fetch_in_flight_withdrawal_amount(Some(&transaction), platform_version)
                .expect("expected the amount"),
            153_000_000
        );
    }
}
