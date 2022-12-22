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
   *
   * @return {Function}
   */
  function wrapInErrorHandler(method) {
    /**
     * @param {*[]} args
     */
    async function methodErrorHandler(...args) {
      try {
        return await method(...args);
      } catch (e) {
        let abciError = e;

        // Wrap all non ABCI errors to an internal ABCI error
        if (!(e instanceof AbstractAbciError)) {
          abciError = new InternalAbciError(e);
        }

        // Log only internal ABCI errors
        if (abciError instanceof InternalAbciError) {
          const originalError = abciError.getError();

          (originalError.contextLogger || logger).error(
            { err: originalError },
            originalError.message,
          );

          if (!isProductionEnvironment) {
            abciError = new VerboseInternalAbciError(abciError);
          }
        }

        return abciError.getAbciResponse();
      }
    }

    return methodErrorHandler;
  }

  return wrapInErrorHandler;
}

module.exports = wrapInErrorHandlerFactory;
