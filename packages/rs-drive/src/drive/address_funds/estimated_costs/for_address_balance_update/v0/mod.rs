use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::SomeSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// Size of a PlatformAddress key (1 byte type + 20 bytes hash)
pub const PLATFORM_ADDRESS_KEY_SIZE: u32 = 21;

/// Average size of nonce stored with balance (8 bytes for u64)
pub const AVERAGE_NONCE_SIZE: u32 = 8;

impl Drive {
    /// Adds estimation costs for address balance updates in version 0.
    ///
    /// This function provides cost estimation for the address balances tree structure:
    /// - Root level: Contains AddressBalances sum tree
    /// - AddressBalances level: Contains CLEAR_ADDRESS_POOL sum tree
    /// - CLEAR_ADDRESS_POOL level: Contains address entries with balance and nonce
    ///
    /// # Parameters
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap`
    ///   that stores estimated layer information based on the key information path.
    pub(super) fn add_estimation_costs_for_address_balance_update_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        //                                                                                DataContract_Documents 64
        //                                 /                                                                                                       \
        //                       Identities 32                                                                                                 Balances 96
        //             /                            \                                                                        /                                                       \
        //       Tokens 16                    Pools 48                                                    WithdrawalTransactions 80                                                Votes  112
        //       /      \                           /                     \                                         /                           \                            /                          \
        //     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40  AddressBalances 56              SpentAssetLockTransactions 72    GroupActions 88             Misc 104                        Versions 120

        // Root level estimation
        // Path to AddressBalances (56): DataContractDocuments (64) → Identities (32) → Pools (48) → AddressBalances (56)
        // - DataContractDocuments: Normal tree
        // - Identities: Normal tree
        // - Pools: Sum tree
        // - AddressBalances: Sum tree
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 2, // AddressBalances and Pools are sum trees
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 2, // Other root trees
                    },
                    None,
                ),
            },
        );

        // AddressBalances level estimation (path: [[0x8c]])
        // This sum tree contains the CLEAR_ADDRESS_POOL subtree
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(Self::addresses_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 1,
                        non_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        // CLEAR_ADDRESS_POOL level estimation (path: [[0x8c], [0x63]])
        // This sum tree contains the actual address entries
        // Keys are 21-byte PlatformAddress, values are ItemWithSumItem (nonce + balance)
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(Self::clear_addresses_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::CountSumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    PLATFORM_ADDRESS_KEY_SIZE as u8,
                    AVERAGE_NONCE_SIZE + AVERAGE_BALANCE_SIZE,
                    None,
                ),
            },
        );
    }
}
