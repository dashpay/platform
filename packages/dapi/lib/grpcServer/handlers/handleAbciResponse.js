const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
      DeadlineExceededGrpcError,
      ResourceExhaustedGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @typedef handleAbciResponse
 * @param {{code: number, log: string}} response
 */
function handleAbciResponse(response) {
  if (response.code === undefined || response.code === 0) {
    return;
  }

  const { error: { message, data } } = JSON.parse(response.log);

  switch (response.code) {
    case 6: // MEMORY_LIMIT_EXCEEDED
      throw new ResourceExhaustedGrpcError(message, data);
    case 5: // EXECUTION_TIMED_OUT
      throw new DeadlineExceededGrpcError(message, data);
    case 2: // INVALID_ARGUMENT
      throw new InvalidArgumentGrpcError(message, data);
    case 1: // INTERNAL
    default: {
      const e = new Error(message);

      throw new InternalGrpcError(e, data);
    }
  }
}

module.exports = handleAbciResponse;
