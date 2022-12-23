const AbstractAbciError = require('./AbstractAbciError');
const InternalAbciError = require('./InternalAbciError');
const VerboseInternalAbciError = require('./VerboseInternalAbciError');

/**
 * @param {BaseLogger} logger
 * @param {boolean} isProductionEnvironment
 *
 * @return wrapInErrorHandler
 */
function wrapInErrorHandlerFactory(logger, isProductionEnvironment) {
  /**
   * Wrap ABCI methods in error handler
   *
   * @typedef wrapInErrorHandler
   *
   * @param {Function} method
   * @param {Object} [options={}]
   * @param {boolean} [options.respondWithInternalError=false]
   *
   * @return {Function}
   */
  function wrapInErrorHandler(method, options = {}) {
    // eslint-disable-next-line no-param-reassign
    options = {
      respondWithInternalError: false,
      ...options,
    };

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

        // Log only internal ABCI errors
        if (error instanceof InternalAbciError) {
          // in consensus ABCI handlers (blockBegin, deliverTx, blockEnd, commit)
          // we should propagate the error upwards
          // to halt the Drive
          // in order cases like query and checkTx
          // we need to respond with internal errors
          if (!options.respondWithInternalError) {
            throw error.getError();
          }

          const originalError = error.getError();

          (originalError.consensusLogger || logger).error(
            { err: originalError },
            originalError.message,
          );

          if (!isProductionEnvironment) {
            error = new VerboseInternalAbciError(error);
          }
        }

        return error.getAbciResponse();
      }
    }

    return methodErrorHandler;
  }

  return wrapInErrorHandler;
}

module.exports = wrapInErrorHandlerFactory;
