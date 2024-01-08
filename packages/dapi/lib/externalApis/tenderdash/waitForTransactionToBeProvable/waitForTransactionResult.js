const BlockchainListener = require('../BlockchainListener');
const TransactionErrorResult = require('./transactionResult/TransactionErrorResult');
const TransactionOkResult = require('./transactionResult/TransactionOkResult');

/**
 * @typedef {waitForTransactionResult}
 * @param {BlockchainListener} blockchainListener
 * @param {string} hashString - Transaction hash string
 * @param {Logger} requestLogger
 * @return {{
 *    promise: Promise<TransactionOkResult|TransactionErrorResult>,
 *    detach: Function
 * }}
 */
function waitForTransactionResult(blockchainListener, hashString, requestLogger) {
  const topic = BlockchainListener.getTransactionEventName(hashString);

  let handler;

  const promise = new Promise((resolve) => {
    handler = (data) => {
      const { data: { value: { result: deliverResult, tx, height } } } = data;

      blockchainListener.off(topic, handler);

      const txBuffer = Buffer.from(tx, 'base64');

      let TransactionResultClass = TransactionOkResult;
      let code = 0;
      if (deliverResult && deliverResult.code !== undefined && deliverResult.code !== 0) {
        code = deliverResult.code;
        TransactionResultClass = TransactionErrorResult;
      }

      requestLogger.debug(`sent transition result with code ${code}`);

      resolve(
        new TransactionResultClass(
          deliverResult,
          parseInt(height, 10),
          txBuffer,
        ),
      );
    };

    requestLogger.debug('subscribed to transition result');

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
