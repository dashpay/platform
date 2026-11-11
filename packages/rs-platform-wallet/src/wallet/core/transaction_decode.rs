//! Consensus-decodes raw transaction bytes into the outpoints they spend and
//! the addresses and amounts they pay.

use dashcore::consensus::{deserialize, encode};
use dashcore::{Address, Network, OutPoint, ScriptBuf, Transaction, Txid};

/// A transaction decoded by [`decode_transaction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTransaction {
    pub txid: Txid,
    pub inputs: Vec<DecodedTxInput>,
    pub outputs: Vec<DecodedTxOutput>,
}

/// One decoded transaction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTxInput {
    pub previous_output: OutPoint,
    /// The sender a P2PKH-shaped scriptSig claims, from
    /// [`dashcore::TxIn::claimed_p2pkh_address`]. Nothing verifies it, so use it
    /// for display or as a matching hint only, never for authentication. The
    /// per-output `(address, value)` pair is the attacker-resistant primitive.
    pub sender_address: Option<Address>,
}

/// One decoded transaction output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTxOutput {
    /// Destination for a standard script (P2PKH, P2SH), or `None` otherwise.
    pub address: Option<Address>,
    pub value: u64,
    pub script_pubkey: ScriptBuf,
}

/// Consensus-decodes `tx_bytes`. `network` only selects how addresses render.
///
/// Bytes left over after a valid transaction are an error, not a silent trim.
pub fn decode_transaction(
    tx_bytes: &[u8],
    network: Network,
) -> Result<DecodedTransaction, encode::Error> {
    let tx: Transaction = deserialize(tx_bytes)?;
    let txid = tx.txid();

    let inputs = tx
        .input
        .into_iter()
        .map(|txin| DecodedTxInput {
            sender_address: txin.claimed_p2pkh_address(network),
            previous_output: txin.previous_output,
        })
        .collect();

    let outputs = tx
        .output
        .into_iter()
        .map(|txout| DecodedTxOutput {
            address: Address::from_script(&txout.script_pubkey, network).ok(),
            value: txout.value,
            script_pubkey: txout.script_pubkey,
        })
        .collect();

    Ok(DecodedTransaction {
        txid,
        inputs,
        outputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::p2pkh_spend_tx;
    use dashcore::consensus::serialize;

    fn decode(tx: &Transaction, network: Network) -> DecodedTransaction {
        decode_transaction(&serialize(tx), network).expect("decodes")
    }

    #[test]
    fn should_decode_outputs_with_addresses_and_values() {
        let addr = Address::dummy(Network::Testnet, 7);
        let tx = Transaction::dummy(&addr, 1..2, &[151_072, 20_002]);
        let decoded = decode(&tx, Network::Testnet);

        assert_eq!(decoded.txid, tx.txid());
        assert_eq!(decoded.outputs.len(), 2);
        assert_eq!(decoded.outputs[0].address, Some(addr.clone()));
        assert_eq!(decoded.outputs[0].value, 151_072);
        assert_eq!(decoded.outputs[0].script_pubkey, addr.script_pubkey());
        assert_eq!(decoded.outputs[1].address, Some(addr.clone()));
        assert_eq!(decoded.outputs[1].value, 20_002);

        // The same script renders as a mainnet address on mainnet.
        let mainnet = decode(&tx, Network::Mainnet).outputs[0]
            .address
            .clone()
            .unwrap();
        assert_ne!(mainnet, addr);
        assert!(mainnet.to_string().starts_with('X'), "{mainnet}");

        let (tx, _) = p2pkh_spend_tx();
        let op_return = &decode(&tx, Network::Testnet).outputs[1];
        assert!(op_return.address.is_none());
        assert!(
            !op_return.script_pubkey.is_empty(),
            "script bytes still present"
        );
    }

    #[test]
    fn should_recover_the_p2pkh_input_address_from_the_script_sig() {
        let (tx, addr) = p2pkh_spend_tx();
        let decoded = decode(&tx, Network::Testnet);

        assert_eq!(decoded.inputs.len(), 1);
        assert_eq!(
            decoded.inputs[0].previous_output,
            tx.input[0].previous_output
        );
        assert_eq!(decoded.inputs[0].sender_address, Some(addr));
    }

    #[test]
    fn should_reject_bytes_that_are_not_exactly_one_transaction() {
        let (tx, _) = p2pkh_spend_tx();
        let mut trailing = serialize(&tx);
        trailing.extend_from_slice(&[0xDE, 0xAD]);
        assert!(decode_transaction(&trailing, Network::Testnet).is_err());
        assert!(decode_transaction(&[0xFFu8; 16], Network::Testnet).is_err());
        assert!(decode_transaction(&[], Network::Testnet).is_err());
    }
}
