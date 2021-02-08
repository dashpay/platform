const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
      DeadlineExceededGrpcError,
      ResourceExhaustedGrpcError,
      NotFoundGrpcError,
      FailedPreconditionGrpcError,
      UnavailableGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @typedef handleAbciResponseError
 * @param {AbciResponseError} error
 */
function handleAbciResponseError(error) {
  const code = error.getErrorCode();
  const message = error.getMessage();
  const data = error.getData();

  switch (code) {
    case 7: // UNAVAILABLE
      throw new UnavailableGrpcError(message, data);
    case 6: // MEMORY_LIMIT_EXCEEDED
      throw new ResourceExhaustedGrpcError(message, data);
    case 5: // EXECUTION_TIMED_OUT
      throw new DeadlineExceededGrpcError(message, data);
    case 4: // INSUFFICIENT_FUNDS
      throw new FailedPreconditionGrpcError(message, data);
    case 3: // NOT_FOUND
      throw new NotFoundGrpcError(message, data);
    case 2: // INVALID_ARGUMENT
      throw new InvalidArgumentGrpcError(message, data);
    case 1: // INTERNAL
    default: {
      if (data && data.toString() === 'Timed out waiting for tx to be included in a block') {
        throw new DeadlineExceededGrpcError(data.toString(), {});
      }

      const e = new Error(message);

      throw new InternalGrpcError(e, data);
    }
  }
}

module.exports = handleAbciResponseError;
