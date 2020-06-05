/**
 *
 * @param {JsonRpcTransport} jsonRpcTransport
 * @returns {getAddressSummary}
 */
function getAddressSummaryFactory(jsonRpcTransport) {
  /**
   * Returns a summary (balance, txs) for a given address
   *
   * @typedef {getAddressSummary}
   * @param {string|string[]} address or array of addresses
   * @param {DAPIClientOptions & getAddressSummaryOptions} [options]
   * @returns {Promise<object>} - an object with basic address info
   */
  function getAddressSummary(address, options = {}) {
    return jsonRpcTransport.request(
      'getAddressSummary',
      {
        address,
        noTxList: options.noTxList,
        from: options.from,
        to: options.to,
        fromHeight: options.fromHeight,
        toHeight: options.toHeight,
      },
      options,
    );
  }

  return getAddressSummary;
}

/**
 * @typedef {object} getAddressSummaryOptions
 * @property {boolean} [noTxList=false] - true if a list of all txs should NOT be included in result
 * @property {number} [from] - start of range for the tx to be included in the tx list
 * @property {number} [to] - end of range for the tx to be included in the tx list
 * @property {number} [fromHeight] - which height to start from (optional, overriding from/to)
 * @property {number} [toHeight] - on which height to end (optional, overriding from/to)
 */

module.exports = getAddressSummaryFactory;
