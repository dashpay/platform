mod create_operator_identity;
mod create_owner_identity;
mod create_voter_identity;
mod disable_identity_keys;
mod get_operator_identifier;
mod get_operator_identity_keys;
mod get_owner_identity_key;
mod get_voter_identifier;
mod get_voter_identity_key;
mod update_masternode_identities;
mod update_operator_identity;
mod update_owner_withdrawal_address;
mod update_voter_identity;

//
//
// #[cfg(test)]
// mod tests {
//     use crate::config::PlatformConfig;
//     use crate::test::helpers::setup::TestPlatformBuilder;
//     use dashcore_rpc::dashcore::ProTxHash;
//     use dashcore_rpc::dashcore_rpc_json::MasternodeListDiffWithMasternodes;
//     use dashcore_rpc::json::MasternodeType::Regular;
//     use dashcore_rpc::json::{DMNState, MasternodeListItem};
//     use std::net::SocketAddr;
//     use std::str::FromStr;
//     use crate::platform_types::platform::Platform;
//
//     // thinking of creating a function that returns identity creation instructions based on the masternode list diff
//     // this way I can confirm that it is doing things correctly on the test level
//     // maybe two functions, 1 for the creation, another for update and another for deletion
//     // but don't think this is the best approach as the list might be very long and we don't want to
//     // store too much information in ram
//     // what should the result of an update function look like?
//     // it should return the key id's to disable and the new set of public keys to add.
//     // alright, let's focus on creation first
//     // we need to pass it the list of added master nodes
//     // we run into the batching problem with that, what we really want is a function that takes
//     // a sinlge masternode list item and then returns the correct identity.
//     // update also works for a very specific identity, hence we are testing on the specific identity level
//     // so create_owner_id ...
//     // update_owner_id ...
//     // we currently have the creation function, but it needs the identifier, is this the case anymore?
//     // we needed to remove the identifier because we had to retrieve before we knew if it was an update or not
//     // but this is no longer the case, so we can just combine it into one step
//
//     fn get_masternode_list_diff() -> MasternodeListDiffWithMasternodes {
//         // TODO: eventually generate this from json
//         MasternodeListDiffWithMasternodes {
//             base_height: 850000,
//             block_height: 867165,
//             added_mns: vec![MasternodeListItem {
//                 node_type: Regular,
//                 pro_tx_hash: ProTxHash::from_str(
//                     "1628e387a7badd30fd4ee391ae0cab7e3bc84e792126c6b7cccd99257dad741d",
//                 )
//                 .expect("expected pro_tx_hash"),
//                 collateral_hash: hex::decode(
//                     "4fde102b0c14c50d58d01cc7a53f9a73ae8283dcfe3f13685682ac6dd93f6210",
//                 )
//                 .unwrap()
//                 .try_into()
//                 .unwrap(),
//                 collateral_index: 1,
//                 collateral_address: [],
//                 operator_reward: 0,
//                 state: DMNState {
//                     service: SocketAddr::from_str("1.2.3.4:1234").unwrap(),
//                     registered_height: 0,
//                     pose_revived_height: 0,
//                     pose_ban_height: 850091,
//                     revocation_reason: 0,
//                     owner_address: [0; 20],
//                     voting_address: [0; 20],
//                     payout_address: [0; 20],
//                     pub_key_operator: [0; 48].to_vec(),
//                     operator_payout_address: None,
//                     platform_node_id: None,
//                     platform_p2p_port: None,
//                     platform_http_port: None,
//                 },
//             }],
//             updated_mns: vec![],
//             removed_mns: vec![],
//         }
//     }
//
//     #[test]
//     fn test_owner_identity() {
//         // todo: get rid of the multiple configs
//         let config = PlatformConfig {
//             verify_sum_trees: true,
//             quorum_size: 100,
//             validator_set_quorum_rotation_block_count: 25,
//             block_spacing_ms: 3000,
//             ..Default::default()
//         };
//         let mut platform = TestPlatformBuilder::new()
//             .with_config(config.clone())
//             .build_with_mock_rpc();
//
//         let mn_diff = get_masternode_list_diff();
//         let added_mn_one = &mn_diff.added_mns[0];
//         let owner_identity = platform.create_owner_identity(added_mn_one).unwrap();
//
//         dbg!(owner_identity);
//         // TODO: perform proper assertions when you have correct data
//         //  just adding this test to guide development and make sure things
//         //  are semi working
//     }
//
//     #[test]
//     fn test_voting_identity() {
//         let config = PlatformConfig {
//             verify_sum_trees: true,
//             quorum_size: 100,
//             validator_set_quorum_rotation_block_count: 25,
//             block_spacing_ms: 3000,
//             ..Default::default()
//         };
//
//         let mn_diff = get_masternode_list_diff();
//         let added_mn_one = &mn_diff.added_mns[0];
//         let voter_identity = Platform::create_voter_identity_from_masternode_list_item(added_mn_one, pla).unwrap();
//
//         dbg!(voter_identity);
//     }
//
//     #[test]
//     fn test_operator_identity() {
//         let config = PlatformConfig {
//             verify_sum_trees: true,
//             quorum_size: 100,
//             validator_set_quorum_rotation_block_count: 25,
//             block_spacing_ms: 3000,
//             ..Default::default()
//         };
//         let mut platform = TestPlatformBuilder::new()
//             .with_config(config.clone())
//             .build_with_mock_rpc();
//
//         let mn_diff = get_masternode_list_diff();
//         let added_mn_one = &mn_diff.added_mns[0];
//         let operator_identity = platform.create_operator_identity(added_mn_one).unwrap();
//
//         dbg!(operator_identity);
//     }
//
//     #[test]
//     fn test_update_owner_identity() {}
// }
//
