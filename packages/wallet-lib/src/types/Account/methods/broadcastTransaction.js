const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');
const {
  ValidTransportLayerRequired,
  InvalidRawTransaction,
  InvalidDashcoreTransaction,
} = require('../../../errors');

function impactAffectedInputs({ transaction }) {
  const {
    storage, network,
  } = this;

  const { inputs, changeIndex } = transaction.toObject();
  const txid = transaction.hash;

  const addresses = storage.getChainStore(network).getAddresses();
  // We iterate out input to substract their balance.
  inputs.forEach((input) => {
    const potentiallySelectedAddresses = [...addresses]
      .reduce((acc, [address, { transactions }]) => {
        if (transactions.includes(input.prevTxId)) acc.push(address);
        return acc;
      }, []);

    potentiallySelectedAddresses.forEach((potentiallySelectedAddress) => {
      // console.log(addresses.get(pot));
      const addressData = addresses.get(potentiallySelectedAddress);
      if (addressData.utxos[`${input.prevTxId}-${input.outputIndex}`]) {
        const inputUTXO = addressData.utxos[`${input.prevTxId}-${input.outputIndex}`];
        // const address = storage.store.wallets[walletId].addresses[type][path];
        // Todo: This modify the balance of an address, we need a std method to do that instead.
        addressData.balanceSat -= inputUTXO.satoshis;
        delete addressData.utxos[`${input.prevTxId}-${input.outputIndex}`];
      }
    });
  });

  const changeOutput = transaction.getChangeOutput();
  if (changeOutput) {
    const addressString = changeOutput.script.toAddress(network).toString();

    const address = addresses.get(addressString);
    const utxoKey = `${txid}-${changeIndex}`;

    /**
     * In some cases, `Storage#importTransaction` function gets called before the
     * `impactAffectedInputs`and this utxo being written as a confirmed one.
     * Skip creation of the unconfirmed UTXOs for such cases.
     */
    if (!address.utxos[utxoKey]) {
      address.utxos[utxoKey] = new Dashcore.Transaction.UnspentOutput(
        {
          txId: txid,
          vout: changeIndex,
          script: changeOutput.script,
          satoshis: changeOutput.satoshis,
          address: addressString,
        },
      );
      address.unconfirmedBalanceSat = changeOutput.satoshis;
      address.used = true;
    }
  }

  return true;
}

// eslint-disable-next-line no-underscore-dangle
async function _broadcastTransaction(transaction, options = {}) {
  const { network, storage } = this;
  if (!this.transport) throw new ValidTransportLayerRequired('broadcast');

  // We still support having in rawtransaction, if this is the case
  // we first need to reform our object
  if (is.string(transaction)) {
    const rawtx = transaction.toString();
    if (!is.rawtx(rawtx)) throw new InvalidRawTransaction(rawtx);
    return _broadcastTransaction.call(this, new Dashcore.Transaction(rawtx));
  }

  if (!is.dashcoreTransaction(transaction)) {
    throw new InvalidDashcoreTransaction(transaction);
  }

  if (!transaction.isFullySigned()) {
    throw new Error('Transaction not signed.');
  }

  const { minRelay: minRelayFeeRate } = storage.getChainStore(network).state.fees;

  // eslint-disable-next-line no-underscore-dangle
  const estimateKbSize = transaction._estimateSize() / 1000;
  const minRelayFee = Math.ceil(estimateKbSize * minRelayFeeRate);

  if (minRelayFee > transaction.getFee() && !options.skipFeeValidation) {
    throw new Error(`Expected minimum fee for transaction ${minRelayFee}. Current: ${transaction.getFee()}`);
  }
  const serializedTransaction = transaction.toString();

  const txid = await this.transport.sendTransaction(serializedTransaction);

  // We now need to impact/update our affected inputs
  // so we clear them out from UTXOset.
  impactAffectedInputs.call(this, {
    transaction,
  });
  return txid;
}

/**
 * Broadcast a Transaction to the transport layer
 * @param {Transaction|RawTransaction} transaction - A txobject or it's hexadecimal representation
 * @param {Object} [options]
 * @param {Boolean} [options.skipFeeValidation=false] - Allow to skip fee validation
 * @return {Promise<transactionId>}
 */
async function broadcastTransaction(transaction, options = {}) {
  if (!this.txISLockListener) {
    const txId = await _broadcastTransaction.call(this, transaction, options);

    this.txISLockListener = new Promise((resolve) => {
      this.subscribeToTransactionInstantLock.call(this, txId, () => {
        this.txISLockListener = null;
        resolve();
      });

      // TODO: Also subscribe to FETCHED_CONFIRMED_TRANSACTION
      // to use as a fallback to resolve the promise
      // (blocked by https://github.com/dashevo/wallet-lib/pull/340)
    });

    return txId;
  }

  await this.txISLockListener;

  return broadcastTransaction.call(this, transaction, options);
}

module.exports = broadcastTransaction;
