use crate::masternode_list_item_helpers::UpdateMasternodeListItem;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash, Txid};
use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
use dpp::bls_signatures::PrivateKey as BlsPrivateKey;
use drive_abci::mimic::test_quorum::TestQuorumInfo;
use rand::prelude::{IteratorRandom, StdRng};
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use strategy_tests::frequency::Frequency;

#[derive(Clone, Debug)]
pub struct GenerateTestMasternodeUpdates<'a> {
    pub start_core_height: u32,
    pub end_core_height: u32,
    pub update_masternode_keys_frequency: &'a Frequency,
    pub update_hpmn_keys_frequency: &'a Frequency,
    pub ban_masternode_frequency: &'a Frequency,
    pub ban_hpmn_frequency: &'a Frequency,
    pub unban_masternode_frequency: &'a Frequency,
    pub unban_hpmn_frequency: &'a Frequency,
    pub change_masternode_ip_frequency: &'a Frequency,
    pub change_hpmn_ip_frequency: &'a Frequency,
    pub change_hpmn_p2p_port_frequency: &'a Frequency,
    pub change_hpmn_http_port_frequency: &'a Frequency,
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

    let mut block_height_to_list_masternode_updates: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_masternode_bans: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_hpmns_updates: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_hpmns_bans: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_masternode_unbans: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_hpmn_unbans: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_masternode_ip_changes: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_hpmn_ip_changes: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_hpmn_p2p_port_changes: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut block_height_to_list_hpmn_http_port_changes: BTreeMap<u32, Vec<u16>> = BTreeMap::new();

    let mut current_masternode_bans: BTreeSet<u16> = BTreeSet::new();
    let mut current_hpmn_bans: BTreeSet<u16> = BTreeSet::new();

    if let Some(GenerateTestMasternodeUpdates {
        start_core_height,
        end_core_height,
        update_masternode_keys_frequency,
        update_hpmn_keys_frequency,
        ban_masternode_frequency,
        ban_hpmn_frequency,
        unban_masternode_frequency,
        unban_hpmn_frequency,
        change_masternode_ip_frequency,
        change_hpmn_ip_frequency,
        change_hpmn_p2p_port_frequency,
        change_hpmn_http_port_frequency,
    }) = updates
    {
        for height in start_core_height..=end_core_height {
            block_height_to_list_masternode_updates.insert(
                height,
                update_masternode_keys_frequency.pick_in_range(rng, 0..masternode_count),
            );
            block_height_to_list_masternode_bans.insert(
                height,
                ban_masternode_frequency.pick_in_range_not_from(
                    rng,
                    0..masternode_count,
                    &current_masternode_bans,
                ),
            );

            let banned_masternodes = &block_height_to_list_masternode_bans[&height];
            current_masternode_bans.extend(banned_masternodes);
            let banned_masternodes = banned_masternodes
                .clone()
                .into_iter()
                .collect::<BTreeSet<u16>>();

            let unbanned_masternodes = unban_masternode_frequency.pick_from_not_in(
                rng,
                &current_masternode_bans,
                &banned_masternodes,
            );
            for masternode in &unbanned_masternodes {
                current_masternode_bans.remove(masternode);
            }
            block_height_to_list_masternode_unbans.insert(height, unbanned_masternodes);

            let unbanned_masternodes =
                unban_masternode_frequency.pick_from(rng, &current_masternode_bans);
            for masternode in &unbanned_masternodes {
                current_masternode_bans.remove(masternode);
            }
            block_height_to_list_masternode_unbans.insert(height, unbanned_masternodes);

            block_height_to_list_hpmns_updates.insert(
                height,
                update_hpmn_keys_frequency.pick_in_range(rng, 0..hpmn_count),
            );

            //hpmn bans
            block_height_to_list_hpmns_bans.insert(
                height,
                ban_hpmn_frequency.pick_in_range_not_from(rng, 0..hpmn_count, &current_hpmn_bans),
            );

            let banned_hpmns = block_height_to_list_hpmns_bans[&height]
                .clone()
                .into_iter()
                .collect::<BTreeSet<u16>>();
            current_hpmn_bans.extend(&banned_hpmns);

            let unbanned_hpmns =
                unban_hpmn_frequency.pick_from_not_in(rng, &current_hpmn_bans, &banned_hpmns);
            for hpmn in &unbanned_hpmns {
                current_hpmn_bans.remove(hpmn);
            }
            block_height_to_list_hpmn_unbans.insert(height, unbanned_hpmns);

            block_height_to_list_masternode_ip_changes.insert(
                height,
                change_masternode_ip_frequency.pick_in_range(rng, 0..masternode_count),
            );
            block_height_to_list_hpmn_ip_changes.insert(
                height,
                change_hpmn_ip_frequency.pick_in_range(rng, 0..hpmn_count),
            );
            block_height_to_list_hpmn_p2p_port_changes.insert(
                height,
                change_hpmn_p2p_port_frequency.pick_in_range(rng, 0..hpmn_count),
            );
            block_height_to_list_hpmn_http_port_changes.insert(
                height,
                change_hpmn_http_port_frequency.pick_in_range(rng, 0..hpmn_count),
            );
        }
    }

    fn invert_btreemap(input: BTreeMap<u32, Vec<u16>>) -> BTreeMap<u16, Vec<u32>> {
        let mut output = BTreeMap::new();

        for (key, values) in input {
            for value in values {
                output.entry(value).or_insert_with(Vec::new).push(key);
            }
        }

        output
    }

    let masternode_number_to_heights_key_updates =
        invert_btreemap(block_height_to_list_masternode_updates);

    let masternode_number_to_heights_bans = invert_btreemap(block_height_to_list_masternode_bans);

    let hpmn_number_to_heights_updates = invert_btreemap(block_height_to_list_hpmns_updates);

    let hpmn_number_to_heights_bans = invert_btreemap(block_height_to_list_hpmns_bans);

    let masternode_number_to_heights_unbans =
        invert_btreemap(block_height_to_list_masternode_unbans);

    let hpmn_number_to_heights_unbans = invert_btreemap(block_height_to_list_hpmn_unbans);

    let masternode_number_to_heights_ip_changes =
        invert_btreemap(block_height_to_list_masternode_ip_changes);

    let hpmn_number_to_heights_ip_changes = invert_btreemap(block_height_to_list_hpmn_ip_changes);

    let hpmn_number_to_heights_p2p_port_changes =
        invert_btreemap(block_height_to_list_hpmn_p2p_port_changes);

    let hpmn_number_to_heights_http_port_changes =
        invert_btreemap(block_height_to_list_hpmn_http_port_changes);

    for i in 0..masternode_count {
        let private_key_operator =
            BlsPrivateKey::generate_dash(rng).expect("expected to generate a private key");
        let pub_key_operator = private_key_operator
            .g1_element()
            .expect("expected to get public key")
            .to_bytes()
            .to_vec();
        let pro_tx_hash = ProTxHash::from_byte_array(rng.gen::<[u8; 32]>());
        let masternode_list_item = MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
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

        struct MasternodeUpdate {
            keys: bool,
            ban: bool,
            try_unban: bool,
            ip: bool,
        }

        let masternode_heights_key_updates = masternode_number_to_heights_key_updates.get(&i);
        let masternode_heights_bans = masternode_number_to_heights_bans.get(&i);
        let masternode_heights_unbans = masternode_number_to_heights_unbans.get(&i);
        let masternode_ip_changes = masternode_number_to_heights_ip_changes.get(&i);

        let mut masternode_updates: BTreeMap<u32, MasternodeUpdate> = BTreeMap::new();

        for &height in masternode_heights_key_updates.unwrap_or(&vec![]) {
            masternode_updates
                .entry(height)
                .or_insert(MasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                })
                .keys = true;
        }

        for &height in masternode_heights_bans.unwrap_or(&vec![]) {
            masternode_updates
                .entry(height)
                .or_insert(MasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                })
                .ban = true;
        }

        for &height in masternode_heights_unbans.unwrap_or(&vec![]) {
            masternode_updates
                .entry(height)
                .or_insert(MasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                })
                .try_unban = true;
        }

        for &height in masternode_ip_changes.unwrap_or(&vec![]) {
            masternode_updates
                .entry(height)
                .or_insert(MasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                })
                .ip = true;
        }

        let masternode_updates = masternode_updates
            .into_iter()
            .map(|(height, update)| {
                let mut masternode_list_item_b = latest_masternode_list_item.clone();
                if update.keys {
                    masternode_list_item_b.random_keys_update(None, rng);
                }
                if update.ban {
                    masternode_list_item_b.state.pose_ban_height = Some(1);
                }
                if update.try_unban {
                    masternode_list_item_b.state.pose_ban_height = None;
                }
                if update.ip {
                    let random_ip = Ipv4Addr::new(
                        rng.gen_range(0..255),
                        rng.gen_range(0..255),
                        rng.gen_range(0..255),
                        rng.gen_range(0..255),
                    );
                    let old_port = masternode_list_item_b.state.service.port();
                    masternode_list_item_b.state.service =
                        SocketAddr::new(IpAddr::V4(random_ip), old_port);
                }

                latest_masternode_list_item = masternode_list_item_b.clone();
                (height, masternode_list_item_b)
            })
            .collect::<BTreeMap<u32, MasternodeListItem>>();

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
            node_type: MasternodeType::Evo,
            pro_tx_hash: ProTxHash::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_hash: Txid::from_byte_array(rng.gen::<[u8; 32]>()),
            collateral_index: 0,
            collateral_address: [0; 20],
            operator_reward: 0.0,
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

        struct HPMasternodeUpdate {
            keys: bool,
            ban: bool,
            try_unban: bool,
            ip: bool,
            p2p_port: bool,
            http_port: bool,
        }

        let hpmn_heights_key_updates = hpmn_number_to_heights_updates.get(&i);
        let hpmn_heights_bans = hpmn_number_to_heights_bans.get(&i);
        let hpmn_heights_unbans = hpmn_number_to_heights_unbans.get(&i);
        let hpmn_ip_changes = hpmn_number_to_heights_ip_changes.get(&i);
        let hpmn_p2p_port_changes = hpmn_number_to_heights_p2p_port_changes.get(&i);
        let hpmn_http_port_changes = hpmn_number_to_heights_http_port_changes.get(&i);

        let mut hpmn_updates: BTreeMap<u32, HPMasternodeUpdate> = BTreeMap::new();

        for &height in hpmn_heights_key_updates.unwrap_or(&vec![]) {
            hpmn_updates
                .entry(height)
                .or_insert(HPMasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                    p2p_port: false,
                    http_port: false,
                })
                .keys = true;
        }

        for &height in hpmn_heights_bans.unwrap_or(&vec![]) {
            hpmn_updates
                .entry(height)
                .or_insert(HPMasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                    p2p_port: false,
                    http_port: false,
                })
                .ban = true;
        }

        for &height in hpmn_heights_unbans.unwrap_or(&vec![]) {
            hpmn_updates
                .entry(height)
                .or_insert(HPMasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                    p2p_port: false,
                    http_port: false,
                })
                .try_unban = true;
        }

        for &height in hpmn_ip_changes.unwrap_or(&vec![]) {
            hpmn_updates
                .entry(height)
                .or_insert(HPMasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                    p2p_port: false,
                    http_port: false,
                })
                .ip = true;
        }

        for &height in hpmn_p2p_port_changes.unwrap_or(&vec![]) {
            hpmn_updates
                .entry(height)
                .or_insert(HPMasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                    p2p_port: false,
                    http_port: false,
                })
                .p2p_port = true;
        }

        for &height in hpmn_http_port_changes.unwrap_or(&vec![]) {
            hpmn_updates
                .entry(height)
                .or_insert(HPMasternodeUpdate {
                    keys: false,
                    ban: false,
                    try_unban: false,
                    ip: false,
                    p2p_port: false,
                    http_port: false,
                })
                .http_port = true;
        }

        let hpmn_updates = hpmn_updates
            .into_iter()
            .map(|(height, update)| {
                let mut hpmn_list_item_b = latest_masternode_list_item.clone();
                if update.keys {
                    hpmn_list_item_b.random_keys_update(None, rng);
                }
                if update.ban {
                    hpmn_list_item_b.state.pose_ban_height = Some(1);
                }
                if update.try_unban {
                    hpmn_list_item_b.state.pose_ban_height = None;
                }
                if update.ip {
                    let random_ip = Ipv4Addr::new(
                        rng.gen_range(0..255),
                        rng.gen_range(0..255),
                        rng.gen_range(0..255),
                        rng.gen_range(0..255),
                    );
                    let old_port = hpmn_list_item_b.state.service.port();
                    hpmn_list_item_b.state.service =
                        SocketAddr::new(IpAddr::V4(random_ip), old_port);
                }
                if update.p2p_port {
                    hpmn_list_item_b
                        .state
                        .platform_p2p_port
                        .as_mut()
                        .map(|port| *port += 1);
                }
                if update.http_port {
                    hpmn_list_item_b
                        .state
                        .platform_http_port
                        .as_mut()
                        .map(|port| *port += 1);
                }

                latest_masternode_list_item = hpmn_list_item_b.clone();
                (height, hpmn_list_item_b)
            })
            .collect::<BTreeMap<u32, MasternodeListItem>>();

        let proposer_with_update = MasternodeListItemWithUpdates {
            masternode: masternode_list_item,
            updates: hpmn_updates,
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
            let quorum_hash: QuorumHash = QuorumHash::from_byte_array(rng.gen());
            let validator_pro_tx_hashes = proposers
                .clone()
                .filter(|m| m.node_type == MasternodeType::Evo)
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

#[derive(Clone, Debug, PartialEq)]
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
        let closest_height = self.updates.range(..=height).next_back().map(|(k, _)| *k);

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
            update_masternode_keys_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            update_hpmn_keys_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            ban_masternode_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            ban_hpmn_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            unban_masternode_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            unban_hpmn_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            change_masternode_ip_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            change_hpmn_ip_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            change_hpmn_p2p_port_frequency: &Frequency {
                times_per_block_range: Range { start: 1, end: 3 },
                chance_per_block: Some(0.5),
            },
            change_hpmn_http_port_frequency: &Frequency {
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
                update_masternode_keys_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                update_hpmn_keys_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                ban_masternode_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                ban_hpmn_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                unban_masternode_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                unban_hpmn_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                change_masternode_ip_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                change_hpmn_ip_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                change_hpmn_p2p_port_frequency: &Frequency {
                    times_per_block_range: Range { start: 1, end: 3 },
                    chance_per_block: Some(0.5),
                },
                change_hpmn_http_port_frequency: &Frequency {
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
