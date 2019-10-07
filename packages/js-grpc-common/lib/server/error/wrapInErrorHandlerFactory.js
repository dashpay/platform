const GrpcError = require('./GrpcError');
const InternalGrpcError = require('./InternalGrpcError');

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
        let error = e;

        // Wrap all non GRPC errors to an internal GRPC error
        if (!(e instanceof GrpcError)) {
          error = new InternalGrpcError(e);
        }

        // Log only internal GRPC errors
        if (error instanceof InternalGrpcError) {
          logger.error(error.getError());
        }

        if (callback) {
          callback(error, null);
        } else {
          call.destroy(error);
        }
      }
    }
    return rpcMethodErrorHandler;
  }

  return wrapInErrorHandler;
};
