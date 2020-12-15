const {
  tendermint: {
    abci: {
      Event,
      EventAttribute,
    },
  },
} = require('@dashevo/abci/types');

const AbciError = require('./AbciError');
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
   * @param {boolean} [options.throwNonABCIErrors=true]
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

          // in consensus ABCI handlers (blockBegin, deliverTx, blockEnd, commit)
          // we should propagate the error upwards
          // to halt the Drive
          // in order cases like query and checkTx
          // we need to respond with internal errors
          if (!options.respondWithInternalError) {
            throw error.getError();
          }

          if (!isProductionEnvironment) {
            error = new VerboseInternalAbciError(error);
          }
        }

        const events = [];

        const attributes = Object.entries(error.getTags())
          .map(([key, value]) => new EventAttribute({ key, value, index: true }));

        if (attributes.length > 0) {
          events.push(new Event({
            type: 'error',
            attributes,
          }));
        }

        return {
          code: error.getCode(),
          log: JSON.stringify({
            error: {
              message: error.getMessage(),
              data: error.getData(),
            },
          }),
          events,
        };
      }
    }

    return methodErrorHandler;
  }

  return wrapInErrorHandler;
}

module.exports = wrapInErrorHandlerFactory;
