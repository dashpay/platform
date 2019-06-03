const GrpcError = require('./GrpcError');
const InternalError = require('./InternalError');

/**
 * @param {Object} logger
 * @return wrapInErrorHandler
 */
module.exports = function wrapInErrorHandlerFactory(logger) {
  /**
   * Wrap RPC method in error handler
   *
   * @typedef wrapInErrorHandler
   * @param {Function} method RPC method
   * @return {Function}
   */
  function wrapInErrorHandler(method) {
    /**
     * @param {Object} call
     * @param {function(Error, *)} callback
     */
    function rpcMethodErrorHandler(call, callback) {
      try {
        method(call, callback);
      } catch (e) {
        if (e instanceof GrpcError) {
          callback(e, null);
        } else {
          const internalError = new InternalError(e);

          logger.error(e);

          callback(internalError, null);
        }
      }
    }
    return rpcMethodErrorHandler;
  }

  return wrapInErrorHandler;
};
