const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
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
    case 2:
      throw new InvalidArgumentGrpcError(message, data);
    case 1:
    default: {
      const e = new Error(message);

      throw new InternalGrpcError(e, data);
    }
  }
}

module.exports = handleAbciResponse;
