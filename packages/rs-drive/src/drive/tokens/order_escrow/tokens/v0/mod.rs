use crate::drive::tokens::paths::{
    token_order_escrow_path, token_order_escrow_path_vec, token_order_escrow_root_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::grove_operations::{BatchInsertTreeApplyType, DirectQueryType};
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::{Element, TransactionArg, TreeType};

/// An escrowed amount is a SumItem, which holds an i64.
fn escrow_sum_value(amount: TokenAmount) -> Result<i64, Error> {
    i64::try_from(amount).map_err(|_| {
        ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
            "token amount to hold in escrow over i64 max, is {amount}"
        ))
        .into()
    })
}

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_order_escrow_operations_v0(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        amount: TokenAmount,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_order_escrow(
                token_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        let tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::BigSumTree,
                tree_type: TreeType::SumTree,
                flags_len: 0,
            }
        };

        // The token's escrow tree is created with its first listing, whether that is in state or
        // queued earlier in the same batch.
        self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKeyRef::<2>((token_order_escrow_root_path(), token_id.as_slice())),
            TreeType::SumTree,
            None,
            tree_apply_type,
            transaction,
            previous_batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            token_order_escrow_path_vec(token_id),
            order_id.to_vec(),
            Element::new_sum_item(escrow_sum_value(amount)?),
        ));

        Ok(drive_operations)
    }

    pub(super) fn set_token_order_escrow_operations_v0(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        amount: TokenAmount,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_order_escrow(
                token_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(vec![
            LowLevelDriveOperation::replace_for_known_path_key_element(
                token_order_escrow_path_vec(token_id),
                order_id.to_vec(),
                Element::new_sum_item(escrow_sum_value(amount)?),
            ),
        ])
    }

    pub(super) fn remove_token_order_escrow_operations_v0(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_order_escrow(
                token_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(vec![GroveOperation(
            QualifiedGroveDbOp::delete_op(token_order_escrow_path_vec(token_id), order_id.to_vec())
                .dont_check_for_backwards_references(),
        )])
    }

    pub(super) fn fetch_token_order_escrow_v0(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        let path = token_order_escrow_path(&token_id);
        let element = self.grove_get_raw_optional(
            (&path).into(),
            order_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        match element {
            None => Ok(None),
            Some(Element::SumItem(amount, _)) if amount >= 0 => Ok(Some(amount as TokenAmount)),
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "a token order escrow entry must be a non-negative sum item",
            ))),
        }
    }
}
