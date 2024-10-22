const UnavailableGrpcError = require('@dashevo/grpc-common/lib/server/error/UnavailableGrpcError');
const ResourceExhaustedGrpcError = require('@dashevo/grpc-common/lib/server/error/ResourceExhaustedGrpcError');
const logger = require('../../../../logger');

/**
 * @param {jaysonClient} rpcClient
 * @param {string} uri
 * @param {Object} params
 * @return {Promise<any>}
 */
async function request(rpcClient, uri, params = {}) {
  let response;
  try {
    response = await rpcClient.request(uri, params);
  } catch (e) {
    if (e.message === 'socket hang up') {
      throw new UnavailableGrpcError('Tenderdash is not available');
    }

    e.message = `Failed to fetch cached transaction: ${e.message}`;

    throw e;
  }

  const { result, error: jsonRpcError } = response;

  if (jsonRpcError) {
    if (typeof jsonRpcError.data === 'string') {
      if (jsonRpcError.data.includes('too_many_resets')) {
        throw new ResourceExhaustedGrpcError('tenderdash is not responding: too many requests');
      }
    }

    const error = new Error();
    Object.assign(error, jsonRpcError);

    logger.error(error, `Unexpected JSON RPC error during broadcasting state transition: ${JSON.stringify(jsonRpcError)}`);

    throw error;
  }

  return result;
}

/**
 *
 * @param {jaysonClient} rpcClient
 * @return {fetchCachedStateTransitionResult}
 */
function fetchCachedStateTransitionResultFactory(rpcClient) {
  /**
   * @typedef fetchCachedStateTransitionResult
   * @param {Buffer} stBytes
   * @return {Promise<Object>}
   */
  return async function fetchCachedStateTransitionResult(stBytes) {
    // Subscribing to future result
    const stHash = crypto.createHash('sha256')
      .update(stBytes)
      .digest();

    // Search cached state transition in mempool
    // rpcClient.request('/unconfirmed_txs');

    // Search in blockchain data
    const result = await request(rpcClient, '/tx', { hash: `0x${stHash.toString('hex')}` });


  };
}

module.exports = fetchCachedStateTransitionResultFactory;
