use crate::BlsPrivateKey;
use dashcore_rpc::json::MasternodeListItem;
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
                    let private_key_operator = BlsPrivateKey::generate_dash(rng)
                        .expect("expected to generate a private key");
                    let pub_key_operator = private_key_operator
                        .g1_element()
                        .expect("expected to get public key")
                        .to_bytes()
                        .to_vec();
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
