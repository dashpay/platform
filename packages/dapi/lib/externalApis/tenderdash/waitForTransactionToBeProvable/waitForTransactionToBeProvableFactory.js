const TransactionWaitPeriodExceededError = require('../../../errors/TransactionWaitPeriodExceededError');
const logger = require('../../../logger');

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
   * @param {number} timeout - timeout to reject after
   * @return {Promise<TransactionOkResult|TransactionErrorResult>}
   */
  function waitForTransactionToBeProvable(blockchainListener, hashString, timeout) {
    const requestLogger = logger.child({
      endpoint: 'waitForStateTransitionResult',
      hash: hashString,
    });

    const {
      promise: waitForTransactionResultPromise,
      detach: detachTransactionResult,
    } = waitForTransactionResult(blockchainListener, hashString, requestLogger);

    const existingTransactionResultPromise = getExistingTransactionResult(hashString);

    const transactionResultPromise = Promise.race([
      // Try to fetch existing tx result
      existingTransactionResultPromise.then((result) => {
        // Do not wait for upcoming result if existing is present
        detachTransactionResult();

        requestLogger.debug('sent existing transition result');

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

    let timeoutId;
    return Promise.race([
      // Get transaction result
      transactionResultPromise.finally(() => clearTimeout(timeoutId)),

      // Throw an error when wait period exceeded
      new Promise((resolve, reject) => {
        timeoutId = setTimeout(() => {
          // Detaching handlers
          detachTransactionResult();

          requestLogger.debug(`request is timed out after ${timeout} ms`);

          reject(new TransactionWaitPeriodExceededError(hashString));
        }, timeout);
      }),
    ]);
  }

  return waitForTransactionToBeProvable;
}

module.exports = waitForTransactionToBeProvableFactory;
