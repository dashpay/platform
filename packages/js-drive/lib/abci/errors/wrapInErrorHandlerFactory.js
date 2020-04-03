const {
  common: {
    KVPair,
  },
} = require('abci/types');

const AbciError = require('./AbciError');
const InternalAbciError = require('./InternalAbciError');

/**
 * @param {Object} logger
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
   * @return {Function}
   */
  function wrapInErrorHandler(method) {
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
