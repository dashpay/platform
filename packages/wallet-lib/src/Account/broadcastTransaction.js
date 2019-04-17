const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../utils');
const {
  ValidTransportLayerRequired,
  InvalidRawTransaction,
  InvalidDashcoreTransaction,
} = require('../errors/index');
const EVENTS = require('../EVENTS');

const impactAffectedInputs = function ({
  inputs,
}) {
  const {
    storage, walletId, events,
  } = this;
  // let totalSatoshis = outputs.reduce((ac, cur) => acc + cur.satoshis, 0);
  const affectedTxs = inputs.reduce((acc, curr) => acc.push(curr.prevTxId) && acc, []);

  let sumSpent = 0;
  affectedTxs.forEach((affectedTxId) => {
    const { path, type } = storage.searchAddressWithTx(affectedTxId);

    if (type !== null) {
      const address = storage.store.wallets[walletId].addresses[type][path];
      const cleanedUtxos = {};
      Object.keys(address.utxos).forEach((utxoTxId) => {
        const utxo = address.utxos[utxoTxId];
        if (utxo.txid === affectedTxId) {
          sumSpent += utxo.satoshis;
          address.balanceSat -= utxo.satoshis;
        } else {
          cleanedUtxos[utxoTxId] = (utxo);
        }
      });

      const currentValue = this.getBalance();
      events.emit(EVENTS.UNCONFIRMED_BALANCE_CHANGED, { delta: -sumSpent, currentValue });

      address.utxos = cleanedUtxos;
      // this.storage.store.addresses[type][path].fetchedLast = 0;// In order to trigger a refresh
    }
  });
  return true;
};
/**
 * Broadcast a Transaction to the transport layer
 * @param transaction {Transaction|RawTransaction} - A txobject or it's hexadecimal representation
 * @param isIs - If the tx is InstantSend tx todo: Should be automatically deducted from the rawtx
 * @return {Promise<*>}
 */
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
    console.error(txid, 'is said to not be a txid!');
  }
  // We now need to impact/update our affected inputs
  // so we clear them out from UTXOset.
  const { inputs, outputs } = new Dashcore.Transaction(transaction).toObject();
  impactAffectedInputs.call(this, {
    inputs, outputs, txid,
  });

  return txid;
}
module.exports = broadcastTransaction;
