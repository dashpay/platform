const RPCError = require('../../rpcServer/RPCError');

/**
 * @param {RpcClient} rpcClient
 * @return {getConsensusParams}
 */
function getConsensusParamsFactory(rpcClient) {
  /**
   * @typedef getConsensusParams
   * @param {number} [height]
   * @returns {Promise<{
   * block: {
   *   max_bytes: string,
   *   max_gas: string,
   *   time_iota_ms: string
   *  },
   *  evidence: {
   *    max_age_num_blocks: string,
   *    max_age_duration: string,
   *    max_bytes: string,
   *  }
   *  }>}
   */
  async function getConsensusParams(height = undefined) {
    const params = {};

    if (height !== undefined) {
      params.height = height.toString();
    }

    const { result, error } = await rpcClient.request('consensus_params', params);

    // Handle JSON RPC error
    if (error) {
      throw new RPCError(
        error.code || -32602,
        error.message || 'Internal error',
        error.data,
      );
    }

    return {
      block: result.consensus_params.block,
      evidence: result.consensus_params.evidence,
    };
  }

  return getConsensusParams;
}

module.exports = getConsensusParamsFactory;
