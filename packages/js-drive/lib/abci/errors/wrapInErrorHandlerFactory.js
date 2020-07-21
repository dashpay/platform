const {
  common: {
    KVPair,
  },
} = require('abci/types');

const AbciError = require('./AbciError');
const InternalAbciError = require('./InternalAbciError');

/**
 * @param {BaseLogger} logger
 *
 * @return wrapInErrorHandler
 */
function wrapInErrorHandlerFactory(logger) {
  /**
   * Wrap ABCI methods in error handler
   *
   * @typedef wrapInErrorHandler
   *
   * @param {Function} method
   * @param {Object} [options={}]
   * @param {boolean} [options.throwNonABCIErrors=true]
   *
   * @return {Function}
   */
  function wrapInErrorHandler(method, options = {}) {
    // eslint-disable-next-line no-param-reassign
    options = {
      throwNonABCIErrors: true,
      ...options,
    };

    const { throwNonABCIErrors } = options;

    /**
     * @param request
     */
    async function methodErrorHandler(request) {
      try {
        return await method(request);
      } catch (e) {
        let error = e;

        // Wrap all non ABCI errors to an internal ABCI error
        if (!(e instanceof AbciError)) {
          // in special cases (e.g. deliverTx call)
          // we should propagate the error upwards
          // to halt the Drive
          if (throwNonABCIErrors) {
            throw e;
          }

          error = new InternalAbciError(e);
        }

        // Log only internal ABCI errors
        if (error instanceof InternalAbciError) {
          logger.error(error.getError());
        }

        const kvPairTags = Object.entries(error.getTags())
          .map(([key, value]) => new KVPair({ key, value }));

        return {
          code: error.getCode(),
          log: JSON.stringify({
            error: {
              message: error.getMessage(),
              data: error.getData(),
            },
          }),
          tags: kvPairTags,
        };
      }
    }

    return methodErrorHandler;
  }

  return wrapInErrorHandler;
}

module.exports = wrapInErrorHandlerFactory;
