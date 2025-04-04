mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Adds estimated costs for the layer information of group updates in a contract.
    ///
    /// This function updates the `estimated_costs_only_with_layer_info` map with the layer information for
    /// the trees involved in adding or updating a group action in the context of a contract. The trees are
    /// organized hierarchically based on their role in the system, such as "Group Actions", "Withdrawal Transactions",
    /// "Balances", and "Contract/Documents". This estimation is used to determine the computational costs associated
    /// with updating these trees, considering whether they are sum trees or normal trees and their expected layer counts.
    ///
    /// The function breaks down the tree layers and their corresponding costs as follows:
    /// 1. **Group Actions Tree**: A normal tree that holds information about group actions in the contract.
    /// 2. **Withdrawal Transactions Tree**: A normal tree that holds withdrawal transaction data.
    /// 3. **Balances Tree**: A sum tree that holds balance information, which is crucial for cost estimation.
    /// 4. **Contract/Documents Tree**: A normal tree that holds contract and document-related data.
    ///
    /// Each tree's cost is estimated based on its depth and whether it's a sum tree or not. The function inserts the
    /// estimated layer information for each relevant tree in the `estimated_costs_only_with_layer_info` map, where
    /// the key represents the path to the specific tree and the value represents its estimated layer information.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: The unique identifier of the contract being updated. Used to construct paths for the trees.
    /// - `groups`: The groups that we will insert.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap` where the estimated layer information
    ///   will be inserted. The keys represent paths to the trees, and the values represent their estimated layer information.
    ///
    /// # Logic Breakdown
    ///
    /// - **Top Layer (Contract/Documents)**: The contract and documents tree is at the top level, with a weight of 2.
    /// - **Balance Tree (Sum Tree)**: The balance tree is a sum tree with a weight of 1.
    /// - **Withdrawal Transactions**: This tree is a normal tree, and it is expected to have a weight of 2.
    /// - **Group Action Tree**: The group action tree is also a normal tree, with an expected weight of 2.
    /// - **Additional Layer Costs**: For specific paths related to actions, signers, etc., further estimations are added with
    ///   appropriate layer counts and subtree size estimations.
    ///
    /// The function constructs the paths based on the contract ID, group contract position, and action ID (if provided).
    /// It then populates the `estimated_costs_only_with_layer_info` map with the estimated costs for each relevant tree
    /// involved in the group action update.
    pub(crate) fn add_estimation_costs_for_add_groups(
        contract_id: [u8; 32],
        groups: &BTreeMap<GroupContractPosition, Group>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.methods.group.cost_estimation.for_add_group {
            0 => {
                Self::add_estimation_costs_for_add_groups_v0(
                    contract_id,
                    groups,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_add_group".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
