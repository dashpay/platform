const TransactionWaitPeriodExceededError = require('../../../errors/TransactionWaitPeriodExceededError');

/**
 * @param {waitForTransactionResult} waitForTransactionResult
 * @param {getExistingTransactionResult} getExistingTransactionResult
 * @return {waitForTransactionToBeProvable}
 */
function waitForTransactionToBeProvableFactory(
  waitForTransactionResult,
  getExistingTransactionResult,
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
        if (error.code === -32603 && error.data.startsWith(`tx (${hashString}) not found`)) {
          return new Promise(() => {});
        }

        return Promise.reject(error);
      }),

      // Wait for upcoming results if transaction is not executed yet and result is not present
      waitForTransactionResultPromise,
    ]);

    return Promise.race([
      // Get transaction result
      transactionResultPromise,

      // Throw an error when wait period exceeded
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
