/**
 *
 * @param {JsonRpcTransport} jsonRpcTransport
 * @returns {getBestBlockHash}
 */
function getBestBlockHashFactory(jsonRpcTransport) {
  /**
   * Returns block hash of chaintip
   *
   * @typedef {getBestBlockHash}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<string>}
   */
  function getBestBlockHash(options = {}) {
    return jsonRpcTransport.request('getBestBlockHash', {}, options);
  }

  return getBestBlockHash;
}

module.exports = getBestBlockHashFactory;
