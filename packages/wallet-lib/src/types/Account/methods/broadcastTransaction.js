const Dashcore = require('@dashevo/dashcore-lib');
const logger = require('../../../logger');
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
 * @param isIs - If the tx is InstantSend tx todo: Should be automatically deducted from the rawtx
 * @return {Promise<*>}
 */
// FIXME : IsIS needs to be removed.
async function broadcastTransaction(transaction, isIs = false) {
  if (!this.transport.isValid) throw new ValidTransportLayerRequired('broadcast');

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

  const txid = await this.transport.sendRawTransaction(transaction.toString(), isIs);
  if (!is.txid(txid)) {
    logger.error(txid, 'is said to not be a txid!');
  }
  // We now need to impact/update our affected inputs
  // so we clear them out from UTXOset.
  const { inputs } = new Dashcore.Transaction(transaction).toObject();
  impactAffectedInputs.call(this, {
    inputs,
  });

  return txid;
}

module.exports = broadcastTransaction;
