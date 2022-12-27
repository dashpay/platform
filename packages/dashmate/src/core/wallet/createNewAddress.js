/**
 * Create new wallet address
 *
 * @typedef {createNewAddress}
 * @param {CoreService} coreService
 * @return {Promise<{privateKey: *, address: *}>}
 */
async function createNewAddress(coreService) {
  const { result: address } = await coreService.getRpcClient().getNewAddress('/wallet/main');

  const { result: privateKey } = await coreService.getRpcClient().dumpPrivKey('/wallet/main', address);

  return {
    address,
    privateKey,
  };
}

module.exports = createNewAddress;
