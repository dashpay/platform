const UnavailableGrpcError = require('@dashevo/grpc-common/lib/server/error/UnavailableGrpcError');
const ResourceExhaustedGrpcError = require('@dashevo/grpc-common/lib/server/error/ResourceExhaustedGrpcError');
const RPCError = require('../../rpcServer/RPCError');

/**
 * @param {jaysonClient} rpcClient
 * @return {requestTenderRpc} A function to make RPC requests to Tenderdash.
 */
function requestTenderRpcFactory(rpcClient) {
  /**
   * @function
   * @typedef requestTenderRpc
   * @param {string} uri
   * @param {Object} [params={}]
   * @return {Promise<Object>}
   */
  async function requestTenderRpc(uri, params = {}) {
    let response;
    try {
      response = await rpcClient.request(uri, params);
    } catch (e) {
      if (e.code === 'ECONNRESET' || e.message === 'socket hang up') {
        throw new UnavailableGrpcError('Tenderdash is not available');
      }

      throw new RPCError(
        e.code || -32602,
        `Failed to request ${uri}: ${e.message}`,
        e,
      );
    }

    const { result, error: jsonRpcError } = response;

    if (jsonRpcError) {
      if (typeof jsonRpcError.data === 'string') {
        if (jsonRpcError.data.includes('too_many_resets')) {
          throw new ResourceExhaustedGrpcError('tenderdash is not responding: too many requests');
        }
      }

      throw new RPCError(
        jsonRpcError.code || -32602,
        jsonRpcError.message || 'Internal error',
        jsonRpcError.data,
      );
    }

    return result;
  }

  return requestTenderRpc;
}

module.exports = requestTenderRpcFactory;
