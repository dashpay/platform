const GrpcError = require('./GrpcError');
const InternalGrpcError = require('./InternalGrpcError');

/**
 * @param {Error} error
 * @param {grpc.ServerWriteableStream} call
 * @param {function(Error, *)} [callback]
 */
function respondWithError(error, call, callback = undefined) {
  if (callback) {
    callback(error, null);
  } else {
    call.destroy(error);
  }
}

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
     * @param {grpc.ServerWriteableStream} call
     * @param {function(Error, *)} [callback]
     */
    async function rpcMethodErrorHandler(call, callback = undefined) {
      try {
        const result = await method(call);

        if (callback) {
          callback(null, result);
        }
      } catch (e) {
        if (e instanceof GrpcError) {
          respondWithError(e, call, callback);
        } else {
          const internalError = new InternalGrpcError(e);

          logger.error(e);

          respondWithError(internalError, call, callback);
        }
      }
    }
    return rpcMethodErrorHandler;
  }

  return wrapInErrorHandler;
};
