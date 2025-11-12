use crate::drive::tokens::paths::{
    token_ms_timed_at_time_distributions_path_vec, token_ms_timed_distributions_path_vec,
    token_pre_programmed_at_time_distribution_path_vec, token_pre_programmed_distributions_path,
    token_root_pre_programmed_distributions_path,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_distribution_key::{
    TokenDistributionKey, TokenDistributionType,
};
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

//                          [ROOT: Tokens]
//                                 │
//             ┌───────────────────┴───────────────────┐
//             │                                       │
//  [TOKEN_STATUS_INFO_KEY]                 [TOKEN_DISTRIBUTIONS_KEY]
//                                                   │
//                                                   ├─────────────────────────┐
//                                                   │                         │
//                                [TOKEN_TIMED_DISTRIBUTIONS_KEY]   [TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY]
//                                                   │                         │
//                                                   │                         │
//                                      ┌────────────┴────────────┐            │
//                                      │                         │            │
//             [TOKEN_MS_TIMED_DISTRIBUTIONS_KEY]   ... (other timed trees)    │
//                                      │                                      │
//                                      │                           For each token (token_id)
//                                      │                                      │
//                                      │                           ┌──────────┴──────────┐
//                                      │                           │                     │
//                                      │                  token_id (e.g., TKN)          ...
//                                      │                           │
//                                      │                           │
//                                      │                    For each distribution time:
//                                      │                           │
//                                      │                   ┌───────┴───────┐
//                                      │                   │   time (ts)   │  <-- Key: timestamp (8 bytes)
//                                      │                   └───────┬───────┘
//                                      │                           │
//                                      │                  [ Sum Tree: Recipient → Amount ]
//                                      │                           │
//                                      │          ┌────────────────┴────────────────┐
//                                      │          │                │                │
//                                      │      recipient A -> amount    recipient B -> amount, etc.  <──────┐
//                                      │                                                                   │
//                                      └────────────────────────────────────────────────────               │
//                                                   (Separate branch)                                      │
//                                                   └─ In the TIMED DISTRIBUTIONS branch:                  │
//                                                        For each time:                                    │
//                                                        ┌─────────────┐                                   │
//                                                        │  time (ts)  │  <-- Key: timestamp (8 bytes)     │
//                                                        └─────┬───────┘                                   │
//                                                              │                                           │
//                                                [ Reference Tree: Serialized Distribution Keys ]  ────────┘

// Explanation of Each Layer
// 	1.	Root Level (Tokens):
// 	•	The top-level of the tree corresponds to the tokens in the system. The path starts with the root identifier for tokens.
// 	2.	Distributions Branch (TOKEN_DISTRIBUTIONS_KEY):
// 	•	Under the Tokens root, there is a branch reserved for token distributions.
// 	•	This branch holds several subtrees for different kinds of distribution data.
// 	3.	Timed vs. Pre-Programmed Distribution Subtrees:
// 	•	Timed Distributions:
// 	•	Located under TOKEN_TIMED_DISTRIBUTIONS_KEY, these trees help organize distributions by time for features like verifying the exact moment a distribution was made.
// 	•	For example, the Millisecond Timed Distributions branch (TOKEN_MS_TIMED_DISTRIBUTIONS_KEY) contains nodes for each timestamp.
// 	•	Pre-Programmed Distributions:
// 	•	Located under TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, this branch stores pre-programmed distribution data.
// 	•	For each token, a subtree is created using the token’s identifier.
// 	•	Under that, each distribution time (converted to an 8‑byte big‑endian key) gets its own node (a sum tree).
// 	4.	Inside Each Pre-Programmed Time Node:
// 	•	Each node for a specific timestamp is a sum tree that holds key–value pairs where:
// 	•	Key: The recipient’s identifier (the person or entity receiving tokens).
// 	•	Value: The token amount (stored as a sum item).
// 	•	This is the core data for a pre-programmed distribution at a specific time.
// 	5.	Reference Insertion in Timed Distributions:
// 	•	In parallel, a reference is inserted into the Millisecond Timed Distributions branch for the same timestamp.
// 	•	This reference links back to the pre-programmed distribution data. It uses a TokenDistributionKey (which includes the token ID, recipient, and distribution type) to serialize and store a reference.
// 	•	This reference is stored in a subtree keyed by the timestamp (again, 8 bytes).

// What the Function Does
// 	1.	Insert the Pre-Programmed Distributions Tree:
// 	•	First, the function ensures that the subtree for pre‑programmed distributions for the token exists.
// 	•	It uses the fixed path:
// [RootTree::Tokens, TOKEN_DISTRIBUTIONS_KEY, TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, token_id].
// 	2.	For Each Distribution Time:
// 	•	The function iterates over each distribution time (from the provided distribution data).
// 	•	It inserts an empty sum tree for that time into the pre‑programmed subtree using the timestamp as a key.
// 	•	It then creates a corresponding entry in the millisecond timed distributions branch.
// This involves:
// 	•	Inserting an empty tree if necessary, with storage flags.
// 	•	Creating a reference path using the distribution key and additional metadata.
// 	•	Finally, for each recipient at that timestamp, it inserts:
// 	•	A sum item into the pre‑programmed distribution tree with the recipient’s identifier and the token amount.
// 	•	A reference into the timed distributions tree that points back to the pre‑programmed entry.
// 	3.	Why This Structure?
// 	•	This hierarchical tree structure allows for efficient queries and proofs.
// 	•	You can query by token, then by time, and then by recipient, and also verify that distributions were made at specific times.
// 	•	The reference links between the pre-programmed and timed distributions trees help verify the ordering and correctness of distribution events.

impl Drive {
    /// Version 0 of `add_perpetual_distribution`
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_pre_programmed_distributions_v0(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        distribution: &TokenPreProgrammedDistribution,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_token_pre_programmed_distribution(
                token_id,
                Some(distribution.distributions().keys()),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;

            Drive::add_estimation_costs_for_root_token_ms_interval_distribution(
                distribution.distributions().keys(),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

        let pre_programmed_distributions_path = token_root_pre_programmed_distributions_path();

        // Insert the tree for this token's perpetual distribution
        let apply_tree_type_no_storage_flags = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        let apply_tree_type_with_storage_flags = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: storage_flags.serialized_size(),
            }
        };

        let token_tree_key_info = DriveKeyInfo::Key(token_id.to_vec());
        let pre_programmed_distributions_path_key_info = token_tree_key_info.add_path_info::<3>(
            PathInfo::PathFixedSizeArray(pre_programmed_distributions_path),
        );

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            pre_programmed_distributions_path_key_info,
            TreeType::NormalTree,
            None, // we will never clean this part up
            apply_tree_type_no_storage_flags,
            transaction,
            &mut None,
            batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution("we can not insert the pre programmed distribution as it already existed, this should have been validated before insertion")));
        }
        let pre_programmed_distributions_path = token_pre_programmed_distributions_path(&token_id);

        self.batch_insert_empty_tree(
            pre_programmed_distributions_path,
            DriveKeyInfo::Key(vec![
                TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
            ]),
            None, // we will never clean this part up
            batch_operations,
            &platform_version.drive,
        )?;

        for (time, distribution) in distribution.distributions() {
            self.batch_insert_empty_sum_tree(
                pre_programmed_distributions_path,
                DriveKeyInfo::Key(time.to_be_bytes().to_vec()),
                None, // we will never clean this part up
                batch_operations,
                &platform_version.drive,
            )?;

            let ms_time_distribution_path = token_ms_timed_distributions_path_vec();

            let time_tree_key_info = DriveKeyInfo::Key(time.to_be_bytes().to_vec());
            let time_tree_reference_path_key_info = time_tree_key_info
                .add_path_info::<0>(PathInfo::PathAsVec(ms_time_distribution_path));

            self.batch_insert_empty_tree_if_not_exists(
                time_tree_reference_path_key_info,
                TreeType::NormalTree,
                Some(&storage_flags),
                apply_tree_type_with_storage_flags,
                transaction,
                &mut None,
                batch_operations,
                &platform_version.drive,
            )?;

            let pre_programmed_at_time_distribution_path =
                token_pre_programmed_at_time_distribution_path_vec(token_id, *time);
            let ms_time_at_time_distribution_path =
                token_ms_timed_at_time_distributions_path_vec(*time);

            for (recipient, amount) in distribution {
                if *amount > i64::MAX as u64 {
                    return Err(Error::Protocol(Box::new(ProtocolError::Overflow(
                        "distribution amount over i64::Max",
                    ))));
                }
                // We use a sum tree to be able to ask "at this time how much was distributed"
                self.batch_insert(
                    PathKeyElementInfo::<0>::PathKeyElement((
                        pre_programmed_at_time_distribution_path.clone(),
                        recipient.to_vec(),
                        Element::new_sum_item(*amount as i64),
                    )),
                    batch_operations,
                    &platform_version.drive,
                )?;

                let distribution_key = TokenDistributionKey {
                    token_id: token_id.into(),
                    recipient: TokenDistributionRecipient::Identity(*recipient),
                    distribution_type: TokenDistributionType::PreProgrammed,
                };

                let serialized_key = distribution_key.serialize_consume_to_bytes()?;

                let remaining_reference = vec![
                    vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
                    token_id.to_vec(),
                    time.to_be_bytes().to_vec(),
                    recipient.to_vec(),
                ];

                let reference =
                    ReferencePathType::UpstreamRootHeightReference(2, remaining_reference);

                // Now we create the reference
                self.batch_insert(
                    PathKeyElementInfo::<0>::PathKeyElement((
                        ms_time_at_time_distribution_path.clone(),
                        serialized_key,
                        Element::new_reference_with_flags(
                            reference,
                            storage_flags.to_some_element_flags(),
                        ),
                    )),
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}
