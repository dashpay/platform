use crate::frequency::Frequency;
use crate::masternode_list_item_helpers::UpdateMasternodeListItem;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash, Txid};
use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
use dpp::bls_signatures::PrivateKey as BlsPrivateKey;
use drive_abci::execution::test_quorum::TestQuorumInfo;
use rand::prelude::{IteratorRandom, StdRng};
use rand::Rng;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct GenerateTestMasternodeUpdates<'a> {
    pub start_core_height: u32,
    pub end_core_height: u32,
    pub update_masternode_frequency: &'a Frequency,
    pub update_hpmn_frequency: &'a Frequency,
}

/// Creates a list of test Masternode identities of size `count` with random data
pub fn generate_test_masternodes(
    masternode_count: u16,
    hpmn_count: u16,
    updates: Option<GenerateTestMasternodeUpdates>,
    rng: &mut StdRng,
) -> (
    Vec<MasternodeListItemWithUpdates>,
    Vec<MasternodeListItemWithUpdates>,
) {
    let mut masternodes: Vec<MasternodeListItemWithUpdates> =
        Vec::with_capacity(masternode_count as usize);
    let mut hpmns: Vec<MasternodeListItemWithUpdates> = Vec::with_capacity(hpmn_count as usize);

    let (block_height_to_list_masternode_updates, block_height_to_list_hpmns_updates): (
        Option<BTreeMap<u32, Vec<u16>>>,
        Option<BTreeMap<u32, Vec<u16>>>,
    ) = updates
        .map(
            |GenerateTestMasternodeUpdates {
                 start_core_height,
                 end_core_height,
                 update_masternode_frequency,
                 update_hpmn_frequency,
             }| {
                (start_core_height..=end_core_height)
                    .map(|height| {
                        // we want to pick what nodes will have updated for that block
                        (
                            (
                                height,
                                update_masternode_frequency.pick_in_range(rng, 0..masternode_count),
                            ),
                            (
                                height,
                                update_hpmn_frequency.pick_in_range(rng, 0..hpmn_count),
                            ),
                        )
                    })
                    .unzip()
            },
        )
        .unzip();

    fn invert_btreemap(input: BTreeMap<u32, Vec<u16>>) -> BTreeMap<u16, Vec<u32>> {
        let mut output = BTreeMap::new();

        for (key, values) in input {
            for value in values {
                output.entry(value).or_insert_with(Vec::new).push(key);
            }
        }

        output
    }

    let masternode_number_to_heights_updates = block_height_to_list_masternode_updates
        .map(|block_height_to_list_masternode_updates| {
            invert_btreemap(block_height_to_list_masternode_updates)
        })
        .unwrap_or_default();

    let hpmn_number_to_heights_updates = block_height_to_list_hpmns_updates
        .map(|block_height_to_list_hpmns_updates| {
            invert_btreemap(block_height_to_list_hpmns_updates)
        })
        .unwrap_or_default();

    for i in 0..masternode_count {
        let private_key_operator =
            BlsPrivateKey::generate_dash(rng).expect("expected to generate a private key");
        let pub_key_operator = private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();
        let pro_tx_hash = ProTxHash::from_inner(rng.gen::<[u8; 32]>());
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_inner(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0,
            state: DMNState {
                service: SocketAddr::from_str(format!("1.0.{}.{}:1234", i / 256, i % 256).as_str())
                    .unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator,
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        };

        let mut latest_masternode_list_item = masternode_list_item.clone();

        let masternode_updates = masternode_number_to_heights_updates
            .get(&i)
            .map(|heights| {
                heights
                    .iter()
                    .map(|height| {
                        let mut masternode_list_item_b = latest_masternode_list_item.clone();
                        masternode_list_item_b.random_keys_update(None, rng);
                        latest_masternode_list_item = masternode_list_item_b.clone();
                        (*height, masternode_list_item_b)
                    })
                    .collect::<BTreeMap<u32, MasternodeListItem>>()
            })
            .unwrap_or_default();

        let masternode_with_update = MasternodeListItemWithUpdates {
            masternode: masternode_list_item,
            updates: masternode_updates,
        };

        masternodes.push(masternode_with_update);
    }

    for i in 0..hpmn_count {
        let private_key_operator =
            BlsPrivateKey::generate_dash(rng).expect("expected to generate a private key");
        let pub_key_operator = private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::HighPerformance,
            pro_tx_hash: ProTxHash::from_inner(rng.gen::<[u8; 32]>()),
            collateral_hash: Txid::from_inner(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0,
            state: DMNState {
                service: SocketAddr::from_str(format!("1.1.{}.{}:1234", i / 256, i % 256).as_str())
                    .unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: rng.gen::<[u8; 20]>(),
                voting_address: rng.gen::<[u8; 20]>(),
                payout_address: rng.gen::<[u8; 20]>(),
                pub_key_operator,
                operator_payout_address: None,
                platform_node_id: Some(rng.gen::<[u8; 20]>()),
                platform_p2p_port: Some(3010),
                platform_http_port: Some(8080),
            },
        };

        let mut latest_masternode_list_item = masternode_list_item.clone();

        let masternode_updates = hpmn_number_to_heights_updates
            .get(&i)
            .map(|heights| {
                heights
                    .iter()
                    .map(|height| {
                        let mut masternode_list_item_b = latest_masternode_list_item.clone();
                        masternode_list_item_b.random_keys_update(None, rng);
                        latest_masternode_list_item = masternode_list_item_b.clone();
                        (*height, masternode_list_item_b)
                    })
                    .collect::<BTreeMap<u32, MasternodeListItem>>()
            })
            .unwrap_or_default();

        let proposer_with_update = MasternodeListItemWithUpdates {
            masternode: masternode_list_item,
            updates: masternode_updates,
        };

        hpmns.push(proposer_with_update);
    }

    (masternodes, hpmns)
}

pub fn generate_test_quorums<'a, I>(
    count: usize,
    proposers: I,
    quorum_size: usize,
    rng: &mut StdRng,
) -> BTreeMap<QuorumHash, TestQuorumInfo>
where
    I: Iterator<Item = &'a MasternodeListItem> + Clone,
{
    (0..count)
        .enumerate()
        .map(|(index, _)| {
            let quorum_hash: QuorumHash = QuorumHash::from_inner(rng.gen());
            let validator_pro_tx_hashes = proposers
                .clone()
                .filter(|m| m.node_type == MasternodeType::HighPerformance)
                .choose_multiple(rng, quorum_size)
                .iter()
                .map(|masternode| masternode.pro_tx_hash)
                .collect(); //choose multiple chooses out of order (as we would like)
            (
                quorum_hash,
                TestQuorumInfo::from_quorum_hash_and_pro_tx_hashes(
                    index as u32 * 24,
                    quorum_hash,
                    validator_pro_tx_hashes,
                    rng,
                ),
            )
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MasternodeListItemWithUpdates {
    pub masternode: MasternodeListItem,
    pub updates: BTreeMap<u32, MasternodeListItem>,
}

impl MasternodeListItemWithUpdates {
    pub(crate) fn pro_tx_hash(&self) -> ProTxHash {
        self.masternode.pro_tx_hash
    }

    pub(crate) fn get_state_at_height(&self, height: u32) -> &MasternodeListItem {
        // Find the closest height less than or equal to the given height
        let closest_height = self.updates.range(..=height).rev().next().map(|(k, _)| *k);

        match closest_height {
            Some(h) => &self.updates[&h],
            None => &self.masternode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use std::ops::Range;

    #[test]
    fn verify_generate_test_masternodes_is_deterministic_no_updates() {
        let masternode_count = 100;
        let hpmn_count = 50;
        let mut rng1 = StdRng::seed_from_u64(12345);
        let mut rng2 = StdRng::seed_from_u64(12345);

        let (masternodes1, hpmn1) =
            generate_test_masternodes(masternode_count, hpmn_count, None, &mut rng1);
        let (masternodes2, hpmn2) =
            generate_test_masternodes(masternode_count, hpmn_count, None, &mut rng2);

        assert_eq!(masternodes1, masternodes2);
        assert_eq!(hpmn1, hpmn2);
    }

    #[test]
    fn verify_generate_test_masternodes_is_deterministic_with_updates() {
        let masternode_count = 100;
        let hpmn_count = 50;
        let updates = Some(GenerateTestMasternodeUpdates {
            start_core_height: 10,
            end_core_height: 20,
            update_masternode_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            update_hpmn_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
        });
        let mut rng1 = StdRng::seed_from_u64(12345);
        let mut rng2 = StdRng::seed_from_u64(12345);

        let (masternodes1, hpmn1) =
            generate_test_masternodes(masternode_count, hpmn_count, updates.clone(), &mut rng1);
        let (masternodes2, hpmn2) =
            generate_test_masternodes(masternode_count, hpmn_count, updates.clone(), &mut rng2);

        assert_eq!(masternodes1, masternodes2);
        assert_eq!(hpmn1, hpmn2);
    }

    #[test]
    fn verify_generate_test_masternodes_is_deterministic_no_updates_with_random_seeds() {
        for _ in 0..20 {
            let mut rng = StdRng::seed_from_u64(0);
            let seed = rng.gen();

            let mut rng1 = StdRng::seed_from_u64(seed);
            let mut rng2 = StdRng::seed_from_u64(seed);

            let masternode_count = if rng.gen::<bool>() {
                0
            } else {
                rng.gen_range(25..=100)
            };
            let hpmn_count = rng.gen_range(50..=150);

            let (masternodes1, hpmn1) =
                generate_test_masternodes(masternode_count, hpmn_count, None, &mut rng1);
            let (masternodes2, hpmn2) =
                generate_test_masternodes(masternode_count, hpmn_count, None, &mut rng2);

            assert_eq!(masternodes1, masternodes2);
            assert_eq!(hpmn1, hpmn2);
        }
    }

    #[test]
    fn verify_generate_test_masternodes_is_deterministic_with_updates_with_random_seeds() {
        for _ in 0..20 {
            let mut rng = StdRng::seed_from_u64(0);
            let seed = rng.gen();

            let mut rng1 = StdRng::seed_from_u64(seed);
            let mut rng2 = StdRng::seed_from_u64(seed);

            let masternode_count = if rng.gen::<bool>() {
                0
            } else {
                rng.gen_range(25..=100)
            };
            let hpmn_count = rng.gen_range(50..=150);

            let updates = Some(GenerateTestMasternodeUpdates {
                start_core_height: 10,
                end_core_height: 20,
                update_masternode_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                update_hpmn_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
            });

            let (masternodes1, hpmn1) =
                generate_test_masternodes(masternode_count, hpmn_count, updates.clone(), &mut rng1);
            let (masternodes2, hpmn2) =
                generate_test_masternodes(masternode_count, hpmn_count, updates.clone(), &mut rng2);

            assert_eq!(masternodes1, masternodes2);
            assert_eq!(hpmn1, hpmn2);
        }
    }
}
