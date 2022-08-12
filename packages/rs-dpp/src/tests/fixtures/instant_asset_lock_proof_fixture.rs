use std::str::FromStr;

use dashcore::secp256k1::rand::thread_rng;
use dashcore::secp256k1::Secp256k1;
use dashcore::{
    secp256k1::SecretKey, InstantLock, Network, OutPoint, PrivateKey, Script, Transaction, TxIn,
    TxOut, Txid,
};

use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::util::vec::hex_to_array;

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn instant_asset_lock_proof_fixture(
    one_time_private_key: Option<PrivateKey>,
) -> AssetLockProof {
    let transaction = instant_asset_lock_proof_transaction_fixture(one_time_private_key);

    let instant_lock = instant_asset_lock_is_lock_fixture(transaction.txid());

    let is_lock_proof = InstantAssetLockProof::new(instant_lock, transaction, 0);

    AssetLockProof::Instant(is_lock_proof)
}

pub fn instant_asset_lock_proof_transaction_fixture(
    one_time_private_key: Option<PrivateKey>,
) -> Transaction {
    let mut rng = thread_rng();
    let secp = Secp256k1::new();

    let private_key_hex = "cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY";
    let private_key = PrivateKey::from_str(private_key_hex).unwrap();
    let public_key = private_key.public_key(&secp);
    let public_key_hash = public_key.pubkey_hash();
    //let from_address = Address::p2pkh(&public_key, Network::Testnet);
    let secret_key = SecretKey::new(&mut rng);
    let one_time_private_key =
        one_time_private_key.unwrap_or_else(|| PrivateKey::new(secret_key, Network::Testnet));
    let one_time_public_key = one_time_private_key.public_key(&secp);

    let txid =
        Txid::from_str("a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458").unwrap();
    let outpoint = OutPoint::new(txid, 0);
    let input = TxIn {
        previous_output: outpoint,
        script_sig: Script::new_p2pkh(&public_key_hash),
        sequence: 0,
        witness: Default::default(),
    };
    let one_time_key_hash = one_time_public_key.pubkey_hash().to_vec();
    let burn_output = TxOut {
        value: 90000,
        script_pubkey: Script::new_op_return(&one_time_key_hash),
    };
    let change_output = TxOut {
        value: 5000,
        script_pubkey: Script::new_p2pkh(&public_key_hash),
    };
    let unrelated_burn_output = TxOut {
        value: 5000,
        script_pubkey: Script::new_op_return(&[1, 2, 3]),
    };
    Transaction {
        version: 0,
        lock_time: 0,
        input: vec![input],
        output: vec![burn_output, change_output, unrelated_burn_output],
    }
}

pub fn instant_asset_lock_is_lock_fixture(tx_id: Txid) -> InstantLock {
    InstantLock {
        version: 1,
        inputs: vec![
            OutPoint { txid: Txid::from_str("6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d").unwrap(), vout: 0 }
        ],
        txid: tx_id,
        cyclehash: hex_to_array::<32>("7c30826123d0f29fe4c4a8895d7ba4eb469b1fafa6ad7b23896a1a591766a536").unwrap(),
        signature: hex_to_array::<96>("8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80").unwrap(),
    }
}
