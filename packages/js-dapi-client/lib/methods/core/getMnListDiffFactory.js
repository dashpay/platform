/**
 * @param {JsonRpcTransport} jsonRpcTransport
 * @returns {getMnListDiff}
 */
function getMnListDiffFactory(jsonRpcTransport) {
  /**
   * Get deterministic masternodelist diff
   *
   * @typedef {getMnListDiff}
   * @param {string} baseBlockHash - hash or height of start block
   * @param {string} blockHash - hash or height of end block
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<object>}
   */
  function getMnListDiff(baseBlockHash, blockHash, options = {}) {
    return jsonRpcTransport.request('getMnListDiff', { baseBlockHash, blockHash }, options);
  }

  return getMnListDiff;
}

module.exports = getMnListDiffFactory;
