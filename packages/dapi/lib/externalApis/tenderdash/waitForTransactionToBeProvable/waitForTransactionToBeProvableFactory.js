const TransactionWaitPeriodExceededError = require('../../../errors/TransactionWaitPeriodExceededError');
const TransactionOkResult = require('./transactionResult/TransactionOkResult');

/**
 * @param {waitForTransactionResult} waitForTransactionResult
 * @param {getExistingTransactionResult} getExistingTransactionResult
 * @param {waitForHeight} waitForHeight
 * @return {waitForTransactionToBeProvable}
 */
function waitForTransactionToBeProvableFactory(
  waitForTransactionResult,
  getExistingTransactionResult,
  waitForHeight,
) {
  /**
   * Returns result for a transaction or rejects after a timeout
   *
   * @typedef {waitForTransactionToBeProvable}
   * @param {BlockchainListener} blockchainListener
   * @param {string} hashString - transaction hash to resolve data for
   * @param {number} [timeout] - timeout to reject after
   * @return {Promise<TransactionOkResult|TransactionErrorResult>}
   */
  function waitForTransactionToBeProvable(blockchainListener, hashString, timeout = 60000) {
    const {
      promise: waitForTransactionResultPromise,
      detach: detachTransactionResult,
    } = waitForTransactionResult(blockchainListener, hashString);

    const existingTransactionResultPromise = getExistingTransactionResult(hashString);

    const transactionResultPromise = Promise.race([
      // Try to fetch existing tx result
      existingTransactionResultPromise.then((result) => {
        // Do not wait for upcoming result if existing is present
        detachTransactionResult();

        return result;
      }).catch((error) => {
        // Do not resolve promise and wait for results if transaction is not found
        if (error.code === -32603 && error.data === `tx (${hashString}) not found`) {
          return new Promise(() => {});
        }

        return Promise.reject(error);
      }),

      // Wait for upcoming results if transaction result doesn't not exist yet
      waitForTransactionResultPromise,
    ]);

    return Promise.race([
      // Wait for transaction results and commitment
      transactionResultPromise.then(async (result) => {
        if (result instanceof TransactionOkResult) {
          await waitForHeight(result.getHeight() + 1);
        }

        return result;
      }),

      // Throw wait period exceeded error after timeout
      new Promise((resolve, reject) => {
        setTimeout(() => {
          // Detaching handlers
          detachTransactionResult();

          reject(new TransactionWaitPeriodExceededError(hashString));
        }, timeout);
      }),
    ]);
  }

  return waitForTransactionToBeProvable;
}

module.exports = waitForTransactionToBeProvableFactory;
