const BlockchainListener = require('../BlockchainListener');
const TransactionErrorResult = require('./transactionResult/TransactionErrorResult');
const TransactionOkResult = require('./transactionResult/TransactionOkResult');

/**
 * @typedef {waitForTransactionResult}
 * @param {BlockchainListener} blockchainListener
 * @param {string} hashString - Transaction hash string
 * @return {{
 *    promise: Promise<TransactionOkResult|TransactionErrorResult>,
 *    detach: Function
 * }}
 */
function waitForTransactionResult(blockchainListener, hashString) {
  const topic = BlockchainListener.getTransactionEventName(hashString);

  let handler;

  const promise = new Promise((resolve, reject) => {
    handler = ({ data: { value: { TxResult: txResult } } }) => {
      blockchainListener.off(topic, handler);

      const { result: deliverResult, tx } = txResult;

      const txBuffer = Buffer.from(tx, 'base64');

      if (deliverResult && deliverResult.code !== undefined && deliverResult.code !== 0) {
        // If a transaction result is error we don't need to wait for next block
        return reject(
          new TransactionErrorResult(deliverResult, txBuffer),
        );
      }

      return resolve(
        new TransactionOkResult(deliverResult, txBuffer),
      );
    };

    blockchainListener.on(topic, handler);
  });

  const detach = () => {
    blockchainListener.off(topic, handler);
  };

  return {
    promise,
    detach,
  };
}

module.exports = waitForTransactionResult;
