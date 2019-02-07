const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../utils');
const {
  ValidTransportLayerRequired,
  InvalidRawTransaction,
} = require('../errors/index');
const EVENTS = require('../EVENTS');
/**
 * Broadcast a Transaction to the transport layer
 * @param tx {String|Transaction} - A Transaction object or it's hexadecimal representation
 * @param isIs - If the tx is InstantSend tx todo: Should be automatically deducted from the rawtx
 * @return {Promise<*>}
 */
async function broadcastTransaction(rawtx, isIs = false) {
  if (is.type(rawtx, 'Transaction')) return broadcastTransaction.call(this, rawtx.toString());
  if (!this.transport.isValid) throw new ValidTransportLayerRequired('broadcast');

  if (!is.rawtx(rawtx)) throw new InvalidRawTransaction(rawtx);
  const txid = await this.transport.sendRawTransaction(rawtx, isIs);

  if (is.txid(txid)) {
    const {
      inputs, outputs,
    } = new Dashcore.Transaction(rawtx).toObject();

    let totalSatoshis = 0;
    outputs.forEach((out) => {
      totalSatoshis += out.satoshis;
    });

    const affectedTxs = [];
    inputs.forEach((input) => {
      affectedTxs.push(input.prevTxId);
    });

    affectedTxs.forEach((affectedtxid) => {
      const { path, type } = this.storage.searchAddressWithTx(affectedtxid);
      const address = this.storage.store.wallets[this.walletId].addresses[type][path];
      const cleanedUtxos = {};
      Object.keys(address.utxos).forEach((utxoTxId) => {
        const utxo = address.utxos[utxoTxId];
        if (utxo.txid === affectedtxid) {
          totalSatoshis -= utxo.satoshis;
          address.balanceSat -= utxo.satoshis;
        } else {
          cleanedUtxos[utxoTxId] = (utxo);
        }
      });
      address.utxos = cleanedUtxos;
      // this.storage.store.addresses[type][path].fetchedLast = 0;// In order to trigger a refresh
      this.events.emit(EVENTS.UNCONFIRMED_BALANCE_CHANGED, { delta: -totalSatoshis, txid });
    });
  }
  return txid;
}
module.exports = broadcastTransaction;
