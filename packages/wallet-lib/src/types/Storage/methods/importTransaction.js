const { Transaction } = require('@dashevo/dashcore-lib');
const { Output } = Transaction;
const { InvalidDashcoreTransaction } = require('../../../errors');
const { FETCHED_CONFIRMED_TRANSACTION } = require('../../../EVENTS');
/**
 * This method is used to import a transaction in Store.
 * @param transaction - A valid Transaction
 */
const importTransaction = function importTransaction(transaction) {
  if (!(transaction instanceof Transaction)) throw new InvalidDashcoreTransaction(transaction);
  const { store, network, mappedAddress } = this;
  const { transactions } = store;
  const { inputs, outputs } = transaction;

  let hasUpdateStorage = false;
  let outputIndex = -1;
  const processedAddressesForTx = [];

  // If we already had this transaction locally, we won't add it again,
  // but we still need to continue processing it as we might have new
  // address generated (on BIP44 wallets) since the first checkup.
  if (!transactions[transaction.hash]) {
    transactions[transaction.hash] = transaction;
  }

  [...inputs, ...outputs].forEach((element, elementIndex) => {
    const isOutput = (element instanceof Output);
    if (isOutput) outputIndex += 1;

    if (element.script) {
      const address = element.script.toAddress(network).toString();

      if (mappedAddress && mappedAddress[address]) {
        const { path, type, walletId } = mappedAddress[address];
        const addressObject = store.wallets[walletId].addresses[type][path];
        if (!addressObject.used) addressObject.used = true;

        if (elementIndex === 0) {
          // If the transactions has already been processed in a previous insertion,
          // we can skip the processing now
          if (addressObject.transactions.includes(transaction.hash)) {
            // We mark it as already processed
            processedAddressesForTx.push(addressObject.address);
            return;
          }
          addressObject.transactions.push(transaction.hash);
          hasUpdateStorage = true;
        }
        // If mark as already procesed on first run, skipping.
        if (processedAddressesForTx.includes(addressObject.address)) {
          return;
        }

        if (!isOutput) {
          const vin = element;
          const utxoKey = `${vin.prevTxId.toString('hex')}-${vin.outputIndex}`;
          if (addressObject.utxos[utxoKey]) {
            const previousOutput = addressObject.utxos[utxoKey];
            addressObject.balanceSat -= previousOutput.satoshis;
            delete addressObject.utxos[utxoKey];
            hasUpdateStorage = true;
          }
        } else {
          const vout = element;

          const utxoKey = `${transaction.hash}-${outputIndex}`;
          if (!addressObject.utxos[utxoKey]) {
            addressObject.utxos[utxoKey] = vout;
            addressObject.balanceSat += vout.satoshis;
            hasUpdateStorage = true;
          }
        }
      }
    }
  });

  if (hasUpdateStorage) {
    this.lastModified = +new Date();
    // Announce only confirmed transaction imported that are our.
    this.announce(FETCHED_CONFIRMED_TRANSACTION, { transaction });
  }
};
module.exports = importTransaction;
