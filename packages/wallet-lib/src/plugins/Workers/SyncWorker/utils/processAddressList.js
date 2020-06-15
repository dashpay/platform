const _ = require('lodash');
const fetchAddressTransactions = require('./fetchAddressTransactions');
const TransactionOrderer = require('./TransactionOrderer/TransactionOrderer');

module.exports = async function processAddressList(addressList) {
  const { transporter, storage } = this;

  const boundFetchAddressTransactions = _.bind(fetchAddressTransactions, null, _, transporter);
  const transactionPromises = addressList.map(boundFetchAddressTransactions);

  const transactionsByAddresses = await Promise.all(transactionPromises);

  const transactions = _.flatten(transactionsByAddresses);

  const ordered = new TransactionOrderer();

  transactions.forEach((tx) => ordered.insert(tx));

  const boundImportTransaction = _.bind(storage.importTransaction, storage, _, transporter);
  const importPromises = ordered.transactions.map(boundImportTransaction);

  await Promise.all(importPromises);
};
