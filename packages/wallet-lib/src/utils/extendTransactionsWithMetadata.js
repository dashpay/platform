const { each } = require('lodash');

const extendTransactionsWithMetadata = (transactions, transactionsMetadata) => {
  const transactionsWithMetadata = [];
  each(transactions, (transaction) => {
    const { hash } = transaction;
    if (!transactionsMetadata[hash]) throw new Error(`Unable to find metadata for ${hash}`);
    const transactionMetadata = transactionsMetadata[hash];
    transactionsWithMetadata.push([transaction, transactionMetadata]);
  });
  return transactionsWithMetadata;
};
module.exports = extendTransactionsWithMetadata;
