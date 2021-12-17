/**
 * @param {RpcClient}
 * @return {fetchProTxInfo}
 */
function fetchProTxInfoFactory(coreRpcClient) {
  /**
   * @typedef fetchProTxInfo
   *
   * @param {string} proTxHash
   *
   * @returns {Promise<{{ state: { service: string } }}>}
   */
  async function fetchProTxInfo(proTxHash) {
    try {
      const {
        result,
      } = await coreRpcClient.protx(
        'info',
        proTxHash,
      );

      return result;
    } catch (e) {
      // RPC_INVALID_PARAMETER: protx not found
      if (e.code === -8) {
        throw new Error(`Protx with hash ${proTxHash} was not found`);
      }

      throw e;
    }
  }

  return fetchProTxInfo;
}

module.exports = fetchProTxInfoFactory;
