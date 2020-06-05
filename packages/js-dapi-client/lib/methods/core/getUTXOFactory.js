/**
 * @param {JsonRpcTransport} jsonRpcTransport
 * @returns {getUTXO}
 */
function getUTXOFactory(jsonRpcTransport) {
  /**
   * Returns UTXO for a given address or multiple addresses (max result 1000)
   *
   * @typedef {getUTXO}
   * @param {string|string[]} address or array of addresses
   * @param {DAPIClientOptions & getUTXOOptions} [options]
   * @returns {Promise<object>} - Object with pagination info and array of unspent outputs
   */
  function getUTXO(address, options = {}) {
    return jsonRpcTransport.request(
      'getUTXO',
      {
        address,
        from: options.from,
        to: options.to,
        fromHeight: options.fromHeight,
        toHeight: options.toHeight,
      },
      options,
    );
  }

  return getUTXO;
}

/**
 * @typedef {object} getUTXOOptions
 * @property {number} [from] - start of range in the ordered list of latest UTXO (optional)
 * @property {number} [to] - end of range in the ordered list of latest UTXO (optional)
 * @property {number} [fromHeight] - which height to start from (optional, overriding from/to)
 * @property {number} [toHeight] - on which height to end (optional, overriding from/to)
 */

module.exports = getUTXOFactory;
