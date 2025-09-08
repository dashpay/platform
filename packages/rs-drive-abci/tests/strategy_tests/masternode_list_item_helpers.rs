use crate::BlsPrivateKey;
use dpp::bls_signatures::Bls12381G2Impl;
use dpp::dashcore_rpc::json::MasternodeListItem;
use rand::prelude::IteratorRandom;
use rand::rngs::StdRng;
use rand::Rng;

pub trait UpdateMasternodeListItem {
    fn random_keys_update(&mut self, num_fields_to_change: Option<usize>, rng: &mut StdRng);
}

impl UpdateMasternodeListItem for MasternodeListItem {
    fn random_keys_update(&mut self, num_fields_to_change: Option<usize>, rng: &mut StdRng) {
        let mut available_fields: Vec<usize> = (0..8)
            .filter(|&field_idx| match field_idx {
                4 => self.state.operator_payout_address.is_some(),
                5 => self.state.platform_node_id.is_some(),
                6 => self.state.platform_p2p_port.is_some(),
                7 => self.state.platform_http_port.is_some(),
                _ => true,
            })
            .collect();

        let fields_to_change =
            num_fields_to_change.unwrap_or(rng.gen_range(1..=available_fields.len()));

        for _ in 0..fields_to_change {
            let field_idx = available_fields.iter().choose(rng).cloned().unwrap();
            available_fields.retain(|&idx| idx != field_idx);

            match field_idx {
                0 => self.state.owner_address = rng.gen::<[u8; 20]>(),
                1 => self.state.voting_address = rng.gen::<[u8; 20]>(),
                2 => self.state.payout_address = rng.gen::<[u8; 20]>(),
                3 => {
                    let private_key_operator_bytes = bls_signatures::PrivateKey::generate_dash(rng)
                        .expect("expected to generate a private key")
                        .to_bytes()
                        .to_vec();
                    let private_key_operator = BlsPrivateKey::<Bls12381G2Impl>::from_be_bytes(
                        &private_key_operator_bytes.try_into().expect("expected the secret key to be 32 bytes"),
                    )
                    .expect("expected the conversion between bls signatures library and blsful to happen without failing");
                    let pub_key_operator =
                        private_key_operator.public_key().0.to_compressed().to_vec();
                    self.state.pub_key_operator = pub_key_operator;
                }
                4 => {
                    if let Some(ref mut address) = self.state.operator_payout_address {
                        *address = rng.gen::<[u8; 20]>();
                    }
                }
                5 => {
                    if let Some(ref mut address) = self.state.platform_node_id {
                        *address = rng.gen::<[u8; 20]>();
                    }
                }
                6 => {
                    if let Some(ref mut port) = self.state.platform_p2p_port {
                        *port = rng.gen_range(1024..=65535);
                    }
                }
                7 => {
                    if let Some(ref mut port) = self.state.platform_http_port {
                        *port = rng.gen_range(1024..=65535);
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeType};

    use rand::SeedableRng;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn test_random_keys_update_determinism() {
        let mut rng = StdRng::seed_from_u64(0);
        let i = 0;
        let pro_tx_hash = ProTxHash::from_byte_array(rng.gen::<[u8; 32]>());
        let private_key_operator_bytes = bls_signatures::PrivateKey::generate_dash(&mut rng)
            .expect("expected to generate a private key")
            .to_bytes()
            .to_vec();
        let private_key_operator = BlsPrivateKey::<Bls12381G2Impl>::from_be_bytes(
            &private_key_operator_bytes.try_into().expect("expected the secret key to be 32 bytes"),
        )
            .expect("expected the conversion between bls signatures library and blsful to happen without failing");
        let pub_key_operator = private_key_operator.public_key().0.to_compressed().to_vec();
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

        for _ in 0..100 {
            let rng_seed = rng.gen();
            let mut rng1 = StdRng::seed_from_u64(rng_seed);
            let mut rng2 = StdRng::seed_from_u64(rng_seed);

            let mut masternode_list_item1 = masternode_list_item.clone(); // Add a constructor for creating a new instance
            let mut masternode_list_item2 = masternode_list_item.clone();

            masternode_list_item1.random_keys_update(None, &mut rng1);
            masternode_list_item2.random_keys_update(None, &mut rng2);

            assert_eq!(masternode_list_item1, masternode_list_item2);
        }
    }
}
