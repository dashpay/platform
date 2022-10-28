const InternalAbciError = require('./InternalAbciError');
const AbstractAbciError = require('./AbstractAbciError');

/**
 * @typedef wrapInDeliverTxResult
 *
 * @param {Function} method
 *
 * @return {Function}
 */
function wrapInDeliverTxResult(method) {
  /**
   * @param {*[]} args
   */
  async function methodErrorHandler(...args) {
    try {
      return await method(...args);
    } catch (e) {
      let error = e;

      // Wrap all non ABCI errors to an internal ABCI error
      if (!(e instanceof AbstractAbciError)) {
        error = new InternalAbciError(e);
      }

      return {
        txResult: error.getAbciResponse(),
        actualProcessingFee: 0,
        actualStorageFee: 0,
      };
    }
  }

  return methodErrorHandler;
}

module.exports = wrapInDeliverTxResult;
