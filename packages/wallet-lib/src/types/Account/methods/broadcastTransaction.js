const Dashcore = require('@dashevo/dashcore-lib');
const { is } = require('../../../utils');
const {
  ValidTransportLayerRequired,
  InvalidRawTransaction,
  InvalidDashcoreTransaction,
} = require('../../../errors');
const EVENTS = require('../../../EVENTS');
const MempoolPropagationTimeoutError = require('../../../errors/MempoolPropagationTimeoutError');
const logger = require('../../../logger');
const sleep = require('../../../utils/sleep');

const MEMPOOL_PROPAGATION_TIMEOUT = 360000;

const MAX_RETRY_ATTEMPTS = 10;
const RETRY_TIMEOUT = 500;

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
 * @param {Number} [options.mempoolPropagationTimeout=60000] - Time to wait for mempool propagation
 * @return {Promise<string>}
 */
async function broadcastTransaction(transaction, options = {
  mempoolPropagationTimeout: MEMPOOL_PROPAGATION_TIMEOUT,
}) {
  let rejectTimeout;
  let cancelMempoolSubscription;

  const mempoolPropagationPromise = new Promise((resolve) => {
    const listener = ({ payload }) => {
      // TODO: consider reworking to use inputs/outputs comparison
      // to ensure that TX malleability is not a problem
      // https://dashcore.readme.io/v18.0.0/docs/core-guide-transactions-transaction-malleability
      if (payload.transaction.hash === transaction.hash) {
        logger.debug(`broadcastTransaction - received from mempool TX "${transaction.hash}"`);
        clearTimeout(rejectTimeout);
        resolve();
      }
    };
    // TODO: change to FETCHED_UNCONFIRMED_TRANSACTION once this event is restored
    this.once(EVENTS.FETCHED_CONFIRMED_TRANSACTION, listener);
    cancelMempoolSubscription = () => {
      logger.debug(`broadcastTransaction - canceled mempool subscription for TX "${transaction.hash}"`);
      this.removeListener(EVENTS.FETCHED_CONFIRMED_TRANSACTION, listener);
    };
  });

  const rejectPromise = new Promise((_, reject) => {
    rejectTimeout = setTimeout(() => {
      reject(new MempoolPropagationTimeoutError(transaction.hash));
    }, options.mempoolPropagationTimeout);
  });

  logger.debug(`broadcastTransaction - subscribe to mempool for TX "${transaction.hash}"`);
  const mempoolPropagationRace = Promise.race([
    mempoolPropagationPromise, rejectPromise,
  ]);

  try {
    await Promise.all([
      mempoolPropagationRace,
      _broadcastTransaction.call(this, transaction, options).then((hash) => {
        logger.debug(`broadcastTransaction - broadcasted TX "${hash}"`);
      }),
    ]);
  } catch (error) {
    cancelMempoolSubscription();

    if (error.message === 'invalid transaction: Missing inputs') {
      if (this.broadcastRetryAttempts === MAX_RETRY_ATTEMPTS) {
        throw error;
      }

      this.broadcastRetryAttempts += 1;
      await sleep(RETRY_TIMEOUT);
      await broadcastTransaction.call(this, transaction, options);
      this.broadcastRetryAttempts = 0;

      return transaction.hash;
    }

    throw error;
  }

  return transaction.hash;
}

module.exports = broadcastTransaction;
