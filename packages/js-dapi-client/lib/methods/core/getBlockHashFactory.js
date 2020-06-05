/**
 * @param {JsonRpcTransport} jsonRpcTransport
 * @returns {getBlockHash}
 */
function getBlockHashFactory(jsonRpcTransport) {
  /**
   * Returns block hash for the given height
   *
   * @typedef {getBlockHash}
   * @param {number} height
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<string>} - block hash
   */
  function getBlockHash(height, options = {}) {
    return jsonRpcTransport.request('getBlockHash', { height }, options);
  }

  return getBlockHash;
}

module.exports = getBlockHashFactory;
