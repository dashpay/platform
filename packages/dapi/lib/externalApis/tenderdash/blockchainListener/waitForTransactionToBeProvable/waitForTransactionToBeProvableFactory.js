const TransactionWaitPeriodExceededError = require('../../../../errors/TransactionWaitPeriodExceededError');
const TransactionErrorResult = require('./transactionResult/TransactionErrorResult');

/**
 * @param {waitForTransactionResult} waitForTransactionResult
 * @param {waitForTransactionCommitment} waitForTransactionCommitment
 * @return {waitForTransactionToBeProvable}
 */
function waitForTransactionToBeProvableFactory(
  waitForTransactionResult,
  waitForTransactionCommitment,
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
    // Wait for transaction result
    const {
      promise: transactionResultPromise,
      detach: detachTransactionResult,
    } = waitForTransactionResult(blockchainListener, hashString);

    // Wait for transaction is committed to a block and proofs are available
    const {
      promise: transactionCommitmentPromise,
      detach: detachTransactionCommitment,
    } = waitForTransactionCommitment(blockchainListener, hashString);

    return Promise.race([

      // Wait for transaction results and commitment

      Promise.all([
        transactionResultPromise,
        transactionCommitmentPromise,
      ]).then(([transactionResult]) => transactionResult)
        .catch((e) => {
          // Stop waiting for next block and return transaction error result
          if (e instanceof TransactionErrorResult) {
            return Promise.resolve(e);
          }

          return Promise.reject(e);
        }),

      // Throw wait period exceeded error after timeout

      new Promise((resolve, reject) => {
        setTimeout(() => {
          // Detaching handlers
          detachTransactionResult();
          detachTransactionCommitment();

          reject(new TransactionWaitPeriodExceededError(hashString));
        }, timeout);
      }),
    ]);
  }

  return waitForTransactionToBeProvable;
}

module.exports = waitForTransactionToBeProvableFactory;
