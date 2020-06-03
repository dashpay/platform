const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');
const {
  ValidTransportLayerRequired,
  InvalidRawTransaction,
  InvalidDashcoreTransaction,
} = require('../../../errors');

function impactAffectedInputs({ inputs }) {
  const {
    storage, walletId,
  } = this;

  // We iterate out input to substract their balance.
  inputs.forEach((input) => {
    const potentiallySelectedAddresses = storage.searchAddressesWithTx(input.prevTxId);
    // Fixme : If you want this check, you will need to modify fixtures of our tests.
    // if (!potentiallySelectedAddresses.found) {
    //   throw new Error('Input is not part of that Wallet.');
    // }
    potentiallySelectedAddresses.results.forEach((potentiallySelectedAddress) => {
      const { type, path } = potentiallySelectedAddress;
      if (potentiallySelectedAddress.utxos[`${input.prevTxId}-${input.outputIndex}`]) {
        const inputUTXO = potentiallySelectedAddress.utxos[`${input.prevTxId}-${input.outputIndex}`];
        const address = storage.store.wallets[walletId].addresses[type][path];
        // Todo: This modify the balance of an address, we need a std method to do that instead.
        address.balanceSat -= inputUTXO.satoshis;
        delete address.utxos[`${input.prevTxId}-${input.outputIndex}`];
      }
    });
  });

  return true;
}

/**
 * Broadcast a Transaction to the transport layer
 * @param transaction {Transaction|RawTransaction} - A txobject or it's hexadecimal representation
 * @return {Promise<*>}
 */
async function broadcastTransaction(transaction) {
  if (!this.transporter.isValid) throw new ValidTransportLayerRequired('broadcast');

  // We still support having in rawtransaction, if this is the case
  // we first need to reform our object
  if (is.string(transaction)) {
    const rawtx = transaction.toString();
    if (!is.rawtx(rawtx)) throw new InvalidRawTransaction(rawtx);
    return broadcastTransaction.call(this, new Dashcore.Transaction(rawtx));
  }

  if (!is.dashcoreTransaction(transaction)) {
    throw new InvalidDashcoreTransaction(transaction);
  }

  if (!transaction.isFullySigned()) {
    throw new Error('Transaction not signed.');
  }
  const txid = await this.transporter.sendTransaction(transaction.toString());
  // We now need to impact/update our affected inputs
  // so we clear them out from UTXOset.
  const { inputs } = new Dashcore.Transaction(transaction).toObject();
  impactAffectedInputs.call(this, {
    inputs,
  });

  return txid;
}

module.exports = broadcastTransaction;
