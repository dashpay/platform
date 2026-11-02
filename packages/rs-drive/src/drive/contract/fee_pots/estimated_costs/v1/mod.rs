use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
use crate::drive::Drive;
use crate::util::estimated_layers::register_keeping_deeper_estimated_layers;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use grovedb::batch::KeyInfoPath;

impl Drive {
    /// v0 (protocol version 15), keeping a deeper description of the prefunded balances layer
    /// that another write of the batch registered: the escrow of the token marketplace's offers
    /// sits a level below the fee pots' keys.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_contract_fee_pot_update_v1(
        pot: ContractFeePot,
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        register_keeping_deeper_estimated_layers(
            estimated_costs_only_with_layer_info,
            &[KeyInfoPath::from_known_path(
                prefunded_specialized_balances_path(),
            )],
            |estimated_costs_only_with_layer_info| {
                Self::add_estimation_costs_for_contract_fee_pot_update_v0(
                    pot,
                    estimated_costs_only_with_layer_info,
                )
            },
        );
    }
}
