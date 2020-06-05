/**
 * @param {JsonRpcTransport} jsonRpcTransport
 * @returns {generateToAddress}
 */
function generateToAddressFactory(jsonRpcTransport) {
  /**
   * ONLY FOR TESTING PURPOSES WITH REGTEST. WILL NOT WORK ON TESTNET/LIVENET.
   *
   * @typedef {generateToAddress}
   * @param {number} blocksNumber - Number of blocks to generate
   * @param {string} address - The address that will receive the newly generated Dash
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<string[]>} - block hashes
   */
  function generateToAddress(blocksNumber, address, options = {}) {
    return jsonRpcTransport.request(
      'generateToAddress',
      { blocksNumber, address },
      options,
    );
  }

  return generateToAddress;
}

module.exports = generateToAddressFactory;
