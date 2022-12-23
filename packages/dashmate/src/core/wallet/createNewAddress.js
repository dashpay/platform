/**
 * Create new wallet address
 *
 * @typedef {createNewAddress}
 * @param {CoreService} coreService
 * @return {Promise<{privateKey: *, address: *}>}
 */
async function createNewAddress(coreService) {
  const { result: address } = await coreService.getRpcClient().getWallet('main').getNewAddress();
  const { result: privateKey } = await coreService.getRpcClient().dumpPrivKey(address);

  return {
    address,
    privateKey,
  };
}

module.exports = createNewAddress;
