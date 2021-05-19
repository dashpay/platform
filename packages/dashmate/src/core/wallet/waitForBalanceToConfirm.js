const { PrivateKey } = require('@dashevo/dashcore-lib');
const wait = require('../../util/wait');
const { toDash } = require('../../util/satoshiConverter');
const { NETWORK_LOCAL } = require('../../constants');

/**
 *
 * @typedef waitForBalanceToConfirm
 * @param {CoreService} coreService
 * @param {string} network
 * @param {string} address
 * @param {function(balance: number)} [progressCallback]
 * @returns {Promise<void>}
 */
async function waitForBalanceToConfirm(
  coreService,
  network,
  address,
  progressCallback = () => {},
) {
  const privateKey = new PrivateKey();
  const randomAddress = privateKey.toAddress(network).toString();

  let balanceImmature = 0;
  do {
    if (network === NETWORK_LOCAL) {
      await coreService.getRpcClient().generateToAddress(1, randomAddress, 10000000);
    } else {
      await wait(2000);
    }

    ({ result: { balance_immature: balanceImmature } } = await coreService
      .getRpcClient()
      .getAddressBalance({
        addresses: [address],
      }));

    await progressCallback(toDash(balanceImmature));
  } while (balanceImmature > 0);
}

module.exports = waitForBalanceToConfirm;
