use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_token_orders_path,
    prefunded_specialized_balances_for_token_orders_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::grove_operations::DirectQueryType;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::{Element, TransactionArg};

/// Escrowed credits are a SumItem, which holds an i64.
fn escrow_sum_value(credits: Credits) -> Result<i64, Error> {
    i64::try_from(credits).map_err(|_| {
        ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
            "credits to hold in escrow over i64 max, are {credits}"
        ))
        .into()
    })
}

impl Drive {
    pub(super) fn add_to_credit_order_escrow_operations_v0(
        &self,
        order_id: [u8; 32],
        credits: Credits,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_credit_order_escrow(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(vec![
            LowLevelDriveOperation::insert_for_known_path_key_element(
                prefunded_specialized_balances_for_token_orders_path_vec(),
                order_id.to_vec(),
                Element::new_sum_item(escrow_sum_value(credits)?),
            ),
        ])
    }

    pub(super) fn set_credit_order_escrow_operations_v0(
        &self,
        order_id: [u8; 32],
        credits: Credits,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_credit_order_escrow(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(vec![
            LowLevelDriveOperation::replace_for_known_path_key_element(
                prefunded_specialized_balances_for_token_orders_path_vec(),
                order_id.to_vec(),
                Element::new_sum_item(escrow_sum_value(credits)?),
            ),
        ])
    }

    pub(super) fn remove_credit_order_escrow_operations_v0(
        &self,
        order_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_credit_order_escrow(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(vec![GroveOperation(
            QualifiedGroveDbOp::delete_op(
                prefunded_specialized_balances_for_token_orders_path_vec(),
                order_id.to_vec(),
            )
            .dont_check_for_backwards_references(),
        )])
    }

    pub(super) fn fetch_credit_order_escrow_v0(
        &self,
        order_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let path = prefunded_specialized_balances_for_token_orders_path();
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
            Some(Element::SumItem(credits, _)) if credits >= 0 => Ok(Some(credits as Credits)),
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "a credit order escrow entry must be a non-negative sum item",
            ))),
        }
    }
}
