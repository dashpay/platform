const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  GetIdentityResponse,
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {jaysonClient} rpcClient
 * @param {handleAbciResponse} handleAbciResponse
 * @returns {getIdentityHandler}
 */
function getIdentityHandlerFactory(rpcClient, handleAbciResponse) {
  /**
   * @typedef getIdentityHandler
   * @param {Object} call
   */
  async function getIdentityHandler(call) {
    const { request } = call;

    const id = request.getId();

    if (!id) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    const path = '/identity';

    const data = Buffer.from(id).toString('hex');

    const { result, error: jsonRpcError } = await rpcClient.request('abci_query', { path, data });

    if (jsonRpcError) {
      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    handleAbciResponse(result.response);

    const { response: { value: identityBase64 } } = result;
    if (!identityBase64) {
      throw new NotFoundGrpcError('Identity not found');
    }

    const response = new GetIdentityResponse();

    response.setIdentity(identityBase64);

    return response;
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
