const { each } = require('lodash');
const logger = require('../logger');

const extendTransactionsWithMetadata = (transactions, transactionsMetadata) => {
  const transactionsWithMetadata = [];
  each(transactions, (transaction) => {
    const { hash } = transaction;
    if (transactionsMetadata[hash]) {
      const transactionMetadata = transactionsMetadata[hash];
      transactionsWithMetadata.push([transaction, transactionMetadata]);
    } else {
      logger.silly(`Unable to find metadata for ${hash}`);
      transactionsWithMetadata.push([transaction, {
        blockHash: null,
        height: -1,
        instantLocked: null,
        chainLocked: false,
      }]);
    }
  });
  return transactionsWithMetadata;
};
module.exports = extendTransactionsWithMetadata;
