const {
  server: {
    error: {
      InvalidArgumentGrpcError,
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

    const { result, error: errorMessage } = await rpcClient.request('abci_query', { path, data });

    if (errorMessage) {
      throw new Error(errorMessage);
    }

    handleAbciResponse(result.response);

    const { response: { value: identityBase64 } } = result;

    const response = new GetIdentityResponse();

    response.setIdentity(identityBase64);

    return response;
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
