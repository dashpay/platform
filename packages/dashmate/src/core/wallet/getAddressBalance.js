const { toDash } = require('../../util/satoshiConverter');

/**
 * Get balance of the address
 *
 * @typedef {getAddressBalance}
 * @param {CoreService} coreService
 * @param {string} address
 * @return {Promise<number>}
 */
async function getAddressBalance(coreService, address) {
  const { result: { balance } } = await coreService.getRpcClient().getAddressBalance({
    addresses: [address],
  });

  return toDash(balance);
}

module.exports = getAddressBalance;
